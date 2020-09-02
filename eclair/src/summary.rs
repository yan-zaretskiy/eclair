use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use anyhow::{self as ah, Context};
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::{
    binary::{BinFile, BinRecord, FlexString},
    errors::{FileError, SummaryError},
    summary_item::{ItemQualifier, SummaryItem},
};

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
    s.insert("NAIMFRAC");
    s
});

/// Raw metadata we collect for each time series.
#[derive(Debug, Default)]
pub struct SmspecItem {
    pub name: FlexString,
    pub wg_name: FlexString,
    pub index: i32,
    pub unit: FlexString,
}

/// Contents of an SMSPEC file. Note that we don't extract everything, only those bits
/// that are presently relevant to us. Notably, there is no data related to LGRs.
/// This type could be extended later.
#[derive(Debug, Default)]
pub struct Smspec {
    units_system: Option<i32>,
    simulator_id: Option<i32>,
    nlist: i32,
    dims: [i32; 3],
    start_date: [i32; 6],

    pub(crate) items: Vec<SmspecItem>,
}

/// Contents of an UNSMRY file stored as raw bytes.
#[derive(Debug, Default)]
struct Unsmry(Vec<Vec<u8>>);

/// Accumulation of data from both SMSPEC & UNSMRY files.
#[derive(Debug, Serialize)]
pub struct Summary {
    /// SMSPEC/UNSMRY filename
    pub name: String,

    /// Unit system for a simulation
    #[serde(skip_serializing_if = "Option::is_none")]
    units_system: Option<i32>,

    /// A simulator identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    simulator_id: Option<i32>,

    /// Region names
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    region_names: HashMap<i32, FlexString>,

    /// Grid dimensions of a simulation
    pub dims: [i32; 3],

    /// Simulation start date
    pub start_date: [i32; 6],

    /// Simulation data
    pub items: Vec<SummaryItem>,
}

impl Smspec {
    pub(crate) fn new<R>(smspec_file: BinFile<R>) -> ah::Result<Self>
    where
        R: Read,
    {
        let mut smspec: Self = Default::default();

        // Parse the SMSPEC file for enough metadata to correctly place data records
        smspec_file.for_each_kw(|kw| {
            match (kw.name.as_str(), kw.data) {
                // This keyword is optional
                ("INTEHEAD", BinRecord::Int(header)) => {
                    log::trace!(target: "Parsing SMSPEC", "INTEHEAD: {:#?}", header);
                    smspec.units_system = Some(header[0]);
                    smspec.simulator_id = Some(header[1]);
                }
                ("DIMENS", BinRecord::Int(dimens)) => {
                    log::trace!(target: "Parsing SMSPEC", "DIMENS: {:#?}", dimens);
                    smspec.nlist = dimens[0];
                    smspec
                        .items
                        .resize_with(dimens[0] as usize, Default::default);
                    smspec.dims.copy_from_slice(&dimens[1..4]);
                }
                ("STARTDAT", BinRecord::Int(start_dat)) => {
                    log::trace!(target: "Parsing SMSPEC", "STARTDAT: {:#?}", start_dat);
                    if start_dat.len() == 3 {
                        smspec.start_date[..3].copy_from_slice(&start_dat);
                    } else if start_dat.len() == 6 {
                        smspec.start_date.copy_from_slice(&start_dat);
                    } else {
                        return Err(SummaryError::InvalidStartDateLength(start_dat.len()).into());
                    }
                }
                ("KEYWORDS", BinRecord::Chars(keywords)) => {
                    log::trace!(target: "Parsing SMSPEC", "KEYWORDS: {:#?}", keywords);
                    for (item, kw_name) in smspec.items.iter_mut().zip(keywords) {
                        item.name = kw_name;
                    }
                }
                (kw @ "WGNAMES", BinRecord::Chars(wg_names))
                | (kw @ "NAMES", BinRecord::Chars(wg_names)) => {
                    log::trace!(target: "Parsing SMSPEC", "{}: {:#?}", kw, wg_names);
                    for (item, wg_name) in smspec.items.iter_mut().zip(wg_names) {
                        item.wg_name = wg_name;
                    }
                }
                ("NUMS", BinRecord::Int(nums)) => {
                    log::trace!(target: "Parsing SMSPEC", "NUMS: {:#?}", nums);
                    for (item, num) in smspec.items.iter_mut().zip(nums) {
                        item.index = num;
                    }
                }
                ("UNITS", BinRecord::Chars(units)) => {
                    log::trace!(target: "Parsing SMSPEC", "UNITS: {:#?}", units);
                    for (item, unit) in smspec.items.iter_mut().zip(units) {
                        item.unit = unit;
                    }
                }
                (name, data) => {
                    if kw.name.as_str() != "MEASRMNT" {
                        log::debug!(target: "Parsing SMSPEC", "Unsupported SMSPEC keyword, name: {}, data: {:#?}", name, data);
                    }
                }
            }
            Ok(())
        })?;
        Ok(smspec)
    }
}

