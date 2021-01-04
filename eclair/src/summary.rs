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

use std::{
    collections::{HashMap, HashSet},
    convert::{TryFrom, TryInto},
    fmt::{Display, Formatter},
    fs::File,
    io::{BufReader, Seek, SeekFrom},
    thread::sleep,
    time,
};

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use crossbeam_channel::{Receiver, Sender};
use itertools::multizip;
use once_cell::sync::Lazy;

use crate::{
    error::EclairError,
    records::{ReadRecord, Record, RecordData, RecordDataKind},
    FlexString, Result,
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

impl ItemQualifier {
    pub fn is_recognized(&self) -> bool {
        !matches!(self, ItemQualifier::Unrecognized { .. })
    }
}

impl Display for ItemQualifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ItemQualifier::*;
        match self {
            Time => write!(f, "Time"),
            Performance => write!(f, "Performance"),
            Field => write!(f, "Field"),
            Aquifer { index } => write!(f, "Aquifer #{}", index),
            Region { wg_name, index } => match wg_name {
                Some(r_name) => write!(f, "Region {}", r_name),
                None => write!(f, "Region #{}", index),
            },
            CrossRegionFlow { from, to } => write!(f, "CrossRegionFlow {} => {}", from, to),
            Well { wg_name } => write!(f, "Well {}", wg_name),
            Completion { wg_name, index } => write!(f, "Completion #{} @ {}", index, wg_name),
            Group { wg_name } => write!(f, "Group {}", wg_name),
            Block { index } => write!(f, "Block #{}", index),
            Unrecognized { wg_name, index } => write!(
                f,
                "Unrecognized qualifier. Name: {}, index: {}",
                wg_name, index
            ),
        }
    }
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

    /// Simulation unix timestamps
    pub timestamps: Vec<i64>,

    /// ItemId to its index in the items vector
    pub item_ids: HashMap<ItemId, usize>,

    /// Simulation data
    pub items: Vec<SummaryItem>,

    // Index of the time item.
    time_index: usize,

    start_timestamp: i64,
}

impl Summary {
    /// Total number of summary items.
    pub fn n_items(&self) -> usize {
        self.items.len()
    }

    /// Number of time iterations that this Summary stores data for.
    pub fn n_steps(&self) -> usize {
        match self.items.first() {
            Some(items) => items.values.len(),
            None => 0,
        }
    }

    /// This function expects the size of params to equal the size of items.
    pub fn append(&mut self, params: Vec<f32>) {
        let new_time = params[self.time_index];
        let new_ts =
            self.start_timestamp + Duration::seconds((new_time * 86400.0) as i64).num_seconds();
        self.timestamps.push(new_ts);

        for (item, param) in self.items.iter_mut().zip(params) {
            item.values.push(param);
        }
    }
}

/// Intermediate type for Smspec data to facilitate input validation. It contains a subset of
/// records from which a valid Summary COULD be constructed. At the point of its construction the
/// only input error we check for is the presence of duplicate records.
#[derive(Debug)]
pub(crate) struct SmspecRecords {
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
    pub(crate) fn new(records: HashMap<&'static str, Option<RecordData>>) -> Self {
        SmspecRecords { records }
    }

    fn is_full(&self) -> bool {
        self.records.values().all(|val| val.is_some())
    }
}

macro_rules! validate {
            ($field_data: ident, $field_name: literal, $kind: ident, $($valid_len: expr),+ $(,)?) => {
                loop {
                    let values = if let RecordData::$kind(values) = $field_data {
                        values
                    } else {
                        return Err(EclairError::InvalidRecordDataType {
                            name: $field_name.to_string(),
                            expected: RecordDataKind::$kind.to_string(),
                            found: $field_data.kind_string(),
                        });
                    };

                    let len_err = |expected| Err(EclairError::UnexpectedRecordDataLength {
                        name: $field_name.to_string(),
                        expected,
                        found: values.len(),
                    });

                    let mut expected_len;
                    $(
                        expected_len = $valid_len;
                        match values.len().cmp(&$valid_len) {
                            std::cmp::Ordering::Less => return len_err($valid_len),
                            std::cmp::Ordering::Equal => break values,
                            std::cmp::Ordering::Greater => (),
                        };
                    )+

                    return len_err(expected_len);
                }
            };
        }

impl TryFrom<SmspecRecords> for Summary {
    type Error = EclairError;

