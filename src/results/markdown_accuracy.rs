//! Renders the accuracy tables for the test split and for `tune`.

use super::markdown_table::{percent, table};
use super::set_names::{family, row_name};
use crate::bench::report::{AUGMENTEDNET_REFERENCE, SetRecord};

const FEATURE_HEADER: [&str; 7] = ["corpus", "key", "degree", "quality", "inversion", "root", "RN"];
const FURTHER_HEADER: [&str; 9] =
    ["corpus", "chord", "RN given key", "offered", "per frame", "key now", "RN now", "pieces", "frames"];

const TEST_INTRODUCTION: &str = "## Accuracy on the test split

Each figure is the percentage of frames on which the engine's top reading matches the expert's label in that respect. For example, a key of 80.0 would mean that the engine names the expert's key for 80% of the music. **RN** is the full Roman numeral, right only when the other five are. AugmentedNet's published figures are in brackets; it publishes none for MPS, nor for the test split as a whole.

Each of the engine's labels is one chord in one key, so a label that is right is right on every feature at once. AugmentedNet predicts each feature separately, so its features tend to be right more often one at a time, but less often all together.
";

const FURTHER_INTRODUCTION: &str = "AugmentedNet publishes nothing comparable to these.
- **Chord:** the expert's root, quality and inversion, whatever the key.
- **RN given key:** RN, counting only the frames where the key is right.
- **Offered:** the expert's RN is among the readings the engine offers. **Per frame** is how many it offers on average.
- **Key now, RN now:** key and RN for the reading the engine holds two quarter notes after each moment, rather than after hearing the whole piece. The gap between RN and RN now shows how much the engine relies on later music to revise earlier labels.
";

const TUNE_INTRODUCTION: &str = "## Accuracy on `tune`

`tune` holds every piece outside the test split. It is the set the engine is refined on, so these are the figures that judge a change. The columns mean the same as for the test split.
";

pub fn render_test(sets: &[SetRecord]) -> String {
    let rows = family(sets, "anet-test", |s| &s.name);
    format!(
        "{TEST_INTRODUCTION}\n{}\n### Further measures on the test split\n\n{FURTHER_INTRODUCTION}\n{}",
        feature_table(&rows),
        further_table(&rows)
    )
}

pub fn render_tune(sets: &[SetRecord]) -> String {
    let rows = family(sets, "tune", |s| &s.name);
    format!(
        "{TUNE_INTRODUCTION}\n{}\n### Further measures on `tune`\n\n{}",
        feature_table(&rows),
        further_table(&rows)
    )
}

fn feature_table(sets: &[&SetRecord]) -> String {
    let row = |set: &&SetRecord| {
        let reference = AUGMENTEDNET_REFERENCE.iter().find(|(name, _)| *name == set.name).map(|(_, r)| r);
        let cell = |i: usize| match reference {
            Some(reference) => format!("{} ({})", rate(set, i), percent(reference[i])),
            None => rate(set, i),
        };
        std::iter::once(row_name(&set.name)).chain((0..6).map(cell)).collect()
    };
    table(&FEATURE_HEADER, &sets.iter().map(row).collect::<Vec<_>>())
}

fn further_table(sets: &[&SetRecord]) -> String {
    let row = |set: &&SetRecord| {
        let offered = set.mean_offered.map_or("".to_string(), |m| format!("{m:.1}"));
        let (pieces, frames) = (set.pieces.to_string(), set.frames.to_string());
        let mut cells = vec![row_name(&set.name), rate(set, 6), rate(set, 7), rate(set, 8), offered];
        cells.extend([rate(set, 9), rate(set, 10), pieces, frames]);
        cells
    };
    table(&FURTHER_HEADER, &sets.iter().map(row).collect::<Vec<_>>())
}

/// Formats the rate of feature `index`, leaving it blank where the record lacks it.
fn rate(set: &SetRecord, index: usize) -> String {
    set.rates.get(index).map_or(String::new(), |r| percent(*r))
}
