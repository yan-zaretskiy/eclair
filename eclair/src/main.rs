use eclair_io::summary_manager::SummaryManager;
use std::{thread, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut summary_manager = SummaryManager::new();
    summary_manager.add_from_network("localhost", 23120, "eclair", None)?;

    let all_ids = summary_manager.all_item_ids();

    println!("{:#?}", all_ids);

    // loop {
    //     summary_manager.refresh()?;
    //     std::thread::sleep(Duration::from_millis(100));
    // }

    Ok(())
}