    fn try_from(mut value: SmspecRecords) -> Result<Self> {
        use EclairError::*;

        macro_rules! extract_and_validate {
            ($field_name: literal, $kind: ident, $($valid_len: expr),+ $(,)?) => {
                {
                   let field_data = value
                    .records
                    .remove($field_name)
                    .unwrap()
                    .ok_or_else(|| MissingRecord($field_name.to_string()))?;

                   validate!(field_data, $field_name, $kind, $($valid_len),+)
                }
            };
        }

        let dimens = extract_and_validate!("DIMENS", Int, 6);
        let nlist = dimens[0] as usize;

        let start_dat = extract_and_validate!("STARTDAT", Int, 3, 6);
        let keywords = extract_and_validate!("KEYWORDS", Chars, nlist);
        let wg_names = extract_and_validate!("WGNAMES", Chars, nlist);
        let nums = extract_and_validate!("NUMS", Int, nlist);
        let units = extract_and_validate!("UNITS", Chars, nlist);

        // Now we prepare to construct the Summary object.
        let dims = dimens[1..4].try_into().unwrap();

        let d = NaiveDate::from_ymd(start_dat[2], start_dat[1] as u32, start_dat[0] as u32);

        let ts = if start_dat.len() == 3 {
            d.and_hms(0, 0, 0)
        } else {
            d.and_hms_milli(
                start_dat[3] as u32,
                start_dat[4] as u32,
                (start_dat[5] / 1_000_000) as u32,
                (start_dat[5] % 1_000_000) as u32,
            )
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

        // We will panic if there is no "TIME" in the data. Make this an error instead.
        let time_index = *item_ids
            .get(&ItemId {
                name: FlexString::from_str("TIME"),
                qualifier: ItemQualifier::Time,
            })
            .unwrap();

        Ok(Summary {
            dims,
            timestamps: vec![],
            item_ids,
            items,
            time_index,
            start_timestamp: ts.timestamp(),
        })
    }
}

/// Implementations of InitializeSummary can build a Summary instance and an object that can be
/// subsequently used to append more data to it.
pub trait InitializeSummary {
    // Updater is meant to be moved to a separate thread.
    type Updater: UpdateSummary + Send + 'static;

    /// Read enough data to build a valid Summary instance and convert yourself to the Updater
    /// object.
    fn init(self) -> Result<(Summary, Self::Updater)>;
}

/// UpdateSummary implementations provide new summary data using the supplied channel.
pub trait UpdateSummary {
    fn update(&mut self, data_snd: Sender<Vec<f32>>, term_rcv: Receiver<bool>) -> Result<()>;
}

/// SummaryFileReader builds Summary data from file-like sources.
pub struct SummaryFileReader {
    smspec_file: BufReader<File>,
    unsmry_file: BufReader<File>,
}

/// FileUpdater updates Summary data from a file-like source.
pub struct SummaryFileUpdater {
    unsmry_file: BufReader<File>,

    n_items: usize,
    n_steps: usize,
}

/// Scan the next two or three UNSMRY records and attempt to extract data for the next time
/// iteration.
fn get_next_params<T: ReadRecord>(
    reader: &mut T,
    step: usize,
    n_items: usize,
) -> Result<Option<(usize, Vec<f32>)>> {
    use EclairError::*;

    macro_rules! unwrap_and_validate {
        ($record: expr, $field_name: literal, $kind: ident, $valid_len: expr) => {
            match $record {
                Some(Record { name, data }) if name == $field_name => {
                    validate!(data, $field_name, $kind, $valid_len)
                }
                _ => {
                    return Err(MissingRecord($field_name.to_string()));
                }
            }
        };
    }

    let mut n_bytes_read = 0;

    let (n_bytes, mut record) = reader.read_record()?;

    // This could be a SEQHDR.
    let read_next = match &record {
        None => return Ok(None),
        Some(Record { name, .. }) => {
            n_bytes_read += n_bytes;
            name == "SEQHDR"
        }
    };

    if read_next {
        let (n_bytes, next_record) = reader.read_record()?;
        record = next_record;
        n_bytes_read += n_bytes;
    }

    // Next one should be MINISTEP. The wrapped counter inside starts at 0.
    let step_index = unwrap_and_validate!(record, "MINISTEP", Int, 1)[0] as usize;

    if step_index != step {
        return Err(EclairError::InvalidMinistepValue {
            expected: step,
            found: step_index,
        });
    }

    let (n_bytes, record) = reader.read_record()?;
    n_bytes_read += n_bytes;

    // Next is PARAMS with as many values as we have items.
    let params = unwrap_and_validate!(record, "PARAMS", F32, n_items);
    Ok(Some((n_bytes_read, params)))
}

impl UpdateSummary for SummaryFileUpdater {
    fn update(&mut self, data_snd: Sender<Vec<f32>>, term_rcv: Receiver<bool>) -> Result<()> {
        // Continuously tries to read from the UNSMRY file and sends new values over the provided
        // channel.
        let mut file_pos = self.unsmry_file.seek(SeekFrom::Current(0)).unwrap();
        let mut last_read_successful = true;
        let mut modified_time = std::time::SystemTime::now();

        loop {
            // First check if we were instructed to stop.
            if let Ok(_) = term_rcv.try_recv() {
                log::info!(
                    target: "SummaryFileUpdater::update",
                    "Received termination request."
                );
                return Ok(());
            }

            // Try to read from the file if necessary.
            let metadata = self.unsmry_file.get_ref().metadata()?;
            let new_modified_time = metadata.modified()?;

            if last_read_successful || new_modified_time > modified_time {
                modified_time = new_modified_time;
                let params = get_next_params(&mut self.unsmry_file, self.n_steps, self.n_items);

                last_read_successful = match params {
                    Ok(params) => {
                        if let Some((n_bytes, params)) = params {
                            file_pos += n_bytes as u64;
                            self.n_steps += 1;

                            if data_snd.send(params).is_err() {
                                log::info!(target: "SummaryFileUpdater::update", "Error while sending params over a channel");
                                return Ok(());
                            }
                            true
                        } else {
                            false
                        }
                    }
                    Err(_) => {
                        self.unsmry_file.seek(SeekFrom::Start(file_pos)).unwrap();
                        false
                    }
                };
            }
            sleep(time::Duration::from_millis(100));
        }
    }
}

impl SummaryFileReader {
    pub fn from_path<P>(input_path: P) -> Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        // If there is no stem, bail early.
        let input_path = input_path.as_ref();

