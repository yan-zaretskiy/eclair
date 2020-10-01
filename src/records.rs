//! ## Eclipse Binary Format
//!
//! Eclipse binary output files are typically written using the **big-endian** ordering. A single
//! binary block is written in the Fortran style, where the binary payload is surrounded by matching
//! leading and tailing record markers. The marker is a 4-byte integer (`int32`), equal to the byte
//! length of the data. For instance, if we have a binary block of 200 bytes, it will be written to
//! disk as:
//!
//! ```
//! +-------+----------+-------+
//! |  200  |   data   |  200  |
//! +-------+----------+-------+
//! ```
//!
//! A single binary record consists of a header and a body sections, written as two or more
//! individual binary blocks. This is because the body section can be subdivided into multiple
//! sub-blocks if the number of elements in it exceeds either 1000 (for non-string elements) or
//! 105 (for 8-character long string elements).
//!
//! The header section contains:
//!
//! 1. An 8-character space-padded string identifier;
//! 2. A 4-byte integer for the number of elements in the block;
//! 3. A 4-character keyword defining the type of data;
//!
//! Possible data type values are:
//!
//! - `INTE` - 4-byte signed integers;
//! - `REAL` - single precision 4-byte floating point numbers;
//! - `DOUB` - double precision 8-byte floating point numbers;
//! - `LOGI` - 4-byte logicals;
//! - `CHAR` - characters (as 8-character words);
//! - `C0nn` - CHARACTER*nn strings (e.g. C042 means a 42-character string);
//! - `MESS` - an indicator type, it contains no data, so its length is zero;
//!
//! ### Example
//!
//! Here is how a data array is laid out on disk if it is called `FOO` and is 1500 integers long:
//!
//! ```
//! +------+------------------+------+------+-----------------+------+------+--------------------+------+
//! | head | NAME LENGTH TYPE | tail | head | VAL1 .. VAL1000 | tail | head | VAL1001 .. VAL1500 | tail |
//! +------+------------------+------+------+-----------------+------+------+--------------------+------+
//! |  16  | FOO  1500   INTE |  16  | 4000 |    1 ..    1000 | 4000 | 2000 |    1001 ..    1500 | 2000 |
//! +------+------------------+------+------+-----------------+------+------+--------------------+------+
//! ```
//!
//! Note that `FOO` will be padded with spaces to be exactly 8 characters long.

use crate::{binary_parsing as bp, error::EclairError, FlexString, Result, FIXED_STRING_LENGTH};

use std::fmt::{Display, Formatter};
use std::{mem, str};

/// The maximum allowed number of elements per binary data sub-block is fixed upfront.
const NUM_BLOCK_LENGTH: usize = 1000;
const STR_BLOCK_LENGTH: usize = 105;

/// A body of data in an Eclipse binary record.
#[derive(Debug, PartialEq)]
pub enum RecordData {
    Int(Vec<i32>),
    Bool(Vec<i32>),
    Chars(Vec<FlexString>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Message,
}

/// Helper enum for type validation.
#[derive(Debug, PartialEq)]
pub enum RecordDataKind {
    Int,
    Bool,
    Chars,
    F32,
    F64,
    Message,
}

impl Display for RecordDataKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use RecordDataKind::*;
        match self {
            Int => write!(f, "INTE"),
            Bool => write!(f, "LOGI"),
            Chars => write!(f, "CHAR"),
            F32 => write!(f, "REAL"),
            F64 => write!(f, "DOUB"),
            Message => write!(f, "MESS"),
        }
    }
}

impl RecordData {
    /// The type mnemonic for the underlying data.
    pub fn kind(&self) -> &RecordDataKind {
        use RecordData::*;
        match self {
            Int(_) => &RecordDataKind::Int,
            Bool(_) => &RecordDataKind::Bool,
            Chars(_) => &RecordDataKind::Chars,
            F32(_) => &RecordDataKind::F32,
            F64(_) => &RecordDataKind::F64,
            Message => &RecordDataKind::Message,
        }
    }

    /// The type mnemonic for the underlying data.
    pub fn kind_string(&self) -> String {
        self.kind().to_string()
    }

    /// Push input bytes into the binary data instance interpreting them as necessary.
    fn push(&mut self, input: &[u8], element_size: usize) {
        // FIXME: How to best validate input bytes before pushing?
        use RecordData::*;
        input
            .chunks_exact(element_size)
            .for_each(|chunk| match self {
                Int(v) | Bool(v) => v.push(bp::read_i32(chunk)),
                F32(v) => v.push(bp::read_f32(chunk)),
                F64(v) => v.push(bp::read_f64(chunk)),
                Chars(v) => v.push(FlexString::from(
                    str::from_utf8(chunk)
                        .unwrap_or("Utf8 error creating string record")
                        .trim(),
                )),
                Message => unimplemented!("Attempted to push into a RecordData::Message instance."),
            });
    }

