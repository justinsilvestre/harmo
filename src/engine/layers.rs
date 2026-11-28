//! The stores of the search's two layers.

use super::model::{Hearing, Hypothesis, Segment};

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
    /// Returns the hearing that a hypothesis interprets.
    pub fn hearing_of(&self, hypothesis: u32) -> &Hearing {
        &self.hearings[self.hypotheses[hypothesis as usize].hearing as usize]
    }

    /// Returns the segment that a hypothesis ends in, as the key layer reads it.
    pub fn segment(&self, hypothesis: u32) -> Segment {
        Segment::join(&self.hearing_of(hypothesis).segment, &self.hypotheses[hypothesis as usize])
    }
}
