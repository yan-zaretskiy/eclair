use std::convert::TryInto;

use anyhow as ah;
use anyhow::Context;

use crate::errors::BinaryError;

pub fn read_i32(input: &[u8]) -> i32 {
    i32::from_be_bytes(input.try_into().unwrap())
}

pub fn read_f32(input: &[u8]) -> f32 {
    f32::from_be_bytes(input.try_into().unwrap())
}

pub fn read_f64(input: &[u8]) -> f64 {
    f64::from_be_bytes(input.try_into().unwrap())
}

pub fn take(size: usize, input: &[u8]) -> ah::Result<(&[u8], &[u8])> {
    if input.len() < size {
        return Err(BinaryError::NotEnoughBytes {
            expected: size,
            found: input.len(),
        }
        .into());
    }
    Ok(input.split_at(size))
}

pub fn take_i32(input: &[u8]) -> ah::Result<(i32, &[u8])> {
    let (left, right) = take(4, input)?;
    Ok((read_i32(left), right))
}

pub fn single_record(input: &[u8]) -> ah::Result<(&[u8], &[u8])> {
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

pub fn single_record_with_size(size: usize, input: &[u8]) -> ah::Result<(&[u8], &[u8])> {
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
