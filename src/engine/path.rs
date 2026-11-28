//! Reconstructs the chord segments of a hypothesis by following parent links.

use super::model::{NONE, Reading};
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
    pub node: u32,
}

/// Returns the segments of the hypothesis ending at `node`, in chronological order.
pub fn segments_of(analysis: &Analysis, slices: &[Slice], node: u32) -> Vec<SegmentReading> {
    let mut segments = Vec::new();
    let mut current = node;
    while current != NONE {
        let last = &analysis.hypotheses[current as usize];
        let segment = analysis.segment(current);
        let (first, end) = (segment.start as usize, last.slice as usize + 1);
        let reading = segment.reading;
        let label = Label::new(reading.key, reading.function.tonicized, reading.chord, segment.bass_tpc());
        let end_time = slices[end - 1].start + slices[end - 1].duration;
        segments.push(SegmentReading {
            slices: (first, end),
            start: slices[first].start,
            end: end_time,
            reading,
            label,
            node: current,
        });
        current = segment.previous;
    }
    segments.reverse();
    segments
}

/// Returns the segments of the best final hypothesis.
pub fn best_segments(analysis: &Analysis, slices: &[Slice]) -> Vec<SegmentReading> {
    analysis.ranking.first().map(|&(node, _)| segments_of(analysis, slices, node)).unwrap_or_default()
}
