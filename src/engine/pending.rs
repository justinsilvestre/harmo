//! What the search proposes at one slice before pruning,
//! and how the hypotheses that pruning keeps are stored.

use super::hypothesis::{Hypothesis, HypothesisId};
use super::layers::{Layers, Store};
use super::model::{ChordSegment, Function, KeyHypothesisId, KeySegment};
use std::marker::PhantomData;

/// The index of a hypothesis of the layer whose unit is `U`
/// among those proposed at one slice, before they are stored.
pub struct PendingId<U>(pub u32, PhantomData<fn() -> U>);

impl<U> PendingId<U> {
    pub const fn new(index: u32) -> PendingId<U> {
        PendingId(index, PhantomData)
    }
}

// These are written out because deriving them would require them of `U` as well.
impl<U> Clone for PendingId<U> {
    fn clone(&self) -> PendingId<U> {
        *self
    }
}

impl<U> Copy for PendingId<U> {}

impl<U> PartialEq for PendingId<U> {
    fn eq(&self, other: &PendingId<U>) -> bool {
        self.0 == other.0
    }
}

impl<U> Eq for PendingId<U> {}

impl<U> std::hash::Hash for PendingId<U> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<U> std::fmt::Debug for PendingId<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PendingId({})", self.0)
    }
}

/// The index of a chord hypothesis among those proposed at one slice.
pub type PendingChordHypothesisId = PendingId<ChordSegment>;

/// A hypothesis proposed at one slice, which reads one of the chord hypotheses proposed with it.
pub type PendingKeyHypothesis = Hypothesis<Function, PendingChordHypothesisId>;

/// The hypotheses of one layer proposed at one slice, each reading an `R`.
pub struct Proposed<U, R> {
    pub hypotheses: Vec<Hypothesis<U, R>>,
}

impl<U, R> Proposed<U, R> {
    pub fn with_capacity(capacity: usize) -> Proposed<U, R> {
        Proposed { hypotheses: Vec::with_capacity(capacity) }
    }

    pub fn get(&self, id: PendingId<U>) -> &Hypothesis<U, R> {
        &self.hypotheses[id.0 as usize]
    }

    pub fn add(&mut self, hypothesis: Hypothesis<U, R>) -> PendingId<U> {
        self.hypotheses.push(hypothesis);
        PendingId::new(self.hypotheses.len() as u32 - 1)
    }
}

/// What the search proposes at one slice, before pruning, in every layer:
/// hypotheses, each extending or opening after one in the beam,
/// and the new chord hypotheses that they read.
pub struct Pending {
    pub chord: Proposed<ChordSegment, u32>,
    pub key: Proposed<Function, PendingChordHypothesisId>,
}

impl Pending {
    /// Returns the score of the complete interpretation that a proposed hypothesis ends:
    /// its chord hypothesis's score plus its own.
    pub fn score(&self, hypothesis: &PendingKeyHypothesis) -> f32 {
        self.chord.get(hypothesis.reads).score + hypothesis.score
    }

    /// Returns the segment that a proposed hypothesis ends in, as the key layer reads it.
    pub fn segment(&self, hypothesis: &PendingKeyHypothesis) -> KeySegment {
        KeySegment { heard: self.chord.get(hypothesis.reads).unit, function: hypothesis.unit }
    }
}

/// Adds the kept hypotheses to the layers, with the pending chord hypotheses that they read,
/// in the order in which the hypotheses first read them, as each layer's ranking at the next slice.
/// Returns the new beam.
pub fn commit(
    layers: &mut Layers,
    chord: &Proposed<ChordSegment, u32>,
    kept: Vec<PendingKeyHypothesis>,
) -> Vec<KeyHypothesisId> {
    let first = layers.key.len() as u32;
    layers.chord.begin_slice();
    layers.key.begin_slice();
    let mut stored = vec![HypothesisId::NONE; chord.hypotheses.len()];
    for hypothesis in kept {
        let reads = store_once(&mut layers.chord, chord, &mut stored, hypothesis.reads, |slice| slice);
        layers.key.push(hypothesis.reading(reads));
    }
    (first..layers.key.len() as u32).map(HypothesisId::new).collect()
}

