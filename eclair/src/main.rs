use eclair::eclipse_summary::Summary;

use anyhow as ah;
use env_logger::{Builder, Env};
use rmp_serde as rmps;
use structopt::StructOpt;

use serde::Serialize;
use std::{fs::File, io::Write, path::PathBuf};

#[derive(StructOpt)]
#[structopt(
    name = "eclair",
    about = "Tool suite to manage Eclipse summary files.",
    author = "Yan Zaretskiy"
)]
enum Opt {
    /// Convert an Eclipse summary file to MessagePack format
    Convert {
        /// Input summary file
        #[structopt(parse(from_os_str))]
        input: PathBuf,

        /// Output MessagePack file
        #[structopt(parse(from_os_str), short, long)]
        output: Option<PathBuf>,
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
        Opt::Convert { input, output } => {
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
            println!("Diff subcommand is presently unsupported...");
        }
    }
    Ok(())
}
