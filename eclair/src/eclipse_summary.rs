use crate::eclipse_binary::{for_keyword_in, BinFile, BinRecord, FlexString};
use crate::errors::{FileError, SummaryError};

use anyhow as ah;
use once_cell::sync::Lazy;
use serde::Serialize;

use std::collections::{HashMap, HashSet};
use std::path::Path;

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

/// Raw metadata we collect for each time series.
#[derive(Debug, Default)]
struct SmspecItem {
    name: FlexString,
    wg_name: FlexString,
    index: i32,
    unit: FlexString,
}

/// Contents of an SMSPEC file. Note that we don't extract everything, only those bits
/// that are presently relevant to us. Notably, there is no data related to LGRs.
/// This type could be extended later.
#[derive(Debug, Default)]
struct Smspec {
    units_system: Option<i32>,
    simulator_id: Option<i32>,
    nlist: i32,
    dims: [i32; 3],
    start_date: [i32; 6],

    items: Vec<SmspecItem>,
}

/// Contents of an UNSMRY file stored as raw bytes.
#[derive(Debug, Default)]
struct Unsmry(Vec<Vec<u8>>);

/// An item identifier derived from SMSPEC metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum ItemId {
    Time {
        name: FlexString,
    },
    Performance {
        name: FlexString,
    },
    Field {
        name: FlexString,
    },
    Aquifer {
        name: FlexString,
        index: i32,
    },
    Region {
        name: FlexString,
        index: i32,
    },
    CrossRegionFlow {
        name: FlexString,
        from: i32,
        to: i32,
    },
    Well {
        name: FlexString,
        location: FlexString,
    },
    Completion {
        name: FlexString,
        location: FlexString,
        index: i32,
    },
    Group {
        name: FlexString,
        location: FlexString,
    },
    Block {
        name: FlexString,
        index: i32,
    },
    Unsupported {
        name: FlexString,
        location: FlexString,
        index: i32,
    },
}

const VEC_EXT_CODE: i8 = 2;

#[derive(Debug, Serialize)]
#[serde(rename = "_ExtStruct")]
struct ExtVec((i8, serde_bytes::ByteBuf));

/// Individual summary item with data combined from both SMSPEC and UNSMRY files.
#[derive(Debug, Serialize)]
struct SummaryItem {
    id: ItemId,
    unit: FlexString,
    values: ExtVec,
}

/// The return type for parsing of SMSPEC+UNSMRY files.
#[derive(Debug, Serialize)]
pub struct Summary {
    /// Unit system for a simulation
    #[serde(skip_serializing_if = "Option::is_none")]
    units_system: Option<i32>,

    /// A simulator identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    simulator_id: Option<i32>,

    /// Grid dimensions of a simulation
    dims: [i32; 3],

    /// Simulation start date
    start_date: [i32; 6],

    /// Region names
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    region_names: HashMap<i32, FlexString>,

    /// Simulation data
    items: Vec<SummaryItem>,
}

impl Smspec {
    fn new(smspec_file: BinFile) -> ah::Result<Self> {
        let mut smspec: Self = Default::default();

        // Parse the SMSPEC file for enough metadata to correctly place data records
        for_keyword_in(smspec_file, |kw| {
            match (kw.name.as_str(), kw.data) {
                // This keyword is optional
                ("INTEHEAD", BinRecord::Int(header)) => {
                    smspec.units_system = Some(header[0]);
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
                        item.name = kw_name;
                    }
                }
                (kw @ "WGNAMES", BinRecord::Chars(wg_names))
                | (kw @ "NAMES", BinRecord::Chars(wg_names)) => {
                    log::trace!(target: "Parsing SMSPEC", "{}: {:?}", kw, wg_names);
                    for (item, wg_name) in smspec.items.iter_mut().zip(wg_names) {
                        item.wg_name = wg_name;
                    }
                }
                ("NUMS", BinRecord::Int(nums)) => {
                    log::trace!(target: "Parsing SMSPEC", "NUMS: {:?}", nums);
                    for (item, num) in smspec.items.iter_mut().zip(nums) {
                        item.index = num;
                    }
                }
                ("UNITS", BinRecord::Chars(units)) => {
                    log::trace!(target: "Parsing SMSPEC", "UNITS: {:?}", units);
                    for (item, unit) in smspec.items.iter_mut().zip(units) {
                        item.unit = unit;
                    }
                }
                (name, data) => {
                    if kw.name.as_str() != "MEASRMNT" {
                        log::debug!(target: "Parsing SMSPEC", "Unsupported SMSPEC keyword, name: {}, data: {:?}", name, data);
                    }
                }
            }
            Ok(())
        })?;
        Ok(smspec)
    }
}

