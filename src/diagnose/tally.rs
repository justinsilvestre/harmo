//! Frame counts collected per piece and merged per benchmark set.

use serde::Serialize;
use std::collections::BTreeMap;

/// How many example spans each kind or class keeps.
pub const EXAMPLE_LIMIT: usize = 8;

/// A run of consecutive frames with the same error and the same labels.
#[derive(Clone, Debug, Serialize)]
pub struct Example {
    pub piece: String,
    pub measure: String,
    pub frames: u64,
    pub expert: String,
    pub engine: String,
}

/// Frames that share one error kind or disagreement class.
#[derive(Clone, Debug, Default, Serialize)]
pub struct KindTally {
    pub group: String,
    pub frames: u64,
    /// Those frames that the equivalents measure counts as right.
    pub equivalent: u64,
    /// Frames by the pair of figures involved, such as "V vs viio".
    pub pairs: BTreeMap<String, u64>,
    /// The longest spans, longest first.
    pub examples: Vec<Example>,
}

impl KindTally {
    pub fn merge(&mut self, other: &KindTally) {
        self.group.clone_from(&other.group);
        self.frames += other.frames;
        self.equivalent += other.equivalent;
        for (pair, frames) in &other.pairs {
            *self.pairs.entry(pair.clone()).or_default() += frames;
        }
        keep_longest(&mut self.examples, &other.examples);
    }
}

/// Adds examples to a list, keeping only the longest few.
pub fn keep_longest(examples: &mut Vec<Example>, more: &[Example]) {
    examples.extend_from_slice(more);
    examples.sort_by_key(|e| std::cmp::Reverse(e.frames));
    examples.truncate(EXAMPLE_LIMIT);
}

/// Merges tallies keyed by name.
pub fn merge_kinds(into: &mut BTreeMap<String, KindTally>, from: &BTreeMap<String, KindTally>) {
    for (name, tally) in from {
        into.entry(name.clone()).or_default().merge(tally);
    }
}

/// The top reading against the main analysis, on every frame the analysis labels.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ErrorTally {
    pub frames: u64,
    pub correct: u64,
    /// Frames that the equivalents measure counts as right, exact matches included.
    pub equivalent: u64,
    /// Frames that each equivalence credits but the exact measure does not, by its name.
    pub credits: BTreeMap<String, u64>,
    pub kinds: BTreeMap<String, KindTally>,
    pub bars: u64,
    pub expert_changes: u64,
    pub engine_changes: u64,
}

impl ErrorTally {
    pub fn merge(&mut self, other: &ErrorTally) {
        self.frames += other.frames;
        self.correct += other.correct;
        self.equivalent += other.equivalent;
        for (name, frames) in &other.credits {
            *self.credits.entry(name.clone()).or_default() += frames;
        }
        self.bars += other.bars;
        self.expert_changes += other.expert_changes;
        self.engine_changes += other.engine_changes;
        merge_kinds(&mut self.kinds, &other.kinds);
    }
}
