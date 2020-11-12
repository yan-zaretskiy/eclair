use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    thread,
};

use crossbeam_channel::Receiver;

use crate::{
    summary::{
        InitializeSummary, ItemId, ItemQualifier, Summary, SummaryFileReader, UpdateSummary,
    },
    FlexString, Result,
};

#[cfg(feature = "read_zmq")]
use crate::zmq::ZmqConnection;

struct UpdatableSummary {
    data: Summary,
    updater_thread: thread::JoinHandle<()>,
    receiver: Receiver<Vec<f32>>,
}

/// SummaryManager owns all summary data from multiple sources. It can update the data and accept
/// queries for individual summary item values.
pub struct SummaryManager {
    summaries: HashMap<String, UpdatableSummary>,
}

impl SummaryManager {
    pub fn new() -> Self {
        SummaryManager {
            summaries: HashMap::new(),
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

        self.summaries.insert(
            name.to_string(),
            UpdatableSummary {
                data,
                updater_thread,
                receiver,
            },
        );

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
    pub fn refresh(&mut self) -> Result<()> {
        for (_, summary) in &mut self.summaries {
            loop {
                if let Ok(params) = summary.receiver.try_recv() {
                    println!("Received params: {:?}", params);
                    summary.data.append(params);
                } else {
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn all_item_ids(&self) -> HashSet<&ItemId> {
        let mut ids = HashSet::new();

        for (_, summary) in &self.summaries {
            for key in summary.data.item_ids.keys() {
                ids.insert(key);
            }
        }
        ids
    }

    /// Get optional values for an item id from all summary sources.
    fn get_items_for_id(&self, id: ItemId) -> HashMap<&str, Option<&[f32]>> {
        let mut items = HashMap::new();

        for (name, summary) in &self.summaries {
            let value = summary
                .data
                .item_ids
                .get(&id)
                .map(|index| summary.data.items[*index].values.as_slice());
            items.insert(name.as_str(), value);
        }
        items
    }

    pub fn performance_item<'a>(&'a self, name: &'_ str) -> HashMap<&'a str, Option<&'a [f32]>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Performance,
        })
    }

    pub fn field_item<'a>(&'a self, name: &'_ str) -> HashMap<&'a str, Option<&'a [f32]>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Field,
        })
    }

    pub fn aquifer_item<'a>(
        &'a self,
        name: &'_ str,
        index: i32,
    ) -> HashMap<&'a str, Option<&'a [f32]>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Aquifer { index },
        })
    }

    pub fn block_item<'a>(
        &'a self,
        name: &'_ str,
        index: i32,
    ) -> HashMap<&'a str, Option<&'a [f32]>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Block { index },
        })
    }

    pub fn well_item<'a>(
        &'a self,
        name: &'_ str,
        well_name: &'_ str,
    ) -> HashMap<&'a str, Option<&'a [f32]>> {
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
    ) -> HashMap<&'a str, Option<&'a [f32]>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Group {
                wg_name: FlexString::from_str(group_name),
            },
        })
    }

    pub fn region_item<'a>(
        &'a self,
        name: &'_ str,
        index: i32,
    ) -> HashMap<&'a str, Option<&'a [f32]>> {
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
    ) -> HashMap<&'a str, Option<&'a [f32]>> {
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
    ) -> HashMap<&'a str, Option<&'a [f32]>> {
        self.get_items_for_id(ItemId {
            name: FlexString::from_str(name),
            qualifier: ItemQualifier::Completion {
                wg_name: FlexString::from_str(well_name),
                index,
            },
        })
    }
}