    /// Populate Data instance from the byte slice. Use header info to infer the number of bytes to
    /// read and how to interpret them. The function will panic if the input slice is not fully
    /// consumed.
    fn populate(&mut self, header: &Header, input: &[u8]) -> Result<()> {
        // keep reading bytes from the input until we collected the requested number of elements
        let mut n_remaining_elements = header.n_elements;
        let mut rest = input;

        while n_remaining_elements > 0 {
            // read at most the block_length number of elements
            let to_read = std::cmp::min(header.block_length, n_remaining_elements);
            let (block_bytes, input) = bp::take_block_exact(to_read * header.element_size, rest)?;

            // add the current block to the constructed instance
            self.push(block_bytes, header.element_size);

            n_remaining_elements -= to_read;
            rest = input;
        }
        assert!(rest.is_empty(), "Record body not completely consumed");

        Ok(())
    }
}

/// A record's header information necessary to populate the record's body.
#[derive(Debug, PartialEq)]
struct Header {
    name: FlexString,
    element_size: usize,
    block_length: usize,
    n_elements: usize,
}

impl Header {
    /// How many bytes are needed to represent the record's body.
    fn len_bytes(&self) -> usize {
        let n_blocks = 1 + (self.n_elements - 1) / self.block_length;
        self.n_elements * self.element_size + n_blocks * 4 * 2
    }

    fn with_record_data(
        name: FlexString,
        type_id: FlexString,
        n_elements: usize,
    ) -> Result<(Self, RecordData)> {
        use RecordData::*;

        let (element_size, block_length, data) = match type_id.as_bytes() {
            b"INTE" => (
                mem::size_of::<i32>(),
                NUM_BLOCK_LENGTH,
                Int(Vec::with_capacity(n_elements)),
            ),
            b"REAL" => (
                mem::size_of::<f32>(),
                NUM_BLOCK_LENGTH,
                F32(Vec::with_capacity(n_elements)),
            ),
            b"DOUB" => (
                mem::size_of::<f64>(),
                NUM_BLOCK_LENGTH,
                F64(Vec::with_capacity(n_elements)),
            ),
            // i32 is the underlying "logical" type in Eclipse files
            b"LOGI" => (
                mem::size_of::<i32>(),
                NUM_BLOCK_LENGTH,
                Bool(Vec::with_capacity(n_elements)),
            ),
            b"MESS" => (0, NUM_BLOCK_LENGTH, Message),
            b"CHAR" => (
                FIXED_STRING_LENGTH,
                STR_BLOCK_LENGTH,
                Chars(Vec::with_capacity(n_elements)),
            ),
            [b'C', b'0', rest @ ..] => {
                let len = if rest.iter().all(u8::is_ascii_digit) {
                    unsafe { str::from_utf8_unchecked(rest).parse().unwrap() }
                } else {
                    return Err(EclairError::InvalidC0nnLength(
                        String::from_utf8_lossy(rest).to_string(),
                    ));
                };
                (len, STR_BLOCK_LENGTH, Chars(Vec::with_capacity(n_elements)))
            }
            _ => {
                return Err(EclairError::InvalidDataType(type_id.to_string()));
            }
        };

        Ok((
            Self {
                name,
                element_size,
                block_length,
                n_elements,
            },
            data,
        ))
    }
}

/// Extract information from the record header. Returns the header and the correct empty RecordData
/// variant to be populated with values.
fn extract_header_info(input: &[u8; 24]) -> Result<(Header, RecordData)> {
    // Strip the head/tail markers
    let (header, _) = bp::take_block_exact(16, input)?;

    // 8-char long record name.
    let (name, header) = bp::take_str(8, header)?;

    // 4-byte integer for the number of elements in the body that follows the current header.
    let (n_elements, header) = bp::take_i32(header)?;

    // 4-char long data type identifier.
    let (type_id, header) = bp::take_str(4, header)?;

    assert!(header.is_empty(), "Record header not completely consumed");

    Header::with_record_data(name, type_id, n_elements as usize)
}

/// A single binary record has an 8-char long ASCII name and a collection of values.
#[derive(Debug, PartialEq)]
pub struct Record {
    pub(crate) name: FlexString,
    pub(crate) data: RecordData,
}

/// Implementors of the `ReadRecord` can produce Eclipse records.
pub trait ReadRecord {
    /// Read a new Eclipse record. If successful, this function will return
    /// the total size of the record in bytes. Zero bytes mean that the stream has reached EOF.
    fn read_record(&mut self) -> Result<(usize, Option<Record>)>;

    /// Returns an iterator over the records of this reader.
    fn records(self) -> Records<Self>
    where
        Self: Sized,
    {
        Records { buf: self }
    }
}

/// An iterator over the records of an instance of ReadRecord.
pub struct Records<B> {
    buf: B,
}

impl<B: ReadRecord> Iterator for Records<B> {
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Result<Record>> {
        match self.buf.read_record() {
            Ok((0, None)) => None,
            Ok((0, Some(_))) => {
                unimplemented!("read_record() returned a record but encountered an EOF.")
            }
            Ok((_n, None)) => {
                unimplemented!("read_record() returned None but did not encounter an EOF.")
            }
            Ok((_n, Some(record))) => Some(Ok(record)),
            Err(e) => Some(Err(e)),
        }
    }
}

