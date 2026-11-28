//! The hooks of the key layer. What they receive is in `key_contexts.rs`.

use super::key_contexts::{KeyCloseContext, KeyDepartureContext, KeySliceContext, KeyTransitionContext};
use super::model::{Function, KeySegment, Reading};
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;
use crate::theory::key::Key;

pub type ProposeKeysFn = fn(&Slice) -> Vec<Key>;
pub type InterpretFn = fn(Key, Chord) -> Vec<Function>;
/// Scoring hooks receive the context and the values of their rule's parameters.
pub type KeySliceFn = fn(&KeySliceContext, &[f64]) -> f32;
pub type KeyTransitionFn = fn(&KeyTransitionContext, &[f64]) -> f32;
pub type KeyDepartureFn = fn(&KeyDepartureContext, &[f64]) -> f32;
pub type KeyCloseFn = fn(&KeyCloseContext, &[f64]) -> f32;
/// Returns an opaque value for a segment; the engine only compares values for equality.
pub type DistinguishFn = fn(&KeySegment, &[f64]) -> u64;
/// Returns a rival reading of a slice for a candidate reading of it, or `None`.
pub type RivalCandidateFn = fn(&Reading, &Slice, &[f64]) -> Option<Reading>;

#[derive(Clone, Copy)]
pub enum KeyHook {
    /// Proposes keys in which a new segment opening on this slice might be read.
    Propose(ProposeKeysFn),
    /// Lists the ways a chord can function in a key.
    Interpret(InterpretFn),
    /// Scores how well one slice fits a reading.
    Slice(KeySliceFn),
    /// Scores opening a new segment after the previous one.
    Transition(KeyTransitionFn),
    /// Scores opening a new segment in a different key from the previous one,
    /// from the hypothesis and the slice alone. Since the new reading plays no part,
    /// the engine scores it once per hypothesis and slice instead of once per new reading.
    Departure(KeyDepartureFn),
    /// Scores a segment once its extent is known.
    Close(KeyCloseFn),
    /// Says which segments count as the same interpretation when the beam is pruned.
    /// Of the hypotheses whose last `Config::distinct_depth` segments get the same values
    /// from every distinguish hook, only the best is kept.
    Distinguish(DistinguishFn),
    /// Names a rival reading of a slice for each candidate reading kept for a segment opening on it.
    /// When the proposers and interpreters produce the rival reading
    /// but its fit to the slice falls below the candidate limit, it is kept as a candidate as well.
    RivalCandidate(RivalCandidateFn),
}

impl KeyHook {
    pub fn kind(&self) -> &'static str {
        match self {
            KeyHook::Propose(_) => "key_propose",
            KeyHook::Interpret(_) => "key_interpret",
            KeyHook::Slice(_) => "key_slice",
            KeyHook::Transition(_) => "key_transition",
            KeyHook::Departure(_) => "key_departure",
            KeyHook::Close(_) => "key_close",
            KeyHook::Distinguish(_) => "key_distinguish",
            KeyHook::RivalCandidate(_) => "key_rival_candidate",
        }
    }
}