/// Stores a proposed hypothesis of a lower layer the first time a kept hypothesis reads it,
/// and returns its index in the store. `stored` holds the index of each proposed hypothesis stored so far,
/// and `stored_reads` gives the stored index of what a proposed hypothesis reads.
fn store_once<U: Copy, R: Copy, S>(
    store: &mut Store<U, S>,
    proposed: &Proposed<U, R>,
    stored: &mut [HypothesisId<U>],
    id: PendingId<U>,
    stored_reads: impl FnOnce(R) -> S,
) -> HypothesisId<U> {
    let index = id.0 as usize;
    if stored[index] == HypothesisId::NONE {
        let hypothesis = proposed.hypotheses[index];
        stored[index] = store.push(hypothesis.reading(stored_reads(hypothesis.reads)));
    }
    stored[index]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{ChordHypothesis, Function};
    use crate::engine::test_layers::chord_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    /// Three pending chord hypotheses, of C, F and G major, opening on slice 1 after nothing.
    fn chord_hypotheses() -> Vec<ChordHypothesis> {
        let chord_hypothesis = |root: &str| {
            let unit = chord_segment(Chord::new(tpc_from_name(root).unwrap(), Quality::Major));
            let none = HypothesisId::NONE;
            Hypothesis { parent: none, previous: none, reads: 1, unit, score: 0.0 }
        };
        vec![chord_hypothesis("C"), chord_hypothesis("F"), chord_hypothesis("G")]
    }

    /// Commits a pending hypothesis for each given pending chord hypothesis, in order,
    /// into layers that already hold one chord hypothesis. Returns the root of each stored chord hypothesis,
    /// and the stored chord hypothesis of each stored hypothesis.
    fn committed(readers: &[u32]) -> (Vec<String>, Vec<u32>) {
        let layers = committed_layers(readers);
        let roots =
            layers.chord.hypotheses.iter().map(|h| crate::pitch::tpc_name(h.unit.chord.root)).collect();
        (roots, layers.key.hypotheses.iter().map(|h| h.reads.0).collect())
    }

    /// Commits a pending hypothesis for each given pending chord hypothesis, as `committed` does.
    fn committed_layers(readers: &[u32]) -> Layers {
        let function = Function::diatonic(Key::new(tpc_from_name("C").unwrap(), Mode::Major));
        let mut layers = Layers::default();
        layers.chord.push(chord_hypotheses()[0]);
        let pending = |&chord_hypothesis: &u32| Hypothesis {
            parent: HypothesisId::NONE,
            previous: HypothesisId::NONE,
            reads: PendingId::new(chord_hypothesis),
            unit: function,
            score: 0.0,
        };
        let proposed = Proposed { hypotheses: chord_hypotheses() };
        commit(&mut layers, &proposed, readers.iter().map(pending).collect());
        layers
    }

    #[test]
    fn the_chord_ranking_holds_each_chord_hypothesis_once_at_the_rank_of_its_best_reader() {
        let layers = committed_layers(&[2, 0, 2]);
        let ranking = layers.chord.ranking(0).unwrap();
        assert_eq!(ranking.iter().map(|h| h.0).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn the_key_ranking_holds_the_kept_hypotheses_in_order() {
        let layers = committed_layers(&[2, 0, 2]);
        assert_eq!(layers.key.ranking(0).unwrap().iter().map(|h| h.0).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn chord_hypotheses_that_no_kept_hypothesis_reads_are_not_stored() {
        assert_eq!(committed(&[1]), (vec!["C".into(), "F".into()], vec![1]));
    }

    #[test]
    fn chord_hypotheses_are_stored_in_the_order_that_hypotheses_first_read_them() {
        assert_eq!(committed(&[2, 0, 2]), (vec!["C".into(), "G".into(), "C".into()], vec![1, 2, 1]));
    }

    #[test]
    fn chord_hypotheses_in_order_are_stored_in_order() {
        assert_eq!(
            committed(&[0, 1, 2]),
            (vec!["C".into(), "C".into(), "F".into(), "G".into()], vec![1, 2, 3])
        );
    }
}
