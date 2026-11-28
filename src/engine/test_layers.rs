//! Builders of segments, and of the layers that hold them, for tests.

use super::bass::Bass;
use super::key_contexts::KeyHistory;
use super::layers::Layers;
use super::model::{
    ChordHypothesis, ChordHypothesisId, ChordSegment, Function, KeyHypothesis, KeyHypothesisId, KeySegment,
    Reading,
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

/// Holds chord segments in a chord layer: segment i becomes the last segment of chord hypothesis i,
/// which ends on slice i and opened after chord hypothesis `previous[i]`,
/// or after nothing if that is `ChordHypothesisId::NONE`.
pub fn chord_layer_of(segments: &[ChordSegment], previous: &[ChordHypothesisId]) -> Vec<ChordHypothesis> {
    let chord_hypothesis = |i: usize| ChordHypothesis {
        parent: previous[i],
        previous: previous[i],
        slice: i as u32,
        segment: segments[i],
        score: 0.0,
    };
    (0..segments.len()).map(chord_hypothesis).collect()
}

/// Builds a key layer over a chord layer: key hypothesis i reads chord hypothesis `read[i]`
/// with the function `functions[i]`, and opened after key hypothesis `previous[i]`,
/// or after nothing if that is `KeyHypothesisId::NONE`.
/// Several key hypotheses may read the same chord hypothesis.
pub fn key_layer_of(
    read: &[ChordHypothesisId],
    functions: &[Function],
    previous: &[KeyHypothesisId],
) -> Vec<KeyHypothesis> {
    let hypothesis = |i: usize| KeyHypothesis {
        parent: previous[i],
        previous: previous[i],
        chord_hypothesis: read[i],
        function: functions[i],
        score: 0.0,
    };
    (0..read.len()).map(hypothesis).collect()
}

/// Holds segments in layers, one chord hypothesis for each key hypothesis:
/// segment i becomes the last segment of hypothesis i, which reads chord hypothesis i.
/// Hypothesis i opened after hypothesis `previous[i]`, or after nothing if that is `KeyHypothesisId::NONE`,
/// and its chord hypothesis after the chord hypothesis with the same index.
pub fn layers_of(segments: &[KeySegment], previous: &[KeyHypothesisId]) -> Layers {
    let chord_index = |id: &KeyHypothesisId| match *id {
        KeyHypothesisId::NONE => ChordHypothesisId::NONE,
        KeyHypothesisId(i) => ChordHypothesisId(i),
    };
    let chord_previous: Vec<ChordHypothesisId> = previous.iter().map(chord_index).collect();
    let heard: Vec<ChordSegment> = segments.iter().map(|s| s.heard).collect();
    let read: Vec<ChordHypothesisId> = (0..segments.len() as u32).map(ChordHypothesisId).collect();
    let functions: Vec<Function> = segments.iter().map(|s| s.function).collect();
    Layers {
        chord_hypotheses: chord_layer_of(&heard, &chord_previous),
        key_hypotheses: key_layer_of(&read, &functions, previous),
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
pub fn history_after<'a>(path: &'a Layers, slices: &'a [Slice]) -> KeyHistory<'a> {
    KeyHistory { layers: path, slices, last: KeyHypothesisId(path.key_hypotheses.len() as u32 - 1) }
}
