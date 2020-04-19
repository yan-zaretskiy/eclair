use crate::eclipse_binary::{for_keyword_in, BinFile, BinRecord, FlexString};
use crate::errors::SummaryError;

use anyhow as ah;
use once_cell::sync::Lazy;
use serde::Serialize;

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
    kw_name: FlexString,
    wg_short_name: FlexString,
    wg_long_name: FlexString,
    num: i32,
    unit: FlexString,
}

enum ItemId<'a> {
    Time,
    Performance,
    Field,
    Aquifer(i32),
    Region(i32),
    NamedRegion(&'a FlexString, i32),
    CrossRegionFlow(i32, i32),
    Well(&'a FlexString),
    Completion(&'a FlexString, i32),
    Group(&'a FlexString),
    Block(i32),
}

impl SmspecItem {
    fn wg_name(&self) -> Option<&FlexString> {
        if !self.wg_short_name.is_empty() && &self.wg_short_name[..] != ":+:+:+:+" {
            Some(&self.wg_short_name)
        } else if !self.wg_long_name.is_empty() && &self.wg_long_name[..] != ":+:+:+:+" {
            Some(&self.wg_long_name)
        } else {
            None
        }
    }

    fn identify(&self) -> Option<ItemId> {
        use ItemId::*;

        let name = self.kw_name.as_str();
        let wg_name = self.wg_name();
        if TIMING_KEYWORDS.contains(name) {
            Some(Time)
        } else if PERFORMANCE_KEYWORDS.contains(name) {
            Some(Performance)
        } else {
            match name.as_bytes() {
                [b'F', ..] => Some(Field),
                [b'A', ..] if self.num > 0 => Some(Aquifer(self.num)),
                [b'R', b'N', b'L', b'F', ..] | [b'R', _, b'F', ..] if self.num > 0 => {
                    let region2 = self.num / 32768 as i32 - 10;
                    let region1 = self.num - 32768 * (region2 + 10);
                    Some(CrossRegionFlow(region1, region2))
                }
                [b'R', ..] if self.num > 0 => {
                    if let Some(wg) = wg_name {
                        Some(NamedRegion(wg, self.num))
                    } else {
                        Some(Region(self.num))
                    }
                }
                [b'W', ..] if wg_name.is_some() => Some(Well(wg_name.unwrap())),
                [b'C', ..] if wg_name.is_some() && self.num > 0 => {
                    Some(Completion(wg_name.unwrap(), self.num))
                }
                [b'G', ..] if wg_name.is_some() => Some(Group(wg_name.unwrap())),
                [b'B', ..] if self.num > 0 => Some(Block(self.num)),

                _ => {
                    log::debug!(target: "Building Summary",
                        "Skipped a summary item. KEYWORD: {}, WGNAME: {}, NAME: {}, NUM: {}",
                        self.kw_name, self.wg_short_name, self.wg_long_name, self.num
                    );
                    None
                }
            }
        }
    }
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
                ("DIMENS", BinRecord::Int(dimens)) => {
                    log::trace!(target: "Parsing SMSPEC", "DIMENS: {:?}", dimens);
                    smspec.nlist = dimens[0];
                    smspec
                        .items
                        .resize_with(dimens[0] as usize, Default::default);
                    smspec.dims.copy_from_slice(&dimens[1..4]);
                }
                ("STARTDAT", BinRecord::Int(start_dat)) => {
                    log::trace!(target: "Parsing SMSPEC", "STARTDAT: {:?}", start_dat);
                    if start_dat.len() == 3 {
                        smspec.start_date[..3].copy_from_slice(&start_dat);
                    } else if start_dat.len() == 6 {
                        smspec.start_date.copy_from_slice(&start_dat);
                    } else {
                        return Err(SummaryError::InvalidStartDateLength(start_dat.len()).into());
                    }
                }
                ("KEYWORDS", BinRecord::Chars(keywords)) => {
                    log::trace!(target: "Parsing SMSPEC", "KEYWORDS: {:?}", keywords);
                    for (item, kw_name) in smspec.items.iter_mut().zip(keywords) {
                        item.kw_name = kw_name.drain().collect();
                    }
                }
                ("WGNAMES", BinRecord::Chars(wgnames)) => {
                    log::trace!(target: "Parsing SMSPEC", "WGNAMES: {:?}", wgnames);
                    for (item, wg_name) in smspec.items.iter_mut().zip(wgnames) {
                        item.wg_short_name = wg_name.drain().collect();
                    }
                }
                ("NAMES", BinRecord::Chars(names)) => {
                    log::trace!(target: "Parsing SMSPEC", "NAMES: {:?}", names);
                    for (item, long_name) in smspec.items.iter_mut().zip(names) {
                        item.wg_long_name = long_name.drain().collect();
                    }
                }
                ("NUMS", BinRecord::Int(nums)) => {
                    log::trace!(target: "Parsing SMSPEC", "NUMS: {:?}", nums);
                    for (item, num) in smspec.items.iter_mut().zip(nums) {
                        item.num = *num;
                    }
                }
                ("UNITS", BinRecord::Chars(units)) => {
                    log::trace!(target: "Parsing SMSPEC", "UNITS: {:?}", units);
                    for (item, unit) in smspec.items.iter_mut().zip(units) {
                        item.unit = unit.drain().collect();
                    }
                }
                _ => {
                    if kw.name.as_str() != "MEASRMNT" {
                        log::debug!(target: "Parsing SMSPEC", "Unsupported SMSPEC keyword: {:#?}", kw);
                    }
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
    unit: FlexString,

    /// Actual data
    values: ExtVec,
}

#[derive(Debug, Serialize, Default)]
pub struct Summary {
    /// Simulation start date
    start_date: [i32; 6],

    /// Time data, should always be present
    time: HashMap<FlexString, SummaryRecord>,

    /// Region names
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    region_names: HashMap<i32, FlexString>,

    /// Performance data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    performance: HashMap<FlexString, SummaryRecord>,

    /// Field data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    field: HashMap<FlexString, SummaryRecord>,

    /// Aquifer data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    aquifers: HashMap<i32, HashMap<FlexString, SummaryRecord>>,

    /// Region data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    regions: HashMap<i32, HashMap<FlexString, SummaryRecord>>,

    /// Cell data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    cross_region_flows: HashMap<(i32, i32), HashMap<FlexString, SummaryRecord>>,

    /// Well data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    wells: HashMap<FlexString, HashMap<FlexString, SummaryRecord>>,

    /// Well completion data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    completions: HashMap<(FlexString, i32), HashMap<FlexString, SummaryRecord>>,

    /// Group data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    groups: HashMap<FlexString, HashMap<FlexString, SummaryRecord>>,

    /// Cell data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    blocks: HashMap<i32, HashMap<FlexString, SummaryRecord>>,
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

        for (item, values) in smspec.items.into_iter().zip(unsmry.0) {
            let id = item.identify();
            if id.is_none() {
                continue;
            }
            let id = id.unwrap();

            let values = ExtVec((VEC_EXT_CODE, serde_bytes::ByteBuf::from(values)));

            let mut hm = HashMap::new();
            hm.insert(
                item.kw_name.clone(),
                SummaryRecord {
                    unit: item.unit.clone(),
                    values,
                },
            );

            use ItemId::*;
            match id {
                Time => {
                    summary.time.extend(hm);
                }
                Performance => {
                    summary.performance.extend(hm);
                }
                Field => {
                    summary.field.extend(hm);
                }
                Aquifer(a) => {
                    summary.aquifers.entry(a).or_default().extend(hm);
                }
                Region(r_num) => {
                    summary.regions.entry(r_num).or_default().extend(hm);
                }
                NamedRegion(r, r_num) => {
                    summary.region_names.insert(r_num, r.clone());
                    summary.regions.entry(r_num).or_default().extend(hm);
                }
                CrossRegionFlow(r1, r2) => {
                    summary
                        .cross_region_flows
                        .entry((r1, r2))
                        .or_default()
                        .extend(hm);
                }
                Well(w) => {
                    summary.wells.entry(w.clone()).or_default().extend(hm);
                }
                Completion(c, w) => {
                    summary
                        .completions
                        .entry((c.clone(), w))
                        .or_default()
                        .extend(hm);
                }
                Group(g) => {
                    summary.groups.entry(g.clone()).or_default().extend(hm);
                }
                Block(b) => {
                    summary.blocks.entry(b).or_default().extend(hm);
                }
            }
        }
        Ok(summary)
    }
}
