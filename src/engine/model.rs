//! The engine's state: readings, open chord segments, and the arenas of the chord layer's hearings
//! and of the key layer's interpretations of them.

use super::bass::Bass;
use crate::pitch::Tpc;
use crate::theory::chord::Chord;
use crate::theory::key::Key;
use serde::Serialize;

/// Marks the absence of a node index.
pub const NONE: u32 = u32::MAX;

/// How a chord relates to the key it is read in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub enum Category {
    Diatonic,
    /// Borrowed from the parallel mode.
    Mixture,
    /// Neapolitan and augmented sixth chords.
    Chromatic,
    /// Applied (secondary) chords that tonicize another degree.
    Applied,
}

/// One interpretation of a span of music: a chord, the key it is heard in and its function there.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub struct Reading {
    pub key: Key,
    pub chord: Chord,
    pub tonicized: Option<Key>,
    pub category: Category,
}

/// How a chord is read in a key: the key, the key it tonicizes if any, and its category.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub struct KeyReading {
    pub key: Key,
    pub tonicized: Option<Key>,
    pub category: Category,
}

impl Reading {
    pub fn key_reading(&self) -> KeyReading {
        KeyReading { key: self.key, tonicized: self.tonicized, category: self.category }
    }

    pub fn of(chord: Chord, key: KeyReading) -> Reading {
        Reading { key: key.key, chord, tonicized: key.tonicized, category: key.category }
    }
}

/// The chord layer's state of a chord span that has been opened and may still be extended.
#[derive(Clone, Copy, Debug)]
pub struct ChordSegment {
    pub chord: Chord,
    /// Index of the first slice in the segment.
    pub start: u32,
    /// Index in the chord arena of the last chord node of the preceding segment, or `NONE`.
    pub previous: u32,
    pub duration: f64,
    /// Bit i is set once chord member i (root, third, fifth, seventh) has sounded.
    pub members_heard: u8,
    pub bass: Bass,
}

/// A hearing of the chords: the chord layer's reading of every slice up to and including `slice`.
#[derive(Clone, Copy, Debug)]
pub struct ChordNode {
    /// Index in the chord arena of the hearing up to the slice before, or `NONE`.
    pub parent: u32,
    pub slice: u32,
    pub segment: ChordSegment,
    /// The sum of the chord rules' weighted scores along the hearing.
    pub score: f32,
}

/// A hypothesis, or complete interpretation: a key node, which reads the latest segment of one
/// hearing of the chords in a key, and through its parents every slice up to and including `slice`.
#[derive(Clone, Copy, Debug)]
pub struct Node {
    /// Index in the arena of the hypothesis up to the slice before, or `NONE`.
    pub parent: u32,
    pub slice: u32,
    /// Index in the chord arena of the hearing of the chords that this hypothesis interprets.
    pub chord: u32,
    pub reading: KeyReading,
    /// Index in the arena of the last node of the preceding segment, or `NONE`.
    pub previous: u32,
    /// The sum of every weighted rule score along the hypothesis, both layers included.
    pub score: f32,
}

/// A segment as both layers read it: the chord layer's segment with the key layer's reading of it.
/// Rules of the key layer receive segments in this form.
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub reading: Reading,
    /// Index of the first slice in the segment.
    pub start: u32,
    /// Arena index of the last node of the preceding segment, or `NONE`.
    pub previous: u32,
    pub duration: f64,
    /// Bit i is set once chord member i (root, third, fifth, seventh) has sounded.
    pub members_heard: u8,
    pub bass: Bass,
}

/// Returns the segment that the hypothesis `node` ends in, as both layers read it.
pub fn segment_of(arena: &[Node], chords: &[ChordNode], node: u32) -> Segment {
    let hypothesis = &arena[node as usize];
    Segment::join(&chords[hypothesis.chord as usize].segment, hypothesis.reading, hypothesis.previous)
}

impl Segment {
    /// Joins a chord segment with its reading in a key; `previous` is the arena index of the last
    /// node of the preceding segment.
    pub fn join(chord: &ChordSegment, reading: KeyReading, previous: u32) -> Segment {
        Segment {
            reading: Reading::of(chord.chord, reading),
            start: chord.start,
            previous,
            duration: chord.duration,
            members_heard: chord.members_heard,
            bass: chord.bass,
        }
    }

    /// Returns the chord layer's part of the segment, with `previous` as given.
    pub fn chord_segment(&self, previous: u32) -> ChordSegment {
        let (chord, start, duration) = (self.reading.chord, self.start, self.duration);
        ChordSegment { chord, start, previous, duration, members_heard: self.members_heard, bass: self.bass }
    }

    /// Returns the bass note that gives the segment's inversion, as (MIDI number, spelling).
    pub fn bass_note(&self) -> Option<(i32, Tpc)> {
        self.bass.note()
    }

    pub fn bass_tpc(&self) -> Tpc {
        self.bass_note().map_or(self.reading.chord.root, |(_, tpc)| tpc)
    }

    pub fn has_heard_member(&self, index: usize) -> bool {
        self.members_heard & (1 << index) != 0
    }
}

impl ChordSegment {
    /// Returns the bass note that gives the segment's inversion, as (MIDI number, spelling).
    pub fn bass_note(&self) -> Option<(i32, Tpc)> {
        self.bass.note()
    }

    pub fn bass_tpc(&self) -> Tpc {
        self.bass_note().map_or(self.chord.root, |(_, tpc)| tpc)
    }

    pub fn has_heard_member(&self, index: usize) -> bool {
        self.members_heard & (1 << index) != 0
    }
}
