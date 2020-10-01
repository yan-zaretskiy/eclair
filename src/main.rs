use eclair_io::{records::ZmqConnection, summary::Summary};
use std::time::Duration;

struct Args {
    server: String,
    port: i32,
    token: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = pico_args::Arguments::from_env();

    let args = Args {
        server: args.value_from_str(["-s", "--server"])?,
        port: args.value_from_str(["-p", "--port"])?,
        token: args.value_from_str(["-t", "--token"])?,
    };

    // A ZeroMQ connection to Echelon
    let mut reader = ZmqConnection::new(&args.server, args.port, "eclair")?;

    // Stream a CSV to screen
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(std::io::stdout());

    // Send the data request.
    reader.send(&args.token, 0)?;

    let mut summary = Summary::new(&mut reader)?;

    let names: Vec<String> = summary
        .item_ids
        .keys()
        .map(|item_id| format!("{:^10}", item_id.name.as_str()))
        .collect();
    wtr.write_record(names)?;

    let quals: Vec<String> = summary
        .item_ids
        .keys()
        .map(|item_id| format!("{:^10}", item_id.qualifier.to_string()))
        .collect();
    wtr.write_record(quals)?;

    let units: Vec<String> = summary
        .item_ids
        .iter()
        .map(|(_, index)| format!("{:^10}", summary.items[*index].unit.as_str()))
        .collect();
    wtr.write_record(units)?;

    loop {
        summary.update(&mut reader, Some(1))?;
        let new_values: Vec<String> = summary
            .item_ids
            .iter()
            .map(|(_, index)| format!("{:^10}", summary.items[*index].values.last().unwrap()))
            .collect();
        wtr.write_record(new_values)?;
        wtr.flush()?;
    }
}
