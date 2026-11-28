//! The keys, chords and numerals offered at each frame one layer at a time, as the beam stood a
//! short time later, and the same ranked in hindsight.

use super::layer_offers::{LayerLimits, LayerOffers, by_shortfall, layer_offers};
use super::runner::{BenchPiece, EvalConfig, lagged_beam, reading_at, slice_values_by_frame};
use crate::engine::search::Analysis;
use crate::theory::label::Label;

/// Returns, for each frame, the readings offered by layer from the beam a short time later (see
/// `EvalConfig::offer_lag`), ranked by the hypotheses' scores at that time.
pub fn layer_frames(
    analysis: &Analysis,
    piece: &BenchPiece,
    count: usize,
    config: &EvalConfig,
) -> Vec<LayerOffers> {
    let slices = &piece.slices;
    let per_slice: Vec<LayerOffers> =
        (0..slices.len()).map(|at| layers_at(analysis, piece, at, config)).collect();
    slice_values_by_frame(slices, count, &per_slice)
}

/// Returns, for each frame, the readings that the same beam offers when each is ranked by the best
/// final score that a hypothesis holding it led to (see `hindsight::descendant_bests`).
/// The Roman numeral layer holds the full readings, ranked the same way.
/// This is a report on the finished analysis and is never used in scoring.
pub fn hindsight_frames(
    analysis: &Analysis,
    piece: &BenchPiece,
    (count, bests): (usize, &[f32]),
    config: &EvalConfig,
) -> Vec<LayerOffers> {
    let slices = &piece.slices;
    let Some(&(_, best)) = analysis.ranking.first() else {
        return vec![LayerOffers::default(); count];
    };
    let at_slice = |at: usize| {
        let Some(beam) = lagged_beam(analysis, slices, at, config.offer_lag) else {
            return LayerOffers::default();
        };
        let held = held_readings(analysis, beam.iter().copied(), at, |n| bests[n as usize]);
        let numerals = by_shortfall(held.iter().copied(), best, config.offer_margin, config.offer_count);
        let offers = layer_offers(&held, best, &config.layer_limits());
        LayerOffers { numerals: numerals.into_iter().map(|(label, _)| label).collect(), ..offers }
    };
    let per_slice: Vec<LayerOffers> = (0..slices.len()).map(at_slice).collect();
    slice_values_by_frame(slices, count, &per_slice)
}

fn layers_at(analysis: &Analysis, piece: &BenchPiece, at: usize, config: &EvalConfig) -> LayerOffers {
    let Some(beam) = lagged_beam(analysis, &piece.slices, at, config.offer_lag) else {
        return LayerOffers::default();
    };
    let Some(&first) = beam.first() else {
        return LayerOffers::default();
    };
    let score = |node: u32| analysis.arena[node as usize].score;
    let limits = config.layer_limits();
    let reach = limits.key_margin.max(limits.chord_margin).max(limits.numeral_margin);
    let close = beam.iter().copied().take_while(|&n| score(first) - score(n) <= reach);
    layer_offers(&held_readings(analysis, close, at, score), score(first), &limits)
}

/// Returns each node's value with its label of slice `at`.
fn held_readings(
    analysis: &Analysis,
    nodes: impl Iterator<Item = u32>,
    at: usize,
    value: impl Fn(u32) -> f32,
) -> Vec<(f32, Label)> {
    nodes.filter_map(|n| Some((value(n), reading_at(analysis, n, at)?))).collect()
}

impl EvalConfig {
    pub fn layer_limits(&self) -> LayerLimits {
        LayerLimits {
            count: self.offer_count,
            key_margin: self.key_offer_margin,
            chord_margin: self.chord_offer_margin,
            numeral_margin: self.numeral_offer_margin,
        }
    }
}
