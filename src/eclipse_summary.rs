use crate::eclipse_binary::{EclBinaryFile, EclData, FixedString};
use serde::Serialize;

use itertools::izip;
use phf::phf_set;

static TIMING_KEYWORDS: phf::Set<&'static str> = phf_set! {
    "TIME",
    "YEARS",
};

static PERFORMANCE_KEYWORDS: phf::Set<&'static str> = phf_set! {
    "ELAPSED",
    "MLINEARS",
    "MSUMLINS",
    "MSUMNEWT",
    "NEWTON",
    "NLINEARS",
    "TCPU",
    "TCPUDAY",
    "TCPUTS",
    "TIMESTEP",
};

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
enum VectorId {
    Unknown,
    Timing,
    Performance,
    Field,
    Well {
        well_name: FixedString,
    },
    WellCompletion {
        well_name: FixedString,
        completion_id: i32,
    },
    Group {
        group_name: FixedString,
    },
    Cell {
        cell_id: i32,
    },
    Region {
        region_id: i32,
    },
}

#[derive(Clone, Debug, Serialize)]
struct EclSummaryVector {
    /// Keyword name
    keyword: FixedString,

    /// Physical units for the values
    unit: FixedString,

    /// Vector identifier (well, field, group, etc)
    id: VectorId,

    /// Actual data
    values: Vec<f32>,
}

impl Default for EclSummaryVector {
    fn default() -> EclSummaryVector {
        EclSummaryVector {
            id: VectorId::Unknown,
            values: Vec::default(),
            keyword: FixedString::default(),
            unit: FixedString::default(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EclSummary {
    /// Simulation start date
    start_date: (i32, i32, i32),

    /// Collection of summary vectors
    data: Vec<EclSummaryVector>,
}

impl EclSummary {
    pub fn new(smspec: EclBinaryFile, unsmry: EclBinaryFile, debug: bool) -> Self {
        let mut start_date = (0, 0, 0);
        let mut data: Vec<EclSummaryVector> = vec![];
        let mut wgnames: Vec<FixedString> = vec![];
        let mut nums: Vec<i32> = vec![];

        // 1. Parse the SMSPEC file for enough metadata to identify individual data vectors
        for kw in smspec {
            match kw.name.as_str() {
                "DIMENS" => {
                    if let EclData::Int(v) = kw.data {
                        data.resize(v[0] as usize, Default::default());
                    }
                }
                "STARTDAT" => {
                    if let EclData::Int(v) = kw.data {
                        start_date = (v[0], v[1], v[2])
                    }
                }
                "KEYWORDS" => {
                    if let EclData::FixStr(v) = kw.data {
                        for (summary_vector, keyword) in data.iter_mut().zip(v) {
                            summary_vector.keyword = keyword;
                        }
                    }
                }
                "UNITS" => {
                    if let EclData::FixStr(v) = kw.data {
                        for (summary_vector, unit) in data.iter_mut().zip(v) {
                            summary_vector.unit = unit;
                        }
                    }
                }
                "WGNAMES" => {
                    if let EclData::FixStr(v) = kw.data {
                        wgnames = v.into_iter().collect();
                    }
                }
                "NUMS" => {
                    if let EclData::Int(v) = kw.data {
                        nums = v.into_iter().collect();
                    }
                }
                _ => continue,
            }
        }

        // 2. Build vector identifiers (global, well, region, cell, etc)
        for (d, wgname, num) in izip!(&mut data, wgnames, nums) {
            let kw = d.keyword.as_str();
            d.id = if TIMING_KEYWORDS.contains(kw) {
                VectorId::Timing
            } else if PERFORMANCE_KEYWORDS.contains(kw) {
                VectorId::Performance
            } else {
                let is_wg_name_valid =
                    wgname.len() > 0 && wgname != FixedString::from(":+:+:+:+").unwrap();

                match &kw[0..1] {
                    "F" => VectorId::Field,
                    "R" if num > 0 => VectorId::Region { region_id: num },
                    "W" if is_wg_name_valid => VectorId::Well { well_name: wgname },
                    "C" if is_wg_name_valid && num > 0 => VectorId::WellCompletion {
                        well_name: wgname,
                        completion_id: num,
                    },
                    "G" if is_wg_name_valid => VectorId::Group { group_name: wgname },
                    "B" if num > 0 => VectorId::Cell { cell_id: num },
                    _ => {
                        if debug {
                            println!("Unknown vector. KEYWORD: {}, WGNAME: {}, NUM: {}", kw, wgname, num);
                        }
                        VectorId::Unknown
                    },
                }
            };
        }

        // 3. Populate vectors with data from the UNSMRY file
        for kw in unsmry {
            if let "PARAMS" = kw.name.as_str() {
                if let EclData::Float(v) = kw.data {
                    for (d, param) in data.iter_mut().zip(v) {
                        match d.id {
                            VectorId::Unknown => continue,
                            _ => d.values.push(param)
                        }
                    }
                }
            }
        }

        Self { start_date, data }
    }
}
