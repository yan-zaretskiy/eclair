use crate::eclipse_binary::{for_keyword_in, BinFile, BinRecord, FixedString};
use crate::errors::SummaryError;

use anyhow as ah;
use log::debug;
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_bytes;

use std::collections::{HashMap, HashSet};

static TIMING_KEYWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("TIME");
    s.insert("YEARS");
    s.insert("DAY");
    s.insert("MONTH");
    s.insert("YEAR");
    s
});

static PERFORMANCE_KEYWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("ELAPSED");
    s.insert("MLINEARS");
    s.insert("MSUMLINS");
    s.insert("MSUMNEWT");
    s.insert("NEWTON");
    s.insert("NLINEARS");
    s.insert("TCPU");
    s.insert("TCPUDAY");
    s.insert("TCPUTS");
    s.insert("TIMESTEP");
    s.insert("MEMGB");
    s.insert("MAXMEMGB");
    s
});

static WEIRD_STRING: Lazy<FixedString> = Lazy::new(|| FixedString::from(":+:+:+:+").unwrap());

/// Units system used for a simulation run
#[derive(Debug)]
enum UnitSystem {
    Metric,
    Field,
    Lab,
    PvtM,
}

/// Represents metadata we collect for each time series.
#[derive(Debug, Default)]
struct SmspecItem {
    kw_name: FixedString,
    wg_name: FixedString,
    wg_long_name: String,
    num: i32,
    unit: FixedString,
}

/// Contents of an SMSPEC file. Note that we don't extract everything, only those bits that
/// are presently relevant to us. Notably, there is  related to LGRs.
/// This type could be extended later.
#[derive(Debug, Default)]
struct Smspec {
    units_system: Option<UnitSystem>,
    simulator_id: Option<i32>,
    nlist: i32,
    dims: [i32; 3],
    start_date: [i32; 6],

    items: Vec<SmspecItem>,
}

impl Smspec {
    pub fn new(smspec_file: BinFile) -> ah::Result<Self> {
        let mut smspec: Self = Default::default();

        // Parse the SMSPEC file for enough metadata to correctly place data records
        for_keyword_in(smspec_file, |kw| {
            match (kw.name.as_str(), &mut kw.data) {
                ("INTEHEAD", BinRecord::Int(header)) => {
                    smspec.units_system = match header[0] {
                        1 => Some(UnitSystem::Metric),
                        2 => Some(UnitSystem::Field),
                        3 => Some(UnitSystem::Lab),
                        4 => Some(UnitSystem::PvtM),
                        id => return Err(SummaryError::InvalidUnitSystemId(id).into()),
                    };
                    smspec.simulator_id = Some(header[1]);
                }
                ("DIMENS", BinRecord::Int(dims)) => {
                    smspec.nlist = dims[0];
                    smspec.items.resize_with(dims[0] as usize, Default::default);
                    smspec.dims.copy_from_slice(&dims[1..4]);
                }
                ("STARTDAT", BinRecord::Int(data)) => {
                    if data.len() == 3 {
                        smspec.start_date[..3].copy_from_slice(&data);
                    } else if data.len() == 6 {
                        smspec.start_date.copy_from_slice(&data);
                    } else {
                        return Err(SummaryError::InvalidStartDateLength(data.len()).into());
                    }
                }
                ("KEYWORDS", BinRecord::FixStr(data)) => {
                    for (item, kw_name) in smspec.items.iter_mut().zip(data) {
                        item.kw_name = *kw_name;
                    }
                }
                ("WGNAMES", BinRecord::FixStr(data)) => {
                    for (item, wg_name) in smspec.items.iter_mut().zip(data) {
                        item.wg_name = *wg_name;
                    }
                }
                ("NAMES", BinRecord::DynStr(_, data)) => {
                    for (item, wg_long_name) in smspec.items.iter_mut().zip(data) {
                        item.wg_long_name = wg_long_name.drain(..).collect();
                    }
                }
                ("NUMS", BinRecord::Int(data)) => {
                    for (item, num) in smspec.items.iter_mut().zip(data) {
                        item.num = *num;
                    }
                }
                ("UNITS", BinRecord::FixStr(data)) => {
                    for (item, unit) in smspec.items.iter_mut().zip(data) {
                        item.unit = *unit;
                    }
                }
                _ => {
                    debug!(target: "Parsing SMSPEC", "Unsupported SMSPEC keyword: {:?}", kw);
                }
            }
            Ok(())
        })?;
        Ok(smspec)
    }
}

