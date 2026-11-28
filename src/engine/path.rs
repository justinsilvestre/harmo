//! Reconstructs the segments of a hypothesis, with their readings, by following its links.

use super::layers::Layers;
use super::model::{KeyHypothesisId, Reading};
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
    /// The segment's last hypothesis.
    pub hypothesis: KeyHypothesisId,
}

/// Returns the segments of the hypothesis ending at `hypothesis`, in chronological order.
pub fn segments_of(
    analysis: &Analysis,
    slices: &[Slice],
    hypothesis: KeyHypothesisId,
) -> Vec<SegmentReading> {
    let layers = &analysis.layers;
    let mut segments: Vec<SegmentReading> =
        layers.key.path_back(hypothesis).map(|last| segment_reading(layers, slices, last)).collect();
    segments.reverse();
    segments
}

/// Returns the segment whose last hypothesis is `last`.
fn segment_reading(layers: &Layers, slices: &[Slice], last: KeyHypothesisId) -> SegmentReading {
    let segment = layers.segment(last);
    let (first, end) = (segment.heard.start as usize, layers.chord_hypothesis_of(last).reads as usize + 1);
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

/// Returns the segment of the hypothesis ending at `hypothesis` that covers slice `at`.
pub fn segment_covering(
    analysis: &Analysis,
    slices: &[Slice],
    hypothesis: KeyHypothesisId,
    at: usize,
) -> Option<SegmentReading> {
    let layers = &analysis.layers;
    let last =
        layers.key.path_back(hypothesis).find(|&last| layers.segment(last).heard.start as usize <= at)?;
    Some(segment_reading(layers, slices, last))
}
