//! Re-evaluates the chord layer's scoring hooks on one segment of a chord hypothesis.

use super::chord_hooks::{
    ChordCloseContext, ChordHistory, ChordHook, ChordSliceContext, ChordSliceFn, ChordTransitionContext,
};
use super::layers::Layers;
use super::model::{ChordHypothesisId, ChordSegment};
use crate::score::slicing::Slice;

/// What the chord layer's hooks received when they scored one segment.
pub struct ChordTrace<'a> {
    slices: &'a [Slice],
    /// Index of the first slice in the segment, and one past the last.
    span: (usize, usize),
    segment: &'a ChordSegment,
    transition: ChordTransitionContext<'a>,
}

impl<'a> ChordTrace<'a> {
    /// Returns what the hooks received for the segment that a chord hypothesis ends in,
    /// given the segment's span of slices.
    pub fn of(
        layers: &'a Layers,
        slices: &'a [Slice],
        last: ChordHypothesisId,
        span: (usize, usize),
    ) -> Self {
        let hypothesis = layers.chord.get(last);
        let segment = &hypothesis.unit;
        let previous = (hypothesis.previous != ChordHypothesisId::NONE)
            .then(|| &layers.chord.get(hypothesis.previous).unit);
        let transition = ChordTransitionContext {
            previous,
            next: segment.chord,
            slice: &slices[span.0],
            history: ChordHistory::new(layers, slices, hypothesis.previous),
        };
        ChordTrace { slices, span, segment, transition }
    }

    /// Returns a scoring hook's unweighted evidence for the segment, or `None` for a hook that does not score.
    pub fn raw(&self, hook: ChordHook, params: &[f64]) -> Option<f32> {
        let (first, end) = self.span;
        Some(match hook {
            ChordHook::Slice(f) => self.slice_sum(f, params),
            ChordHook::Transition(f) => f(&self.transition, params),
            ChordHook::Close(f) => {
                let slices = &self.slices[first..end];
                f(
                    &ChordCloseContext { segment: self.segment, slices, next_slice: self.slices.get(end) },
                    params,
                )
            }
            ChordHook::Propose(_) | ChordHook::Bass(_) => return None,
        })
    }

    fn slice_sum(&self, f: ChordSliceFn, params: &[f64]) -> f32 {
        let (first, end) = self.span;
        let context = |i: usize| ChordSliceContext::at(self.slices, i, self.segment.chord, i == first);
        (first..end).map(|i| f(&context(i), params)).sum()
    }
}
