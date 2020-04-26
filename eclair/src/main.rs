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
    let input_path = opt.input;

    // Parse SMSPEC & UNSMRY files.
    let summary = Summary::from_path(&input_path)?;

    // Create an output file.
    let mut out_file = File::create(
        opt.output
            .unwrap_or_else(|| input_path.with_extension("mpk")),
    )?;

    // serialize summary data to the output file in the MessagePack format.
    let mut se = rmps::encode::Serializer::new(&mut out_file)
        .with_struct_map()
        .with_string_variants();
    summary.serialize(&mut se)?;

    Ok(())
}
