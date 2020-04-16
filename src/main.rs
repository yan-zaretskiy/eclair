mod eclipse_binary;
mod eclipse_summary;
mod errors;

use crate::eclipse_binary::BinFile;
use crate::eclipse_summary::Summary;
use crate::errors::FileError;

use anyhow as ah;
use env_logger::{Builder, Env};
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
}

fn init_logger() {
    let env = Env::default()
        .filter_or("ECLAIR_LOG_LEVEL", "info")
        .write_style_or("ECLAIR_LOG_STYLE", "auto");

    let mut builder = Builder::from_env(env);

    builder.format_timestamp(None).init();
}

fn main() -> ah::Result<()> {
    // Initialize the logger as soon as possible
    init_logger();

    // Read the CLI arguments
    let opt = Opt::from_args();

    let input_path = opt.input;

    // If there is no stem, bail early
    if input_path.file_stem().is_none() {
        return Err(FileError::InvalidFilePath.into());
    }

    // we allow either extension or no extension at all
    if let Some(ext) = input_path.extension() {
        let ext = ext.to_str();
        if ext != Some("SMSPEC") && ext != Some("UNSMRY") {
            return Err(FileError::InvalidFileExt.into());
        }
    }

    let smspec = BinFile::new(input_path.with_extension("SMSPEC"))?;
    let unsmry = BinFile::new(input_path.with_extension("UNSMRY"))?;

    let summary = Summary::new(smspec, unsmry)?;

    let mut out_file = File::create(
        opt.output
            .unwrap_or_else(|| input_path.with_extension("mpk")),
    )?;

    // serialize summary data in the MessagePack format
    rmps::encode::write_named(&mut out_file, &summary)?;

    Ok(())
}
