//! The hypothesis record and typed index that every layer of the search shares.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A hypothesis of the layer whose unit is `U`: the layer's reading of every slice
/// up to the last slice of what it reads.
///
/// `R` is what the hypothesis reads: a hypothesis of the layer below,
/// or for the first layer, the last slice of the notes.
#[derive(Clone, Copy, Debug)]
pub struct Hypothesis<U, R> {
    /// The hypothesis up to the slice before, or `HypothesisId::NONE`.
    pub parent: HypothesisId<U>,
    /// The last hypothesis of the preceding segment, or `HypothesisId::NONE`.
    pub previous: HypothesisId<U>,
    pub reads: R,
    /// The layer's own reading of the latest segment.
    pub unit: U,
    /// The sum of the layer's own weighted rule scores along the hypothesis.
    pub score: f32,
}

impl<U, R> Hypothesis<U, R> {
    /// Returns the hypothesis reading `reads` in place of what it reads.
    pub fn reading<S>(self, reads: S) -> Hypothesis<U, S> {
        let Hypothesis { parent, previous, unit, score, .. } = self;
        Hypothesis { parent, previous, reads, unit, score }
    }
}

/// The index of a hypothesis in the store of the layer whose unit is `U`.
pub struct HypothesisId<U>(pub u32, PhantomData<fn() -> U>);

impl<U> HypothesisId<U> {
    /// Marks the absence of a hypothesis.
    pub const NONE: HypothesisId<U> = HypothesisId::new(u32::MAX);

    pub const fn new(index: u32) -> HypothesisId<U> {
        HypothesisId(index, PhantomData)
    }
}

// These are written out because deriving them would require them of `U` as well.
impl<U> Clone for HypothesisId<U> {
    fn clone(&self) -> HypothesisId<U> {
        *self
    }
}

impl<U> Copy for HypothesisId<U> {}

impl<U> PartialEq for HypothesisId<U> {
    fn eq(&self, other: &HypothesisId<U>) -> bool {
        self.0 == other.0
    }
}

impl<U> Eq for HypothesisId<U> {}

impl<U> Hash for HypothesisId<U> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<U> fmt::Debug for HypothesisId<U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "HypothesisId({})", self.0)
    }
}
