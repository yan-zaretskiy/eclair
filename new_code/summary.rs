//! ## Eclipse Summary Format
//!
//! Conceptually, Eclipse summary data is a collection of time series together with enough metadata
//! to uniquely identify them. The exact representation of the summary data consists of two files:
//!
//! - A specification file (`.SMSPEC`) which holds the metadata;
//!
//! - A "unified" summary file (`.UNSMRY`) which holds the time series.
//!
//! Both are standard Eclipse binary files, i.e. the consist of a series of Eclipse binary records.
//!
//! ### Specification file layout
//!
//! A full list of records present in the `.SMSPEC` files can be found it the Eclipse manual. Here
//! we list only those read by `eclair-io`:
//!
//! - `DIMENS`: 6 INTE items. The first one (NLIST) in the most important - it indicates the total
//!   number of time series in the summary. The next three items correspond to the nubmer of cells
//!   in X, Y and Z directions;
//! - `KEYWORDS`: NLIST CHAR items - mnemonic names for all time series;
//! - `WGNAMES`: NLIST CHAR items - well or group names for all time series;
//! - `NAMES`: NLIST C0nn items - alternative to `WGNAMES` when long (>8 chars) names are used;
//! - `NUMS`: NLIST INTE items - integer cell or region numbers associated with time series;
//! - `UNITS`: NLIST CHAR items - physical units for time series;
//! - `STARTDAT`: 6 INTE items - day (1-31), month (1-12), year (YYYY), hour (0-23), minute (0-59),
//!   microsecond (0 - 59,999,999) for the datetime of the simulation start.
//!
//! ### Summary file layout
//!
//! An `.UNSMRY` file contains a series of keyword triplets, of which only the latter two are
//! relevant:
//! - `SEQHDR`: 1 INTE item - ignored;
//! - `MINISTEP`: 1 INTE item - the running timestep counter;
//! - `PARAMS`: NLIST REAL items - time series data for the current timestep.
//!
//! In the code and comments below, time series are referred to as summary items.

use once_cell::sync::Lazy;

use crate::{
    error::EclairError,
    records::{Record, RecordData},
    FlexString, Result,
};

use crate::records::ReadRecord;
use itertools::multizip;
use std::collections::HashMap;
use std::{
    collections::HashSet,
    convert::{TryFrom, TryInto},
};

static SMSPEC_RECORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("INTEHEAD");
    s.insert("RESTART");
    s.insert("DIMENS");
    s.insert("KEYWORDS");
    s.insert("WGNAMES");
    s.insert("NAMES");
    s.insert("NUMS");
    s.insert("LGRS");
    s.insert("NUMLX");
    s.insert("NUMLY");
    s.insert("NUMLZ");
    s.insert("LENGTHS");
    s.insert("LENUNITS");
    s.insert("MEASRMNT");
    s.insert("UNITS");
    s.insert("STARTDAT");
    s.insert("LGRNAMES");
    s.insert("LGRVEC");
    s.insert("LGRTIMES");
    s.insert("RUNTIMEI");
    s.insert("RUNTIMED");
    s.insert("STEPRESN");
    s.insert("XCOORD");
    s.insert("YCOORD");
    s.insert("TIMESTMP");
    s
});

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

const UNKNOWN_WG_NAME: &str = ":+:+:+:+";

/// ItemId is an item identifier derived from the SMSPEC metadata. It consists of a name, which
/// corresponds to the physical quantity the item represents (e.g. WBHP for the well bottom hole
/// pressure) and a qualifier, which roughly corresponds to the location (e.g. well named WELL_1).
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct ItemId {
    pub name: FlexString,
    pub qualifier: ItemQualifier,
}

