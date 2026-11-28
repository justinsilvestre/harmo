//! Measures how soon after an expert's label begins the engine offers its reading.
//!
//! The beam after each slice is read with the offer count and margin,
//! for the label's first slice and for the latest slice of the label heard so far.
//! The delay is the time from the label's onset
//! to the start of the first slice whose beam offers the label's reading in either of those.
//! The search ends two bars after the onset or at the end of the label, whichever is later.

use super::frames::{FRAME, frame_slices, label_runs_by};
use super::metrics::{feature_matches, is_rn_match};
use super::runner::{BenchPiece, EvalConfig, offered_from};
use crate::engine::search::Analysis;
use crate::score::meter::beat_length;
use crate::score::model::{Measure, Offset};
use crate::score::slicing::Slice;
use crate::theory::label::Label;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const EPSILON: f64 = 1e-6;

/// The index of "chord" in the flags of `feature_matches`.
const CHORD: usize = 6;

/// Tests whether two labels agree in some respect.
pub type Matches = fn(&Label, &Label) -> bool;

/// The respects in which an offered reading must match an expert's label, by name.
pub const DELAY_LAYERS: [(&str, Matches); 3] =
    [("rn", is_rn_match), ("key", is_same_key), ("chord", is_same_chord)];

/// How soon each expert label's reading was offered, for each of `DELAY_LAYERS`.
pub type OfferDelays = [LayerDelays; DELAY_LAYERS.len()];

#[derive(Clone, Debug, Default)]
pub struct LayerDelays {
    pub labels: u64,
    pub within_beat: u64,
    pub within_bar: u64,
    pub within_two_bars: u64,
    /// The delay of each label whose reading was offered before the search ended, in quarter notes.
    pub delays: Vec<f64>,
}

/// The figures of `LayerDelays` for one layer of a set.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DelaySummary {
    pub layer: String,
    pub labels: u64,
    /// The median delay of the labels whose reading was offered, in quarter notes.
    pub median: Option<f64>,
    pub within_beat: f64,
    pub within_bar: f64,
    pub within_two_bars: f64,
    /// The share of labels whose reading was not offered before the search ended.
    pub not_offered: f64,
}

impl LayerDelays {
    /// Adds a label, given its delay (if its reading was offered) and its measure's beat and bar.
    pub fn add(&mut self, delay: Option<f64>, beat: Offset, bar: Offset) {
        self.labels += 1;
        let Some(delay) = delay else {
            return;
        };
        self.within_beat += (delay <= beat + EPSILON) as u64;
        self.within_bar += (delay <= bar + EPSILON) as u64;
        self.within_two_bars += (delay <= 2.0 * bar + EPSILON) as u64;
        self.delays.push(delay);
    }

    pub fn merge(&mut self, other: &LayerDelays) {
        self.labels += other.labels;
        self.within_beat += other.within_beat;
        self.within_bar += other.within_bar;
        self.within_two_bars += other.within_two_bars;
        self.delays.extend(&other.delays);
    }

    pub fn summary(&self, layer: &str) -> DelaySummary {
        let share = |count: u64| if self.labels == 0 { 0.0 } else { count as f64 / self.labels as f64 };
        DelaySummary {
            layer: layer.to_string(),
            labels: self.labels,
            median: median(&self.delays),
            within_beat: share(self.within_beat),
            within_bar: share(self.within_bar),
            within_two_bars: share(self.within_two_bars),
            not_offered: share(self.labels - self.delays.len() as u64),
        }
    }
}

/// Measures, for every label of every analysis of the piece, how soon its reading is offered.
pub fn offer_delays(analysis: &Analysis, piece: &BenchPiece, config: &EvalConfig) -> OfferDelays {
    let frames = piece.analyses[0].1.len();
    let mut search = DelaySearch {
        analysis,
        config,
        slices: &piece.slices,
        frame_slices: frame_slices(&piece.slices, frames),
        offered: HashMap::new(),
    };
    std::array::from_fn(|layer| {
        let matches = DELAY_LAYERS[layer].1;
        let mut delays = LayerDelays::default();
        for (_, labels) in &piece.analyses {
            for (start, end) in label_runs_by(labels, matches) {
                let Some(truth) = &labels[start] else {
                    continue;
                };
                let measure = measure_at(&piece.measures, start as f64 * FRAME);
                let (beat, bar) = measure.map_or((1.0, 4.0), |m| (beat_length(m), m.nominal_duration()));
                delays.add(search.delay(truth, (start, end), bar, matches), beat, bar);
            }
        }
        delays
    })
}

struct DelaySearch<'a> {
    analysis: &'a Analysis,
    config: &'a EvalConfig,
    slices: &'a [Slice],
    frame_slices: Vec<usize>,
    /// The readings offered by the beam after a slice (first) for a slice (second).
    offered: HashMap<(usize, usize), Vec<Label>>,
}

impl DelaySearch<'_> {
    /// Returns the delay in quarter notes before `truth`, labeling the frames `start..end`,
    /// is offered, or `None` if it is not offered before the search ends.
    fn delay(
        &mut self,
        truth: &Label,
        (start, end): (usize, usize),
        bar: Offset,
        matches: Matches,
    ) -> Option<f64> {
        let onset = start as f64 * FRAME;
        let search_end = (end as f64 * FRAME).max(onset + 2.0 * bar);
        let (first, last) = (self.frame_slices[start], self.frame_slices[end - 1]);
        for heard in first..self.slices.len() {
            if self.slices[heard].start > search_end + EPSILON {
                break;
            }
            let latest = heard.min(last);
            if [first, latest].into_iter().any(|at| self.offered(heard, at).iter().any(|o| matches(o, truth)))
            {
                return Some((self.slices[heard].start - onset).max(0.0));
            }
        }
        None
    }

    fn offered(&mut self, heard: usize, at: usize) -> &[Label] {
        let (analysis, config) = (self.analysis, self.config);
        self.offered.entry((heard, at)).or_insert_with(|| {
            analysis.layers.key.ranking(heard).map_or(vec![], |beam| offered_from(analysis, beam, at, config))
        })
    }
}

fn is_same_key(a: &Label, b: &Label) -> bool {
    a.key == b.key
}

fn is_same_chord(a: &Label, b: &Label) -> bool {
    feature_matches(a, b)[CHORD]
}

/// Returns the measure sounding at `time`.
fn measure_at(measures: &[Measure], time: Offset) -> Option<&Measure> {
    let after = measures.partition_point(|m| m.start <= time + EPSILON);
    measures.get(after.checked_sub(1)?)
}

fn median(values: &[f64]) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    match sorted.len() {
        0 => None,
        n if n % 2 == 0 => Some((sorted[middle - 1] + sorted[middle]) / 2.0),
        _ => Some(sorted[middle]),
    }
}

#[cfg(test)]
#[path = "offer_delay_tests.rs"]
mod tests;
