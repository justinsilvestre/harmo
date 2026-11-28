//! The stores of the search's two layers.

use super::model::{ChordHypothesis, ChordHypothesisId, KeyHypothesis, KeyHypothesisId, KeySegment};

/// What the search keeps of both layers: the chord layer's hypotheses,
/// and the key layer's hypotheses, each of which interprets one chord hypothesis.
/// Each links to the one before it by index.
#[derive(Clone, Debug, Default)]
pub struct Layers {
    pub chord_hypotheses: Vec<ChordHypothesis>,
    pub key_hypotheses: Vec<KeyHypothesis>,
}

/// Layers that hold nothing.
pub static NO_LAYERS: Layers = Layers { chord_hypotheses: Vec::new(), key_hypotheses: Vec::new() };

impl Layers {
    pub fn chord_hypothesis(&self, id: ChordHypothesisId) -> &ChordHypothesis {
        &self.chord_hypotheses[id.0 as usize]
    }

    pub fn key_hypothesis(&self, id: KeyHypothesisId) -> &KeyHypothesis {
        &self.key_hypotheses[id.0 as usize]
    }

    /// Returns the chord hypothesis that a hypothesis interprets.
    pub fn chord_hypothesis_of(&self, hypothesis: KeyHypothesisId) -> &ChordHypothesis {
        self.chord_hypothesis(self.key_hypothesis(hypothesis).chord_hypothesis)
    }

    /// Returns the segment that a hypothesis ends in, as the key layer reads it.
    pub fn segment(&self, hypothesis: KeyHypothesisId) -> KeySegment {
        let function = self.key_hypothesis(hypothesis).function;
        KeySegment { heard: self.chord_hypothesis_of(hypothesis).segment, function }
    }

    /// Returns the last hypothesis of each segment on the path to `last`,
    /// from `last` back to the first segment. A `last` of `KeyHypothesisId::NONE` gives no segments.
    pub fn path_back(&self, last: KeyHypothesisId) -> impl Iterator<Item = KeyHypothesisId> + '_ {
        let linked =
            |hypothesis: KeyHypothesisId| (hypothesis != KeyHypothesisId::NONE).then_some(hypothesis);
        std::iter::successors(linked(last), move |&h| linked(self.key_hypothesis(h).previous))
    }
}
