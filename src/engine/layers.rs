//! The stores of the search's layers.

use super::hypothesis::{Hypothesis, HypothesisId};
use super::model::{ChordHypothesis, ChordHypothesisId, ChordSegment, Function, KeyHypothesisId, KeySegment};

/// What the search keeps of every layer, in order: the chord layer's hypotheses,
/// and the key layer's hypotheses, each of which reads one chord hypothesis.
#[derive(Clone, Debug, Default)]
pub struct Layers {
    pub chord: Store<ChordSegment, u32>,
    pub key: Store<Function, ChordHypothesisId>,
}

/// Layers that hold nothing.
pub static NO_LAYERS: Layers = Layers { chord: Store::new(), key: Store::new() };

/// The hypotheses of the layer whose unit is `U`, each reading an `R`,
/// and each linked by index to its own layer's past.
#[derive(Clone, Debug)]
pub struct Store<U, R> {
    pub hypotheses: Vec<Hypothesis<U, R>>,
}

impl<U, R> Default for Store<U, R> {
    fn default() -> Store<U, R> {
        Store::new()
    }
}

impl<U, R> Store<U, R> {
    pub const fn new() -> Store<U, R> {
        Store { hypotheses: Vec::new() }
    }

    pub fn get(&self, id: HypothesisId<U>) -> &Hypothesis<U, R> {
        &self.hypotheses[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.hypotheses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hypotheses.is_empty()
    }

    /// Adds a hypothesis and returns its index.
    pub fn push(&mut self, hypothesis: Hypothesis<U, R>) -> HypothesisId<U> {
        self.hypotheses.push(hypothesis);
        HypothesisId::new(self.hypotheses.len() as u32 - 1)
    }

    /// Returns the last hypothesis of each segment on the path to `last`,
    /// from `last` back to the first segment. A `last` of `HypothesisId::NONE` gives no segments.
    pub fn path_back(&self, last: HypothesisId<U>) -> impl Iterator<Item = HypothesisId<U>> + '_ {
        let linked = |id: HypothesisId<U>| (id != HypothesisId::NONE).then_some(id);
        std::iter::successors(linked(last), move |&id| linked(self.get(id).previous))
    }
}

impl Layers {
    /// Returns the score of the complete interpretation that ends in a hypothesis:
    /// its chord hypothesis's score plus its own.
    pub fn score(&self, hypothesis: KeyHypothesisId) -> f32 {
        let key_hypothesis = self.key.get(hypothesis);
        self.chord.get(key_hypothesis.reads).score + key_hypothesis.score
    }

    /// Returns the chord hypothesis that a hypothesis reads.
    pub fn chord_hypothesis_of(&self, hypothesis: KeyHypothesisId) -> &ChordHypothesis {
        self.chord.get(self.key.get(hypothesis).reads)
    }

    /// Returns the segment that a hypothesis ends in, as the key layer reads it.
    pub fn segment(&self, hypothesis: KeyHypothesisId) -> KeySegment {
        let function = self.key.get(hypothesis).unit;
        KeySegment { heard: self.chord_hypothesis_of(hypothesis).unit, function }
    }
}
