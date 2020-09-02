use eclair::{binary::BinFile, diff::diff, dump::dump, summary::Summary, to_csv::to_csv_from_path};

use anyhow as ah;
use env_logger::{Builder, Env};
use rmp_serde as rmps;
use serde::Serialize;
use structopt::StructOpt;

use std::{fs::File, io::Write, path::PathBuf};

#[derive(StructOpt)]
#[structopt(
    name = "eclair",
    about = "Tool suite to manage Eclipse summary files.",
    author = "Yan Zaretskiy"
)]
enum Opt {
    /// Convert an Eclipse summary file to MessagePack format
    ToMpk {
        /// Input summary file
        #[structopt(parse(from_os_str))]
        input: PathBuf,

        /// Output MessagePack file
        #[structopt(parse(from_os_str), short, long)]
        output: Option<PathBuf>,
    },
    /// Print Eclipse summary file in CSV format
    ToCsv {
        /// Input summary file
        #[structopt(parse(from_os_str))]
        input: PathBuf,
    },
    /// Compare two Eclipse summary files
    Diff {
        /// Input summary file
        #[structopt(parse(from_os_str))]
        input: PathBuf,

        /// Reference summary file
        #[structopt(parse(from_os_str))]
        reference: PathBuf,

        /// Output HTML file
        #[structopt(parse(from_os_str), short, long)]
        output: Option<PathBuf>,
    },
    /// Dump raw contents of an Eclipse binary file
    Dump {
        /// Input file
        #[structopt(parse(from_os_str))]
        input: PathBuf,
    },
}

fn init_logger() {
    let env = Env::default()
        .filter_or("ECLAIR_LOG_LEVEL", "info")
        .write_style_or("ECLAIR_LOG_STYLE", "auto");

    let mut builder = Builder::from_env(env);
    builder
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} - {}] {}",
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();
}

fn main() -> ah::Result<()> {
    // Initialize the logger as soon as possible.
    init_logger();

    // Read the CLI arguments.
    let opt = Opt::from_args();

    match opt {
        Opt::ToMpk { input, output } => {
            // Parse SMSPEC & UNSMRY files.
            let summary = Summary::from_path(&input)?;
            // Create an output file.
            let mut out_file = File::create(output.unwrap_or_else(|| input.with_extension("mpk")))?;

            // serialize summary data to the output file in the MessagePack format.
            let mut se = rmps::encode::Serializer::new(&mut out_file)
                .with_struct_map()
                .with_string_variants();
            summary.serialize(&mut se)?;
        }
        Opt::Diff {
            input,
            reference,
            output,
        } => {
            // Parse SMSPEC & UNSMRY files.
            let candidate = Summary::from_path(&input)?;
            let reference = Summary::from_path(&reference)?;
            diff(&candidate, &reference, output.as_ref());
        }
        Opt::Dump { input } => {
            let bin_file = BinFile::from_path(&input)?;
            dump(bin_file)?;
        }
        Opt::ToCsv { input } => {
            to_csv_from_path(&input)?;
        }
    }
    Ok(())
}
