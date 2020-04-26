use crate::errors::BinaryError;

use anyhow as ah;
use anyhow::Context;
use byteorder::{BigEndian, ByteOrder};
use smallstr::SmallString;

use std::{
    cmp::min,
    convert::TryInto,
    fs::File,
    io,
    io::{prelude::*, BufReader},
    path::Path,
    str,
};

pub type FlexString = SmallString<[u8; 8]>;

/// Represents a body of data in a binary record in an Eclipse file
#[derive(Debug, PartialEq)]
pub enum BinRecord {
    Int(Vec<i32>),
    Boolean(Vec<i32>),
    Chars(Vec<FlexString>),

    /// FP data is copied directly as bytes, their contents don't need to be examined
    F32Bytes(Vec<u8>),
    F64Bytes(Vec<u8>),

    /// A tag type with no data
    Message,
}

impl BinRecord {
    const NUM_BLOCK_SIZE: usize = 1000;
    const STR_BLOCK_SIZE: usize = 105;

    fn new(type_string: [u8; 4]) -> ah::Result<(Self, usize, usize)> {
        use BinRecord::*;
        match &type_string {
            b"INTE" => Ok((Int(Vec::new()), 4, BinRecord::NUM_BLOCK_SIZE)),
            b"REAL" => Ok((F32Bytes(Vec::new()), 4, BinRecord::NUM_BLOCK_SIZE)),
            b"DOUB" => Ok((F64Bytes(Vec::new()), 8, BinRecord::NUM_BLOCK_SIZE)),
            b"LOGI" => Ok((Boolean(Vec::new()), 4, BinRecord::NUM_BLOCK_SIZE)),
            b"MESS" => Ok((Message, 0, BinRecord::NUM_BLOCK_SIZE)),
            b"CHAR" => Ok((Chars(Vec::new()), 8, BinRecord::STR_BLOCK_SIZE)),
            [b'C', b'0', rest @ ..] => {
                let len = if rest.iter().all(u8::is_ascii_digit) {
                    unsafe { str::from_utf8_unchecked(rest).parse().unwrap() }
                } else {
                    return Err(BinaryError::InvalidStringLength(
                        String::from_utf8_lossy(rest).to_string(),
                    )
                    .into());
                };

                Ok((Chars(Vec::new()), len, BinRecord::STR_BLOCK_SIZE))
            }
            _ => Err(BinaryError::InvalidDataType(
                String::from_utf8_lossy(&type_string).to_string(),
            )
            .into()),
        }
    }

    fn append(&mut self, raw_data: &[u8], element_size: usize) {
        use BinRecord::*;
        raw_data.chunks(element_size).for_each(|chunk| match self {
            Int(v) | Boolean(v) => v.push(BigEndian::read_i32(chunk)),
            F32Bytes(v) | F64Bytes(v) => v.extend_from_slice(chunk),
            Chars(v) => v.push(FlexString::from(str::from_utf8(chunk).unwrap().trim())),
            Message => {}
        });
    }
}

/// Helper functions for parsing binary files
mod parsing {
    use super::*;

    fn take(size: usize, input: &[u8]) -> ah::Result<(&[u8], &[u8])> {
        if input.len() < size {
            return Err(BinaryError::NotEnoughBytes {
                expected: size,
                found: input.len(),
            }
            .into());
        }
        Ok(input.split_at(size))
    }

    fn take_i32(input: &[u8]) -> ah::Result<(i32, &[u8])> {
        let (left, right) = take(4, input)?;
        Ok((BigEndian::read_i32(left), right))
    }

    pub(super) fn single_record(input: &[u8]) -> ah::Result<(&[u8], &[u8])> {
        // head marker
        let (head, input) = take_i32(input).with_context(|| "Failed to read a head marker.")?;

        // actual data
        let (data, input) =
            take(head as usize, input).with_context(|| "Failed to read binary record contents.")?;

        // tail marker, make sure it equals the head one
        let (tail, input) = take_i32(input).with_context(|| "Failed to read a tail marker.")?;

        if head == tail {
            Ok((data, input))
        } else {
            Err(BinaryError::HeadTailMismatch { head, tail }.into())
        }
    }

    fn single_record_with_size(size: usize, input: &[u8]) -> ah::Result<(&[u8], &[u8])> {
        let (data, input) = single_record(input)?;
        if data.len() != size {
            return Err(BinaryError::RecordSizeMismatch {
                expected: size,
                found: data.len(),
            }
            .into());
        }
        Ok((data, input))
    }

