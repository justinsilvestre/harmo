//! The stores of the search's layers.

use super::hypothesis::{Hypothesis, HypothesisId};
use super::model::{ChordHypothesis, ChordHypothesisId, ChordSegment, Function, KeyHypothesisId, KeySegment};
use super::ranking::Ranking;

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
/// Each slice's hypotheses are stored together, best first (see `ranking`).
#[derive(Clone, Debug)]
pub struct Store<U, R> {
    pub hypotheses: Vec<Hypothesis<U, R>>,
    /// The index of the first hypothesis stored at each slice.
    pub slice_starts: Vec<u32>,
}

impl<U, R> Default for Store<U, R> {
    fn default() -> Store<U, R> {
        Store::new()
    }
}

impl<U, R> Store<U, R> {
    pub const fn new() -> Store<U, R> {
        Store { hypotheses: Vec::new(), slice_starts: Vec::new() }
    }

    /// Holds hypotheses that were not stored slice by slice, so that it has no rankings.
    pub fn of(hypotheses: Vec<Hypothesis<U, R>>) -> Store<U, R> {
        Store { hypotheses, slice_starts: Vec::new() }
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

    /// Marks the hypotheses pushed from now on as those of the next slice.
    pub fn begin_slice(&mut self) {
        self.slice_starts.push(self.hypotheses.len() as u32);
    }

    /// Returns the layer's ranking at a slice: the hypotheses stored at it, best first,
    /// or `None` if the store holds no slice of that index.
    pub fn ranking(&self, slice: usize) -> Option<Ranking<U>> {
        let start = *self.slice_starts.get(slice)?;
        let end = self.slice_starts.get(slice + 1).copied().unwrap_or(self.hypotheses.len() as u32);
        Some(Ranking::new(start, end))
    }

    /// Returns the layer's ranking at every slice, in order.
    pub fn rankings(&self) -> impl DoubleEndedIterator<Item = Ranking<U>> + '_ {
        (0..self.slice_starts.len()).map(|slice| self.ranking(slice).expect("the slice is stored"))
    }

    /// Returns the hypothesis of `ranking` from which `hypothesis` descends by parent links (or is itself),
    /// or `None` if there is none.
    pub fn ancestor_in(&self, hypothesis: HypothesisId<U>, ranking: Ranking<U>) -> Option<HypothesisId<U>> {
        let parent = |&id: &HypothesisId<U>| Some(self.get(id).parent).filter(|&p| p != HypothesisId::NONE);
        std::iter::successors(Some(hypothesis), parent).find(|&id| ranking.position(id).is_some())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A store of five hypotheses: two stored at slice 0, none at slice 1 and three at slice 2.
    fn store() -> Store<(), ()> {
        let none = HypothesisId::NONE;
        let hypothesis = Hypothesis { parent: none, previous: none, reads: (), unit: (), score: 0.0 };
        let mut store = Store::new();
        for count in [2, 0, 3] {
            store.begin_slice();
            (0..count).for_each(|_| _ = store.push(hypothesis));
        }
        store
    }

    #[test]
    fn a_slice_ranks_the_hypotheses_stored_at_it() {
        assert_eq!(store().ranking(0).unwrap().iter().map(|h| h.0).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn a_slice_at_which_nothing_was_stored_ranks_nothing() {
        assert!(store().ranking(1).unwrap().is_empty());
    }

    #[test]
    fn the_last_slice_ranks_the_hypotheses_up_to_the_end_of_the_store() {
        assert_eq!(store().ranking(2).unwrap().len(), 3);
    }

    #[test]
    fn a_slice_the_store_does_not_hold_has_no_ranking() {
        assert!(store().ranking(3).is_none());
    }

    #[test]
    fn a_hypothesis_descends_from_its_parent_in_the_ranking_before() {
        let mut store = store();
        store.hypotheses[4].parent = HypothesisId::new(1);
        let ranking = store.ranking(0).unwrap();
        assert_eq!(store.ancestor_in(HypothesisId::new(4), ranking), Some(HypothesisId::new(1)));
    }

    #[test]
    fn a_hypothesis_without_parents_has_no_ancestor_in_an_earlier_ranking() {
        let store = store();
        assert_eq!(store.ancestor_in(HypothesisId::new(4), store.ranking(0).unwrap()), None);
    }

    #[test]
    fn a_store_gives_a_ranking_for_every_slice() {
        assert_eq!(store().rankings().count(), 3);
    }
}
