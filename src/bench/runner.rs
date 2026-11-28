//! Loads benchmark pieces once and evaluates rule sets on them in parallel.

use super::equivalence::{ExpertEquivalents, ExpertView, FrameSounds};
use super::frames::{FRAME, frame_labels, frame_slices};
use super::groundtruth::load_labels;
use super::hindsight::descendant_bests;
use super::layer_frames::{hindsight_frames, layer_frames};
use super::layer_offers::LayerOffers;
use super::manifest::{Manifest, ManifestPiece};
use super::metrics::{Accuracy, Agreement, EquivalentMatches};
use super::offer_delay::{OfferDelays, offer_delays};
use crate::engine::history::segment_at;
use crate::engine::model::{Function, KeyHypothesisId};
use crate::engine::path::segments_of;
use crate::engine::ranking::Ranking;
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
    /// What sounds at each frame.
    pub sounds: FrameSounds,
    /// What each analysis implies about equivalent labels, in the order of `analyses`.
    pub equivalents: Vec<ExpertEquivalents>,
}

impl BenchPiece {
    /// Returns the analysis at `index` with what is needed to judge equivalent labels against it,
    /// counting a label as early or late by up to `tolerance` quarter notes.
    pub fn expert_view(&self, index: usize, tolerance: f64) -> ExpertView<'_> {
        ExpertView {
            labels: &self.analyses[index].1,
            equivalents: &self.equivalents[index],
            sounds: &self.sounds,
            reach: (tolerance / FRAME).round() as usize,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EvalConfig {
    pub engine: Config,
    /// At most this many distinct readings are offered...
    pub offer_count: usize,
    /// ...taken from the hypotheses within this score margin of the best.
    pub offer_margin: f32,
    /// Readings are offered as the beam stood this many quarter notes after the moment in question,
    /// modeling a listener who has heard a little further.
    pub offer_lag: f64,
    /// The equivalents measure counts a label
    /// as right when it matches the expert's within this many quarter notes.
    pub timing_tolerance: f64,
    /// Whether to measure how soon each expert reading is offered (see `offer_delay`).
    pub measure_offer_delays: bool,
    /// When keys are offered on their own (see `layer_offers`), the most a key's shortfall may be.
    pub key_offer_margin: f32,
    /// When chords are offered on their own, the most a chord's shortfall may be.
    pub chord_offer_margin: f32,
    /// When numerals are built from the keys and chords offered,
    /// the most their combined shortfall may be.
    pub numeral_offer_margin: f32,
    /// Whether to measure coverage in hindsight (see `hindsight`).
    pub measure_hindsight: bool,
}

/// The default timing tolerance of the equivalents measure, an eighth note.
/// No source gives a value; it was chosen from measurements on `tune` at a sixteenth,
/// an eighth and a quarter note, recorded in the README's Benchmarks section.
pub const TIMING_TOLERANCE: f64 = 0.5;

impl Default for EvalConfig {
    fn default() -> EvalConfig {
        EvalConfig {
            engine: Config::default(),
            offer_count: 4,
            offer_margin: 4.0,
            offer_lag: 2.0,
            timing_tolerance: TIMING_TOLERANCE,
            measure_offer_delays: false,
            key_offer_margin: 4.0,
            chord_offer_margin: 4.0,
            numeral_offer_margin: 4.0,
            measure_hindsight: false,
        }
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
    /// How soon each expert reading was offered, when measured.
    #[serde(skip)]
    pub offer_delays: Option<OfferDelays>,
    pub timing: Timing,
}

/// Seconds spent on one piece in each part of its evaluation, and the size of its search.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Timing {
    pub search: f64,
    /// Offering keys, chords and numerals one layer at a time.
    pub layers: f64,
    /// The backward pass and the readings offered in hindsight.
    pub hindsight: f64,
    /// The number of hypotheses that the search kept, over all slices.
    pub hypotheses: usize,
    /// The number of chord hypotheses that the search kept, over all slices.
    pub chord_hypotheses: usize,
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
    let slices = slice_score(&score);
    let sounds = FrameSounds::new(&slices, &frame_slices(&slices, analyses[0].1.len()));
    let equivalents = analyses.iter().map(|(_, labels)| ExpertEquivalents::new(labels, &sounds)).collect();
    Ok(BenchPiece {
        id: entry.id.clone(),
        sets: entry.sets.clone(),
        slices,
        measures: score.measures,
        analyses,
        sounds,
        equivalents,
    })
}

pub fn evaluate(pieces: &[BenchPiece], rules: &RuleSet, config: &EvalConfig) -> Vec<PieceResult> {
    pieces.par_iter().map(|piece| evaluate_piece(piece, rules, config)).collect()
}

pub fn evaluate_piece(piece: &BenchPiece, rules: &RuleSet, config: &EvalConfig) -> PieceResult {
    let started = Instant::now();
    let analysis = analyze(&piece.slices, rules, &config.engine);
    let search = started.elapsed().as_secs_f64();
    let top = top_frames(&analysis, piece);
    let offered = offered_frames(&analysis, piece, top.len(), config);
    let now = now_frames(&analysis, piece, top.len(), config);
    let main = piece.expert_view(0, config.timing_tolerance);
    let mut accuracy = Accuracy::default();
    for (frame, truth) in piece.analyses[0].1.iter().enumerate() {
        if let Some(truth) = truth {
            accuracy.add_frame(top[frame].as_ref(), &offered[frame], now[frame].as_ref(), truth);
            let credit = |label: &Option<Label>| label.as_ref().and_then(|l| main.credit(l, frame));
            accuracy.add_equivalence(credit(&top[frame]), credit(&now[frame]));
        }
    }
    let (layers, hindsight, timing) = layered_frames(&analysis, piece, top.len(), config);
    let timing = Timing {
        search,
        hypotheses: analysis.layers.key.hypotheses.len(),
        chord_hypotheses: analysis.layers.chord.hypotheses.len(),
        ..timing
    };
    let layered = (layers.as_slice(), hindsight.as_deref());
    let agreement = (piece.analyses.len() >= 2)
        .then(|| agreement(piece, (&top, &offered), layered, config.timing_tolerance));
    let offer_delays = config.measure_offer_delays.then(|| offer_delays(&analysis, piece, config));
    PieceResult {
        id: piece.id.clone(),
        sets: piece.sets.clone(),
        slices: piece.slices.len(),
        seconds: started.elapsed().as_secs_f64(),
        accuracy,
        agreement,
        offer_delays,
        timing,
    }
}

/// Returns the readings offered layer by layer at each frame,
/// those offered in hindsight when measured, and the time each took.
fn layered_frames(
    analysis: &Analysis,
    piece: &BenchPiece,
    count: usize,
    config: &EvalConfig,
) -> (Vec<LayerOffers>, Option<Vec<LayerOffers>>, Timing) {
    let started = Instant::now();
    let layers = layer_frames(analysis, piece, count, config);
    let layers_seconds = started.elapsed().as_secs_f64();
    let hindsight = config.measure_hindsight.then(|| {
        let bests = descendant_bests(analysis);
        hindsight_frames(analysis, piece, (count, &bests), config)
    });
    let hindsight_seconds = started.elapsed().as_secs_f64() - layers_seconds;
    (layers, hindsight, Timing { layers: layers_seconds, hindsight: hindsight_seconds, ..Timing::default() })
}

/// Returns the label of the best final hypothesis at each frame of the piece.
pub fn top_frames(analysis: &Analysis, piece: &BenchPiece) -> Vec<Option<Label>> {
    let count = piece.analyses[0].1.len();
    let top = analysis
        .ranking
        .first()
        .map(|&(hypothesis, _)| hypothesis_frames(analysis, piece, hypothesis, count));
    top.unwrap_or_else(|| vec![None; count])
}

fn hypothesis_frames(
    analysis: &Analysis,
    piece: &BenchPiece,
    hypothesis: KeyHypothesisId,
    count: usize,
) -> Vec<Option<Label>> {
    let segments = segments_of(analysis, &piece.slices, hypothesis);
    let timed: Vec<_> = segments.iter().map(|s| (s.start, Some(s.label))).collect();
    let mut frames = frame_labels(&timed, count as f64 * FRAME);
    frames.resize(count, None);
    frames
}

/// Compares the engine's top and offered readings with every pair of analysts of the piece.
fn agreement(
    piece: &BenchPiece,
    (top, offered): (&[Option<Label>], &[Vec<Label>]),
    (layers, hindsight): (&[LayerOffers], Option<&[LayerOffers]>),
    tolerance: f64,
) -> Agreement {
    let mut agreement =
        Agreement { hindsight: hindsight.map(|_| Default::default()), ..Agreement::default() };
    let count = piece.analyses.len();
    for (i, j) in (0..count).flat_map(|i| (i + 1..count).map(move |j| (i, j))) {
        let (first, second) = (piece.expert_view(i, tolerance), piece.expert_view(j, tolerance));
        for frame in 0..top.len() {
            if let (Some(a), Some(b)) = (&first.labels[frame], &second.labels[frame]) {
                let is_credited = |view: &ExpertView, label: &Label| view.credit(label, frame).is_some();
                let offers = |view: &ExpertView| offered[frame].iter().any(|o| is_credited(view, o));
                let top = top[frame].as_ref();
                let equivalents = EquivalentMatches {
                    analysts: is_credited(&first, b) || is_credited(&second, a),
                    top_first: top.is_some_and(|t| is_credited(&first, t)),
                    top_second: top.is_some_and(|t| is_credited(&second, t)),
                    offers_first: offers(&first),
                    offers_second: offers(&second),
                };
                agreement.add_frame(top, &offered[frame], (a, b), equivalents);
                agreement.layers.add_frame(&layers[frame], (a, b));
                if let (Some(coverage), Some(hindsight)) = (agreement.hindsight.as_mut(), hindsight) {
                    coverage.add_frame(&hindsight[frame], (a, b));
                }
            }
        }
    }
    agreement
}

/// Returns, for each frame, the distinct readings of its slice held
/// by the leading hypotheses of the beam a short time later (see `EvalConfig::offer_lag`).
pub fn offered_frames(
    analysis: &Analysis,
    piece: &BenchPiece,
    count: usize,
    config: &EvalConfig,
) -> Vec<Vec<Label>> {
    let slices = &piece.slices;
    let per_slice: Vec<Vec<Label>> =
        (0..slices.len()).map(|i| offered_at(analysis, slices, i, config)).collect();
    slice_values_by_frame(slices, count, &per_slice)
}

/// Returns, for each frame, the reading of its slice held by the best hypothesis
/// of the beam a short time later (see `EvalConfig::offer_lag`):
/// the reading a listener would hold at that moment.
pub fn now_frames(
    analysis: &Analysis,
    piece: &BenchPiece,
    count: usize,
    config: &EvalConfig,
) -> Vec<Option<Label>> {
    let slices = &piece.slices;
    let per_slice: Vec<Option<Label>> =
        (0..slices.len()).map(|i| now_at(analysis, slices, i, config)).collect();
    slice_values_by_frame(slices, count, &per_slice)
}

pub(super) fn slice_values_by_frame<T: Clone + Default>(
    slices: &[Slice],
    count: usize,
    per_slice: &[T],
) -> Vec<T> {
    let value_at = |slice: usize| per_slice.get(slice).cloned().unwrap_or_default();
    frame_slices(slices, count).into_iter().map(value_at).collect()
}

/// Returns the beam as it stood `lag` quarter notes after the slice `at` began, best first.
pub fn lagged_beam(analysis: &Analysis, slices: &[Slice], at: usize, lag: f64) -> Option<Ranking<Function>> {
    analysis.layers.key.ranking(lagged_slice(slices, at, lag))
}

/// Returns the last slice that begins within `lag` quarter notes of the slice `at`.
pub fn lagged_slice(slices: &[Slice], at: usize, lag: f64) -> usize {
    let horizon = slices[at].start + lag;
    let later = slices[at..].partition_point(|s| s.start <= horizon + 1e-6);
    at + later.saturating_sub(1)
}

fn offered_at(analysis: &Analysis, slices: &[Slice], at: usize, config: &EvalConfig) -> Vec<Label> {
    let Some(beam) = lagged_beam(analysis, slices, at, config.offer_lag) else {
        return vec![];
    };
    offered_from(analysis, beam, at, config)
}

/// Returns the distinct readings of slice `at` that `beam` offers under the offer count and margin.
pub fn offered_from(
    analysis: &Analysis,
    beam: Ranking<Function>,
    at: usize,
    config: &EvalConfig,
) -> Vec<Label> {
    let Some(best) = beam.first() else {
        return vec![];
    };
    let best_score = analysis.layers.score(best);
    let is_close =
        |hypothesis: &KeyHypothesisId| best_score - analysis.layers.score(*hypothesis) <= config.offer_margin;
    let mut labels: Vec<Label> = Vec::new();
    for hypothesis in beam.iter().filter(is_close) {
        if labels.len() == config.offer_count {
            break;
        }
        let Some(label) = reading_at(analysis, hypothesis, at) else {
            continue;
        };
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    labels
}

fn now_at(analysis: &Analysis, slices: &[Slice], at: usize, config: &EvalConfig) -> Option<Label> {
    let best = lagged_beam(analysis, slices, at, config.offer_lag)?.first()?;
    reading_at(analysis, best, at)
}

/// Returns the label that the hypothesis `hypothesis` gives the slice `at`.
pub(super) fn reading_at(analysis: &Analysis, hypothesis: KeyHypothesisId, at: usize) -> Option<Label> {
    segment_at(analysis, hypothesis, at).map(|segment| segment.label())
}
