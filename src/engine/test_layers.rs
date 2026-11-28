//! Builders of segments, and of the layers that hold them, for tests.

use super::bass::Bass;
use super::layers::Layers;
use super::model::{ChordSegment, Hearing, Hypothesis, KeySegment, NONE, Reading};
use super::rule::History;
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
    let (key, function) = (reading.key, reading.function);
    KeySegment { chord: chord_segment(reading.chord), key, function, previous: NONE }
}

/// Holds segments in layers: segment i becomes the last segment of hypothesis i, which interprets
/// hearing i and ends on slice i. Each hypothesis and hearing follows the one that the segment's
/// `previous` gives.
pub fn layers_of(segments: &[KeySegment]) -> Layers {
    let hypothesis = |(i, s): (usize, &KeySegment)| Hypothesis {
        parent: s.previous,
        previous: s.previous,
        slice: i as u32,
        hearing: i as u32,
        key: s.key,
        function: s.function,
        score: 0.0,
    };
    let hearing = |(i, s): (usize, &KeySegment)| Hearing {
        parent: s.previous,
        previous: s.previous,
        slice: i as u32,
        segment: s.chord,
        score: 0.0,
    };
    let hearings = segments.iter().enumerate().map(hearing).collect();
    Layers { hearings, hypotheses: segments.iter().enumerate().map(hypothesis).collect() }
}

/// Holds segments in layers as the path of one hypothesis, in which segment i opens on slice i and
/// follows segment i - 1.
pub fn path_of(segments: &[KeySegment]) -> Layers {
    let link = |(i, s): (usize, &KeySegment)| KeySegment {
        chord: ChordSegment { start: i as u32, ..s.chord },
        previous: if i == 0 { NONE } else { i as u32 - 1 },
        ..*s
    };
    layers_of(&segments.iter().enumerate().map(link).collect::<Vec<_>>())
}

/// Returns the history that a transition after the last segment of a path reads.
pub fn history_after<'a>(path: &'a Layers, slices: &'a [Slice]) -> History<'a> {
    History { layers: path, slices, last: path.hypotheses.len() as u32 - 1 }
}
