//! The stores of the search's two layers.

use super::model::{Hearing, HearingId, Hypothesis, KeySegment, NONE};

/// What the search keeps of both layers: the chord layer's hearings, and the key layer's
/// hypotheses, each of which interprets one hearing. Each links to the one before it by index.
#[derive(Clone, Debug, Default)]
pub struct Layers {
    pub hearings: Vec<Hearing>,
    pub hypotheses: Vec<Hypothesis>,
}

/// Layers that hold nothing.
pub static NO_LAYERS: Layers = Layers { hearings: Vec::new(), hypotheses: Vec::new() };

impl Layers {
    pub fn hearing(&self, id: HearingId) -> &Hearing {
        &self.hearings[id.0 as usize]
    }

    /// Returns the hearing that a hypothesis interprets.
    pub fn hearing_of(&self, hypothesis: u32) -> &Hearing {
        self.hearing(self.hypotheses[hypothesis as usize].hearing)
    }

    /// Returns the segment that a hypothesis ends in, as the key layer reads it.
    pub fn segment(&self, hypothesis: u32) -> KeySegment {
        let function = self.hypotheses[hypothesis as usize].function;
        KeySegment { heard: self.hearing_of(hypothesis).segment, function }
    }

    /// Returns the last hypothesis of each segment on the path to `last`, from `last` back to the
    /// first segment. A `last` of `NONE` gives no segments.
    pub fn path_back(&self, last: u32) -> impl Iterator<Item = u32> + '_ {
        let linked = |hypothesis: u32| (hypothesis != NONE).then_some(hypothesis);
        std::iter::successors(linked(last), move |&h| linked(self.hypotheses[h as usize].previous))
    }
}