impl ItemId {
    /// This implementation contains the messy logic of interpreting the item mnemonic name.
    /// Details of how these mnemonics relate to the physical nature of a summary item can be found
    /// in the Eclipse manual.
    fn new(name: FlexString, wg_name: FlexString, index: i32) -> Self {
        use ItemQualifier::*;

        let wg_valid = !wg_name.is_empty() && wg_name != UNKNOWN_WG_NAME;
        let num_valid = index > 0;

        let qualifier = if TIMING_KEYWORDS.contains(name.as_str()) {
            Time
        } else if PERFORMANCE_KEYWORDS.contains(name.as_str()) {
            Performance
        } else {
            match name.as_bytes() {
                [b'F', ..] => Field,
                [b'A', ..] if num_valid => Aquifer { index },
                [b'R', b'N', b'L', b'F', ..] | [b'R', _, b'F', ..] if num_valid => {
                    let region2 = index / 32768 as i32 - 10;
                    let region1 = index - 32768 * (region2 + 10);
                    CrossRegionFlow {
                        from: region1,
                        to: region2,
                    }
                }
                [b'R', ..] if num_valid => Region {
                    wg_name: if wg_valid { Some(wg_name) } else { None },
                    index,
                },
                [b'W', ..] if wg_valid => Well { wg_name },
                [b'C', ..] if wg_valid && num_valid => Completion { wg_name, index },
                [b'G', ..] if wg_valid => Group { wg_name },
                [b'B', ..] if num_valid => Block { index },
                _ => {
                    log::info!(target: "Building SummaryItem",
                        "Unrecognized summary item. KEYWORD: {}, WGNAME: {}, NUM: {}",
                        name, wg_name, index
                    );
                    Unrecognized { wg_name, index }
                }
            }
        };
        ItemId { name, qualifier }
    }
}

/// ItemQualifier is used to associate a location or a category with a summary item.
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum ItemQualifier {
    Time,
    Performance,
    Field,
    Aquifer {
        index: i32,
    },
    Region {
        wg_name: Option<FlexString>,
        index: i32,
    },
    CrossRegionFlow {
        from: i32,
        to: i32,
    },
    Well {
        wg_name: FlexString,
    },
    Completion {
        wg_name: FlexString,
        index: i32,
    },
    Group {
        wg_name: FlexString,
    },
    Block {
        index: i32,
    },
    Unrecognized {
        wg_name: FlexString,
        index: i32,
    },
}

/// An individual summary item.
#[derive(Debug)]
pub struct SummaryItem {
    /// Physical unit
    pub unit: FlexString,

    /// Time series values
    pub values: Vec<f32>,
}

/// A union of (a subset of) data from both `SMSPEC` and `UNSMRY` files. The subset may eventually
/// expand to cover more of the summary data, but right now we ignore data related to LGRs,
/// horizontal wells, measurement descriptions, completion coordinates, run-time monitoring.
#[derive(Debug)]
pub struct Summary {
    /// Grid dimensions of a simulation
    pub dims: [i32; 3],

    /// Simulation start date
    pub start_date: [i32; 6],

    /// ItemId to its index in the items vector
    pub item_ids: HashMap<ItemId, usize>,

    /// Simulation data
    pub items: Vec<SummaryItem>,
}

/// Intermediate type for Smspec data to facilitate input validation. It contains a subset of
/// records from which a valid Summary COULD be constructed. At the point of its construction the
/// only input error we check for is the presence of duplicate records.
#[derive(Debug)]
struct SmspecRecords {
    records: HashMap<&'static str, Option<RecordData>>,
}

impl Default for SmspecRecords {
    fn default() -> Self {
        let mut records = HashMap::new();
        records.insert("DIMENS", None);
        records.insert("STARTDAT", None);
        records.insert("KEYWORDS", None);
        records.insert("WGNAMES", None);
        records.insert("NUMS", None);
        records.insert("UNITS", None);
        SmspecRecords { records }
    }
}

impl SmspecRecords {
    fn is_full(&self) -> bool {
        self.records.values().all(|val| val.is_some())
    }

    fn new<I: ReadRecord>(reader: &mut I) -> Result<Self> {
        use EclairError::*;

        let mut smspec_records = Self::default();

        loop {
            let (_, record) = reader.read_record()?;
            if record.is_none() {
                break;
            }

            let Record { name, data } = record.unwrap();
            // Stop reading records if we encounter a name that does not belong in SMSPEC.
            if !SMSPEC_RECORDS.contains(&name.as_str()) {
                log::debug!(target: "Parsing SMSPEC", "Non-SMSPEC record name encountered: {}.", name);
                break;
            }

            // If we encounter a record that we wish to consume, first check whether we've already
            // read it. "NAMES" is looked up as "WGNAMES" because only one of them is allowed in
            // a given SMSPEC at the same time.
            let lookup_name = if &name == "NAMES" { "WGNAMES" } else { &name };
            if let Some(val) = smspec_records.records.get_mut(lookup_name) {
                if val.is_some() {
                    return Err(RecordEncounteredTwice(name.to_string()));
                }
                *val = Some(data);
            }

            // If we found all the records we need, stop reading further. This allows us to chain
            // valid SMSPEC and UNSMRY records in a single stream.
            if smspec_records.is_full() {
                log::debug!(target: "Parsing SMSPEC", "Found all the neccessary records.");
                break;
            }
        }

        Ok(smspec_records)
    }
}

