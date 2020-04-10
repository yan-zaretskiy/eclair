use thiserror::Error;

/// erichdongubler: I'm a bit confused about the error story now. Looks like you're using this enum
/// in some but not all places, and they're immediately getting converted into `anyhow::Error`s?
/// Perhaps this is an artifact of being a WIP.
///
/// Before you go publishing a crate, it'd be best to have one or maybe more families of error
/// enumerations for your library operations. In the meantime, using `anyhow` in the lib is fine, I
/// just want to make sure I understand and that the ideal I have is understood. :)
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
