//! Re-evaluates the key layer's scoring hooks on one segment of a key hypothesis.

use super::key_contexts::{KeyCloseContext, KeyHistory, KeySliceContext, KeyTransitionContext};
use super::key_hooks::{KeyHook, KeySliceFn};
use super::layers::Layers;
use super::model::{KeyHypothesisId, KeySegment, Reading};
use crate::score::slicing::Slice;

/// What the key layer's hooks received when they scored one segment.
pub struct KeyTrace<'a> {
    layers: &'a Layers,
    slices: &'a [Slice],
    /// Index of the first slice in the segment, and one past the last.
    span: (usize, usize),
    segment: KeySegment,
    reading: Reading,
    previous: Option<KeySegment>,
    /// The last hypothesis of the segment before, or `KeyHypothesisId::NONE`.
    previous_hypothesis: KeyHypothesisId,
}

impl<'a> KeyTrace<'a> {
    /// Returns what the hooks received for the segment that a key hypothesis ends in,
    /// given the segment's span of slices.
    pub fn of(layers: &'a Layers, slices: &'a [Slice], last: KeyHypothesisId, span: (usize, usize)) -> Self {
        let segment = layers.segment(last);
        let previous_hypothesis = layers.key.get(last).previous;
        let previous =
            (previous_hypothesis != KeyHypothesisId::NONE).then(|| layers.segment(previous_hypothesis));
        let reading = segment.reading();
        KeyTrace { layers, slices, span, segment, reading, previous, previous_hypothesis }
    }

    /// Returns a scoring hook's unweighted evidence for the segment, or `None` for a hook that does not score.
    pub fn raw(&self, hook: KeyHook, params: &[f64]) -> Option<f32> {
        let (first, end) = self.span;
        let transition = KeyTransitionContext {
            previous: self.previous.as_ref(),
            next: &self.reading,
            slice: &self.slices[first],
            history: KeyHistory::new(self.layers, self.slices, self.previous_hypothesis),
        };
        Some(match hook {
            KeyHook::Slice(f) => self.slice_sum(f, params),
            KeyHook::Transition(f) => f(&transition, params),
            KeyHook::Departure(f) => transition.departure().map_or(0.0, |context| f(&context, params)),
            KeyHook::Close(f) => {
                let slices = &self.slices[first..end];
                f(
                    &KeyCloseContext { segment: &self.segment, slices, next_slice: self.slices.get(end) },
                    params,
                )
            }
            KeyHook::Propose(_)
            | KeyHook::Interpret(_)
            | KeyHook::Distinguish(_)
            | KeyHook::Rival(_)
            | KeyHook::RivalCandidate(_) => return None,
        })
    }

    fn slice_sum(&self, f: KeySliceFn, params: &[f64]) -> f32 {
        let (first, end) = self.span;
        let context = |i: usize| KeySliceContext::at(self.slices, i, &self.reading, i == first);
        (first..end).map(|i| f(&context(i), params)).sum()
    }
}
