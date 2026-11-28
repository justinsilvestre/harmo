//! The engine's state: readings, open chord segments, and the arenas of the chord layer's hearings
//! and of the key layer's interpretations of them.

use super::bass::Bass;
use crate::pitch::Tpc;
use crate::theory::chord::Chord;
use crate::theory::key::Key;
use serde::Serialize;

/// Marks the absence of an index into the hypotheses or the hearings.
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

/// How a chord functions in the key it is read in: the key it tonicizes, if any, and its category.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub struct Function {
    pub tonicized: Option<Key>,
    pub category: Category,
}

impl Function {
    /// A diatonic chord that tonicizes nothing.
    pub const DIATONIC: Function = Function { tonicized: None, category: Category::Diatonic };
}

/// A reading of a span of music: the chord that the chord layer hears, and the key that the key
/// layer hears it in, with its function there.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub struct Reading {
    pub key: Key,
    pub chord: Chord,
    #[serde(flatten)]
    pub function: Function,
}

/// The chord layer's state of a chord span that has been opened and may still be extended.
#[derive(Clone, Copy, Debug)]
pub struct ChordSegment {
    pub chord: Chord,
    /// Index of the first slice in the segment.
    pub start: u32,
    pub duration: f64,
    /// Bit i is set once chord member i (root, third, fifth, seventh) has sounded.
    pub members_heard: u8,
    pub bass: Bass,
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

/// A hearing of the chords: the chord layer's reading of every slice up to and including `slice`.
#[derive(Clone, Copy, Debug)]
pub struct Hearing {
    /// Index in the hearings of the hearing up to the slice before, or `NONE`.
    pub parent: u32,
    /// Index in the hearings of the last hearing of the preceding segment, or `NONE`.
    pub previous: u32,
    pub slice: u32,
    /// The segment that the hearing ends in.
    pub segment: ChordSegment,
    /// The sum of the chord rules' weighted scores along the hearing.
    pub score: f32,
}

/// A hypothesis, or complete interpretation, which reads the latest segment of one hearing of the
/// chords in a key, and through its parents every slice up to and including `slice`.
#[derive(Clone, Copy, Debug)]
pub struct Hypothesis {
    /// Index in the hypotheses of the hypothesis up to the slice before, or `NONE`.
    pub parent: u32,
    /// Index in the hypotheses of the last hypothesis of the preceding segment, or `NONE`.
    pub previous: u32,
    pub slice: u32,
    /// Index in the hearings of the hearing of the chords that this hypothesis interprets.
    pub hearing: u32,
    /// The key that the hypothesis reads the hearing's latest segment in.
    pub key: Key,
    /// The function of the segment's chord in that key.
    pub function: Function,
    /// The sum of every weighted rule score along the hypothesis: its hearing's score plus the
    /// key layer's rule scores.
    pub score: f32,
}

/// A segment as the key layer reads it: a chord segment, with the key it is heard in and its
/// function there. Rules of the key layer receive segments in this form.
#[derive(Clone, Copy, Debug)]
pub struct KeySegment {
    pub chord: ChordSegment,
    pub key: Key,
    pub function: Function,
    /// Index in the hypotheses of the last hypothesis of the preceding segment, or `NONE`.
    pub previous: u32,
}

impl KeySegment {
    /// Returns a hypothesis's reading of a chord segment.
    pub fn of(chord: ChordSegment, hypothesis: &Hypothesis) -> KeySegment {
        let (key, function, previous) = (hypothesis.key, hypothesis.function, hypothesis.previous);
        KeySegment { chord, key, function, previous }
    }

    pub fn reading(&self) -> Reading {
        Reading { key: self.key, chord: self.chord.chord, function: self.function }
    }
}