    pub(super) fn keyword_header(input: &[u8; 24]) -> ah::Result<(HeaderData, BinRecord)> {
        // header record, must be 16 bytes long in total
        let (header, _) =
            single_record_with_size(16, input).with_context(|| "Failed to read a data header.")?;

        // 8-character string for a keyword name
        let (name, header) = take(8, header).with_context(|| "Failed to read a keyword name.")?;
        let name = FlexString::from(
            str::from_utf8(name)
                .with_context(|| "Failed to parse a keyword name as an 8-char string.")?
                .trim(),
        );

        // 4-byte integer for a number of elements in an array
        let (n_elements, header) = take_i32(header)
            .with_context(|| "Failed to read a number of elements in a keyword array.")?;

        // 4-character string for a data type
        let (type_string, header) =
            take(4, header).with_context(|| "Failed to read a data type string.")?;
        let type_string = type_string.try_into().unwrap();

        assert!(header.is_empty(), "Keyword header not completely consumed");

        // Init the data storage with the correct type
        let (data, element_size, block_length) =
            BinRecord::new(type_string).with_context(|| "Failed to parse a data type.")?;

        Ok((
            HeaderData {
                name,
                n_elements: n_elements as usize,
                element_size,
                block_length,
            },
            data,
        ))
    }

    pub(super) fn keyword_data(
        mut data: BinRecord,
        header: &HeaderData,
        input: &[u8],
    ) -> ah::Result<BinRecord> {
        // populate the actual array body from sub-blocks
        let mut n_remaining_elements = header.n_elements;
        let mut remaining_input = input;

        while n_remaining_elements > 0 {
            let to_read = min(header.block_length, n_remaining_elements);
            let (raw_data, input) =
                single_record_with_size(to_read * header.element_size, remaining_input)
                    .with_context(|| {
                        format!(
                            "Failed to read a data body sub-block of size {}.",
                            to_read * header.element_size
                        )
                    })?;
            data.append(raw_data, header.element_size);
            n_remaining_elements -= to_read;
            remaining_input = input;
        }
        Ok(data)
    }
}

/// Data extracted from a keyword header needed to populate the record
#[derive(Debug)]
struct HeaderData {
    name: FlexString,
    n_elements: usize,
    element_size: usize,
    block_length: usize,
}

impl HeaderData {
    fn bytes_for_elements(&self) -> usize {
        let n_blocks = 1 + (self.n_elements - 1) / self.block_length;
        self.n_elements * self.element_size + n_blocks * 4 * 2
    }
}

/// A single binary keyword.
#[derive(Debug, PartialEq)]
pub struct BinKeyword {
    pub name: FlexString,
    pub data: BinRecord,
}

/// A binary file is an iterator over keywords read from a file one at a time.
#[derive(Debug)]
pub struct BinFile {
    reader: BufReader<File>,
}

impl BinFile {
    pub fn new<P: AsRef<Path>>(path: P) -> ah::Result<Self> {
        let file = File::open(path).with_context(|| "Failed to open file at requested path")?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }

    pub fn next_keyword(mut self) -> ah::Result<(BinKeyword, Self)> {
        // Look at the next 24 bytes and try reading the header
        let mut header_buf = [0u8; 24];
        self.reader.read_exact(&mut header_buf)?;

        let (header, data) = parsing::keyword_header(&header_buf)?;

        // Compute how many bytes we need to read from the data and extract it
        let need_bytes = header.bytes_for_elements();
        let mut data_buf = vec![0; need_bytes];
        self.reader.read_exact(&mut data_buf)?;

        let data = parsing::keyword_data(data, &header, &data_buf)?;

        Ok((
            BinKeyword {
                name: header.name,
                data,
            },
            self,
        ))
    }
}

/// A helper function for processing keywords in a binary file.
pub fn for_keyword_in<F>(mut bin: BinFile, mut fun: F) -> ah::Result<()>
where
    F: FnMut(BinKeyword) -> ah::Result<()>,
{
    loop {
        match bin.next_keyword() {
            Ok((kw, remaining)) => {
                bin = remaining;
                fun(kw)?;
            }
            // we break from the loop when we encounter the EOF,
            Err(e) => {
                let is_eof = e
                    .downcast_ref::<io::Error>()
                    .map(|e| e.kind() == io::ErrorKind::UnexpectedEof);
                if let Some(true) = is_eof {
                    break;
                }
                return Err(e);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn single_record_test() {
        let input = include_bytes!("../../assets/single_record.bin");
        let (result, _) = parsing::single_record(input).unwrap();

        let mut buf = [0.0; 10];
        BigEndian::read_f64_into(result, &mut buf);
        assert_eq!(
            buf,
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0] as &[_]
        );
    }

    #[test]
    fn single_data_array_short() {
        let input = include_bytes!("../../assets/single_data_array.bin");
        let (header, data) = parsing::keyword_header(&input[..24].try_into().unwrap()).unwrap();
        let data = parsing::keyword_data(data, &header, &input[24..]).unwrap();
        let kw = BinKeyword {
            name: header.name,
            data,
        };

        assert_eq!(kw.name, FlexString::from("KEYWORDS"));

        assert_eq!(
            kw.data,
            BinRecord::Chars(
                vec!["FOPR", "FGPR", "FWPR", "WOPR", "WGPR"]
                    .iter()
                    .map(|&s| FlexString::from(s))
                    .collect()
            )
        );
    }
}
