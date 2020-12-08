use std::{borrow::Cow, collections::HashSet, thread};

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use crossbeam_channel::Receiver;

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
    receiver: Receiver<Vec<f32>>,
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

    fn add<R: InitializeSummary>(&mut self, name: &str, reader: R) -> Result<()> {
        let (data, mut updater) = reader.init()?;

        // TODO: Once I'm done experimenting, make the channel size a SummaryManager config option.
        let (sender, receiver) = crossbeam_channel::bounded(10);

        let updater_thread = thread::spawn(move || {
            if let Err(err) = updater.update(sender) {
                println!("Error during updating: {}", err);
            }
        });

        self.summaries.push(UpdatableSummary {
            name: name.to_string(),
            data,
            updater_thread,
            receiver,
        });

        Ok(())
    }

    pub fn remove(&mut self, index: usize) -> Result<()> {
        self.summaries.remove(index);
        Ok(())
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
                if let Ok(params) = summary.receiver.try_recv() {
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

    pub fn summary_names(&self) -> Vec<&str> {
        self.summaries.iter().map(|s| s.name.as_str()).collect()
    }

    /// Get optional unit and values for an item id from all summary sources.
    fn get_items_for_id(&self, id: ItemId) -> Vec<Option<(&str, &[f32])>> {
        self.summaries
            .iter()
            .map(|s| {
                s.data.item_ids.get(&id).map(|index| {
                    let item = &s.data.items[*index];
                    (item.unit.as_str(), item.values.as_slice())
                })
            })
            .collect()
    }

    pub fn unix_time(&self) -> Vec<Vec<i64>> {
        // All summaries contain the "TIME" vector, unwrap is OK.
        let times: Vec<&[f32]> = self
            .get_items_for_id(ItemId {
                name: FlexString::from_str("TIME"),
                qualifier: ItemQualifier::Time,
            })
            .iter()
            .map(|data| data.unwrap().1)
            .collect();

        let start_dates: Vec<&[i32; 6]> =
            self.summaries.iter().map(|s| &s.data.start_date).collect();

        times
            .into_iter()
            .zip(start_dates)
            .map(|(time, start_date)| {
                let d =
                    NaiveDate::from_ymd(start_date[2], start_date[1] as u32, start_date[0] as u32);
                let t = NaiveTime::from_hms_milli(
                    start_date[3] as u32,
                    start_date[4] as u32,
                    (start_date[5] / 1_000_000) as u32,
                    (start_date[5] % 1_000_000) as u32,
                );
                let dt = NaiveDateTime::new(d, t);

                time.iter()
                    .map(|days| {
                        let next_dt = dt + Duration::seconds((days * 86400.0) as i64);
                        next_dt.timestamp()
                    })
                    .collect()
            })
            .collect()
    }

    pub fn time_item<'a>(&'a self, name: &'_ str) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Time,
        })
    }

    pub fn performance_item<'a>(&'a self, name: &'_ str) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Performance,
        })
    }

    pub fn field_item<'a>(&'a self, name: &'_ str) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Field,
        })
    }

    pub fn aquifer_item<'a>(&'a self, name: &'_ str, index: i32) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Aquifer { index },
        })
    }

    pub fn block_item<'a>(&'a self, name: &'_ str, index: i32) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Block { index },
        })
    }

    pub fn well_item<'a>(
        &'a self,
        name: &'_ str,
        well_name: &'_ str,
    ) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Well {
                wg_name: FlexString::from_str(well_name),
            },
        })
    }

    pub fn group_item<'a>(
        &'a self,
        name: &'_ str,
        group_name: &'_ str,
    ) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Group {
                wg_name: FlexString::from_str(group_name),
            },
        })
    }

    pub fn region_item<'a>(&'a self, name: &'_ str, index: i32) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Region {
                wg_name: None,
                index,
            },
        })
    }

    pub fn cross_region_item<'a>(
        &'a self,
        name: &'_ str,
        from: i32,
        to: i32,
    ) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::CrossRegionFlow { from, to },
        })
    }

    pub fn completion_item<'a>(
        &'a self,
        name: &'_ str,
        well_name: &'_ str,
        index: i32,
    ) -> Vec<Option<(&str, &[f32])>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Completion {
                wg_name: FlexString::from_str(well_name),
                index,
            },
        })
    }
}
