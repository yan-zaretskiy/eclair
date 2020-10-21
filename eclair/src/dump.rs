use std::{io::Read, mem};

use anyhow as ah;

use crate::{
    binary::{BinFile, BinRecord},
    binary_parsing::{read_f32, read_f64},
};

pub fn dump<R>(bin_file: BinFile<R>, pretty: bool) -> ah::Result<()>
where
    R: Read,
{
    use BinRecord::*;

    fn print<T>(name: &str, val: T, pretty: bool)
    where
        T: std::fmt::Debug,
    {
        if pretty {
            println!("{}: {:#?}", name, val);
        } else {
            println!("{}: {:?}", name, val);
        }
    }

    bin_file.for_each_kw(|kw| {
        let (name, data) = (kw.name.as_str(), kw.data);
        match data {
            Int(v) => print(name, v, pretty),
            Boolean(v) => print(name, v, pretty),
            Chars(v) => print(name, v, pretty),
            F32Bytes(v) => {
                let v_f32: Vec<f32> = v
                    .as_slice()
                    .chunks_exact(mem::size_of::<f32>())
                    .map(|chunk| read_f32(chunk))
                    .collect();
                print(name, v_f32, pretty)
            }
            F64Bytes(v) => {
                let v_f64: Vec<f64> = v
                    .as_slice()
                    .chunks_exact(mem::size_of::<f64>())
                    .map(|chunk| read_f64(chunk))
                    .collect();
                print(name, v_f64, pretty)
            }
            Message => println!("{}: Message record with no values.", name),
        }
        Ok(())
    })?;
    Ok(())
}
