//! What a layer's transition hooks read of the layer's own earlier segments.

use super::hypothesis::HypothesisId;
use super::layers::{Layers, NO_LAYERS};
use crate::score::slicing::Slice;

/// Read-only access to the segments of the layer whose unit is `U` that precede a transition,
/// most recent first. Each layer gives its segments in the form its rules receive them,
/// through its own `segments` method, and gives nothing of the layers above it.
#[derive(Clone, Copy)]
pub struct History<'a, U> {
    pub(in crate::engine) layers: &'a Layers,
    /// Every slice of the score, so that a past segment's slices can be read from its `start`.
    pub slices: &'a [Slice],
    /// The last hypothesis of the most recent segment, or `HypothesisId::NONE`.
    pub last: HypothesisId<U>,
}

impl<'a, U> History<'a, U> {
    pub fn new(layers: &'a Layers, slices: &'a [Slice], last: HypothesisId<U>) -> History<'a, U> {
        History { layers, slices, last }
    }

    /// A history with no segments before it.
    pub fn empty(slices: &'a [Slice]) -> History<'a, U> {
        History::new(&NO_LAYERS, slices, HypothesisId::NONE)
    }
}
