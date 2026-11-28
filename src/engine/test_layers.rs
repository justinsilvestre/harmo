//! Builders of segments, and of the layers that hold them, for tests.

use super::bass::Bass;
use super::key_contexts::History;
use super::layers::Layers;
use super::model::{
    ChordHypothesis, ChordHypothesisId, ChordSegment, KeyHypothesis, KeyHypothesisId, KeySegment, Reading,
};
use crate::pitch::tpc_from_name;
use crate::score::slicing::{Slice, SliceNote};
use crate::theory::chord::Chord;

/// A chord segment that opens on slice 0, lasts one quarter note,
/// has heard none of the chord's members and has no bass note yet.
pub fn chord_segment(chord: Chord) -> ChordSegment {
    ChordSegment { chord, start: 0, duration: 1.0, members_heard: 0, bass: Bass::default() }
}

/// A segment that reads the reading's chord, as `chord_segment` gives it, in the reading's key,
/// and follows no other segment.
pub fn key_segment(reading: Reading) -> KeySegment {
    KeySegment { heard: chord_segment(reading.chord), function: reading.function }
}

/// Holds segments in layers: segment i becomes the last segment of hypothesis i,
/// which interprets chord hypothesis i and ends on slice i. Hypothesis i follows hypothesis `previous[i]`,
/// or nothing if that is `KeyHypothesisId::NONE`, and its chord hypothesis follows the chord hypothesis with the same index.
pub fn layers_of(segments: &[KeySegment], previous: &[KeyHypothesisId]) -> Layers {
    let hypothesis = |i: usize| KeyHypothesis {
        parent: previous[i],
        previous: previous[i],
        chord_hypothesis: ChordHypothesisId(i as u32),
        function: segments[i].function,
        score: 0.0,
    };
    let previous_chord_hypothesis = |i: usize| {
        if previous[i] == KeyHypothesisId::NONE {
            ChordHypothesisId::NONE
        } else {
            ChordHypothesisId(previous[i].0)
        }
    };
    let chord_hypothesis = |i: usize| ChordHypothesis {
        previous: previous_chord_hypothesis(i),
        slice: i as u32,
        segment: segments[i].heard,
        score: 0.0,
    };
    Layers {
        chord_hypotheses: (0..segments.len()).map(chord_hypothesis).collect(),
        key_hypotheses: (0..segments.len()).map(hypothesis).collect(),
    }
}

/// Holds segments in layers as the path of one hypothesis,
/// in which segment i opens on slice i and follows segment i - 1.
pub fn path_of(segments: &[KeySegment]) -> Layers {
    let opened = |(i, s): (usize, &KeySegment)| KeySegment {
        heard: ChordSegment { start: i as u32, ..s.heard },
        ..*s
    };
    let previous: Vec<KeyHypothesisId> = (0..segments.len() as u32)
        .map(|i| i.checked_sub(1).map_or(KeyHypothesisId::NONE, KeyHypothesisId))
        .collect();
    layers_of(&segments.iter().enumerate().map(opened).collect::<Vec<_>>(), &previous)
}

/// A one-quarter-note slice on a downbeat, sounding the given notes, each as (spelling, MIDI number).
pub fn slice_of(start: f64, notes: &[(&str, i32)]) -> Slice {
    let note = |&(name, midi): &(&str, i32)| SliceNote {
        tpc: tpc_from_name(name).unwrap(),
        midi,
        attacked: true,
        onset_level: 4,
    };
    let notes = notes.iter().map(note).collect();
    Slice {
        start,
        duration: 1.0,
        notes,
        measure: start as usize,
        metric_level: 4,
        phrase: 0,
        key_signature: None,
    }
}

/// The slices of I–V–I in C major, one per quarter note.
pub fn i_v_i() -> Vec<Slice> {
    let (tonic, dominant) = ([("C", 48), ("E", 64), ("G", 67)], [("G", 43), ("B", 62), ("D", 67)]);
    vec![slice_of(0.0, &tonic), slice_of(1.0, &dominant), slice_of(2.0, &tonic)]
}

/// Returns the history that a transition after the last segment of a path reads.
pub fn history_after<'a>(path: &'a Layers, slices: &'a [Slice]) -> History<'a> {
    History { layers: path, slices, last: KeyHypothesisId(path.key_hypotheses.len() as u32 - 1) }
}