impl Unsmry {
    fn new(unsmry_file: BinFile, nlist: i32) -> ah::Result<Self> {
        let mut unsmry = Unsmry(vec![Default::default(); nlist as usize]);

        // Read data from the UNSMRY file
        for_keyword_in(unsmry_file, |kw| {
            if let ("PARAMS", BinRecord::F32Bytes(params)) = (kw.name.as_str(), kw.data) {
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

impl Summary {
    fn new(smspec_file: BinFile, unsmry_file: BinFile) -> ah::Result<Self> {
        let smspec = Smspec::new(smspec_file)?;
        let unsmry = Unsmry::new(unsmry_file, smspec.nlist)?;

        let mut summary = Summary {
            start_date: smspec.start_date,
            units_system: smspec.units_system,
            simulator_id: smspec.simulator_id,
            dims: smspec.dims,
            region_names: Default::default(),
            items: vec![],
        };

        for (item, values) in smspec.items.into_iter().zip(unsmry.0) {
            let name = item.name;

            let wg_valid = !item.wg_name.is_empty() && &item.wg_name != ":+:+:+:+";
            let num_valid = item.index > 0;

            let id = if TIMING_KEYWORDS.contains(name.as_str()) {
                ItemId::Time { name }
            } else if PERFORMANCE_KEYWORDS.contains(name.as_str()) {
                ItemId::Performance { name }
            } else {
                match name.as_bytes() {
                    [b'F', ..] => ItemId::Field { name },
                    [b'A', ..] if num_valid => ItemId::Aquifer {
                        name,
                        index: item.index,
                    },
                    [b'R', b'N', b'L', b'F', ..] | [b'R', _, b'F', ..] if num_valid => {
                        let region2 = item.index / 32768 as i32 - 10;
                        let region1 = item.index - 32768 * (region2 + 10);
                        ItemId::CrossRegionFlow {
                            name,
                            from: region1,
                            to: region2,
                        }
                    }
                    [b'R', ..] if num_valid => {
                        if wg_valid {
                            summary.region_names.insert(item.index, item.wg_name);
                        }
                        ItemId::Region {
                            name,
                            index: item.index,
                        }
                    }
                    [b'W', ..] if wg_valid => ItemId::Well {
                        name,
                        location: item.wg_name,
                    },
                    [b'C', ..] if wg_valid && num_valid => ItemId::Completion {
                        name,
                        location: item.wg_name,
                        index: item.index,
                    },
                    [b'G', ..] if wg_valid => ItemId::Group {
                        name,
                        location: item.wg_name,
                    },
                    [b'B', ..] if num_valid => ItemId::Block {
                        name,
                        index: item.index,
                    },
                    _ => {
                        log::debug!(target: "Building Summary",
                            "Skipped a summary item. KEYWORD: {}, WGNAME: {}, NUM: {}",
                            name, item.wg_name, item.index
                        );
                        ItemId::Unsupported {
                            name,
                            location: item.wg_name,
                            index: item.index,
                        }
                    }
                }
            };

            summary.items.push(SummaryItem {
                id,
                unit: item.unit,
                values: ExtVec((VEC_EXT_CODE, serde_bytes::ByteBuf::from(values))),
            });
        }
        Ok(summary)
    }

    pub fn from_path<P: AsRef<Path>>(input_path: P) -> ah::Result<Self> {
        // If there is no stem, bail early
        let input_path = input_path.as_ref();
        if input_path.file_stem().is_none() {
            return Err(FileError::InvalidFilePath.into());
        }

        // we allow either extension or no extension at all
        if let Some(ext) = input_path.extension() {
            let ext = ext.to_str();
            if ext != Some("SMSPEC") && ext != Some("UNSMRY") {
                return Err(FileError::InvalidFileExt.into());
            }
        }

        let smspec = BinFile::new(input_path.with_extension("SMSPEC"))?;
        let unsmry = BinFile::new(input_path.with_extension("UNSMRY"))?;

        Summary::new(smspec, unsmry)
    }
}
