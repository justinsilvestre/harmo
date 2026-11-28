//! What the search's two layers hold: readings, the chord layer's segments and hypotheses,
//! and the key layer's segments and hypotheses.

use super::bass::Bass;
use super::hypothesis::{Hypothesis, HypothesisId};
use crate::pitch::Tpc;
use crate::theory::chord::Chord;
use crate::theory::key::Key;
use crate::theory::label::Label;
use serde::Serialize;

/// The index of a chord hypothesis in the chord layer's store.
pub type ChordHypothesisId = HypothesisId<ChordSegment>;

/// A chord hypothesis: the chord layer's reading of every slice up to and including the slice it reads,
/// ending in a chord segment.
pub type ChordHypothesis = Hypothesis<ChordSegment, u32>;

/// The index of a key hypothesis in the key layer's store.
pub type KeyHypothesisId = HypothesisId<Function>;

/// A key hypothesis, or complete interpretation, which reads the latest segment of one chord hypothesis
/// in a key. Its unit is the function of that segment's chord in the key.
/// Through its parents it reads every slice up to its chord hypothesis's last slice.
///
/// `R` is the type of the index of its chord hypothesis.
/// The search proposes hypotheses that read chord hypotheses not yet stored,
/// and indexes those with a type of its own.
pub type KeyHypothesis<R = ChordHypothesisId> = Hypothesis<Function, R>;

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

/// How a chord functions: the key it is heard in, the key it tonicizes there, if any,
/// and its category. This is what the key layer hears in a chord.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub struct Function {
    pub key: Key,
    pub tonicized: Option<Key>,
    pub category: Category,
}

impl Function {
    /// A diatonic chord in a key, tonicizing nothing.
    pub fn diatonic(key: Key) -> Function {
        Function { key, tonicized: None, category: Category::Diatonic }
    }
}

/// A reading of a span of music: the chord that the chord layer hears,
/// and its function as the key layer hears it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
pub struct Reading {
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
    /// Returns the spelling of the segment's bass note, or of its root if it has none.
    pub fn bass_tpc(&self) -> Tpc {
        self.bass.note.map_or(self.chord.root, |(_, tpc)| tpc)
    }

    pub fn has_heard_member(&self, index: usize) -> bool {
        self.members_heard & (1 << index) != 0
    }
}

/// A segment as the key layer reads it: a chord segment that the chord layer heard,
/// with the chord's function. Rules of the key layer receive segments in this form.
#[derive(Clone, Copy, Debug)]
pub struct KeySegment {
    pub heard: ChordSegment,
    pub function: Function,
}

impl KeySegment {
    pub fn reading(&self) -> Reading {
        Reading { chord: self.heard.chord, function: self.function }
    }

    /// Returns the segment's Roman numeral label, in the inversion that its bass gives.
    pub fn label(&self) -> Label {
        Label::new(self.function.key, self.function.tonicized, self.heard.chord, self.heard.bass_tpc())
    }
}
