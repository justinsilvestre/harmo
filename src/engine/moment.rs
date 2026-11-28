//! The point that the search has reached in the score.

use super::hypothesis::HypothesisId;
use super::layer_history::History;
use super::layers::Layers;
use crate::score::slicing::Slice;

/// The slice that the search hears now, within the score,
/// and what both layers have kept from the slices before it.
#[derive(Clone, Copy)]
pub struct Moment<'a> {
    pub slices: &'a [Slice],
    pub index: usize,
    pub layers: &'a Layers,
}

impl<'a> Moment<'a> {
    pub fn slice(&self) -> &'a Slice {
        &self.slices[self.index]
    }

    /// Returns the history of a layer's segments up to and including the last segment of a hypothesis.
    pub fn history<U>(&self, last: HypothesisId<U>) -> History<'a, U> {
        History::new(self.layers, self.slices, last)
    }
}
