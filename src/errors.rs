use thiserror::Error;

#[derive(Error, Debug)]
pub enum EclError {
    #[error("Not enough bytes in the input. Expected {expected:?}, found {found:?}.")]
    NotEnoughBytes { expected: usize, found: usize },

    #[error("Head and tail markers mismatch in a binary record. Head {head:?}, tail {tail:?}.")]
    HeadTailMismatch { head: i32, tail: i32 },

    #[error("Invalid string value: {0}")]
    InvalidString(String),

    #[error("Record size mismatch. Expected {expected:?}, found {found:?}.")]
    RecordSize { expected: usize, found: usize },

    #[error("Invalid file path")]
    InvalidFilePath,

    #[error("Invalid file extension")]
    InvalidFileExt,
}
