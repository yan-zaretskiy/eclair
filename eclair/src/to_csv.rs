use crate::{
    binary::{BinFile, BinRecord},
    binary_parsing::read_f32,
    summary::{files_from_path, Smspec},
};
use anyhow as ah;
use std::{io, io::Read, path::Path};

fn to_csv<R1, R2>(smspec: R1, unsmry: R2) -> ah::Result<()>
where
    R1: Read,
    R2: Read,
{
    let smspec = Smspec::new(BinFile::new(smspec))?;
    let unsmry_file = BinFile::new(unsmry);

    let mut wtr = csv::Writer::from_writer(io::stdout());

    // headers
    wtr.write_record(smspec.items.iter().map(|item| item.name.as_str()))?;
    wtr.write_record(smspec.items.iter().map(|item| item.wg_name.as_str()))?;
    wtr.write_record(smspec.items.iter().map(|item| item.index.to_string()))?;
    wtr.write_record(smspec.items.iter().map(|item| item.unit.as_str()))?;

    // values
    // Read data from the UNSMRY file
    unsmry_file.for_each_kw(|kw| {
        if let ("PARAMS", BinRecord::F32Bytes(params)) = (kw.name.as_str(), kw.data) {
            wtr.write_record(
                params
                    .chunks_exact(std::mem::size_of::<f32>())
                    .map(|chunk| read_f32(chunk).to_string()),
            )?;
        }
        Ok(())
    })?;

    wtr.flush()?;
    Ok(())
}

pub fn to_csv_from_path<P: AsRef<Path>>(input_path: P) -> ah::Result<()> {
    let (smspec, unsmry) = files_from_path(input_path)?;
    to_csv(smspec, unsmry)?;
    Ok(())
}
