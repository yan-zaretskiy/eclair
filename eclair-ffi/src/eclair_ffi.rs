use eclair_io::{
    error::EclairError,
    summary::{ItemId as EclItemId, ItemQualifier as EclQualifier},
    summary_manager::SummaryManager as EclSM,
};
use std::collections::HashMap;

#[cxx::bridge]
mod ffi {
    pub(crate) enum ItemQualifier {
        Time,
        Performance,
        Field,
        Aquifer,
        Region,
        CrossRegionFlow,
        Well,
        Completion,
        Group,
        Block,
        Unrecognized,
    }

    pub(crate) struct ItemId {
        name: String,
        qualifier: ItemQualifier,
        index: i32,
        wg_name: String,
    }

    pub(crate) struct TimeSeries {
        // &str in shared types are not supported yet.
        pub(crate) name: String,
        // This is terrible. Can't wait for cxx to support slices.
        pub(crate) values: Vec<f32>,
    }

    extern "Rust" {
        type SummaryManager;

        fn make_manager() -> Box<SummaryManager>;

        fn add_from_files(&mut self, input_path: &str, name: &str) -> Result<()>;
        fn add_from_network(
            &mut self,
            server: &str,
            port: i32,
            identity: &str,
            name: &str,
        ) -> Result<()>;

        fn refresh(&mut self) -> Result<()>;

        fn count_items(&self) -> usize;

        fn all_item_ids(&self) -> Vec<ItemId>;

        fn performance_item(&self, name: &str) -> Vec<TimeSeries>;
        fn field_item(&self, name: &str) -> Vec<TimeSeries>;
        fn aquifer_item(&self, name: &str, index: i32) -> Vec<TimeSeries>;
        fn block_item(&self, name: &str, index: i32) -> Vec<TimeSeries>;
        fn well_item(&self, name: &str, well_name: &str) -> Vec<TimeSeries>;
        fn group_item(&self, name: &str, group_name: &str) -> Vec<TimeSeries>;
        fn region_item(&self, name: &str, index: i32) -> Vec<TimeSeries>;
        fn cross_region_item(&self, name: &str, from: i32, to: i32) -> Vec<TimeSeries>;
        fn completion_item(&self, name: &str, well_name: &str, index: i32) -> Vec<TimeSeries>;
    }
}

impl From<&EclItemId> for ffi::ItemId {
    fn from(value: &EclItemId) -> Self {
        let name = value.name.to_string();
        let (qualifier, index, wg_name) = match &value.qualifier {
            EclQualifier::Time => (ffi::ItemQualifier::Time, -1, String::new()),
            EclQualifier::Performance => (ffi::ItemQualifier::Performance, -1, String::new()),
            EclQualifier::Field => (ffi::ItemQualifier::Field, -1, String::new()),
            EclQualifier::Aquifer { index } => (ffi::ItemQualifier::Aquifer, *index, String::new()),
            EclQualifier::Region { wg_name, index } => (
                ffi::ItemQualifier::Region,
                *index,
                if let Some(s) = wg_name {
                    s.to_string()
                } else {
                    String::new()
                },
            ),
            EclQualifier::CrossRegionFlow { from, to } => (
                ffi::ItemQualifier::CrossRegionFlow,
                *from + 32768 * (*to + 10),
                String::new(),
            ),
            EclQualifier::Well { wg_name } => (ffi::ItemQualifier::Well, -1, wg_name.to_string()),
            EclQualifier::Completion { wg_name, index } => {
                (ffi::ItemQualifier::Completion, *index, wg_name.to_string())
            }
            EclQualifier::Group { wg_name } => (ffi::ItemQualifier::Group, -1, wg_name.to_string()),
            EclQualifier::Block { index } => (ffi::ItemQualifier::Block, *index, String::new()),
            EclQualifier::Unrecognized { wg_name, index } => (
                ffi::ItemQualifier::Unrecognized,
                *index,
                wg_name.to_string(),
            ),
        };

        ffi::ItemId {
            name,
            qualifier,
            index,
            wg_name,
        }
    }
}

// Simple wrapper around the actual SummaryManager, required by cxx.
pub struct SummaryManager(EclSM);

pub fn make_manager() -> Box<SummaryManager> {
    Box::new(SummaryManager(EclSM::new()))
}

impl SummaryManager {
    pub fn add_from_files(&mut self, input_path: &str, name: &str) -> Result<(), EclairError> {
        self.0
            .add_from_files(input_path, if name.is_empty() { None } else { Some(name) })
    }

    pub fn add_from_network(
        &mut self,
        server: &str,
        port: i32,
        identity: &str,
        name: &str,
    ) -> Result<(), EclairError> {
        self.0.add_from_network(
            server,
            port,
            identity,
            if name.is_empty() { None } else { Some(name) },
        )
    }

    pub fn refresh(&mut self) -> Result<(), EclairError> {
        self.0.refresh()
    }

    pub fn count_items(&self) -> usize {
        self.0.all_item_ids().len()
    }

    pub fn all_item_ids(&self) -> Vec<ffi::ItemId> {
        self.0.all_item_ids().iter().map(|&el| el.into()).collect()
    }

    fn item_to_ffi<'a>(item: HashMap<&'a str, Option<&'a [f32]>>) -> Vec<ffi::TimeSeries> {
        item.iter()
            .map(|(name, data)| ffi::TimeSeries {
                name: name.to_string(),
                values: if let Some(data) = data {
                    data.to_vec()
                } else {
                    vec![]
                },
            })
            .collect()
    }

    pub fn performance_item(&self, name: &str) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.performance_item(name))
    }

    pub fn field_item(&self, name: &str) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.field_item(name))
    }

    pub fn aquifer_item(&self, name: &str, index: i32) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.aquifer_item(name, index))
    }

    pub fn block_item(&self, name: &str, index: i32) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.block_item(name, index))
    }

    pub fn well_item(&self, name: &str, well_name: &str) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.well_item(name, well_name))
    }

    pub fn group_item(&self, name: &str, group_name: &str) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.group_item(name, group_name))
    }

    pub fn region_item(&self, name: &str, index: i32) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.region_item(name, index))
    }

    pub fn cross_region_item(&self, name: &str, from: i32, to: i32) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.cross_region_item(name, from, to))
    }

    pub fn completion_item(&self, name: &str, well_name: &str, index: i32) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.completion_item(name, well_name, index))
    }
}
