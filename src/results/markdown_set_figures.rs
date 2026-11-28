//! Renders the chord layer split by whether the key is right,
//! and the chord changes and beam figures of each diagnosed set.

use super::markdown_table::{share, table};
use super::set_names::row_name;
use crate::diagnose::key_dependence::ChordLayerCounts;
use crate::diagnose::summary::SetDiagnosis;

pub fn render_key_dependence(sets: &[&SetDiagnosis]) -> String {
    let side = |counts: &ChordLayerCounts| {
        let rate = |part: u64| share(part, counts.frames);
        [
            counts.frames.to_string(),
            rate(counts.quality),
            rate(counts.inversion),
            rate(counts.root),
            rate(counts.chord),
        ]
    };
    let row = |set: &&SetDiagnosis| {
        let counts = &set.key_dependence;
        let mut cells = vec![row_name(&set.name)];
        cells.extend(side(&counts.key_right));
        cells.extend(side(&counts.key_wrong));
        cells.push(format!("{:.1}", 100.0 * counts.chord_wrong_key_right_share()));
        cells
    };
    let header = [
        "set",
        "key right: frames",
        "quality",
        "inversion",
        "root",
        "chord",
        "key wrong: frames",
        "quality",
        "inversion",
        "root",
        "chord",
        "chord errors with the key right",
    ];
    format!(
        "### Chord accuracy with the key right and wrong\n\nThe frames of each set are split by whether the top reading's key is right. Each side gives its number of frames, then the percentage of them with the quality, inversion, root and whole chord right. The last column is the percentage of all chord-wrong frames whose key is right: where it is high, chord errors are not a side effect of key errors.\n\n{}",
        table(&header, &sets.iter().map(row).collect::<Vec<_>>())
    )
}

pub fn render_beam(sets: &[&SetDiagnosis], offer_count: usize) -> String {
    let row = |set: &&SetDiagnosis| {
        let bars = set.bars.max(1) as f64;
        let beam = &set.beam;
        let per_slice = |sum: u64| format!("{:.1}", sum as f64 / beam.slices.max(1) as f64);
        vec![
            row_name(&set.name),
            format!("{:.2}", set.expert_changes as f64 / bars),
            format!("{:.2}", set.engine_changes as f64 / bars),
            per_slice(beam.hypotheses),
            per_slice(beam.readings),
            per_slice(beam.chord_keys),
            per_slice(beam.leading_readings),
        ]
    };
    let leading = format!("distinct readings among the best {offer_count}");
    let header = [
        "set",
        "expert changes per bar",
        "engine changes per bar",
        "hypotheses",
        "distinct readings",
        "distinct chord-and-key pairs",
        &leading,
    ];
    format!(
        "### Chord changes and the beam\n\nThe first two columns give how often the expert's and the engine's labels change, per bar. The rest describe the beam, the set of hypotheses the engine keeps after each slice, as averages per slice: how many hypotheses it holds, how many distinct readings and distinct chord-and-key pairs they amount to, and how many distinct readings the best few hold, from which the offered readings are drawn.\n\n{}",
        table(&header, &sets.iter().map(row).collect::<Vec<_>>())
    )
}
