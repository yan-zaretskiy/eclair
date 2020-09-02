use std::mem;

use serde::Serialize;

use crate::binary::FlexString;
use crate::binary_parsing::{read_f32, read_f64};

const VEC_EXT_CODE: i8 = 2;

/// An item identifier derived from SMSPEC metadata.
#[derive(Debug, Eq, PartialEq, Serialize, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub struct ItemId {
    pub name: FlexString,
    pub qualifier: ItemQualifier,
}

#[derive(Debug, Eq, PartialEq, Serialize, Ord, PartialOrd)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ItemQualifier {
    Time,
    Performance,
    Field,
    Aquifer {
        index: i32,
    },
    Region {
        index: i32,
    },
    CrossRegionFlow {
        from: i32,
        to: i32,
    },
    Well {
        location: FlexString,
    },
    Completion {
        location: FlexString,
        index: i32,
    },
    Group {
        location: FlexString,
    },
    Block {
        index: i32,
    },
    Unrecognized {
        location: FlexString,
        index: i32,
    },
}

impl ItemId {
    pub fn location(&self) -> Option<String> {
        use ItemQualifier::*;
        match &self.qualifier {
            Time | Performance => None,
            Field => Some("Field".to_owned()),
            Aquifer { index } => Some(format!("Aquifer {}", index)),
            Region { index } => Some(format!("Region {}", index)),
            CrossRegionFlow { from, to } => Some(format!("X-Region: {} → {}", from, to)),
            Well { location } => Some(format!("Well {}", location)),
            Completion { location, index } => Some(format!("Well {}, Completion {}", location, index)),
            Group { location } => Some(format!("Group {}", location)),
            Block { index } => Some(format!("Block {}", index)),
            Unrecognized { location, index } => Some(format!("Unrecognized k/w. WGNAME: {}, NUMS: {}", location, index))
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "_ExtStruct")]
struct ExtVec(pub (i8, serde_bytes::ByteBuf));

/// Individual summary item with data combined from both SMSPEC and UNSMRY files.
#[derive(Debug, Serialize)]
pub struct SummaryItem {
    pub id: ItemId,
    pub unit: FlexString,

    values: ExtVec,
}

impl SummaryItem {
    pub fn new(name: FlexString, kind: ItemQualifier, unit: FlexString, values: Vec<u8>) -> Self {
        SummaryItem {
            id: ItemId{name, qualifier: kind },
            unit,
            values: ExtVec((VEC_EXT_CODE, serde_bytes::ByteBuf::from(values))),
        }
    }

    pub fn as_vec_of_f32(&self) -> Vec<f32> {
        self.values.
        0.1[..].chunks_exact(mem::size_of::<f32>()).map(|chunk| read_f32(chunk)).collect()
    }

    pub fn as_vec_of_f64(&self) -> Vec<f64> {
        self.values.
        0.1[..].chunks_exact(mem::size_of::<f64>()).map(|chunk| read_f64(chunk)).collect()
    }

    pub fn full_name(&self) -> String {
        if let Some(loc) = self.id.location() {
            format!("{} @ {}", self.id.name, loc)
        } else {
            self.id.name.to_string()
        }
    }
}