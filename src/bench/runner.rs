//! Loads benchmark pieces once and evaluates rule sets on them in parallel.

use super::frames::{FRAME, frame_labels, frame_slices};
use super::groundtruth::load_labels;
use super::manifest::{Manifest, ManifestPiece};
use super::metrics::{Accuracy, Agreement};
use crate::engine::history::segment_at;
use crate::engine::path::segments_of;
use crate::engine::ruleset::RuleSet;
use crate::engine::search::{Analysis, Config, analyze};
use crate::score::model::Measure;
use crate::score::musicxml::read_score;
use crate::score::slicing::{Slice, slice_score};
use crate::theory::label::Label;
use rayon::prelude::*;
use serde::Serialize;
use std::time::Instant;

/// A piece ready for evaluation: sliced score plus frame labels from each analyst ("main" first).
pub struct BenchPiece {
    pub id: String,
    pub sets: Vec<String>,
    pub slices: Vec<Slice>,
    pub measures: Vec<Measure>,
    pub analyses: Vec<(String, Vec<Option<Label>>)>,
}

#[derive(Clone, Debug)]
pub struct EvalConfig {
    pub engine: Config,
    /// At most this many final hypotheses count as "offered" readings...
    pub offer_count: usize,
    /// ...and only those within this score margin of the best.
    pub offer_margin: f32,
    /// Readings are offered as the beam stood this many quarter notes after the moment in question,
    /// modeling a listener who has heard a little further.
    pub offer_lag: f64,
}

impl Default for EvalConfig {
    fn default() -> EvalConfig {
        EvalConfig { engine: Config::default(), offer_count: 8, offer_margin: 4.0, offer_lag: 2.0 }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PieceResult {
    pub id: String,
    pub sets: Vec<String>,
    pub slices: usize,
    pub seconds: f64,
    pub accuracy: Accuracy,
    pub agreement: Option<Agreement>,
}

/// Loads pieces in parallel, returning the loaded pieces and (piece id, error) pairs for failures.
pub fn load_pieces(
    manifest: &Manifest,
    selected: &[&ManifestPiece],
) -> (Vec<BenchPiece>, Vec<(String, String)>) {
    let results: Vec<Result<BenchPiece, (String, String)>> =
        selected.par_iter().map(|p| load_piece(manifest, p).map_err(|e| (p.id.clone(), e))).collect();
    let mut pieces = Vec::new();
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(piece) => pieces.push(piece),
            Err(error) => errors.push(error),
        }
    }
    (pieces, errors)
}

fn load_piece(manifest: &Manifest, entry: &ManifestPiece) -> Result<BenchPiece, String> {
    let score = read_score(&manifest.root.join(&entry.score))?;
    let mut analyses = Vec::new();
    for (analyst, path) in &entry.analyses {
        let labels = load_labels(&manifest.root.join(path), &score)?;
        analyses.push((analyst.clone(), frame_labels(&labels, score.end())));
    }
    analyses.sort_by_key(|(analyst, _)| analyst != "main");
    if analyses.first().is_none_or(|(analyst, _)| analyst != "main") {
        return Err("no main analysis".into());
    }
    Ok(BenchPiece {
        id: entry.id.clone(),
        sets: entry.sets.clone(),
        slices: slice_score(&score),
        measures: score.measures,
        analyses,
    })
}

pub fn evaluate(pieces: &[BenchPiece], rules: &RuleSet, config: &EvalConfig) -> Vec<PieceResult> {
    pieces.par_iter().map(|piece| evaluate_piece(piece, rules, config)).collect()
}

pub fn evaluate_piece(piece: &BenchPiece, rules: &RuleSet, config: &EvalConfig) -> PieceResult {
    let started = Instant::now();
    let needs_snapshots = piece.analyses.len() >= 2;
    let engine = Config { record_snapshots: needs_snapshots, ..config.engine.clone() };
    let analysis = analyze(&piece.slices, rules, &engine);
    let top = top_frames(&analysis, piece);
    let mut accuracy = Accuracy::default();
    for (predicted, truth) in top.iter().zip(&piece.analyses[0].1) {
        if let Some(truth) = truth {
            accuracy.add_frame(predicted.as_ref(), truth);
        }
    }
    let agreement = (piece.analyses.len() >= 2).then(|| agreement(&analysis, piece, &top, config));
    PieceResult {
        id: piece.id.clone(),
        sets: piece.sets.clone(),
        slices: piece.slices.len(),
        seconds: started.elapsed().as_secs_f64(),
        accuracy,
        agreement,
    }
}

