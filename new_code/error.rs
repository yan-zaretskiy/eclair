use thiserror::Error;

/// Crate errors
#[derive(Error, Debug)]
pub enum EclairError {
    // BINARY PARSING ERRORS
    #[error("Not enough bytes in the input. Expected {expected:?}, found {found:?}.")]
    NotEnoughBytes { expected: usize, found: usize },

    #[error("Head and tail mismatch in a binary record. Head {head:?}, tail {tail:?}.")]
    HeadTailMismatch { head: i32, tail: i32 },

    #[error("Invalid data type value: {0}")]
    InvalidDataType(String),

    #[error("Invalid length for the dynamic string data type: {0}")]
    InvalidC0nnLength(String),

    #[error("Record length mismatch. Expected {expected:?}, found {found:?}.")]
    RecordByteLengthMismatch { expected: usize, found: usize },

    #[error("Failed to convert bytes to the UTF8 string")]
    InvalidStringBytes(#[from] std::str::Utf8Error),

    #[error("Failed to read bytes from the std::io::Read instance")]
    ReadError(#[from] std::io::Error),

    // RECORD CONTENT ERRORS
    #[error("Binary record {name:?} has unexpected data type {found:?}. Expected {expected:?}.")]
    InvalidRecordDataType {
        name: String,
        expected: String,
        found: String,
    },

    #[error("Binary record {0} has been encountered twice in the SMSPEC file.")]
    RecordEncounteredTwice(String),

    #[error("Binary record {name:?} contains unexpected number of elements: Expected {expected:?}, found {found:?}.")]
    UnexpectedRecordDataLength {
        name: String,
        expected: usize,
        found: usize,
    },

    #[error("Missing or invalid record: {0}")]
    MissingRecord(String),

    #[error("MINISTEP value does not match the current amount of stored UNSMRY records. Expected {expected:?}, found {found:?}.")]
    InvalidMinistepValue { expected: usize, found: usize },
}
