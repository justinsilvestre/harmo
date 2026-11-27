//! Finds runs of labels that fit the notes markedly better when moved in time or read in another key.
//!
//! Each label's gain is the change in its chord-tone share, weighted by the frames it covers.
//! A candidate is a run of consecutive labels whose gains add up to a large total,
//! found as the maximal-scoring runs of the gain sequence.

use super::piece::{LabelSpan, PieceFit};
use super::rekey::root_motion;
use super::sounding::{Fit, pitch_class_set};
use crate::theory::key::{Key, Mode};
use serde::Serialize;

/// The least total gain, in frames, for a run to be reported by default: a whole 4/4 bar going from no fit
/// to a full one.
pub const MINIMUM_GAIN: f64 = 32.0;

/// A change to the labels whose effect on the fit is tested.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Alteration {
    /// Every label moves this many bars later (negative for earlier).
    Bars(i32),
    /// Every label moves this many beats later.
    Beats(i32),
    /// Every label's figure is read in the key this many fifths away, in the same mode or the other one.
    Key { fifths: i32, other_mode: bool },
}

/// The largest move in time tested, in bars. A first ending that the analysis leaves out can be this long.
const MAXIMUM_BARS: i32 = 8;

/// Returns every alteration tested: moves by a beat or by up to eight bars, and the 23 other keys.
pub fn alterations() -> Vec<Alteration> {
    let bars = (-MAXIMUM_BARS..=MAXIMUM_BARS).filter(|b| *b != 0).map(Alteration::Bars);
    let beats = [-1, 1].map(Alteration::Beats);
    let keys =
        (-5..=6).flat_map(|fifths| [false, true].map(|other_mode| Alteration::Key { fifths, other_mode }));
    let keys = keys.filter(|a| *a != Alteration::Key { fifths: 0, other_mode: false });
    bars.chain(beats).chain(keys).collect()
}

#[derive(Clone, Debug, Serialize)]
pub struct Candidate {
    pub piece: String,
    pub analyst: String,
    pub alteration: Alteration,
    pub description: String,
    pub first_measure: String,
    pub last_measure: String,
    pub labels: usize,
    /// The frames of the run's labels that can be compared, which excludes those that would leave the score.
    pub frames: f64,
    /// The chord-tone share of the run's labels as they stand and as altered, weighted by their frames.
    pub fit_before: f64,
    pub fit_after: f64,
    /// The summed gain in frames: frames at stake times the gain in chord-tone share.
    pub gain: f64,
}

/// Returns the runs of labels whose gain under any alteration reaches `minimum_gain`, best first,
/// keeping only the best of runs that overlap.
pub fn find_candidates(piece: &PieceFit, minimum_gain: f64) -> Vec<Candidate> {
    let mut found: Vec<(usize, usize, Candidate)> = Vec::new();
    for alteration in alterations() {
        let altered: Vec<Option<Fit>> =
            piece.spans.iter().map(|s| altered_fit(piece, s, alteration)).collect();
        let gains: Vec<f64> = piece.spans.iter().zip(&altered).map(|(s, a)| gain(s, *a)).collect();
        for (first, last) in best_runs(&gains, (0, gains.len()), minimum_gain) {
            found.push((first, last, describe(piece, alteration, (first, last), &altered)));
        }
    }
    found.sort_by(|a, b| b.2.gain.total_cmp(&a.2.gain));
    let mut kept: Vec<(usize, usize, Candidate)> = Vec::new();
    for (first, last, candidate) in found {
        let overlaps = |k: &(usize, usize, Candidate)| first <= k.1 && k.0 <= last;
        if !kept.iter().any(overlaps) {
            kept.push((first, last, candidate));
        }
    }
    kept.into_iter().map(|(_, _, c)| c).collect()
}

/// Returns the fit of a label under an alteration, or `None` when it would fall outside the score.
pub fn altered_fit(piece: &PieceFit, span: &LabelSpan, alteration: Alteration) -> Option<Fit> {
    let measure = piece.measure_at(span.start);
    let offset = match alteration {
        Alteration::Bars(bars) => f64::from(bars) * measure.nominal_duration(),
        Alteration::Beats(beats) => f64::from(beats) * beat_length(measure.beats, measure.beat_type),
        Alteration::Key { fifths, other_mode } => {
            let mode = match (span.label.key.mode, other_mode) {
                (Mode::Major, false) | (Mode::Minor, true) => Mode::Major,
                _ => Mode::Minor,
            };
            let motion = root_motion(&span.label, Key::new(span.label.key.tonic + fifths, mode));
            let chord = pitch_class_set(span.pitches.iter().map(|p| p + motion));
            return Some(piece.sounding.fit(span.start, span.end, chord));
        }
    };
    let (start, end) = (span.start + offset, span.end + offset);
    let score_end = piece.measures.last().map_or(0.0, |m| m.start + m.duration);
    (start >= 0.0 && end <= score_end + 1e-6).then(|| piece.sounding.fit(start, end, span.chord))
}

/// Returns the length of a beat in quarter notes, counting compound meters such as 6/8 in dotted beats.
fn beat_length(beats: u32, beat_type: u32) -> f64 {
    let unit = 4.0 / f64::from(beat_type);
    if beat_type >= 8 && beats.is_multiple_of(3) && beats > 3 { 3.0 * unit } else { unit }
}

fn gain(span: &LabelSpan, altered: Option<Fit>) -> f64 {
    match (span.fit.share(), altered.and_then(Fit::share)) {
        (Some(before), Some(after)) => span.frames() * (after - before),
        _ => 0.0,
    }
}

