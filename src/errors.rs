use thiserror::Error;

/// File reading/writing errors
#[derive(Error, Debug)]
pub enum EclFileError {
    #[error("Invalid file path")]
    InvalidFilePath,

    #[error("Invalid file extension")]
    InvalidFileExt,
}

/// Binary parsing errors
#[derive(Error, Debug)]
pub enum EclBinaryError {
    #[error("Not enough bytes in the input. Expected {expected:?}, found {found:?}.")]
    NotEnoughBytes { expected: usize, found: usize },

    #[error("Head and tail markers mismatch in a binary record. Head {head:?}, tail {tail:?}.")]
    HeadTailMismatch { head: i32, tail: i32 },

    #[error("Invalid data type value: {0}")]
    InvalidDataType(String),

    #[error("Record size mismatch. Expected {expected:?}, found {found:?}.")]
    RecordSizeMismatch { expected: usize, found: usize },
}

/// Summary parsing errors
#[derive(Error, Debug)]
pub enum EclSummaryError {
    #[error("Invalid length for start date data: {0}.")]
    InvalidStartDateLength(usize),

    #[error("Error parsing the SMSPEC file: {0}.")]
    SmspecParsing(EclBinaryError),

    #[error("Error parsing the UNSMRY file: {0}.")]
    UnsmryParsing(EclBinaryError),
}
