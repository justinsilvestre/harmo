//! Counts the frames at which a single line sounds alone, by error kind.

use serde::Serialize;
use std::collections::BTreeMap;

/// Frames at which every sounding note has the same spelled pitch class, as in a solo line or a
/// passage in unison or octaves.
#[derive(Clone, Debug, Default, Serialize)]
pub struct MonophonyTally {
    pub frames: u64,
    pub correct: u64,
    /// Those frames that the equivalents measure counts as right, exact matches included.
    pub equivalent: u64,
    /// Wrong frames by error kind.
    pub kinds: BTreeMap<String, u64>,
}

impl MonophonyTally {
    pub fn merge(&mut self, other: &MonophonyTally) {
        self.frames += other.frames;
        self.correct += other.correct;
        self.equivalent += other.equivalent;
        for (name, frames) in &other.kinds {
            *self.kinds.entry(name.clone()).or_default() += frames;
        }
    }
}