impl TryFrom<SmspecRecords> for Summary {
    type Error = EclairError;

    // FIXME: There has to be a more intelligent way to validate these records. Macros?
    fn try_from(mut value: SmspecRecords) -> Result<Self> {
        use EclairError::*;
        use RecordData::*;

        // 1. DIMENS precedes any other record whose size is specified by DIMENS's first element.
        let dimens = value.records.remove("DIMENS").unwrap();
        let dimens = if let Some(data) = dimens {
            if let Int(values) = data {
                if values.len() == 6 {
                    values
                } else {
                    return Err(UnexpectedRecordDataLength {
                        name: "DIMENS".to_string(),
                        expected: 6,
                        found: values.len(),
                    });
                }
            } else {
                return Err(InvalidRecordDataType {
                    name: "DIMENS".to_string(),
                    expected: "INTE".to_string(),
                    found: data.type_string(),
                });
            }
        } else {
            return Err(EclairError::MissingRecord("DIMENS".to_string()));
        };
        let nlist = dimens[0] as usize;

        // 2. STARTDAT is either 3 or 6 elements long.
        let start_dat = value.records.remove("STARTDAT").unwrap();
        let start_dat = if let Some(data) = start_dat {
            if let Int(values) = data {
                if values.len() == 3 || values.len() == 6 {
                    values
                } else {
                    return Err(UnexpectedRecordDataLength {
                        name: "STARTDAT".to_string(),
                        expected: if values.len() < 3 { 3 } else { 6 },
                        found: values.len(),
                    });
                }
            } else {
                return Err(InvalidRecordDataType {
                    name: "STARTDAT".to_string(),
                    expected: "INTE".to_string(),
                    found: data.type_string(),
                });
            }
        } else {
            return Err(EclairError::MissingRecord("STARTDAT".to_string()));
        };

        // 3. KEYWORDS and the rest of the records must be nlist long.
        let keywords = value.records.remove("KEYWORDS").unwrap();
        let keywords = if let Some(data) = keywords {
            if let Chars(values) = data {
                if values.len() == nlist {
                    values
                } else {
                    return Err(UnexpectedRecordDataLength {
                        name: "KEYWORDS".to_string(),
                        expected: nlist,
                        found: values.len(),
                    });
                }
            } else {
                return Err(InvalidRecordDataType {
                    name: "KEYWORDS".to_string(),
                    expected: "CHAR".to_string(),
                    found: data.type_string(),
                });
            }
        } else {
            return Err(EclairError::MissingRecord("KEYWORDS".to_string()));
        };

        // 4. WGNAMES
        let wg_names = value.records.remove("WGNAMES").unwrap();
        let wg_names = if let Some(data) = wg_names {
            if let Chars(values) = data {
                if values.len() == nlist {
                    values
                } else {
                    return Err(UnexpectedRecordDataLength {
                        name: "WGNAMES".to_string(),
                        expected: nlist,
                        found: values.len(),
                    });
                }
            } else {
                return Err(InvalidRecordDataType {
                    name: "WGNAMES".to_string(),
                    expected: "CHAR".to_string(),
                    found: data.type_string(),
                });
            }
        } else {
            return Err(EclairError::MissingRecord("WGNAMES".to_string()));
        };

        // 5. NUMS
        let nums = value.records.remove("NUMS").unwrap();
        let nums = if let Some(data) = nums {
            if let Int(values) = data {
                if values.len() == nlist {
                    values
                } else {
                    return Err(UnexpectedRecordDataLength {
                        name: "NUMS".to_string(),
                        expected: nlist,
                        found: values.len(),
                    });
                }
            } else {
                return Err(InvalidRecordDataType {
                    name: "NUMS".to_string(),
                    expected: "CHAR".to_string(),
                    found: data.type_string(),
                });
            }
        } else {
            return Err(EclairError::MissingRecord("NUMS".to_string()));
        };

        // 6. UNITS
        let units = value.records.remove("UNITS").unwrap();
        let units = if let Some(data) = units {
            if let Chars(values) = data {
                if values.len() == nlist {
                    values
                } else {
                    return Err(UnexpectedRecordDataLength {
                        name: "UNITS".to_string(),
                        expected: nlist,
                        found: values.len(),
                    });
                }
            } else {
                return Err(InvalidRecordDataType {
                    name: "UNITS".to_string(),
                    expected: "CHAR".to_string(),
                    found: data.type_string(),
                });
            }
        } else {
            return Err(EclairError::MissingRecord("UNITS".to_string()));
        };

        // Now we prepare to construct the Summary object.
        let dims = dimens[1..4].try_into().unwrap();

        let start_date = if start_dat.len() == 3 {
            let mut v = [0, 0, 0, 0, 0, 0];
            v[..3].copy_from_slice(&start_dat);
            v
        } else {
            start_dat.as_slice().try_into().unwrap()
        };

        let mut item_ids = HashMap::new();
        let mut items = Vec::with_capacity(nlist);

        for vals in multizip((keywords, wg_names, nums, units)) {
            let (name, wg_name, index, unit) = vals;
            let item_id = ItemId::new(name, wg_name, index);
            item_ids.insert(item_id, items.len());
            items.push(SummaryItem {
                unit,
                values: Vec::new(),
            });
        }

        Ok(Summary {
            dims,
            start_date,
            item_ids,
            items,
        })
    }
}

