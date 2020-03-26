mod eclipse_binary;
mod eclipse_summary;
mod errors;

use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow as ah;
use structopt::StructOpt;

use crate::eclipse_binary::EclBinaryFile;
use crate::eclipse_summary::EclSummary;
use crate::errors::EclError;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "ecl2hdf",
    about = "A converter of Eclipse summary files to HDF5."
)]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}

fn main() -> ah::Result<()> {
    let opt = Opt::from_args();

    let input_path = opt.input;

    // If there is no stem, bail early
    if input_path.file_stem().is_none() {
        return Err(EclError::InvalidFilePath.into());
    }

    let mut smspec_path = input_path.clone();
    let mut unsmry_path = input_path.clone();

    let smspec_ext = OsStr::new("SMSPEC");
    let unsmry_ext = OsStr::new("UNSMRY");

    // we allow either extension or no extension at all
    match input_path.extension() {
        Some(v) if v == smspec_ext || v == unsmry_ext => {
            smspec_path.set_extension(smspec_ext);
            unsmry_path.set_extension(unsmry_ext);
        }
        // same action, but you can't mix pattern guards and multiple patterns together in Rust
        None => {
            smspec_path.set_extension(smspec_ext);
            unsmry_path.set_extension(unsmry_ext);
        }
        Some(_) => return Err(EclError::InvalidFileExt.into()),
    };

    let smspec = EclBinaryFile::new(smspec_path)?;
    let unsmry = EclBinaryFile::new(unsmry_path)?;

    let summary = EclSummary::new(smspec, unsmry);
    println!("Read: {:#?}", summary);

    Ok(())
}
