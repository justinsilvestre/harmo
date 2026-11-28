//! Explains a hypothesis by re-evaluating each rule along its path.
//!
//! Rules are pure, so the search stores only scores and the trace is rebuilt on demand.

use super::chord_hooks::{
    ChordCloseContext, ChordHook, ChordSliceContext, ChordSliceFn, ChordTransitionContext,
};
use super::key_hooks::{
    History, KeyCloseContext, KeyHook, KeySliceContext, KeySliceFn, KeyTransitionContext,
};
use super::model::NONE;
use super::path::SegmentReading;
use super::rule::{Hook, Layer};
use super::ruleset::RuleSet;
use super::search::Analysis;
use crate::score::slicing::Slice;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Contribution {
    pub rule: &'static str,
    pub hook: &'static str,
    pub layer: Layer,
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
    let layers = &analysis.layers;
    let state = layers.segment(segment.hypothesis);
    let previous_hypothesis = layers.hypotheses[segment.hypothesis as usize].previous;
    let heard = layers.hearing_of(segment.hypothesis);
    let hearing = &heard.segment;
    let previous = (previous_hypothesis != NONE).then(|| layers.segment(previous_hypothesis));
    let previous_hearing =
        (heard.previous != NONE).then(|| &layers.hearings[heard.previous as usize].segment);
    let (first, end) = segment.slices;
    let transition = KeyTransitionContext {
        previous: previous.as_ref(),
        next: &segment.reading,
        slice: &slices[first],
        history: History { layers, slices, last: previous_hypothesis },
    };
    let chord_transition = ChordTransitionContext {
        previous: previous_hearing,
        next: hearing.chord,
        slice: &slices[first],
        slices,
    };
    let close = KeyCloseContext { segment: &state, slices: &slices[first..end], next_slice: slices.get(end) };
    let chord_close =
        ChordCloseContext { segment: hearing, slices: &slices[first..end], next_slice: slices.get(end) };
    let slice_sum = |f: KeySliceFn, params: &[f64]| -> f32 {
        let context = |i: usize| KeySliceContext {
            slice: &slices[i],
            next_slice: slices.get(i + 1),
            slices,
            index: i,
            reading: &segment.reading,
            is_opening: i == first,
        };
        (first..end).map(|i| f(&context(i), params)).sum()
    };
    let chord_slice_sum = |f: ChordSliceFn, params: &[f64]| -> f32 {
        let context = |i: usize| ChordSliceContext {
            slice: &slices[i],
            next_slice: slices.get(i + 1),
            slices,
            index: i,
            chord: hearing.chord,
            is_opening: i == first,
        };
        (first..end).map(|i| f(&context(i), params)).sum()
    };
    let mut contributions = Vec::new();
    let rules_with_params = rules.rules.iter().zip(&rules.weights).zip(&rules.params);
    for ((rule, &weight), params) in rules_with_params.filter(|((_, w), _)| **w != 0.0) {
        let raw = match rule.hook {
            Hook::Chord(ChordHook::Slice(f)) => chord_slice_sum(f, params),
            Hook::Chord(ChordHook::Transition(f)) => f(&chord_transition, params),
            Hook::Chord(ChordHook::Close(f)) => f(&chord_close, params),
            Hook::Key(KeyHook::Slice(f)) => slice_sum(f, params),
            Hook::Key(KeyHook::Transition(f)) => f(&transition, params),
            Hook::Key(KeyHook::Departure(f)) => {
                transition.departure().map_or(0.0, |context| f(&context, params))
            }
            Hook::Key(KeyHook::Close(f)) => f(&close, params),
            _ => continue,
        };
        contributions.push(Contribution {
            rule: rule.id,
            hook: rule.hook.kind(),
            layer: rule.layer,
            raw,
            weighted: raw * weight,
        });
    }
    contributions
}
