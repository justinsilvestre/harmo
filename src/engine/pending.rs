//! What the search proposes at one slice before pruning,
//! and how the hypotheses that pruning keeps are stored.

use super::layers::Layers;
use super::model::{ChordHypothesis, ChordHypothesisId, KeyHypothesis, KeyHypothesisId, KeySegment};

/// The index of a chord hypothesis among those proposed at one slice, before they are stored.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PendingChordHypothesisId(pub u32);

/// A hypothesis proposed at one slice, which reads one of the chord hypotheses proposed with it.
pub type PendingKeyHypothesis = KeyHypothesis<PendingChordHypothesisId>;

/// What the search proposes at one slice, before pruning: hypotheses,
/// each extending or opening after one in the beam,
/// and the new chord hypotheses that they interpret.
#[derive(Default)]
pub struct Pending {
    pub chord_hypotheses: Vec<ChordHypothesis>,
    pub key_hypotheses: Vec<PendingKeyHypothesis>,
}

impl Pending {
    pub fn add_chord_hypothesis(&mut self, chord_hypothesis: ChordHypothesis) -> PendingChordHypothesisId {
        self.chord_hypotheses.push(chord_hypothesis);
        PendingChordHypothesisId(self.chord_hypotheses.len() as u32 - 1)
    }

    pub fn chord_hypothesis(&self, id: PendingChordHypothesisId) -> &ChordHypothesis {
        &self.chord_hypotheses[id.0 as usize]
    }

    /// Returns the score of the complete interpretation that a proposed hypothesis ends:
    /// its chord hypothesis's score plus its own.
    pub fn score(&self, hypothesis: &PendingKeyHypothesis) -> f32 {
        self.chord_hypothesis(hypothesis.chord_hypothesis).score + hypothesis.score
    }

    /// Returns the segment that a proposed hypothesis ends in, as the key layer reads it.
    pub fn segment(&self, hypothesis: &PendingKeyHypothesis) -> KeySegment {
        KeySegment {
            heard: self.chord_hypothesis(hypothesis.chord_hypothesis).segment,
            function: hypothesis.function,
        }
    }
}

/// Adds the kept hypotheses to the layers, with the pending chord hypotheses that they interpret,
/// in the order in which the hypotheses first interpret them. Returns the new beam.
pub fn commit(
    layers: &mut Layers,
    chord_hypotheses: &[ChordHypothesis],
    kept: Vec<PendingKeyHypothesis>,
) -> Vec<KeyHypothesisId> {
    let first = layers.key_hypotheses.len() as u32;
    let mut stored = vec![ChordHypothesisId::NONE; chord_hypotheses.len()];
    for hypothesis in kept {
        let pending = hypothesis.chord_hypothesis.0 as usize;
        if stored[pending] == ChordHypothesisId::NONE {
            stored[pending] = ChordHypothesisId(layers.chord_hypotheses.len() as u32);
            layers.chord_hypotheses.push(chord_hypotheses[pending]);
        }
        let (parent, previous, function, score) =
            (hypothesis.parent, hypothesis.previous, hypothesis.function, hypothesis.score);
        layers.key_hypotheses.push(KeyHypothesis {
            parent,
            previous,
            chord_hypothesis: stored[pending],
            function,
            score,
        });
    }
    (first..layers.key_hypotheses.len() as u32).map(KeyHypothesisId).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::Function;
    use crate::engine::test_layers::chord_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    /// Three pending chord hypotheses, of C, F and G major, opening on slice 1 after nothing.
    fn chord_hypotheses() -> Vec<ChordHypothesis> {
        let chord_hypothesis = |root: &str| {
            let segment = chord_segment(Chord::new(tpc_from_name(root).unwrap(), Quality::Major));
            let none = ChordHypothesisId::NONE;
            ChordHypothesis { parent: none, previous: none, slice: 1, segment, score: 0.0 }
        };
        vec![chord_hypothesis("C"), chord_hypothesis("F"), chord_hypothesis("G")]
    }

    /// Commits a pending hypothesis for each given pending chord hypothesis, in order,
    /// into layers that already hold one chord hypothesis. Returns the root of each stored chord hypothesis,
    /// and the stored chord hypothesis of each stored hypothesis.
    fn committed(readers: &[u32]) -> (Vec<String>, Vec<u32>) {
        let function = Function::diatonic(Key::new(tpc_from_name("C").unwrap(), Mode::Major));
        let mut layers =
            Layers { chord_hypotheses: chord_hypotheses()[..1].to_vec(), key_hypotheses: vec![] };
        let pending = |&chord_hypothesis: &u32| KeyHypothesis {
            parent: KeyHypothesisId::NONE,
            previous: KeyHypothesisId::NONE,
            chord_hypothesis: PendingChordHypothesisId(chord_hypothesis),
            function,
            score: 0.0,
        };
        commit(&mut layers, &chord_hypotheses(), readers.iter().map(pending).collect());
        let roots =
            layers.chord_hypotheses.iter().map(|h| crate::pitch::tpc_name(h.segment.chord.root)).collect();
        (roots, layers.key_hypotheses.iter().map(|h| h.chord_hypothesis.0).collect())
    }

    #[test]
    fn chord_hypotheses_that_no_kept_hypothesis_interprets_are_not_stored() {
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
