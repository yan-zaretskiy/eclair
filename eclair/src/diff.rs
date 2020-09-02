use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::prelude::*,
    path::Path,
};

use askama::Template;
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use itertools::{EitherOrBoth, Itertools};

use crate::{
    summary::Summary,
    summary_item::{ItemId, ItemQualifier},
};

type Merged = Vec<EitherOrBoth<(usize, i64), (usize, i64)>>;

fn merge(t1: &[i64], t2: &[i64]) -> Merged {
    t1.iter()
        .copied()
        .enumerate()
        .merge_join_by(t2.iter().copied().enumerate(), |i, j| i.1.cmp(&j.1))
        .collect()
}

type Padded = Vec<[Option<f32>; 2]>;

fn pad_data(merged_time: &Merged, y1: &[f32], y2: &[f32]) -> Padded {
    use EitherOrBoth::*;

    merged_time
        .iter()
        .map(|either| match either {
            Left((index, _)) => [Some(y1[*index]), None],
            Right((index, _)) => [None, Some(y2[*index])],
            Both((index1, _), (index2, _)) => [Some(y1[*index1]), Some(y2[*index2])],
        })
        .collect()
}

struct UPlotChart<'a> {
    merged_time: &'a Merged,
    padded_data: Padded,

    title: String,
    y_label: String,
}

impl<'a> UPlotChart<'a> {
    // This is terrible API, but I might never use it more than once
    pub fn new(
        merged_time: &'a Merged,
        padded_data: Padded,
        title: String,
        y_label: String,
    ) -> Self {
        UPlotChart {
            merged_time,
            padded_data,
            title,
            y_label,
        }
    }

    pub fn data(&self) -> String {
        use EitherOrBoth::*;

        let mut result = String::from("[");

        // add a timestamp array
        let time_string = self
            .merged_time
            .iter()
            .map(|either| match either {
                Left((_, t)) | Right((_, t)) | Both((_, t), (_, _)) => t,
            })
            .join(", ");
        result.push_str(time_string.as_str());

        // add data arrays
        result.push_str("],\n[");

        let first_data_string = self
            .padded_data
            .iter()
            .map(|pair| match pair[0] {
                Some(val) => val.to_string(),
                None => "null".to_owned(),
            })
            .join(", ");
        result.push_str(first_data_string.as_str());
        result.push_str("],\n[");

        let second_data_string = self
            .padded_data
            .iter()
            .map(|pair| match pair[1] {
                Some(val) => val.to_string(),
                None => "null".to_owned(),
            })
            .join(", ");
        result.push_str(second_data_string.as_str());
        result.push_str("],");
        result
    }
}

#[derive(Template)]
#[template(path = "diff-common.html", escape = "none")]
struct DiffTemplate<'a> {
    candidate_name: &'a str,
    reference_name: &'a str,
    common_plots: Vec<UPlotChart<'a>>,
}

fn extract_time_and_data(summary: &Summary) -> (Vec<i64>, BTreeMap<&ItemId, usize>) {
    let mut times = Vec::new();
    let mut data_item_map = BTreeMap::new();

    for (index, item) in summary.items.iter().enumerate() {
        match item.id.qualifier {
            ItemQualifier::Time if { item.id.name == "TIME" } => {
                let date = &summary.start_date;
                let d = NaiveDate::from_ymd(date[2], date[1] as u32, date[0] as u32);
                let t = NaiveTime::from_hms_milli(
                    date[3] as u32,
                    date[4] as u32,
                    (date[5] / 1_000_000) as u32,
                    (date[5] % 1_000_000) as u32,
                );
                let dt = NaiveDateTime::new(d, t);

                times = summary.items[index]
                    .as_vec_of_f32()
                    .into_iter()
                    .map(|days| dt + Duration::seconds((days * 86400.0) as i64))
                    .map(|dt| dt.timestamp())
                    .collect();
            }
            ItemQualifier::Time | ItemQualifier::Unrecognized { .. } => continue,
            _ => {
                data_item_map.insert(&item.id, index);
            }
        }
    }

    (times, data_item_map)
}

pub fn diff<P: AsRef<Path>>(candidate: &Summary, reference: &Summary, output: Option<P>) {
    let (candidate_times, candidate_data_map) = extract_time_and_data(&candidate);
    let (reference_times, reference_data_map) = extract_time_and_data(&reference);

    let merged_time = merge(&candidate_times, &reference_times);

    let candidate_keys: BTreeSet<&ItemId> = candidate_data_map.keys().cloned().collect();
    let reference_keys: BTreeSet<&ItemId> = reference_data_map.keys().cloned().collect();

    let common_keys = candidate_keys.intersection(&reference_keys);

    let mut common_plots = Vec::new();

    for key in common_keys {
        let index = candidate_data_map[key];
        let candidate_data = candidate.items[index].as_vec_of_f32();

        let index = reference_data_map[key];
        let ref_item = &reference.items[index];
        let reference_data = ref_item.as_vec_of_f32();

        let padded_data = pad_data(&merged_time, &candidate_data, &reference_data);

        let title = ref_item.full_name();
        let y_title = format!("{} [{}]", ref_item.id.name, ref_item.unit);

        common_plots.push(UPlotChart::new(&merged_time, padded_data, title, y_title));
    }

    let diff_html = DiffTemplate {
        candidate_name: &candidate.name,
        reference_name: &reference.name,
        common_plots,
    };

    let rendered = diff_html.render().unwrap();

    let mut file = if let Some(name) = output {
        File::create(name).unwrap()
    } else {
        File::create("diff.html").unwrap()
    };
    file.write_all(rendered.as_bytes()).unwrap();
}
