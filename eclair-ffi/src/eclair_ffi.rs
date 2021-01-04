use eclair::{
    error::EclairError,
    summary::{ItemId as EclItemId, ItemQualifier as EclQualifier},
    summary_manager::SummaryManager as EclSM,
};

#[cxx::bridge(namespace = "eclair")]
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

    extern "Rust" {
        type SummaryManager;

        fn enable_logger();

        fn make_manager() -> Box<SummaryManager>;

        fn add_from_files(&mut self, input_path: &str, name: &str) -> Result<()>;
        fn add_from_network(
            &mut self,
            server: &str,
            port: i32,
            identity: &str,
            name: &str,
        ) -> Result<()>;

        fn remove(&mut self, index: usize) -> Result<()>;

        fn refresh(&mut self) -> Result<bool>;

        fn length(&self) -> usize;

        fn summary_name(&self, index: usize) -> &str;

        fn all_item_ids(&self) -> Vec<ItemId>;

        // TODO: Units.
        unsafe fn timestamps<'a>(&'a self, summary_idx: usize) -> &'a [i64];

        unsafe fn time_item<'a>(&'a self, summary_idx: usize, name: &'_ str) -> &'a [f32];

        unsafe fn performance_item<'a>(&'a self, summary_idx: usize, name: &'_ str) -> &'a [f32];

        unsafe fn field_item<'a>(&'a self, summary_idx: usize, name: &'_ str) -> &'a [f32];

        unsafe fn aquifer_item<'a>(
            &'a self,
            summary_idx: usize,
            name: &'_ str,
            index: i32,
        ) -> &'a [f32];

        unsafe fn block_item<'a>(
            &'a self,
            summary_idx: usize,
            name: &'_ str,
            index: i32,
        ) -> &'a [f32];

        unsafe fn well_item<'a>(
            &'a self,
            summary_idx: usize,
            name: &'_ str,
            well_name: &'_ str,
        ) -> &'a [f32];

        unsafe fn group_item<'a>(
            &'a self,
            summary_idx: usize,
            name: &'_ str,
            group_name: &'_ str,
        ) -> &'a [f32];

        unsafe fn region_item<'a>(
            &'a self,
            summary_idx: usize,
            name: &'_ str,
            index: i32,
        ) -> &'a [f32];

        unsafe fn cross_region_item<'a>(
            &'a self,
            summary_idx: usize,
            name: &'_ str,
            index: i32,
        ) -> &'a [f32];

        unsafe fn completion_item<'a>(
            &'a self,
            summary_idx: usize,
            name: &'_ str,
            well_name: &'_ str,
            index: i32,
        ) -> &'a [f32];
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

pub fn enable_logger() {
    env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .init()
}

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

    pub fn remove(&mut self, index: usize) -> Result<(), EclairError> {
        self.0.remove(index)
    }

    pub fn refresh(&mut self) -> Result<bool, EclairError> {
        self.0.refresh()
    }

    pub fn length(&self) -> usize {
        self.0.length()
    }

    pub fn summary_name(&self, index: usize) -> &str {
        self.0.name(index)
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

    pub fn timestamps(&self, summary_idx: usize) -> &[i64] {
        self.0.timestamps(summary_idx)
    }

    pub fn time_item<'a>(&'a self, summary_idx: usize, name: &'_ str) -> &'a [f32] {
        self.0.time_item(summary_idx, name).unwrap_or_default()
    }

    pub fn performance_item<'a>(&'a self, summary_idx: usize, name: &'_ str) -> &'a [f32] {
        self.0
            .performance_item(summary_idx, name)
            .unwrap_or_default()
    }

    pub fn field_item<'a>(&'a self, summary_idx: usize, name: &'_ str) -> &'a [f32] {
        self.0.field_item(summary_idx, name).unwrap_or_default()
    }

    pub fn aquifer_item<'a>(&'a self, summary_idx: usize, name: &'_ str, index: i32) -> &'a [f32] {
        self.0
            .aquifer_item(summary_idx, name, index)
            .unwrap_or_default()
    }

    pub fn block_item<'a>(&'a self, summary_idx: usize, name: &'_ str, index: i32) -> &'a [f32] {
        self.0
            .block_item(summary_idx, name, index)
            .unwrap_or_default()
    }

    pub fn well_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        well_name: &'_ str,
    ) -> &[f32] {
        self.0
            .well_item(summary_idx, name, well_name)
            .unwrap_or_default()
    }

    pub fn group_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        group_name: &'_ str,
    ) -> &'a [f32] {
        self.0
            .group_item(summary_idx, name, group_name)
            .unwrap_or_default()
    }

    pub fn region_item<'a>(&'a self, summary_idx: usize, name: &'_ str, index: i32) -> &'a [f32] {
        self.0
            .region_item(summary_idx, name, index)
            .unwrap_or_default()
    }

    pub fn cross_region_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        index: i32,
    ) -> &'a [f32] {
        let to = index / 32768 as i32 - 10;
        let from = index - 32768 * (to + 10);
        self.0
            .cross_region_item(summary_idx, name, from, to)
            .unwrap_or_default()
    }

    pub fn completion_item<'a>(
        &'a self,
        summary_idx: usize,
        name: &'_ str,
        well_name: &'_ str,
        index: i32,
    ) -> &'a [f32] {
        self.0
            .completion_item(summary_idx, name, well_name, index)
            .unwrap_or_default()
    }
}
