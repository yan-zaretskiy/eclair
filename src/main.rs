mod eclipse_binary;
mod eclipse_summary;
mod errors;

use crate::eclipse_binary::EclBinFile;
use crate::eclipse_summary::EclSummary;
use crate::errors::EclError;

use anyhow as ah;
use rmp_serde as rmps;
use structopt::StructOpt;

use std::{fs::File, io::prelude::*, path::PathBuf};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "eclair",
    about = "A converter of Eclipse summary files to MessagePack."
)]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Output file
    #[structopt(parse(from_os_str), short, long)]
    output: Option<PathBuf>,

    /// Prints debug info
    #[structopt(short, long)]
    debug: bool,

    /// Dump SMSPEC contents
    #[structopt(long)]
    dump_smspec: bool
}

fn main() -> ah::Result<()> {
    let opt = Opt::from_args();

    let input_path = opt.input;

    // If there is no stem, bail early
    if input_path.file_stem().is_none() {
        return Err(EclError::InvalidFilePath.into());
    }

    // we allow either extension or no extension at all
    if let Some(ext) = input_path.extension() {
        let ext = ext.to_str();
        if ext != Some("SMSPEC") && ext != Some("UNSMRY") {
            return Err(EclError::InvalidFileExt.into());
        }
    }

    let smspec = EclBinFile::new(input_path.with_extension("SMSPEC"))?;
    let unsmry = EclBinFile::new(input_path.with_extension("UNSMRY"))?;

    if opt.dump_smspec {
        for kw in smspec {
            println!("{:#?}", kw);
        }
        return Ok(());
    }

    let summary = EclSummary::new(smspec, unsmry, opt.debug)?;

    // serialize summary data in the MessagePack format
    let res = rmps::to_vec_named(&summary)?;

    let mut out_file = match opt.output {
        Some(p) => File::create(p)?,
        None => File::create(input_path.with_extension("mpk"))?,
    };

    out_file.write_all(&res)?;

    Ok(())
}