/// Returns the label of the best final hypothesis at each frame of the piece.
pub fn top_frames(analysis: &Analysis, piece: &BenchPiece) -> Vec<Option<Label>> {
    let count = piece.analyses[0].1.len();
    let top = analysis.ranking.first().map(|&(node, _)| hypothesis_frames(analysis, piece, node, count));
    top.unwrap_or_else(|| vec![None; count])
}

fn hypothesis_frames(analysis: &Analysis, piece: &BenchPiece, node: u32, count: usize) -> Vec<Option<Label>> {
    let segments = segments_of(analysis, &piece.slices, node);
    let timed: Vec<_> = segments.iter().map(|s| (s.start, Some(s.label))).collect();
    let mut frames = frame_labels(&timed, count as f64 * FRAME);
    frames.resize(count, None);
    frames
}

/// Compares the engine with every pair of analysts of the piece.
fn agreement(
    analysis: &Analysis,
    piece: &BenchPiece,
    top: &[Option<Label>],
    config: &EvalConfig,
) -> Agreement {
    let offered = offered_frames(analysis, piece, top.len(), config);
    let mut agreement = Agreement::default();
    for (i, (_, first)) in piece.analyses.iter().enumerate() {
        for (_, second) in &piece.analyses[i + 1..] {
            for frame in 0..top.len() {
                if let (Some(a), Some(b)) = (&first[frame], &second[frame]) {
                    agreement.add_frame(top[frame].as_ref(), &offered[frame], a, b);
                }
            }
        }
    }
    agreement
}

/// Returns, for each frame, the distinct readings of its slice held by the leading hypotheses of the
/// beam a short time later (see `EvalConfig::offer_lag`).
pub fn offered_frames(
    analysis: &Analysis,
    piece: &BenchPiece,
    count: usize,
    config: &EvalConfig,
) -> Vec<Vec<Label>> {
    let slices = &piece.slices;
    let per_slice: Vec<Vec<Label>> =
        (0..slices.len()).map(|i| offered_at(analysis, slices, i, config)).collect();
    let offers_at = |slice: usize| per_slice.get(slice).cloned().unwrap_or_default();
    frame_slices(slices, count).into_iter().map(offers_at).collect()
}

/// Returns the beam as it stood `lag` quarter notes after the slice `at` began, best first.
/// Requires an analysis run with `Config::record_snapshots`.
pub fn lagged_beam<'a>(analysis: &'a Analysis, slices: &[Slice], at: usize, lag: f64) -> Option<&'a [u32]> {
    let horizon = slices[at].start + lag;
    let later = slices[at..].partition_point(|s| s.start <= horizon + 1e-6);
    analysis.snapshots.get(at + later.saturating_sub(1)).map(Vec::as_slice)
}

fn offered_at(analysis: &Analysis, slices: &[Slice], at: usize, config: &EvalConfig) -> Vec<Label> {
    let Some(beam) = lagged_beam(analysis, slices, at, config.offer_lag) else {
        return vec![];
    };
    let Some(&best) = beam.first() else {
        return vec![];
    };
    let best_score = analysis.arena[best as usize].score;
    let is_close = |node: &&u32| best_score - analysis.arena[**node as usize].score <= config.offer_margin;
    let mut labels: Vec<Label> = Vec::new();
    for &node in beam.iter().take(config.offer_count).filter(is_close) {
        let Some(segment) = segment_at(analysis, node, at) else {
            continue;
        };
        let reading = segment.reading;
        let label = Label::new(reading.key, reading.tonicized, reading.chord, segment.bass_tpc());
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    labels
}
