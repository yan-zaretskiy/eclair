//! This crate provides a reader for the binary files written out by the Eclipse reservoir simulator.

mod binary_parsing;
pub mod error;
pub mod records;
pub mod summary;

use smallstr::SmallString;

/// Convenience type alias for a string with the SSO.
const FIXED_STRING_LENGTH: usize = 8;
pub(crate) type FlexString = SmallString<[u8; FIXED_STRING_LENGTH]>;

/// Crate's Result type.
pub(crate) type Result<T> = std::result::Result<T, error::EclairError>;
