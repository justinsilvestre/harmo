//! What the search proposes at one slice before pruning, and how the hypotheses that pruning keeps
//! are stored.

use super::layers::Layers;
use super::model::{Hearing, HearingId, Hypothesis, KeySegment};

/// The index of a hearing among those proposed at one slice, before they are stored.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PendingHearingId(pub u32);

/// A hypothesis proposed at one slice, which reads one of the hearings proposed with it.
pub type PendingHypothesis = Hypothesis<PendingHearingId>;

/// What the search proposes at one slice, before pruning: hypotheses, each extending or opening
/// after one in the beam, and the new hearings of the chords that they interpret.
#[derive(Default)]
pub struct Pending {
    pub hearings: Vec<Hearing>,
    pub hypotheses: Vec<PendingHypothesis>,
}

impl Pending {
    pub fn add_hearing(&mut self, hearing: Hearing) -> PendingHearingId {
        self.hearings.push(hearing);
        PendingHearingId(self.hearings.len() as u32 - 1)
    }

    pub fn hearing(&self, id: PendingHearingId) -> &Hearing {
        &self.hearings[id.0 as usize]
    }

    /// Returns the segment that a proposed hypothesis ends in, as the key layer reads it.
    pub fn segment(&self, hypothesis: &PendingHypothesis) -> KeySegment {
        KeySegment { heard: self.hearing(hypothesis.hearing).segment, function: hypothesis.function }
    }
}

/// Adds the kept hypotheses to the layers, with the pending hearings that they interpret, in the
/// order in which the hypotheses first interpret them. Returns the new beam.
pub fn commit(layers: &mut Layers, hearings: &[Hearing], kept: Vec<PendingHypothesis>) -> Vec<u32> {
    let first = layers.hypotheses.len() as u32;
    let mut stored = vec![HearingId::NONE; hearings.len()];
    for hypothesis in kept {
        let pending = hypothesis.hearing.0 as usize;
        if stored[pending] == HearingId::NONE {
            stored[pending] = HearingId(layers.hearings.len() as u32);
            layers.hearings.push(hearings[pending]);
        }
        let (parent, previous, function, score) =
            (hypothesis.parent, hypothesis.previous, hypothesis.function, hypothesis.score);
        layers.hypotheses.push(Hypothesis { parent, previous, hearing: stored[pending], function, score });
    }
    (first..layers.hypotheses.len() as u32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Function, NONE};
    use crate::engine::test_layers::chord_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    /// Three pending hearings, of C, F and G major, opening on slice 1 after nothing.
    fn hearings() -> Vec<Hearing> {
        let hearing = |root: &str| {
            let segment = chord_segment(Chord::new(tpc_from_name(root).unwrap(), Quality::Major));
            Hearing { previous: HearingId::NONE, slice: 1, segment, score: 0.0 }
        };
        vec![hearing("C"), hearing("F"), hearing("G")]
    }

    /// Commits a pending hypothesis for each given pending hearing, in order, into layers that
    /// already hold one hearing. Returns the root of each stored hearing, and the stored hearing of
    /// each stored hypothesis.
    fn committed(readers: &[u32]) -> (Vec<String>, Vec<u32>) {
        let function = Function::diatonic(Key::new(tpc_from_name("C").unwrap(), Mode::Major));
        let mut layers = Layers { hearings: hearings()[..1].to_vec(), hypotheses: vec![] };
        let pending = |&hearing: &u32| Hypothesis {
            parent: NONE,
            previous: NONE,
            hearing: PendingHearingId(hearing),
            function,
            score: 0.0,
        };
        commit(&mut layers, &hearings(), readers.iter().map(pending).collect());
        let roots = layers.hearings.iter().map(|h| crate::pitch::tpc_name(h.segment.chord.root)).collect();
        (roots, layers.hypotheses.iter().map(|h| h.hearing.0).collect())
    }

    #[test]
    fn hearings_that_no_kept_hypothesis_interprets_are_not_stored() {
        assert_eq!(committed(&[1]), (vec!["C".into(), "F".into()], vec![1]));
    }

    #[test]
    fn hearings_are_stored_in_the_order_that_hypotheses_first_read_them() {
        assert_eq!(committed(&[2, 0, 2]), (vec!["C".into(), "G".into(), "C".into()], vec![1, 2, 1]));
    }

    #[test]
    fn hearings_in_order_are_stored_in_order() {
        assert_eq!(
            committed(&[0, 1, 2]),
            (vec!["C".into(), "C".into(), "F".into(), "G".into()], vec![1, 2, 3])
        );
    }
}