/// Contents of an UNSMRY file stored as raw bytes.
#[derive(Debug, Default)]
struct Unsmry(Vec<Vec<u8>>);

impl Unsmry {
    pub fn new(unsmry_file: BinFile, nlist: i32) -> ah::Result<Self> {
        let mut unsmry = Unsmry(vec![Default::default(); nlist as usize]);

        // Read data from the UNSMRY file
        for_keyword_in(unsmry_file, |kw| {
            if let ("PARAMS", BinRecord::F32Bytes(params)) = (kw.name.as_str(), &kw.data) {
                for (values, param) in unsmry
                    .0
                    .iter_mut()
                    .zip(params.chunks(std::mem::size_of::<f32>()))
                {
                    values.extend_from_slice(param)
                }
            }
            Ok(())
        })?;
        Ok(unsmry)
    }
}

const VEC_EXT_CODE: i8 = 2;

#[derive(Debug, Default, Serialize)]
#[serde(rename = "_ExtStruct")]
struct ExtVec((i8, serde_bytes::ByteBuf));

#[derive(Debug, Default, Serialize)]
struct SummaryRecord {
    /// Physical units for the values
    unit: FixedString,

    /// Actual data
    values: ExtVec,
}

#[derive(Debug, Serialize, Default)]
pub struct Summary {
    /// Simulation start date
    start_date: [i32; 6],

    /// Time data, should always be present
    time: HashMap<FixedString, SummaryRecord>,

    /// Performance data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    performance: HashMap<FixedString, SummaryRecord>,

    /// Field data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    field: HashMap<FixedString, SummaryRecord>,

    /// Region data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    regions: HashMap<i32, HashMap<FixedString, SummaryRecord>>,

    /// Aquifer data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    aquifers: HashMap<i32, HashMap<FixedString, SummaryRecord>>,

    /// Well data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    wells: HashMap<FixedString, HashMap<FixedString, SummaryRecord>>,

    /// Well completion data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    completions: HashMap<(FixedString, i32), HashMap<FixedString, SummaryRecord>>,

    /// Group data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    groups: HashMap<FixedString, HashMap<FixedString, SummaryRecord>>,

    /// Cell data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    blocks: HashMap<i32, HashMap<FixedString, SummaryRecord>>,
}

impl Summary {
    pub fn new(smspec_file: BinFile, unsmry_file: BinFile) -> ah::Result<Self> {
        let smspec = Smspec::new(smspec_file)?;
        let unsmry = Unsmry::new(unsmry_file, smspec.nlist)?;

        // We read all the data, now we place it in appropriate hash maps.
        let mut summary = Summary {
            start_date: smspec.start_date,
            ..Default::default()
        };

        for (item, values) in smspec.items.iter().zip(unsmry.0) {
            let mut hm = HashMap::new();

            hm.insert(
                item.kw_name,
                SummaryRecord {
                    unit: item.unit,
                    values: ExtVec((VEC_EXT_CODE, serde_bytes::ByteBuf::from(values))),
                },
            );

            let name = item.kw_name.as_str();
            if TIMING_KEYWORDS.contains(name) {
                summary.time.extend(hm);
            } else if PERFORMANCE_KEYWORDS.contains(name) {
                summary.performance.extend(hm);
            } else {
                let wg_is_valid = item.wg_name.len() > 0 && item.wg_name != *WEIRD_STRING;
                let num_is_valid = item.num > 0;

                match &name[0..1] {
                    "F" => {
                        summary.field.extend(hm);
                    }
                    "R" if num_is_valid => {
                        summary.regions.entry(item.num).or_default().extend(hm);
                    }
                    "A" if num_is_valid => {
                        summary.aquifers.entry(item.num).or_default().extend(hm);
                    }
                    "W" if wg_is_valid => {
                        summary.wells.entry(item.wg_name).or_default().extend(hm);
                    }
                    "C" if wg_is_valid && num_is_valid => {
                        summary
                            .completions
                            .entry((item.wg_name, item.num))
                            .or_default()
                            .extend(hm);
                    }
                    "G" if wg_is_valid => {
                        summary.groups.entry(item.wg_name).or_default().extend(hm);
                    }
                    "B" if num_is_valid => {
                        summary.blocks.entry(item.num).or_default().extend(hm);
                    }
                    _ => {
                        debug!(target: "Building Summary",
                            "Skipped a summary item. KEYWORD: {}, WGNAME: {}, NUM: {}",
                            name, item.wg_name, item.num
                        );
                        continue;
                    }
                }
            }
        }
        Ok(summary)
    }
}