impl Unsmry {
    pub(crate) fn new<R>(unsmry_file: BinFile<R>, nlist: i32) -> ah::Result<Self>
    where
        R: Read,
    {
        let mut unsmry = Unsmry(vec![Default::default(); nlist as usize]);

        // Read data from the UNSMRY file
        unsmry_file.for_each_kw(|kw| {
            if let ("PARAMS", BinRecord::F32Bytes(params)) = (kw.name.as_str(), kw.data) {
                for (values, param) in unsmry
                    .0
                    .iter_mut()
                    .zip(params.chunks_exact(std::mem::size_of::<f32>()))
                {
                    values.extend_from_slice(param)
                }
            }
            Ok(())
        })?;
        Ok(unsmry)
    }
}

pub fn files_from_path<P>(input_path: P) -> ah::Result<(BufReader<File>, BufReader<File>)>
where
    P: AsRef<Path>,
{
    // If there is no stem, bail early
    let input_path = input_path.as_ref();

    let stem = input_path.file_stem();
    if stem.is_none() || stem.unwrap().to_str().is_none() {
        return Err(FileError::InvalidFilePath.into());
    }

    // we allow either extension or no extension at all
    if let Some(ext) = input_path.extension() {
        let ext = ext.to_str();
        if ext != Some("SMSPEC") && ext != Some("UNSMRY") {
            return Err(FileError::InvalidFileExt.into());
        }
    }

    let open_file = |path| -> ah::Result<_> {
        Ok(BufReader::new(File::open(path).with_context(|| {
            "Failed to open a file at the requested path"
        })?))
    };

    let smspec = open_file(input_path.with_extension("SMSPEC"))?;
    let unsmry = open_file(input_path.with_extension("UNSMRY"))?;
    Ok((smspec, unsmry))
}

impl Summary {
    pub fn new<R1, R2>(name: &str, smspec: R1, unsmry: R2) -> ah::Result<Self>
    where
        R1: Read,
        R2: Read,
    {
        let smspec = Smspec::new(BinFile::new(smspec))?;
        let unsmry = Unsmry::new(BinFile::new(unsmry), smspec.nlist)?;

        let mut summary = Summary {
            name: name.to_owned(),
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
                ItemQualifier::Time
            } else if PERFORMANCE_KEYWORDS.contains(name.as_str()) {
                ItemQualifier::Performance
            } else {
                match name.as_bytes() {
                    [b'F', ..] => ItemQualifier::Field,
                    [b'A', ..] if num_valid => ItemQualifier::Aquifer { index: item.index },
                    [b'R', b'N', b'L', b'F', ..] | [b'R', _, b'F', ..] if num_valid => {
                        let region2 = item.index / 32768 as i32 - 10;
                        let region1 = item.index - 32768 * (region2 + 10);
                        ItemQualifier::CrossRegionFlow {
                            from: region1,
                            to: region2,
                        }
                    }
                    [b'R', ..] if num_valid => {
                        if wg_valid {
                            summary.region_names.insert(item.index, item.wg_name);
                        }
                        ItemQualifier::Region { index: item.index }
                    }
                    [b'W', ..] if wg_valid => ItemQualifier::Well {
                        location: item.wg_name,
                    },
                    [b'C', ..] if wg_valid && num_valid => ItemQualifier::Completion {
                        location: item.wg_name,
                        index: item.index,
                    },
                    [b'G', ..] if wg_valid => ItemQualifier::Group {
                        location: item.wg_name,
                    },
                    [b'B', ..] if num_valid => ItemQualifier::Block { index: item.index },
                    _ => {
                        log::debug!(target: "Building Summary",
                            "Skipped a summary item. KEYWORD: {}, WGNAME: {}, NUM: {}",
                            name, item.wg_name, item.index
                        );
                        ItemQualifier::Unrecognized {
                            location: item.wg_name,
                            index: item.index,
                        }
                    }
                }
            };
            summary.add(name, id, item.unit, values);
        }
        Ok(summary)
    }

    fn add(&mut self, name: FlexString, kind: ItemQualifier, unit: FlexString, values: Vec<u8>) {
        self.items.push(SummaryItem::new(name, kind, unit, values))
    }

    pub fn from_path<P: AsRef<Path>>(input_path: P) -> ah::Result<Self> {
        let (smspec, unsmry) = files_from_path(&input_path)?;

        // at this point we know we won't panic here
        let stem = input_path.as_ref().file_stem().unwrap().to_str().unwrap();

        Summary::new(stem, smspec, unsmry)
    }
}
