//! The interface between the engine and the rules it hooks into.

use super::model::{Category, NONE, Node, Reading, Segment};
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;
use crate::theory::key::Key;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Layer {
    Chord,
    Key,
    Function,
}

/// A pointer into the source catalog, e.g. a page range in a textbook.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Citation {
    pub source: &'static str,
    pub locator: &'static str,
}

/// A rule: one hook into the engine, documented with its sources.
///
/// Scoring hooks return unweighted evidence; the engine multiplies it by the rule's weight.
/// Generator and constraint hooks are switched off by a weight of zero.
#[derive(Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub layer: Layer,
    pub summary: &'static str,
    pub citations: &'static [Citation],
    pub default_weight: f32,
    pub hook: Hook,
}

/// A way a chord can function in a key: the key it tonicizes, if any, and its category.
pub type Interpretation = (Option<Key>, Category);

pub type ProposeFn = fn(&Slice) -> Vec<Chord>;
pub type InterpretFn = fn(Key, Chord) -> Vec<Interpretation>;
pub type SliceFn = fn(&SliceContext) -> f32;
pub type TransitionFn = fn(&TransitionContext) -> f32;
pub type CloseFn = fn(&CloseContext) -> f32;
pub type AdmitFn = fn(&TransitionContext) -> bool;

#[derive(Clone, Copy)]
pub enum Hook {
    /// Proposes chords that a new segment opening on this slice might have.
    ProposeChords(ProposeFn),
    /// Lists the ways a chord can function in a key, as (tonicized key, category) pairs.
    Interpret(InterpretFn),
    /// Scores how well one slice fits a reading.
    Slice(SliceFn),
    /// Scores opening a new segment after the previous one.
    Transition(TransitionFn),
    /// Scores a segment once its extent is known.
    Close(CloseFn),
    /// Rejects transitions outright (a hard constraint).
    Admit(AdmitFn),
}

impl Hook {
    pub fn kind(&self) -> &'static str {
        match self {
            Hook::ProposeChords(_) => "propose_chords",
            Hook::Interpret(_) => "interpret",
            Hook::Slice(_) => "slice",
            Hook::Transition(_) => "transition",
            Hook::Close(_) => "close",
            Hook::Admit(_) => "admit",
        }
    }
}

pub struct SliceContext<'a> {
    pub slice: &'a Slice,
    /// The slice after this one in the score, if any.
    pub next_slice: Option<&'a Slice>,
    pub reading: &'a Reading,
    /// True when the slice opens the segment.
    pub is_opening: bool,
}

pub struct TransitionContext<'a> {
    /// The segment being closed, or `None` at the start of the piece.
    pub previous: Option<&'a Segment>,
    pub next: &'a Reading,
    pub slice: &'a Slice,
    pub history: History<'a>,
}

pub struct CloseContext<'a> {
    pub segment: &'a Segment,
    /// The slices covered by the segment.
    pub slices: &'a [Slice],
    /// The slice that opens the following segment, if any.
    pub next_slice: Option<&'a Slice>,
}

/// Read-only access to the segments preceding a transition, most recent first.
#[derive(Clone, Copy)]
pub struct History<'a> {
    pub arena: &'a [Node],
    /// Arena index of the last node of the most recent segment, or `NONE`.
    pub last: u32,
}

impl<'a> History<'a> {
    pub fn segments(self) -> impl Iterator<Item = &'a Segment> {
        let arena = self.arena;
        std::iter::successors((self.last != NONE).then(|| &arena[self.last as usize].segment), move |s| {
            (s.previous != NONE).then(|| &arena[s.previous as usize].segment)
        })
    }
}
