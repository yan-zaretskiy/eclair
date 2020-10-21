use eclair_io::{
    summary::{InitializeSummary, UpdateSummary},
    zmq::ZmqConnection,
};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = ZmqConnection::new("hostname", 55555, "eclair")?;
    let (summary, mut updater) = reader.init()?;

    println!("{:#?}", summary);

    let (s, r) = crossbeam_channel::bounded(10);

    thread::spawn(move || {
        if let Err(err) = updater.update(s) {
            println!("Error during updating: {}", err)
        }
    });

    loop {
        match r.try_recv() {
            Ok(params) => println!("Received:\n{:?}", params),
            Err(crossbeam_channel::TryRecvError::Empty) => continue,
            Err(crossbeam_channel::TryRecvError::Disconnected) => break,
        }
    }

    Ok(())
}
