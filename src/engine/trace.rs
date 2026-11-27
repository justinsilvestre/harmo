//! Explains a hypothesis by re-evaluating each rule along its path.
//!
//! Rules are pure, so the search stores only scores and the trace is rebuilt on demand.

use super::model::NONE;
use super::path::SegmentReading;
use super::rule::{CloseContext, History, Hook, SliceContext, SliceFn, TransitionContext};
use super::ruleset::RuleSet;
use super::search::Analysis;
use crate::score::slicing::Slice;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Contribution {
    pub rule: &'static str,
    pub hook: &'static str,
    /// The rule's unweighted evidence, summed over the segment.
    pub raw: f32,
    pub weighted: f32,
}

/// Returns each scoring rule's contribution to one segment of a hypothesis.
pub fn trace_segment(
    analysis: &Analysis,
    slices: &[Slice],
    rules: &RuleSet,
    segment: &SegmentReading,
) -> Vec<Contribution> {
    let state = &analysis.arena[segment.node as usize].segment;
    let previous = (state.previous != NONE).then(|| &analysis.arena[state.previous as usize].segment);
    let (first, end) = segment.slices;
    let transition = TransitionContext {
        previous,
        next: &segment.reading,
        slice: &slices[first],
        history: History { arena: &analysis.arena, slices, last: state.previous },
    };
    let close = CloseContext { segment: state, slices: &slices[first..end], next_slice: slices.get(end) };
    let slice_sum = |f: SliceFn, params: &[f64]| -> f32 {
        let context = |i: usize| SliceContext {
            slice: &slices[i],
            next_slice: slices.get(i + 1),
            slices,
            index: i,
            reading: &segment.reading,
            is_opening: i == first,
        };
        (first..end).map(|i| f(&context(i), params)).sum()
    };
    let mut contributions = Vec::new();
    let rules_with_params = rules.rules.iter().zip(&rules.weights).zip(&rules.params);
    for ((rule, &weight), params) in rules_with_params.filter(|((_, w), _)| **w != 0.0) {
        let raw = match rule.hook {
            Hook::Slice(f) => slice_sum(f, params),
            Hook::Transition(f) => f(&transition, params),
            Hook::Departure(f) => transition.departure().map_or(0.0, |context| f(&context, params)),
            Hook::Close(f) => f(&close, params),
            _ => continue,
        };
        contributions.push(Contribution {
            rule: rule.id,
            hook: rule.hook.kind(),
            raw,
            weighted: raw * weight,
        });
    }
    contributions
}
