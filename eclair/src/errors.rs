use thiserror::Error;

/// File reading/writing errors
#[derive(Error, Debug)]
pub enum FileError {
    #[error("Invalid file path")]
    InvalidFilePath,

    #[error("Invalid file extension")]
    InvalidFileExt,
}

/// Binary parsing errors
#[derive(Error, Debug)]
pub enum BinaryError {
    #[error("Not enough bytes in the input. Expected {expected:?}, found {found:?}.")]
    NotEnoughBytes { expected: usize, found: usize },

    #[error("Head and tail markers mismatch in a binary record. Head {head:?}, tail {tail:?}.")]
    HeadTailMismatch { head: i32, tail: i32 },

    #[error("Invalid data type value: {0}")]
    InvalidDataType(String),

    #[error("Invalid length for a dynamic string data type: {0}")]
    InvalidStringLength(String),

    #[error("Record size mismatch. Expected {expected:?}, found {found:?}.")]
    RecordSizeMismatch { expected: usize, found: usize },
}

/// Summary parsing errors
#[derive(Error, Debug)]
pub enum SummaryError {
    #[error("Invalid length for start date data: {0}.")]
    InvalidStartDateLength(usize),
}
