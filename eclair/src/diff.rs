use std::collections::{HashMap, HashSet};

use chrono::Duration;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use plotly::common::{Mode, Title};
use plotly::layout::{Axis, Layout};
use plotly::{Plot, Scatter};

use crate::summary::Summary;
use crate::summary_item::{ItemId, ItemQualifier};

fn extract_time_and_data(summary: &Summary) -> (Vec<String>, HashMap<&ItemId, usize>) {
    let mut times = Vec::new();
    let mut data_item_map = HashMap::new();

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
                    .map(|dt| dt.format("%F %T").to_string())
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

pub fn diff(candidate: &Summary, reference: &Summary) {
    let (candidate_times, candidate_data_map) = extract_time_and_data(&candidate);
    let (reference_times, reference_data_map) = extract_time_and_data(&reference);

    let candidate_keys: HashSet<&ItemId> = candidate_data_map.keys().cloned().collect();
    let reference_keys: HashSet<&ItemId> = reference_data_map.keys().cloned().collect();

    let common_keys = candidate_keys.intersection(&reference_keys);
    for key in common_keys {
        let index = candidate_data_map[key];
        let candidate_data = candidate.items[index].as_vec_of_f32();

        let trace1 = Scatter::new(candidate_times, candidate_data)
            .name(&candidate.name)
            .mode(Mode::Lines);

        let index = reference_data_map[key];
        let reference_data = reference.items[index].as_vec_of_f32();

        let trace2 = Scatter::new(reference_times, reference_data)
            .name(&reference.name)
            .mode(Mode::Lines);

        let mut plot = Plot::new();
        plot.add_trace(trace1);
        plot.add_trace(trace2);

        let id = &reference.items[index].id;

        let title = if let Some(loc) = id.location() {
            format!("{} @ {}", id.name, loc)
        } else {
            id.name.to_string()
        };

        let y_title = format!("{} [{}]", id.name, reference.items[index].unit);

        let layout = Layout::new()
            .height(800)
            .title(Title::new(title.as_str()))
            .x_axis(Axis::new().title(Title::new("Date")))
            .y_axis(Axis::new().title(Title::new(y_title.as_str())));
        plot.set_layout(layout);

        println!("{}", plot.to_inline_html(Some("My plot")));
        break;
    }
}
