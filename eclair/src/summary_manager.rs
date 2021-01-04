use std::{borrow::Cow, collections::HashSet, thread};

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use crossbeam_channel::{Receiver, Sender};

#[cfg(feature = "read_zmq")]
use crate::zmq::ZmqConnection;
use crate::{
    summary::{
        InitializeSummary, ItemId, ItemQualifier, Summary, SummaryFileReader, UpdateSummary,
    },
    FlexString, Result,
};

struct UpdatableSummary {
    name: String,
    data: Summary,
    updater_thread: thread::JoinHandle<()>,

    // To receive data from the updater threads
    data_rcv: Receiver<Vec<f32>>,

    // To signal the threads that they need to terminate.
    term_snd: Sender<bool>,
}

/// SummaryManager owns all summary data from multiple sources. It can update the data and accept
/// queries for individual summary item values.
pub struct SummaryManager {
    summaries: Vec<UpdatableSummary>,
}

impl SummaryManager {
    pub fn new() -> Self {
        SummaryManager {
            summaries: Vec::new(),
        }
    }

    pub fn name(&self, index: usize) -> &str {
        self.summaries.get(index).map_or("", |s| s.name.as_str())
    }

    fn add<R: InitializeSummary>(&mut self, name: &str, reader: R) -> Result<()> {
        let (data, mut updater) = reader.init()?;

        // TODO: Once I'm done experimenting, make the channel size a SummaryManager config option.
        let (data_snd, data_rcv) = crossbeam_channel::bounded(10);

        let (term_snd, term_rcv) = crossbeam_channel::bounded(1);

        let updater_thread = thread::spawn(move || {
            if let Err(err) = updater.update(data_snd, term_rcv) {
                println!("Error during updating: {}", err);
            }
        });

        self.summaries.push(UpdatableSummary {
            name: name.to_string(),
            data,
            updater_thread,
            data_rcv,
            term_snd,
        });

        log::info!(target: "Summary Manager", "Added new summary object: {}", name);

        Ok(())
    }

    pub fn remove(&mut self, index: usize) -> Result<()> {
        // This should not fail unless there's a bug.
        self.summaries[index]
            .term_snd
            .send(true)
            .expect("Error sending a term request to the summary thread.");

        let s = self.summaries.remove(index);

        log::info!(target: "Summary Manager", "Removed summary object: {}", s.name);

        s.updater_thread
            .join()
            .expect("Error when waiting for the summary thread to join");

        Ok(())
    }

    pub fn length(&self) -> usize {
        self.summaries.len()
    }

    /// Add a new file-based summary data source.
    pub fn add_from_files<P>(&mut self, input_path: P, name: Option<&str>) -> Result<()>
    where
        P: AsRef<std::path::Path>,
    {
        let reader = SummaryFileReader::from_path(&input_path)?;
        let name = if let Some(n) = name {
            Cow::Borrowed(n)
        } else {
            // If we get here the file stem exists, so unwrapping if fine.
            input_path.as_ref().file_stem().unwrap().to_string_lossy()
        };

        self.add(&name, reader)
    }

    /// Add a new ZeroMQ-based summary data source.
    #[cfg(feature = "read_zmq")]
    pub fn add_from_network(
        &mut self,
        server: &str,
        port: i32,
        identity: &str,
        name: Option<&str>,
    ) -> Result<()> {
        let reader = ZmqConnection::new(server, port, identity)?;
        let name = if let Some(name) = name {
            name.to_owned()
        } else {
            format!("{}:{}", server, port)
        };

        self.add(&name, reader)
    }

    /// For each summary it tries to pull values from the corresponding receiver channel.
    pub fn refresh(&mut self) -> Result<bool> {
        let mut new_values = false;
        for summary in &mut self.summaries {
            loop {
                if let Ok(params) = summary.data_rcv.try_recv() {
                    new_values = true;
                    summary.data.append(params);
                } else {
                    break;
                }
            }
        }
        Ok(new_values)
    }

    pub fn all_item_ids(&self) -> HashSet<&ItemId> {
        let mut ids = HashSet::new();

        for summary in &self.summaries {
            ids.extend(summary.data.item_ids.keys());
        }
        ids
    }

    /// Get optional values for an item id from all summary sources.
    fn get_items_for_id(&self, summary_idx: usize, id: ItemId) -> Option<&[f32]> {
        self.summaries[summary_idx]
            .data
            .item_ids
            .get(&id)
            .map(|index| {
                self.summaries[summary_idx].data.items[*index]
                    .values
                    .as_slice()
            })
    }

    pub fn timestamps(&self, summary_idx: usize) -> &[i64] {
        self.summaries[summary_idx].data.timestamps.as_slice()
    }

    pub fn time_item<'a>(&'a self, summary_idx: usize, name: &'_ str) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::Time,
            },
        )
    }

    pub fn performance_item<'a>(&'a self, summary_idx: usize, name: &'_ str) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::Performance,
            },
        )
    }

    pub fn field_item<'a>(&'a self, summary_idx: usize, name: &'_ str) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::Field,
            },
        )
    }

    pub fn aquifer_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        index: i32,
    ) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::Aquifer { index },
            },
        )
    }

    pub fn block_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        index: i32,
    ) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::Block { index },
            },
        )
    }

    pub fn well_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        well_name: &'_ str,
    ) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::Well {
                    wg_name: FlexString::from_str(well_name),
                },
            },
        )
    }

    pub fn group_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        group_name: &'_ str,
    ) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::Group {
                    wg_name: FlexString::from_str(group_name),
                },
            },
        )
    }

    pub fn region_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        index: i32,
    ) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::Region {
                    wg_name: None,
                    index,
                },
            },
        )
    }

    pub fn cross_region_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        from: i32,
        to: i32,
    ) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::CrossRegionFlow { from, to },
            },
        )
    }

    pub fn completion_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        well_name: &'_ str,
        index: i32,
    ) -> Option<&[f32]> {
        self.get_items_for_id(
            summary_idx,
            ItemId {
                name: FlexString::from_str(name),
                qualifier: ItemQualifier::Completion {
                    wg_name: FlexString::from_str(well_name),
                    index,
                },
            },
        )
    }
}
