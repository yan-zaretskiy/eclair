use std::{io::Read, mem};

use anyhow as ah;

use crate::binary::{BinFile, BinRecord};
use crate::binary_parsing::{read_f32, read_f64};

pub fn print<R>(bin_file: BinFile<R>) -> ah::Result<()>
where
    R: Read,
{
    use BinRecord::*;

    bin_file.for_each_kw(|kw| {
        let (name, data) = (kw.name.as_str(), kw.data);
        match data {
            Int(v) => println!("{}: {:#?}", name, v),
            Boolean(v) => println!("{}: {:#?}", name, v),
            Chars(v) => println!("{}: {:#?}", name, v),
            F32Bytes(v) => {
                let v_f32: Vec<f32> = v
                    .as_slice()
                    .chunks_exact(mem::size_of::<f32>())
                    .map(|chunk| read_f32(chunk))
                    .collect();
                println!("{}: {:#?}", name, v_f32)
            }
            F64Bytes(v) => {
                let v_f64: Vec<f64> = v
                    .as_slice()
                    .chunks_exact(mem::size_of::<f64>())
                    .map(|chunk| read_f64(chunk))
                    .collect();
                println!("{}: {:#?}", name, v_f64)
            }
            Message => println!("{}: Message record with no values.", name),
        }
        Ok(())
    })?;
    Ok(())
}
