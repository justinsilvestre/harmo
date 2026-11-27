//! Explains why the engine does not offer an analyst's reading.

use super::beam_diversity::segment_label;
use super::piece_context::PieceContext;
use crate::bench::metrics::is_rn_match;
use crate::bench::runner::{EvalConfig, lagged_beam};
use crate::engine::history::segment_at;
use crate::theory::label::Label;

/// Why an analyst's reading is not among the offered readings.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MissingReason {
    Unproducible,
    /// The best hypothesis in the beam that holds the reading has this rank (1 = best) and lies
    /// this far below the best hypothesis.
    BelowCutoff {
        rank: usize,
        gap: f32,
    },
    OutsideBeam,
}

/// Looks for the reading in the whole beam from which readings are offered, and otherwise asks
/// whether the engine could produce it at all.
pub fn missing_reason(
    context: &PieceContext,
    (analyst, frame): (usize, usize),
    label: &Label,
    config: &EvalConfig,
) -> MissingReason {
    let (analysis, slices) = (&context.analysis, &context.piece.slices);
    let at = context.frame_slices[frame];
    let beam = lagged_beam(analysis, slices, at, config.offer_lag).unwrap_or(&[]);
    let best = beam.first().map_or(0.0, |&node| analysis.arena[node as usize].score);
    let holds =
        |node: &&u32| segment_at(analysis, **node, at).is_some_and(|s| is_rn_match(&segment_label(s), label));
    match beam.iter().position(|node| holds(&node)) {
        Some(rank) => MissingReason::BelowCutoff {
            rank: rank + 1,
            gap: best - analysis.arena[beam[rank] as usize].score,
        },
        None if context.gaps[analyst][frame].is_some() => MissingReason::Unproducible,
        None => MissingReason::OutsideBeam,
    }
}
