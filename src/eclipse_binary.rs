use std::{
    cmp::min,
    fs::File,
    io::{prelude::*, BufReader},
    path::Path,
    str,
};

use anyhow as ah;
use anyhow::Context;
use arrayvec::ArrayString;
use byteorder::{BigEndian, ByteOrder};

use crate::errors::EclError;

pub type FixedString = ArrayString<[u8; 8]>;

/// Represents a body of data in a binary record in an Eclipse file.
#[derive(Debug, PartialEq)]
pub enum EclData {
    Int(Vec<i32>),
    Float(Vec<f32>),
    Double(Vec<f64>),
    Logical(Vec<i32>),
    FixStr(Vec<FixedString>),
    DynStr(usize, Vec<String>),
    Message,
}

impl EclData {
    const NUM_BLOCK_SIZE: usize = 1000;
    const STR_BLOCK_SIZE: usize = 105;

    fn new(raw_dtype: &[u8]) -> ah::Result<Self> {
        use EclData::*;
        match str::from_utf8(raw_dtype) {
            Ok("INTE") => Ok(Int(vec![])),
            Ok("REAL") => Ok(Float(vec![])),
            Ok("DOUB") => Ok(Double(vec![])),
            Ok("LOGI") => Ok(Logical(vec![])),
            Ok("CHAR") => Ok(FixStr(vec![])),
            Ok("MESS") => Ok(Message),
            Ok(s) if s.starts_with("C0") && s.len() == 4 => match s[2..].parse::<usize>() {
                Ok(len) => Ok(DynStr(len, vec![])),
                Err(_) => Err(EclError::InvalidString(s[2..].to_owned()).into()),
            },
            _ => Err(EclError::InvalidString("<String failed to parse>".to_owned()).into()),
        }
    }

    fn block_length(&self) -> usize {
        use EclData::*;
        match self {
            FixStr(_) | DynStr(_, _) => EclData::STR_BLOCK_SIZE,
            _ => EclData::NUM_BLOCK_SIZE,
        }
    }

    fn element_size(&self) -> usize {
        use EclData::*;
        match self {
            Int(_) | Float(_) | Logical(_) => 4,
            Double(_) | FixStr(_) => 8,
            Message => 0,
            DynStr(len, _) => *len,
        }
    }

    fn bytes_for_elements(&self, n: usize) -> usize {
        let n_blocks = 1 + (n - 1) / self.block_length();
        n * self.element_size() + n_blocks * 4 * 2
    }

    fn push(&mut self, raw_chunk: &[u8]) {
        use EclData::*;
        match self {
            Int(v) | Logical(v) => v.push(BigEndian::read_i32(raw_chunk)),
            Float(v) => v.push(BigEndian::read_f32(raw_chunk)),
            Double(v) => v.push(BigEndian::read_f64(raw_chunk)),
            FixStr(v) => {
                v.push(FixedString::from(str::from_utf8(raw_chunk).unwrap().trim()).unwrap())
            }
            DynStr(_, v) => v.push(String::from(str::from_utf8(raw_chunk).unwrap())),
            Message => {}
        }
    }

    fn append(&mut self, raw_data: &[u8]) {
        for chunk in raw_data.chunks(self.element_size()) {
            self.push(chunk);
        }
    }
}

/// Helper function for parsing binary files.
mod parsing {
    use super::*;