/// W

/// Implementation of ReadRecord for any type that implements std::io::Read (e.g. a file or
/// a network socket).
impl<T> ReadRecord for T
where
    T: std::io::Read,
{
    fn read_record(&mut self) -> Result<(usize, Option<Record>)> {
        // Read the header from the next 24 bytes.
        let mut header_buf = [0u8; 24];
        let header_bytes = self.read(&mut header_buf)?;

        if header_bytes == 0 {
            // reached EOF
            return Ok((0, None));
        }

        // If we are close to the EOF, we might not get the entire header from calling the read()
        // above.
        if header_bytes < 24 {
            self.read_exact(&mut header_buf[header_bytes..])?;
        }

        let (header, mut data) = extract_header_info(&header_buf)?;

        let mut body_buf = vec![0u8; header.len_bytes()];
        self.read_exact(&mut body_buf)?;

        data.populate(&header, &body_buf)?;

        let total_bytes = 24 + header.len_bytes();

        Ok((
            total_bytes,
            Some(Record {
                name: header.name,
                data,
            }),
        ))
    }
}

/// Encapsulation of the ZeroMQ monitored connection. The field order is important, because member
/// variables has custom Drop implementations.
#[cfg(feature = "read_zmq")]
pub struct ZmqConnection {
    monitor: zmq::Socket,
    sock: zmq::Socket,
    ctx: zmq::Context,
}

#[cfg(feature = "read_zmq")]
impl ZmqConnection {
    /// Creates a new ZeroMQ-based connection to the server. Expects the server address, the port
    /// number and the identity for the underlying socket. The socket type is fixed to be of the
    /// DEALER type.
    pub fn new(server: &str, port: i32, identity: &str) -> Result<Self> {
        let ctx = zmq::Context::new();
        let sock = ctx.socket(zmq::DEALER)?;
        sock.set_identity(identity.as_bytes())?;

        // Connect to the server.
        let address = format!("tcp://{}:{}", server, port);
        log::info!("Connecting to {}", address);
        sock.connect(&address)?;

        // Setup the connection monitor socket.
        sock.monitor(
            "inproc://monitor-client",
            zmq::SocketEvent::DISCONNECTED as i32,
        )?;
        let monitor = ctx.socket(zmq::PAIR)?;
        monitor.connect("inproc://monitor-client")?;

        Ok(ZmqConnection { monitor, sock, ctx })
    }

    pub fn send<T>(&self, data: T, flags: i32) -> Result<()>
    where
        T: zmq::Sendable,
    {
        data.send(&self.sock, flags)
            .map_err(EclairError::ZeroMqError)
    }
}

#[cfg(feature = "read_zmq")]
impl ReadRecord for ZmqConnection {
    fn read_record(&mut self) -> Result<(usize, Option<Record>)> {
        // Watch for received messages and for the server disconnect event.
        let mut items = [
            self.sock.as_poll_item(zmq::POLLIN),
            self.monitor.as_poll_item(zmq::POLLIN),
        ];

        loop {
            zmq::poll(&mut items, 1000)?;
            if items[0].is_readable() {
                // We assume that one message corresponds to one record.
                let msg = self.sock.recv_bytes(0)?;
                let mut cursor = std::io::Cursor::new(msg.as_slice());
                return cursor.read_record();
            }
            if items[1].is_readable() {
                return Err(EclairError::ZeroMqSocketDisconnected);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        fs::File,
        io::{BufReader, Cursor},
    };

    #[test]
    fn single_data_array_short() {
        let input = include_bytes!("../assets/single_data_array.bin");
        let mut cursor = Cursor::new(input.as_ref());

        let (n_bytes, record) = cursor.read_record().unwrap();
        assert_eq!(n_bytes, 16 + 5 * 8 + 16);

        let record = record.unwrap();
        assert_eq!(&record.name, "KEYWORDS");
        assert_eq!(
            record.data,
            RecordData::Chars(
                vec!["FOPR", "FGPR", "FWPR", "WOPR", "WGPR"]
                    .into_iter()
                    .map(|s| FlexString::from(s))
                    .collect()
            )
        );

        let next_result = cursor.read_record();
        assert!(next_result.is_ok());
        let (n_bytes, record) = next_result.unwrap();
        assert_eq!(n_bytes, 0);
        assert!(record.is_none());
    }

    #[test]
    fn read_spe_10() {
        let file = File::open("assets/SPE10.SMSPEC").unwrap();
        let buf_reader = BufReader::new(file);

        let records: Vec<Record> = buf_reader.records().map(|r| r.unwrap()).collect();

        assert_eq!(records.len(), 10);

        assert_eq!(
            records[1],
            Record {
                name: FlexString::from("DIMENS"),
                data: RecordData::Int(vec![34, 100, 100, 30, 0, -1])
            }
        );

        assert_eq!(
            records[7],
            Record {
                name: FlexString::from("STARTDAT"),
                data: RecordData::Int(vec![1, 3, 2005, 0, 0, 0])
            }
        );
    }
}
