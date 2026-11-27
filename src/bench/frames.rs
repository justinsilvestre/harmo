//! Samples labels on a fixed time grid, as AugmentedNet does when it evaluates.

use crate::score::model::Offset;
use crate::score::slicing::Slice;
use crate::theory::label::Label;

/// AugmentedNet evaluates on a grid of thirty-second notes.
pub const FRAME: Offset = 0.125;

/// Returns the label in force at each frame, given labels sorted by start time.
pub fn frame_labels(labels: &[(Offset, Option<Label>)], end: Offset) -> Vec<Option<Label>> {
    let count = (end / FRAME).ceil() as usize;
    let mut frames = Vec::with_capacity(count);
    let mut next = 0;
    let mut current = None;
    for index in 0..count {
        let time = index as f64 * FRAME + 1e-6;
        while next < labels.len() && labels[next].0 <= time {
            current = Some(labels[next].1);
            next += 1;
        }
        frames.push(current.flatten());
    }
    frames
}

/// Returns, for each of `count` frames, the index of the slice sounding at that frame.
pub fn frame_slices(slices: &[Slice], count: usize) -> Vec<usize> {
    let mut slice = 0;
    let mut slice_at = |frame: usize| {
        let time = frame as f64 * FRAME + 1e-6;
        while slice + 1 < slices.len() && slices[slice + 1].start <= time {
            slice += 1;
        }
        slice
    };
    (0..count).map(&mut slice_at).collect()
}