impl Summary {
    /// Construct a new Summary instance from an implementor of ReadRecord.
    pub fn new<I: ReadRecord>(reader: &mut I) -> Result<Self> {
        let records = SmspecRecords::new(reader)?;
        Summary::try_from(records)
    }

    /// Add time series values. This method skips records until it encounters SEQHDR.
    /// After that it reads, validates and consumes the next two records. It continues in this
    /// fashion until it encounters the EOF.
    pub fn extend<I: ReadRecord>(&mut self, reader: &mut I) -> Result<()> {
        use EclairError::*;

        loop {
            let (_, record) = reader.read_record()?;
            if record.is_none() {
                break;
            }

            if &record.unwrap().name != "SEQHDR" {
                continue;
            }

            // We've encountered a SEQHDR record. Now inspect the next two records.
            let (_, record) = reader.read_record()?;

            // Next one should be MINISTEP. The wrapped value inside starts at 0.
            let step_index = match record {
                Some(Record { name, data }) if name == "MINISTEP" => {
                    if let RecordData::Int(values) = data {
                        if values.len() != 1 {
                            return Err(UnexpectedRecordDataLength {
                                name: "MINISTEP".to_string(),
                                expected: 1,
                                found: values.len(),
                            });
                        }
                        values[0] as usize
                    } else {
                        return Err(InvalidRecordDataType {
                            name: "MINISTEP".to_string(),
                            expected: "INTE".to_string(),
                            found: data.type_string(),
                        });
                    }
                }
                _ => {
                    return Err(MissingRecord("MINISTEP".to_string()));
                }
            };

            // All items have the same length of their values by construction, we pick the first one.
            if step_index != self.items[0].values.len() {
                return Err(InvalidMinistepValue {
                    expected: self.items[0].values.len(),
                    found: step_index,
                });
            }

            let (_, record) = reader.read_record()?;

            // Next is PARAMS with as many values as we have items.
            let params = match record {
                Some(Record { name, data }) if name == "PARAMS" => {
                    if let RecordData::F32(values) = data {
                        if values.len() != self.items.len() {
                            return Err(UnexpectedRecordDataLength {
                                name: "PARAMS".to_string(),
                                expected: self.items.len(),
                                found: values.len(),
                            });
                        }
                        values
                    } else {
                        return Err(InvalidRecordDataType {
                            name: "PARAMS".to_string(),
                            expected: "REAL".to_string(),
                            found: data.type_string(),
                        });
                    }
                }
                _ => {
                    return Err(MissingRecord("PARAMS".to_string()));
                }
            };

            for (item, param) in self.items.iter_mut().zip(params) {
                item.values.push(param);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::{fs::File, io::BufReader};

    #[test]
    fn read_spe_10() {
        let f1 = File::open("assets/SPE10.SMSPEC").unwrap();
        let f2 = File::open("assets/SPE10.UNSMRY").unwrap();
        let stream = f1.chain(f2);
        let mut reader = BufReader::new(stream);
        let mut summary = Summary::new(&mut reader).unwrap();

        assert_eq!(summary.dims, [100, 100, 30]);
        assert_eq!(summary.start_date, [1, 3, 2005, 0, 0, 0]);
        assert_eq!(summary.items.len(), 34);
        assert!(summary.extend(&mut reader).is_ok());
        println!("{:?}", summary);
    }
}
