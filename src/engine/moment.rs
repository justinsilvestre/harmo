//! The point that the search has reached in the score.

use super::chord_hooks::ChordHistory;
use super::key_contexts::KeyHistory;
use super::layers::Layers;
use super::model::{ChordHypothesisId, KeyHypothesisId};
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

    /// Returns the history of the chord segments up to and including the last segment of a chord hypothesis.
    pub fn chord_history(&self, last: ChordHypothesisId) -> ChordHistory<'a> {
        ChordHistory::new(self.layers, self.slices, last)
    }

    /// Returns the history of the segments up to and including the last segment of a hypothesis.
    pub fn key_history(&self, last: KeyHypothesisId) -> KeyHistory<'a> {
        KeyHistory { layers: self.layers, slices: self.slices, last }
    }
}
