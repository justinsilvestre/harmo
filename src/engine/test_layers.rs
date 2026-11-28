//! Builders of segments, and of the layers that hold them, for tests.

use super::bass::Bass;
use super::key_hooks::History;
use super::layers::Layers;
use super::model::{ChordSegment, Hearing, Hypothesis, KeySegment, NONE, Reading};
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;

/// A chord segment that opens on slice 0, lasts one quarter note, has heard none of the chord's
/// members and has no bass note yet.
pub fn chord_segment(chord: Chord) -> ChordSegment {
    ChordSegment { chord, start: 0, duration: 1.0, members_heard: 0, bass: Bass::default() }
}

/// A segment that reads the reading's chord, as `chord_segment` gives it, in the reading's key,
/// and follows no other segment.
pub fn key_segment(reading: Reading) -> KeySegment {
    KeySegment { heard: chord_segment(reading.chord), function: reading.function }
}

/// Holds segments in layers: segment i becomes the last segment of hypothesis i, which interprets
/// hearing i and ends on slice i. Hypothesis i follows hypothesis `previous[i]`, or nothing if that is
/// `NONE`, and its hearing follows the hearing with the same index.
pub fn layers_of(segments: &[KeySegment], previous: &[u32]) -> Layers {
    let hypothesis = |i: usize| Hypothesis {
        parent: previous[i],
        previous: previous[i],
        hearing: i as u32,
        function: segments[i].function,
        score: 0.0,
    };
    let hearing =
        |i: usize| Hearing { previous: previous[i], slice: i as u32, segment: segments[i].heard, score: 0.0 };
    Layers {
        hearings: (0..segments.len()).map(hearing).collect(),
        hypotheses: (0..segments.len()).map(hypothesis).collect(),
    }
}

/// Holds segments in layers as the path of one hypothesis, in which segment i opens on slice i and
/// follows segment i - 1.
pub fn path_of(segments: &[KeySegment]) -> Layers {
    let opened = |(i, s): (usize, &KeySegment)| KeySegment {
        heard: ChordSegment { start: i as u32, ..s.heard },
        ..*s
    };
    let previous: Vec<u32> = (0..segments.len() as u32).map(|i| i.checked_sub(1).unwrap_or(NONE)).collect();
    layers_of(&segments.iter().enumerate().map(opened).collect::<Vec<_>>(), &previous)
}

/// Returns the history that a transition after the last segment of a path reads.
pub fn history_after<'a>(path: &'a Layers, slices: &'a [Slice]) -> History<'a> {
    History { layers: path, slices, last: path.hypotheses.len() as u32 - 1 }
}