        let stem = input_path.file_stem();
        if stem.is_none() || stem.unwrap().to_str().is_none() {
            return Err(EclairError::InvalidFilePath(
                input_path.to_string_lossy().to_string(),
            ));
        }

        // We allow SMSPEC and UNSMRY extensions or no extension at all.
        if let Some(ext) = input_path.extension() {
            let ext = ext.to_str();
            if ext != Some("SMSPEC") && ext != Some("UNSMRY") {
                return Err(EclairError::InvalidFilePath(
                    input_path.to_string_lossy().to_string(),
                ));
            }
        }

        let open_file = |path| -> Result<_> { Ok(BufReader::new(File::open(path)?)) };
        Ok(Self {
            smspec_file: open_file(input_path.with_extension("SMSPEC"))?,
            unsmry_file: open_file(input_path.with_extension("UNSMRY"))?,
        })
    }
}

impl InitializeSummary for SummaryFileReader {
    type Updater = SummaryFileUpdater;

    fn init(mut self) -> Result<(Summary, Self::Updater)> {
        use EclairError::*;

        // First build the SmspecRecords object from the Smspec source.
        let mut smspec_records = SmspecRecords::default();

        loop {
            let (_, record) = self.smspec_file.read_record()?;
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

        let mut summary = Summary::try_from(smspec_records)?;

        let n_items = summary.items.len();
        let mut n_steps = 0;

        // Get the current size and don't read data past it (strictly speaking, we can go past by a
        // fraction of a single UNSMRY triplet length).
        let unsmry_size = self.unsmry_file.seek(SeekFrom::End(0)).unwrap();
        let mut unsmry_pos = self.unsmry_file.seek(SeekFrom::Start(0)).unwrap();

        // We store the current file position before the read and try to read as many timestep data
        // as we can.
        loop {
            let params = get_next_params(&mut self.unsmry_file, n_steps, n_items);

            match params {
                Ok(params) => {
                    match params {
                        None => break,
                        Some((n_bytes, params)) => {
                            summary.append(params);
                            n_steps += 1;
                            unsmry_pos += n_bytes as u64;
                            // In case we're reading from a file that's still being written to, we stop here
                            // and continue reading during subsequent updates.
                            if unsmry_pos >= unsmry_size {
                                break;
                            }
                        }
                    }
                }
                Err(_) => {
                    self.unsmry_file.seek(SeekFrom::Start(unsmry_pos)).unwrap();
                    break;
                }
            }
        }

        Ok((
            summary,
            SummaryFileUpdater {
                unsmry_file: self.unsmry_file,
                n_items,
                n_steps,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn read_spe_10() {
        // let f1 = File::open("assets/SPE10.SMSPEC").unwrap();
        // let f2 = File::open("assets/SPE10.UNSMRY").unwrap();
        // let stream = f1.chain(f2);
        // let mut reader = BufReader::new(stream);
        // let mut summary = Summary::new(&mut reader).unwrap();
        //
        // assert_eq!(summary.dims, [100, 100, 30]);
        // assert_eq!(summary.start_date, [1, 3, 2005, 0, 0, 0]);
        // assert_eq!(summary.items.len(), 34);
        // let n_timesteps = summary.update(&mut reader, None);
        // assert!(n_timesteps.is_ok());
        // assert_eq!(n_timesteps.unwrap(), 58);
    }
}
