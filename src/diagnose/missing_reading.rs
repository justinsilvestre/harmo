//! Explains why the engine does not offer an analyst's reading.

use super::beam_absence::{Absence, find_pruning};
use super::piece_context::PieceContext;
use crate::bench::metrics::is_rn_match;
use crate::bench::runner::{EvalConfig, lagged_beam, lagged_slice};
use crate::engine::history::segment_at;
use crate::engine::model::KeyHypothesisId;
use crate::engine::ranking::Ranking;
use crate::theory::label::Label;

/// Why an analyst's reading is not among the offered readings.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MissingReason {
    Unproducible,
    /// The best hypothesis in the beam that holds the reading has this rank (1 = best)
    /// and lies this far below the best hypothesis.
    BelowCutoff {
        rank: usize,
        gap: f32,
    },
    OutsideBeam(Absence),
}

/// Looks for the reading in the whole beam from which readings are offered,
/// and otherwise asks whether the engine could produce it at all.
pub fn missing_reason(
    context: &PieceContext,
    (analyst, frame): (usize, usize),
    label: &Label,
    config: &EvalConfig,
) -> MissingReason {
    let (analysis, slices) = (&context.analysis, &context.piece.slices);
    let at = context.frame_slices[frame];
    let beam = lagged_beam(analysis, slices, at, config.offer_lag).unwrap_or(Ranking::EMPTY);
    let best = beam.first().map_or(0.0, |hypothesis| analysis.layers.score(hypothesis));
    let holds = |hypothesis: KeyHypothesisId| {
        segment_at(analysis, hypothesis, at).is_some_and(|s| is_rn_match(&s.label(), label))
    };
    match beam.iter().position(holds) {
        Some(rank) => {
            MissingReason::BelowCutoff { rank: rank + 1, gap: best - analysis.layers.score(beam.get(rank)) }
        }
        None if context.gaps[analyst][frame].is_some() => MissingReason::Unproducible,
        None => {
            let until = lagged_slice(slices, at, config.offer_lag);
            let pruning = find_pruning(analysis, slices, (at, until), label);
            MissingReason::OutsideBeam(pruning.unwrap_or(Absence::NeverHeld))
        }
    }
}