    fn take(size: usize, input: &[u8]) -> ah::Result<(&[u8], &[u8])> {
        if input.len() < size {
            return Err(EclError::NotEnoughBytes {
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
            Err(EclError::HeadTailMismatch { head, tail }.into())
        }
    }

    fn single_record_with_size(size: usize, input: &[u8]) -> ah::Result<(&[u8], &[u8])> {
        let (data, input) = single_record(input)?;
        if data.len() != size {
            return Err(EclError::RecordSize {
                expected: size,
                found: data.len(),
            }
            .into());
        }
        Ok((data, input))
    }

    pub(super) fn keyword_header(input: &[u8]) -> ah::Result<(FixedString, usize, EclData)> {
        // header record, must be 16 bytes long in total
        let (header, _) =
            single_record_with_size(16, input).with_context(|| "Failed to read a data header.")?;

        // 8-character string for a keyword name
        let (name, header) = take(8, header).with_context(|| "Failed to read a keyword name.")?;
        let name = FixedString::from(
            str::from_utf8(name)
                .with_context(|| "Failed to parse a keyword name as an 8-char string.")?
                .trim(),
        )
        .unwrap(); // this unwrap is fine, because we pass exactly 8 characters

        // 4-byte integer for a number of elements in an array
        let (n_elements, header) = take_i32(header)
            .with_context(|| "Failed to read a number of elements in a keyword array.")?;

        // 4-character string for a data type
        let (dtype, _) = take(4, header).with_context(|| "Failed to read a data type string.")?;

        // Init the data storage with the correct type
        let data = EclData::new(dtype).with_context(|| "Failed to parse a data type.")?;
        Ok((name, n_elements as usize, data))
    }

    pub(super) fn keyword_data(
        n_elements: usize,
        mut data: EclData,
        input: &[u8],
    ) -> ah::Result<EclData> {
        // populate the actual array body from sub-blocks
        let mut n_remaining_elements = n_elements;
        let mut remaining_input = input;

        while n_remaining_elements > 0 {
            let to_read = min(data.block_length(), n_remaining_elements);
            let (raw_data, input) =
                single_record_with_size(to_read * data.element_size(), remaining_input)
                    .with_context(|| {
                        format!(
                            "Failed to read a data body sub-block of size {}.",
                            to_read * data.element_size()
                        )
                    })?;
            data.append(raw_data);
            n_remaining_elements -= to_read;
            remaining_input = input;
        }
        Ok(data)
    }
}

/// A single binary keyword.
#[derive(Debug, PartialEq)]
pub struct EclKeyword {
    pub name: FixedString,
    pub data: EclData,
}

/// A binary file is an iterator over keywords read from a file one at a time.
#[derive(Debug)]
pub struct EclBinaryFile {
    reader: BufReader<File>,
}

impl EclBinaryFile {
    pub fn new<P: AsRef<Path>>(path: P) -> ah::Result<Self> {
        let file = File::open(path).with_context(|| "Failed to open file at requested path")?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }

    fn next_keyword(&mut self) -> ah::Result<EclKeyword> {
        // Look at the next 24 bytes and try reading the header;
        let mut header_buf = [0u8; 24];
        self.reader.read_exact(&mut header_buf)?;

        let (name, n_elements, data) = parsing::keyword_header(&header_buf)?;

        // Compute how much bytes we need to read the data and extract it;
        let need_bytes = data.bytes_for_elements(n_elements);
        let mut data_buf = vec![0; need_bytes];
        self.reader.read_exact(&mut data_buf)?;

        let data = parsing::keyword_data(n_elements, data, &data_buf)?;

        Ok(EclKeyword { name, data })
    }
}

impl Iterator for EclBinaryFile {
    type Item = EclKeyword;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_keyword() {
            Ok(kw) => Some(kw),
            Err(e) => match e.downcast_ref::<EclError>() {
                Some(_) => {
                    println!("{:?}", e);
                    None
                }
                None => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_record_test() {
        let input = include_bytes!("../assets/single_record.bin");
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
        let input = include_bytes!("../assets/single_data_array.bin");
        let (name, n_elements, data) = parsing::keyword_header(&input[..24]).unwrap();
        let data = parsing::keyword_data(n_elements, data, &input[24..]).unwrap();
        let kw = EclKeyword { name, data };

        assert_eq!(kw.name, FixedString::from("KEYWORDS").unwrap());

        assert_eq!(
            kw.data,
            EclData::FixStr(
                vec!["FOPR", "FGPR", "FWPR", "WOPR", "WGPR"]
                    .iter()
                    .map(|&s| FixedString::from(s).unwrap())
                    .collect()
            )
        );
    }
}
