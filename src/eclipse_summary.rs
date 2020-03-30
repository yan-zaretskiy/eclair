use std::collections::{HashMap, HashSet};

use itertools::izip;
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::eclipse_binary::{EclBinData, EclBinFile, FixedString};

static TIMING_KEYWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("TIME");
    s.insert("YEARS");
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
    s
});

static WEIRD_STRING: Lazy<FixedString> = Lazy::new(|| FixedString::from(":+:+:+:+").unwrap());

#[derive(Clone, Debug, Default, Serialize)]
struct EclSummaryRecord {
    /// Physical units for the values
    unit: FixedString,

    /// Actual data
    values: Vec<f32>,
}

#[derive(Debug, Serialize, Default)]
pub struct EclSummary {
    /// Simulation start date
    start_date: (i32, i32, i32),

    /// Time data, should always be present
    time: HashMap<FixedString, EclSummaryRecord>,

    /// Performance data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    perf: HashMap<FixedString, EclSummaryRecord>,

    /// Field data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    field: HashMap<FixedString, EclSummaryRecord>,

    /// Region data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    regions: HashMap<i32, HashMap<FixedString, EclSummaryRecord>>,

    /// Well data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    wells: HashMap<FixedString, HashMap<FixedString, EclSummaryRecord>>,

    /// Well completion data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    completions: HashMap<(FixedString, i32), HashMap<FixedString, EclSummaryRecord>>,

    /// Group data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    groups: HashMap<FixedString, HashMap<FixedString, EclSummaryRecord>>,

    /// Cell data
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    cells: HashMap<i32, HashMap<FixedString, EclSummaryRecord>>,
}

impl EclSummary {
    pub fn new(smspec: EclBinFile, unsmry: EclBinFile, debug: bool) -> Self {
        // 1. Parse the SMSPEC file for enough metadata to correctly place data records
        let mut start_date = (0, 0, 0);
        let mut names = Vec::new();
        let mut wgnames = Vec::new();
        let mut nums = Vec::new();
        let mut units = Vec::new();
        let mut all_values: Vec<Vec<f32>> = Vec::new();

        for kw in smspec {
            match (kw.name.as_str(), kw.data) {
                ("DIMENS", EclBinData::Int(dims)) => {
                    all_values.resize(dims[0] as usize, Default::default());
                }
                ("STARTDAT", EclBinData::Int(data)) => start_date = (data[0], data[1], data[2]),
                ("KEYWORDS", EclBinData::FixStr(data)) => {
                    names = data;
                }
                ("UNITS", EclBinData::FixStr(data)) => {
                    units = data;
                }
                ("WGNAMES", EclBinData::FixStr(data)) => {
                    wgnames = data;
                }
                ("NUMS", EclBinData::Int(data)) => {
                    nums = data;
                }
                _ => continue,
            }
        }

        // 2. Read data from the UNSMRY file
        for unsmry_kw in unsmry {
            match (unsmry_kw.name.as_str(), unsmry_kw.data) {
                ("PARAMS", EclBinData::Float(params)) => {
                    for (values, param) in izip!(&mut all_values, params) {
                        values.push(param)
                    }
                }
                _ => continue,
            }
        }

        // 3. Now we have all the data read, let't put it in where it belongs
        let mut summary = EclSummary {
            start_date,
            ..Default::default()
        };

        for (name, wg, num, unit, values) in izip!(names, wgnames, nums, units, all_values) {
            let mut hm = HashMap::new();
            hm.insert(name, EclSummaryRecord { unit, values });

            let name = name.as_str();
            if TIMING_KEYWORDS.contains(name) {
                summary.time.extend(hm);
            } else if PERFORMANCE_KEYWORDS.contains(name) {
                summary.perf.extend(hm);
            } else {
                let is_wg_valid = wg.len() > 0 && wg != *WEIRD_STRING;
                let is_num_valid = num > 0;

                match &name[0..1] {
                    "F" => {
                        summary.field.extend(hm);
                    }
                    "R" if is_num_valid => {
                        summary.regions.entry(num).or_default().extend(hm);
                    }
                    "W" if is_wg_valid => {
                        summary.wells.entry(wg).or_default().extend(hm);
                    }
                    "C" if is_wg_valid && is_num_valid => {
                        summary.completions.entry((wg, num)).or_default().extend(hm);
                    }
                    "G" if is_wg_valid => {
                        summary.groups.entry(wg).or_default().extend(hm);
                    }
                    "B" if is_num_valid => {
                        summary.cells.entry(num).or_default().extend(hm);
                    }
                    _ => {
                        if debug {
                            println!(
                                "Unknown vector. KEYWORD: {}, WGNAME: {}, NUM: {}",
                                name, wg, num
                            );
                        }
                        continue;
                    }
                }
            }
        }
        summary
    }
}
