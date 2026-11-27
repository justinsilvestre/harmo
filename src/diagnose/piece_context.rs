//! Everything known about one piece once the engine has run on it, laid out frame by frame.

use super::frame_runs::{key_run_lengths, label_runs};
use super::producibility::{Gap, find_gap};
use crate::bench::frames::{FRAME, frame_slices};
use crate::bench::metrics::is_rn_match;
use crate::bench::runner::{BenchPiece, EvalConfig, top_frames};
use crate::engine::ruleset::RuleSet;
use crate::engine::search::{Analysis, Config, analyze};
use crate::score::slicing::measure_at;
use crate::theory::label::Label;

/// Key errors that last this many bars or more count as sustained.
const SUSTAINED_BARS: f64 = 2.0;

pub struct PieceContext<'a> {
    pub piece: &'a BenchPiece,
    /// An analysis with beam snapshots and pruned hypotheses recorded.
    pub analysis: Analysis,
    /// The top reading at each frame.
    pub top: Vec<Option<Label>>,
    /// The slice sounding at each frame.
    pub frame_slices: Vec<usize>,
    /// For each analyst, why the engine cannot produce the analyst's label at each frame.
    pub gaps: Vec<Vec<Option<Gap>>>,
    /// The length in frames of the run over which the main analyst's key and the engine's stay the same.
    pub key_runs: Vec<usize>,
    pub sustained_frames: usize,
}

impl<'a> PieceContext<'a> {
    pub fn new(piece: &'a BenchPiece, rules: &RuleSet, config: &EvalConfig) -> PieceContext<'a> {
        let engine = Config { record_snapshots: true, record_pruning: true, ..config.engine.clone() };
        let analysis = analyze(&piece.slices, rules, &engine);
        let top = top_frames(&analysis, piece);
        let frame_slices = frame_slices(&piece.slices, top.len());
        let gap_frames =
            |(_, frames): &(String, Vec<Option<Label>>)| frame_gaps(piece, rules, &frame_slices, frames);
        let gaps = piece.analyses.iter().map(gap_frames).collect();
        let key_runs = key_run_lengths(&piece.analyses[0].1, &top);
        let sustained_frames = (SUSTAINED_BARS * mean_bar(piece) / FRAME).round() as usize;
        PieceContext { piece, analysis, top, frame_slices, gaps, key_runs, sustained_frames }
    }

    pub fn main_frames(&self) -> &[Option<Label>] {
        &self.piece.analyses[0].1
    }

    /// Reports whether another analyst labels the frame differently from the main analyst.
    pub fn is_disagreement(&self, frame: usize) -> bool {
        let Some(main) = &self.main_frames()[frame] else {
            return false;
        };
        let differs = |(_, frames): &(String, Vec<Option<Label>>)| {
            frames[frame].is_some_and(|l| !is_rn_match(&l, main))
        };
        self.piece.analyses[1..].iter().any(differs)
    }

    /// Returns the measure number, as written in the score, at the frame.
    pub fn measure_of(&self, frame: usize) -> String {
        let measures = &self.piece.measures;
        measures.get(measure_at(measures, frame as f64 * FRAME)).map_or("?".into(), |m| m.number.clone())
    }
}

/// Checks each run of equal labels once, on the slices it spans.
fn frame_gaps(
    piece: &BenchPiece,
    rules: &RuleSet,
    slice_of: &[usize],
    frames: &[Option<Label>],
) -> Vec<Option<Gap>> {
    let mut gaps = vec![None; frames.len()];
    for (start, end) in label_runs(frames) {
        let Some(label) = &frames[start] else {
            continue;
        };
        let slices = piece.slices.get(slice_of[start]..=slice_of[end - 1]).unwrap_or(&[]);
        gaps[start..end].fill(find_gap(rules, label, slices));
    }
    gaps
}

fn mean_bar(piece: &BenchPiece) -> f64 {
    let total: f64 = piece.measures.iter().map(|m| m.duration).sum();
    if piece.measures.is_empty() { 4.0 } else { total / piece.measures.len() as f64 }
}
