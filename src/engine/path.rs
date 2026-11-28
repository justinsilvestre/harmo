//! Reconstructs the segments of a hypothesis, with their readings, by following its links.

use super::layers::Layers;
use super::model::Reading;
use super::search::Analysis;
use crate::score::model::Offset;
use crate::score::slicing::Slice;
use crate::theory::label::Label;

#[derive(Clone, Debug)]
pub struct SegmentReading {
    /// Index of the first slice, and one past the last.
    pub slices: (usize, usize),
    pub start: Offset,
    pub end: Offset,
    pub reading: Reading,
    pub label: Label,
    /// Index in the hypotheses of the segment's last hypothesis.
    pub hypothesis: u32,
}

/// Returns the segments of the hypothesis ending at `hypothesis`, in chronological order.
pub fn segments_of(analysis: &Analysis, slices: &[Slice], hypothesis: u32) -> Vec<SegmentReading> {
    let layers = &analysis.layers;
    let mut segments: Vec<SegmentReading> =
        layers.path_back(hypothesis).map(|last| segment_reading(layers, slices, last)).collect();
    segments.reverse();
    segments
}

/// Returns the segment whose last hypothesis is `last`.
fn segment_reading(layers: &Layers, slices: &[Slice], last: u32) -> SegmentReading {
    let segment = layers.segment(last);
    let (first, end) = (segment.heard.start as usize, layers.chord_hypothesis_of(last).slice as usize + 1);
    let last_slice = &slices[end - 1];
    SegmentReading {
        slices: (first, end),
        start: slices[first].start,
        end: last_slice.start + last_slice.duration,
        reading: segment.reading(),
        label: segment.label(),
        hypothesis: last,
    }
}

/// Returns the segments of the best final hypothesis.
pub fn best_segments(analysis: &Analysis, slices: &[Slice]) -> Vec<SegmentReading> {
    analysis
        .ranking
        .first()
        .map(|&(hypothesis, _)| segments_of(analysis, slices, hypothesis))
        .unwrap_or_default()
}
