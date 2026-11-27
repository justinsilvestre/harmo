//! The engine's state: readings, open chord segments and the hypothesis arena.

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

/// The state of a chord span that has been opened and may still be extended.
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

/// A hypothesis: the reading of every slice up to and including `slice`.
#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub parent: u32,
    pub slice: u32,
    pub segment: Segment,
    pub score: f32,
}

impl Segment {
    /// Returns the bass note that gives the segment's inversion, as (MIDI number, spelling).
    pub fn bass_note(&self) -> Option<(i32, Tpc)> {
        self.bass.note(self.reading.chord)
    }

    pub fn bass_tpc(&self) -> Tpc {
        self.bass_note().map_or(self.reading.chord.root, |(_, tpc)| tpc)
    }

    pub fn has_heard_member(&self, index: usize) -> bool {
        self.members_heard & (1 << index) != 0
    }
}
