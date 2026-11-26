//! Loads ground-truth harmonies and places them on the score's timeline.

use crate::pitch::tpc_from_name;
use crate::score::model::{Measure, Offset};
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
}

#[derive(Deserialize)]
struct KeyName {
    tonic: String,
    mode: String,
}

/// A ground-truth harmony starting at a time; `None` marks a label that could not be decoded.
pub type TimedLabel = (Offset, Option<Label>);

/// Loads a ground-truth file and returns its labels in time order.
/// Harmonies in measures that the score lacks are dropped.
pub fn load_labels(path: &Path, measures: &[Measure]) -> Result<Vec<TimedLabel>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file: File = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let locate = measure_locator(measures, &file.harmonies);
    let mut labels: Vec<TimedLabel> = file
        .harmonies
        .iter()
        .filter_map(|h| locate(h.measure, &h.measure_suffix).map(|m| (time_in(m, h.offset), decode(h))))
        .collect();
    labels.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(labels)
}

/// Matches RomanText measure numbers to score measures. When the analysis has a pickup
/// measure 0 but the score numbers its pickup 1, analysis numbers are shifted up by one.
fn measure_locator<'a>(
    measures: &'a [Measure],
    harmonies: &[Harmony],
) -> impl Fn(i64, &str) -> Option<&'a Measure> {
    let parsed: Vec<(i64, String)> = measures.iter().map(|m| split_number(&m.number)).collect();
    let score_has_zero = parsed.iter().any(|(n, _)| *n == 0);
    let shift = if !score_has_zero && harmonies.iter().any(|h| h.measure == 0) { 1 } else { 0 };
    move |number, suffix| {
        let target = number + shift;
        let exact = parsed.iter().position(|(n, s)| *n == target && s == suffix);
        let any = || parsed.iter().position(|(n, _)| *n == target);
        exact.or_else(any).map(|i| &measures[i])
    }
}

fn split_number(number: &str) -> (i64, String) {
    let digits: String = number.chars().take_while(char::is_ascii_digit).collect();
    (digits.parse().unwrap_or(-1), number[digits.len()..].to_string())
}

/// Places an offset within a measure on the timeline. RomanText counts a pickup measure's beats as if
/// the measure were complete, so offsets beyond a pickup's actual length are pulled back.
fn time_in(measure: &Measure, offset: f64) -> Offset {
    let shortfall = measure.nominal_duration() - measure.duration;
    let is_beyond_pickup = shortfall > 1e-6 && offset >= measure.duration - 1e-6;
    measure.start + if is_beyond_pickup { (offset - shortfall).max(0.0) } else { offset }
}

fn decode(harmony: &Harmony) -> Option<Label> {
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
