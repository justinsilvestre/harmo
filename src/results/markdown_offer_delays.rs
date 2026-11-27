//! Renders how soon after an expert's label begins the engine offers its reading.

use super::markdown_table::{percent, table};
use super::set_names::row_name;
use crate::bench::offer_delay::{DELAY_LAYERS, DelaySummary};
use crate::bench::report::SetRecord;

/// The sets given a row, in order.
const SETS: [&str; 4] = ["anet-test", "tune", "agreement", "agreement-tune"];

const HEADER: [&str; 7] =
    ["set", "labels", "median delay", "within a beat", "within a bar", "within two bars", "not offered"];

const INTRODUCTION: &str = "## How soon readings are offered

Offered readings, and the readings behind **key now** and **RN now**, are read from the beam two quarter notes after each moment. These tables show how soon each expert label's reading is first offered. Each label counts once, from its first frame to its last, and every analyst's labels count, so a piece with two analyses counts twice.

The beam after each slice is read with the usual offer count and margin, for the label's first slice and for the latest slice of the label heard so far. The **delay** is the time from the label's onset to the start of the first slice whose beam offers the reading, and 0 when the beam of the slice where the label begins already offers it. The search stops two bars after the onset or at the end of the label, whichever is later, and **not offered** is the share of labels whose reading is not offered by then.
- **Median delay** is in quarter notes, over the labels whose reading is offered.
- **Within a beat, a bar, two bars** are shares of all labels, measured by the time signature at the label's onset (a beat of 6/8 is a dotted quarter). Each includes the ones before it.
";

pub fn render_offer_delays(sets: &[SetRecord]) -> String {
    let rows: Vec<&SetRecord> =
        SETS.iter().filter_map(|name| sets.iter().find(|s| s.name == *name)).collect();
    let tables: Vec<String> = DELAY_LAYERS
        .iter()
        .enumerate()
        .map(|(layer, (name, _))| format!("### {}\n\n{}", title(name), layer_table(&rows, layer)))
        .collect();
    format!("{INTRODUCTION}\n{}", tables.join("\n"))
}

fn title(layer: &str) -> &'static str {
    match layer {
        "rn" => "RN",
        "key" => "Key",
        _ => "Chord (root, quality and inversion, whatever the key)",
    }
}

fn layer_table(sets: &[&SetRecord], layer: usize) -> String {
    let row = |set: &&SetRecord| {
        let summary = set.offer_delays.as_ref().and_then(|d| d.get(layer));
        std::iter::once(row_name(&set.name)).chain(summary.map_or(vec![], cells)).collect()
    };
    table(&HEADER, &sets.iter().map(row).collect::<Vec<_>>())
}

fn cells(summary: &DelaySummary) -> Vec<String> {
    vec![
        summary.labels.to_string(),
        summary.median.map_or(String::new(), |m| format!("{m:.2}")),
        percent(summary.within_beat),
        percent(summary.within_bar),
        percent(summary.within_two_bars),
        percent(summary.not_offered),
    ]
}
