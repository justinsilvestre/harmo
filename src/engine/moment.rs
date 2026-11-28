//! The point that the search has reached in the score.

use super::key_contexts::History;
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

    /// Returns the history of the segments up to and including the last segment of a hypothesis.
    pub fn history(&self, last: u32) -> History<'a> {
        History { layers: self.layers, slices: self.slices, last }
    }
}
