use std::{
    collections::HashMap,
    convert::{From, TryFrom, TryInto},
    thread::sleep,
    time::Duration,
};

use crossbeam_channel::Sender;
use serde::Deserialize;

use crate::{
    binary_parsing::{read_f32, read_i32},
    error::EclairError,
    records::RecordData,
    summary::{InitializeSummary, SmspecRecords, Summary, UpdateSummary},
    FlexString, Result,
};

/// Encapsulation of the ZeroMQ monitored connection. The field order is important, because member
/// variables has custom Drop implementations.
pub struct ZmqConnection {
    monitor: zmq::Socket,
    sock: zmq::Socket,
    ctx: zmq::Context,
}

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

    pub fn recv_msg(&mut self, flags: i32) -> Result<zmq::Message> {
        self.sock.recv_msg(flags).map_err(EclairError::ZeroMqError)
    }

    pub fn recv_multipart(&mut self, flags: i32) -> Result<Vec<Vec<u8>>> {
        self.sock
            .recv_multipart(flags)
            .map_err(EclairError::ZeroMqError)
    }
}

pub struct ZmqUpdater {
    conn: ZmqConnection,
    n_items: usize,
    n_steps: usize,
}

impl UpdateSummary for ZmqUpdater {
    fn update(&mut self, sender: Sender<Vec<f32>>) -> Result<()> {
        let mut items = [
            self.conn.monitor.as_poll_item(zmq::POLLIN),
            self.conn.sock.as_poll_item(zmq::POLLIN),
        ];

        let mut is_connected = true;
        loop {
            zmq::poll(&mut items, 0)?;

            if is_connected && items[0].is_readable() {
                eprintln!("Detected ZeroMQ socket disconnect.");
                is_connected = false;
                //return Err(EclairError::ZeroMqSocketDisconnected);
            }

            if items[1].is_readable() {
                is_connected = true;
                let msg = self.conn.sock.recv_multipart(0)?;

                // Make sure the time iteration is correct.
                let current_step = read_i32(msg[0].as_slice()) as usize;
                if current_step != self.n_steps {
                    return Err(EclairError::InvalidMinistepValue {
                        expected: self.n_steps,
                        found: current_step,
                    });
                }

                let params: Vec<f32> = msg[1]
                    .chunks_exact(std::mem::size_of::<f32>())
                    .map(|chunk| read_f32(chunk))
                    .collect();

                if params.len() != self.n_items {
                    return Err(EclairError::UnexpectedRecordDataLength {
                        name: "ZMQ_PARAMS".to_owned(),
                        expected: self.n_items,
                        found: params.len(),
                    });
                }

                self.n_steps += 1;

                if sender.send(params).is_err() {
                    log::debug!(target: "Updating Summary", "Error while sending params over a channel");
                    return Ok(());
                }
            }

            sleep(Duration::from_millis(100));
        }
    }
}

#[derive(Deserialize)]
struct SmspecJson {
    #[serde(rename = "DIMENS")]
    dimens: Vec<i32>,

    #[serde(rename = "KEYWORDS")]
    keywords: Vec<FlexString>,

    #[serde(rename = "NAMES")]
    names: Vec<FlexString>,

    #[serde(rename = "NUMS")]
    nums: Vec<i32>,

    #[serde(rename = "STARTDAT")]
    start_date: Vec<i32>,

    #[serde(rename = "UNITS")]
    units: Vec<FlexString>,
}

impl From<SmspecJson> for SmspecRecords {
    fn from(smspec_json: SmspecJson) -> Self {
        use RecordData::*;

        let mut records = HashMap::new();
        records.insert("DIMENS", Some(Int(smspec_json.dimens)));
        records.insert("STARTDAT", Some(Int(smspec_json.start_date)));
        records.insert("KEYWORDS", Some(Chars(smspec_json.keywords)));
        records.insert("WGNAMES", Some(Chars(smspec_json.names)));
        records.insert("NUMS", Some(Int(smspec_json.nums)));
        records.insert("UNITS", Some(Chars(smspec_json.units)));

        SmspecRecords::new(records)
    }
}

impl InitializeSummary for ZmqConnection {
    type Updater = ZmqUpdater;

    fn init(self) -> Result<(Summary, Self::Updater)> {
        // Initial handshake.
        self.sock.send("", 0)?;

        // receive SMSPEC first
        let mut items = [
            self.monitor.as_poll_item(zmq::POLLIN),
            self.sock.as_poll_item(zmq::POLLIN),
        ];

        let smspec_json: SmspecJson = loop {
            zmq::poll(&mut items, 0)?;

            if items[0].is_readable() {
                return Err(EclairError::ZeroMqSocketDisconnected);
            }

            if items[1].is_readable() {
                let json = self.sock.recv_msg(0)?;
                match json.as_str() {
                    None => return Err(EclairError::InvalidSmspecJson),
                    Some(v) => break serde_json::from_str(v)?,
                };
            }
        };

        let smspec_records = SmspecRecords::from(smspec_json);
        let summary = Summary::try_from(smspec_records)?;
        let n_items = summary.n_items();

        Ok((
            summary,
            ZmqUpdater {
                conn: self,
                n_items,
                n_steps: 0,
            },
        ))
    }
}
