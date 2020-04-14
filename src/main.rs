mod eclipse_binary;
mod eclipse_summary;
mod errors;

use crate::eclipse_binary::EclBinFile;
use crate::eclipse_summary::EclSummary;
use crate::errors::EclFileError;

use anyhow as ah;
use rmp_serde as rmps;
use structopt::StructOpt;

use std::{fs::File, path::PathBuf};

#[derive(StructOpt)]
#[structopt(
    name = "eclair",
    about = "A converter of Eclipse summary files to MessagePack.",
    author = "Yan Zaretskiy"
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
}

fn main() -> ah::Result<()> {
    let opt = Opt::from_args();

    let input_path = opt.input;

    // If there is no stem, bail early
    if input_path.file_stem().is_none() {
        return Err(EclFileError::InvalidFilePath.into());
    }

    // we allow either extension or no extension at all
    if let Some(ext) = input_path.extension() {
        let ext = ext.to_str();
        if ext != Some("SMSPEC") && ext != Some("UNSMRY") {
            return Err(EclFileError::InvalidFileExt.into());
        }
    }

    let smspec = EclBinFile::new(input_path.with_extension("SMSPEC"))?;
    let unsmry = EclBinFile::new(input_path.with_extension("UNSMRY"))?;

    let summary = EclSummary::new(smspec, unsmry, opt.debug)?;

    let mut out_file = File::create(
        opt.output
            .unwrap_or_else(|| input_path.with_extension("mpk")),
    )?;

    // serialize summary data in the MessagePack format
    rmps::encode::write_named(&mut out_file, &summary)?;

    Ok(())
}
