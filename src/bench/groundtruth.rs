//! Loads ground-truth harmonies and places them on the score's timeline.

use super::groundtruth_measures::{AnalysisMeasure, locate_measures};
use crate::pitch::{pitch_class, tpc_from_name};
use crate::score::model::{Measure, Note, Offset, Score};
use crate::theory::chord::{Chord, classify};
use crate::theory::key::Key;
use crate::theory::label::Label;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct File {
    harmonies: Vec<Harmony>,
}

#[derive(Deserialize)]
struct Harmony {
    measure: i64,
    measure_suffix: String,
    offset: f64,
    key: KeyName,
    tonicized_key: Option<KeyName>,
    root: String,
    bass: String,
    pitches: Vec<String>,
    /// Why the harmony is left out of the benchmark, when the label is known to be wrong.
    #[serde(default)]
    excluded: Option<String>,
}

#[derive(Deserialize)]
struct KeyName {
    tonic: String,
    mode: String,
}

/// A ground-truth harmony starting at a time; `None` marks a label that could not be decoded or was excluded.
pub type TimedLabel = (Offset, Option<Label>);

/// Loads a ground-truth file and returns its labels in time order.
/// Harmonies in measures that the score lacks are dropped.
pub fn load_labels(path: &Path, score: &Score) -> Result<Vec<TimedLabel>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file: File = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let references: Vec<AnalysisMeasure> =
        file.harmonies.iter().map(|h| (h.measure, h.measure_suffix.as_str())).collect();
    let fit = |index: usize, bar: &Measure| chord_tone_share(&file.harmonies[index], &score.notes, bar);
    let (bars, located) = locate_measures(&score.measures, &references, fit);
    let place = |(harmony, bar): (&Harmony, Option<usize>)| {
        bar.map(|i| (time_in(&bars[i], harmony.offset), decode(harmony)))
    };
    let mut labels: Vec<TimedLabel> = file.harmonies.iter().zip(located).filter_map(place).collect();
    labels.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(labels)
}

/// Returns the share of the bar's sounding duration that belongs to the harmony's pitch classes.
fn chord_tone_share(harmony: &Harmony, notes: &[Note], bar: &Measure) -> f64 {
    let classes: Vec<u8> = harmony.pitches.iter().filter_map(|p| tpc_from_name(p)).map(pitch_class).collect();
    let end = bar.start + bar.duration;
    let in_bar = notes.iter().filter(|n| n.onset >= bar.start - 1e-6 && n.onset < end - 1e-6);
    let (matching, total) = in_bar.fold((0.0, 0.0), |(matching, total), note| {
        let is_chord_tone = classes.contains(&pitch_class(note.tpc));
        (matching + if is_chord_tone { note.duration } else { 0.0 }, total + note.duration)
    });
    if total > 0.0 { matching / total } else { 0.0 }
}

/// Places an offset within a measure on the timeline. RomanText counts a pickup measure's beats as if
/// the measure were complete, so offsets beyond a pickup's actual length are pulled back.
fn time_in(measure: &Measure, offset: f64) -> Offset {
    let shortfall = measure.nominal_duration() - measure.duration;
    let is_beyond_pickup = shortfall > 1e-6 && offset >= measure.duration - 1e-6;
    measure.start + if is_beyond_pickup { (offset - shortfall).max(0.0) } else { offset }
}

fn decode(harmony: &Harmony) -> Option<Label> {
    if harmony.excluded.is_some() {
        return None;
    }
    let key = Key::parse(&harmony.key.tonic, &harmony.key.mode)?;
    let tonicized = match &harmony.tonicized_key {
        Some(name) => Some(Key::parse(&name.tonic, &name.mode)?),
        None => None,
    };
    let root = tpc_from_name(&harmony.root)?;
    let pitches: Option<Vec<_>> = harmony.pitches.iter().map(|p| tpc_from_name(p)).collect();
    let chord = Chord::new(root, classify(root, &pitches?));
    Some(Label::new(key, tonicized, chord, tpc_from_name(&harmony.bass)?))
}
