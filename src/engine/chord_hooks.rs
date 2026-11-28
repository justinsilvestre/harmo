//! The hooks of the chord layer, and what they receive. They see only the chord layer's state,
//! never a key.

use super::bass::{Bass, BassNote};
use super::model::ChordSegment;
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;

pub type ProposeChordsFn = fn(&Slice) -> Vec<Chord>;
/// Scoring hooks receive the context and the values of their rule's parameters.
pub type ChordSliceFn = fn(&ChordSliceContext, &[f64]) -> f32;
pub type ChordTransitionFn = fn(&ChordTransitionContext, &[f64]) -> f32;
pub type ChordCloseFn = fn(&ChordCloseContext, &[f64]) -> f32;
/// Returns a segment's bass evidence after it hears one more chord tone.
pub type BassFn = fn(Bass, &BassNote, &[f64]) -> Bass;

#[derive(Clone, Copy)]
pub enum ChordHook {
    /// Proposes chords that a new segment opening on this slice might have.
    Propose(ProposeChordsFn),
    /// Scores how well one slice fits a chord.
    Slice(ChordSliceFn),
    /// Scores opening a new chord segment after the previous one.
    Transition(ChordTransitionFn),
    /// Scores a chord segment once its extent is known.
    Close(ChordCloseFn),
    /// Decides a segment's bass note, and so its inversion, as the segment hears each chord tone.
    /// Bass hooks run in rule order, each on the evidence the previous one returned.
    Bass(BassFn),
}

impl ChordHook {
    pub fn kind(&self) -> &'static str {
        match self {
            ChordHook::Propose(_) => "propose_chords",
            ChordHook::Slice(_) => "chord_slice",
            ChordHook::Transition(_) => "chord_transition",
            ChordHook::Close(_) => "chord_close",
            ChordHook::Bass(_) => "bass",
        }
    }
}

pub struct ChordSliceContext<'a> {
    pub slice: &'a Slice,
    /// The slice after this one in the score, if any.
    pub next_slice: Option<&'a Slice>,
    /// Every slice of the score, for rules that weigh the musical context around this slice.
    pub slices: &'a [Slice],
    /// The index of this slice in `slices`.
    pub index: usize,
    pub chord: Chord,
    /// True when the slice opens the segment.
    pub is_opening: bool,
}

pub struct ChordTransitionContext<'a> {
    /// The chord segment being closed, or `None` at the start of the piece.
    pub previous: Option<&'a ChordSegment>,
    pub next: Chord,
    pub slice: &'a Slice,
    /// Every slice of the score, so that the previous segment's slices can be read from its `start`.
    pub slices: &'a [Slice],
}

pub struct ChordCloseContext<'a> {
    pub segment: &'a ChordSegment,
    /// The slices covered by the segment.
    pub slices: &'a [Slice],
    /// The slice that opens the following segment, if any.
    pub next_slice: Option<&'a Slice>,
}
