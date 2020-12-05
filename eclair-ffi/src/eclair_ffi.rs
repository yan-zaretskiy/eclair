use eclair_io::{
    error::EclairError,
    summary::{ItemId as EclItemId, ItemQualifier as EclQualifier},
    summary_manager::SummaryManager as EclSM,
};

#[cxx::bridge]
mod ffi {
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
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

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    pub(crate) struct ItemId {
        name: String,
        qualifier: ItemQualifier,
        index: i32,
        wg_name: String,
    }

    pub(crate) struct TimeSeries {
        // This is terrible. Can't wait for cxx to support slices.
        pub(crate) values: Vec<f64>,
        // In principle, units may differ between summary files for a given item.
        pub(crate) unit: String,
    }

    pub(crate) struct TimeStamps {
        pub(crate) values: Vec<f64>,
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

        fn refresh(&mut self) -> Result<bool>;

        fn length(&self) -> usize;
        fn summary_name(&self, index: usize) -> &str;

        fn all_item_ids(&self) -> Vec<ItemId>;

        fn unix_time(&self) -> Vec<TimeStamps>;
        fn time_item(&self, name: &str) -> Vec<TimeSeries>;
        fn performance_item(&self, name: &str) -> Vec<TimeSeries>;
        fn field_item(&self, name: &str) -> Vec<TimeSeries>;
        fn aquifer_item(&self, name: &str, index: i32) -> Vec<TimeSeries>;
        fn block_item(&self, name: &str, index: i32) -> Vec<TimeSeries>;
        fn well_item(&self, name: &str, well_name: &str) -> Vec<TimeSeries>;
        fn group_item(&self, name: &str, group_name: &str) -> Vec<TimeSeries>;
        fn region_item(&self, name: &str, index: i32) -> Vec<TimeSeries>;
        fn cross_region_item(&self, name: &str, index: i32) -> Vec<TimeSeries>;
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

    pub fn refresh(&mut self) -> Result<bool, EclairError> {
        self.0.refresh()
    }

    pub fn length(&self) -> usize {
        self.0.summary_names().len()
    }

    pub fn summary_name(&self, index: usize) -> &str {
        self.0.summary_names().get(index).map_or("", |name| *name)
    }

    pub fn summary_names(&self) -> Vec<String> {
        self.0
            .summary_names()
            .iter()
            .map(|name| name.to_string())
            .collect()
    }

    pub fn all_item_ids(&self) -> Vec<ffi::ItemId> {
        let mut ids: Vec<ffi::ItemId> = self
            .0
            .all_item_ids()
            .iter()
            .filter(|el| el.qualifier.is_recognized())
            .map(|&el| el.into())
            .collect();
        ids.sort();
        ids
    }

    fn item_to_ffi<'a>(item: Vec<Option<(&str, &[f32])>>) -> Vec<ffi::TimeSeries> {
        item.iter()
            .map(|data| {
                let (unit, values) = if let Some(data) = data {
                    (data.0.to_string(), data.1.to_vec())
                } else {
                    (String::new(), vec![])
                };

                ffi::TimeSeries {
                    values: values.iter().map(|el| *el as f64).collect(),
                    unit,
                }
            })
            .collect()
    }

    pub fn unix_time(&self) -> Vec<ffi::TimeStamps> {
        self.0
            .unix_time()
            .into_iter()
            .map(|values| ffi::TimeStamps {
                values: values.iter().map(|el| *el as f64).collect(),
            })
            .collect()
    }

    pub fn time_item(&self, name: &str) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.time_item(name))
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

    pub fn cross_region_item(&self, name: &str, index: i32) -> Vec<ffi::TimeSeries> {
        let to = index / 32768 as i32 - 10;
        let from = index - 32768 * (to + 10);
        SummaryManager::item_to_ffi(self.0.cross_region_item(name, from, to))
    }

    pub fn completion_item(&self, name: &str, well_name: &str, index: i32) -> Vec<ffi::TimeSeries> {
        SummaryManager::item_to_ffi(self.0.completion_item(name, well_name, index))
    }
}