/// Returns disjoint runs (first and last index) whose gains sum to at least the minimum,
/// taking the best run within the range and then searching the parts on either side of it.
fn best_runs(gains: &[f64], (from, to): (usize, usize), minimum_gain: f64) -> Vec<(usize, usize)> {
    let Some((first, last, total)) = best_run(&gains[from..to]) else {
        return Vec::new();
    };
    if total < minimum_gain {
        return Vec::new();
    }
    let (first, last) = (from + first, from + last);
    let mut runs = best_runs(gains, (from, first), minimum_gain);
    runs.push((first, last));
    runs.extend(best_runs(gains, (last + 1, to), minimum_gain));
    runs
}

/// Returns the contiguous run with the largest sum (Kadane's algorithm), with its sum.
fn best_run(gains: &[f64]) -> Option<(usize, usize, f64)> {
    let mut best: Option<(usize, usize, f64)> = None;
    let (mut start, mut sum) = (0, 0.0);
    for (index, gain) in gains.iter().enumerate() {
        if sum <= 0.0 {
            (start, sum) = (index, 0.0);
        }
        sum += gain;
        if best.is_none_or(|b| sum > b.2) {
            best = Some((start, index, sum));
        }
    }
    best
}

fn describe(
    piece: &PieceFit,
    alteration: Alteration,
    run: (usize, usize),
    altered: &[Option<Fit>],
) -> Candidate {
    let spans = &piece.spans[run.0..=run.1];
    let shares = spans.iter().zip(&altered[run.0..=run.1]).filter_map(|(span, altered)| {
        Some((span.frames(), span.fit.share()?, altered.and_then(Fit::share)?))
    });
    let (frames, before, after) = shares.fold((0.0, 0.0, 0.0), |(f, b, a), (frames, before, after)| {
        (f + frames, b + frames * before, a + frames * after)
    });
    Candidate {
        piece: piece.id.clone(),
        analyst: piece.analyst.clone(),
        alteration,
        description: describe_alteration(&spans[0], alteration),
        first_measure: spans[0].measure_name(),
        last_measure: spans[spans.len() - 1].measure_name(),
        labels: spans.len(),
        frames,
        fit_before: before / frames.max(1e-9),
        fit_after: after / frames.max(1e-9),
        gain: after - before,
    }
}

fn describe_alteration(first: &LabelSpan, alteration: Alteration) -> String {
    match alteration {
        Alteration::Bars(bars) => format!("labels {} bar(s) {}", bars.abs(), direction(bars)),
        Alteration::Beats(beats) => format!("labels {} beat(s) {}", beats.abs(), direction(beats)),
        Alteration::Key { fifths, other_mode } => {
            let key = first.label.key;
            let mode = if (key.mode == Mode::Major) == other_mode { Mode::Minor } else { Mode::Major };
            format!("read {} as {}", key.name(), Key::new(key.tonic + fifths, mode).name())
        }
    }
}

fn direction(amount: i32) -> &'static str {
    if amount < 0 { "earlier" } else { "later" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::groundtruth::PlacedHarmony;
    use crate::pitch::tpc_from_name;
    use crate::score::model::{Measure, Note, Score};
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::label::Label;

    const PROGRESSION: [&str; 6] = ["C", "F", "G", "C", "F", "G"];

    /// A score of one major triad per 4/4 bar, labeled in C with each label one bar late.
    fn labeled_a_bar_late() -> PieceFit {
        let tpc = |name: &str| tpc_from_name(name).unwrap();
        let triad = |root: &str| [0, 4, 1].map(|fifths| tpc(root) + fifths);
        let note = |(bar, tpc): (usize, i32)| Note {
            onset: 4.0 * bar as f64,
            duration: 4.0,
            tpc,
            midi: 60,
            part: 0,
            tied_from_previous: false,
            has_fermata: false,
        };
        let notes = PROGRESSION.iter().enumerate().flat_map(|(bar, root)| triad(root).map(|t| (bar, t)));
        let measure = |bar: usize| Measure {
            number: (bar + 1).to_string(),
            start: 4.0 * bar as f64,
            duration: 4.0,
            beats: 4,
            beat_type: 4,
        };
        let score = Score {
            notes: notes.map(note).collect(),
            measures: (0..PROGRESSION.len() + 1).map(measure).collect(),
            ..Score::default()
        };
        let key = Key::parse("C", "major").unwrap();
        let harmony = |(bar, root): (usize, &&str)| PlacedHarmony {
            time: 4.0 * (bar + 1) as f64,
            label: Some(Label::new(key, None, Chord::new(tpc(root), Quality::Major), tpc(root))),
            is_excluded: false,
            is_beyond_bar: false,
            measure: bar as i64 + 2,
            measure_suffix: String::new(),
            pitches: triad(root).to_vec(),
        };
        let harmonies: Vec<PlacedHarmony> = PROGRESSION.iter().enumerate().map(harmony).collect();
        PieceFit::new(("piece".into(), "main".into(), vec![]), &score, &harmonies)
    }

    #[test]
    fn labels_a_bar_late_are_found_to_fit_a_bar_earlier() {
        let candidates = find_candidates(&labeled_a_bar_late(), MINIMUM_GAIN);
        assert_eq!(candidates.first().map(|c| c.alteration), Some(Alteration::Bars(-1)));
    }

    #[test]
    fn best_run_skips_losses_at_either_end() {
        assert_eq!(best_run(&[-5.0, 3.0, -1.0, 4.0, -9.0]), Some((1, 3, 6.0)));
    }

    #[test]
    fn separate_runs_are_both_found() {
        let gains = [40.0, -100.0, 50.0];
        assert_eq!(best_runs(&gains, (0, gains.len()), MINIMUM_GAIN), vec![(0, 0), (2, 2)]);
    }

    #[test]
    fn compound_meter_beats_are_dotted() {
        assert_eq!(beat_length(6, 8), 1.5);
    }
}
