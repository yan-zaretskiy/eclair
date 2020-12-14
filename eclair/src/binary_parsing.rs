use crate::{error::EclairError, FlexString, Result};
use std::{convert::TryInto, str};

/// Interpret a byte slice as an `i32` integer number.
pub(crate) fn read_i32(input: &[u8]) -> i32 {
    i32::from_be_bytes(input.try_into().unwrap())
}

/// Interpret a byte slice as an `f32` floating point number.
pub(crate) fn read_f32(input: &[u8]) -> f32 {
    f32::from_be_bytes(input.try_into().unwrap())
}

/// Interpret a byte slice as an `f64` floating point number.
pub(crate) fn read_f64(input: &[u8]) -> f64 {
    f64::from_be_bytes(input.try_into().unwrap())
}

/// A fallible wrapper around the byte slice's `split_at`.
fn take(size: usize, input: &[u8]) -> Result<(&[u8], &[u8])> {
    if input.len() < size {
        return Err(EclairError::NotEnoughBytes {
            expected: size,
            found: input.len(),
        });
    }
    Ok(input.split_at(size))
}

/// Take the requested number of bytes from the slice front as a UTF8 string and return it along
/// with the rest of the slice.
pub(crate) fn take_str(size: usize, input: &[u8]) -> Result<(FlexString, &[u8])> {
    let (left, right) = take(size, input)?;
    Ok((FlexString::from(str::from_utf8(left)?.trim()), right))
}

/// Take a single i32 integer from the slice front and return it along with the rest of the slice.
pub(crate) fn take_i32(input: &[u8]) -> Result<(i32, &[u8])> {
    let (left, right) = take(4, input)?;
    Ok((read_i32(left), right))
}

/// Extract a single binary block from the byte slice and return it along with the rest of the slice.
/// The surrounding size markers are excluded from the resulting slice.
fn take_block(input: &[u8]) -> Result<(&[u8], &[u8])> {
    // head marker
    let (head, input) = take_i32(input)?;

    // actual data
    let (data, input) = take(head as usize, input)?;

    // tail marker
    let (tail, input) = take_i32(input)?;

    if head == tail {
        Ok((data, input))
    } else {
        Err(EclairError::HeadTailMismatch { head, tail })
    }
}

/// Extract a single binary block of the exact byte size from the byte slice and return it along
/// with the rest of the slice. The surrounding size markers are excluded from the resulting slice.
pub(crate) fn take_block_exact(size: usize, input: &[u8]) -> Result<(&[u8], &[u8])> {
    take_block(input).and_then(|data| {
        if data.0.len() != size {
            Err(EclairError::RecordByteLengthMismatch {
                expected: size,
                found: data.0.len(),
            })
        } else {
            Ok(data)
        }
    })
}
