//! Counts for one class of disagreement frames, merged per benchmark set.

use super::tally::{Example, keep_longest};
use serde::Serialize;

/// Disagreement frames of one class, counted per pair of analysts as the benchmark counts them.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ClassTally {
    pub frames: u64,
    pub top_matches_either: u64,
    pub both_offered: u64,
    /// Analyst readings missing from the offered readings, by reason.
    pub missing_unproducible: u64,
    pub missing_below_cutoff: u64,
    pub missing_outside_beam: u64,
    /// Sum of the score gaps between the best hypothesis and the best one holding a reading
    /// that was in the beam but below the cut-off.
    pub below_cutoff_gap: f64,
    /// Sum of the ranks (1 = best) of the best hypotheses holding those readings.
    pub below_cutoff_rank: u64,
    pub examples: Vec<Example>,
}

impl ClassTally {
    pub fn merge(&mut self, other: &ClassTally) {
        self.frames += other.frames;
        self.top_matches_either += other.top_matches_either;
        self.both_offered += other.both_offered;
        self.missing_unproducible += other.missing_unproducible;
        self.missing_below_cutoff += other.missing_below_cutoff;
        self.missing_outside_beam += other.missing_outside_beam;
        self.below_cutoff_gap += other.below_cutoff_gap;
        self.below_cutoff_rank += other.below_cutoff_rank;
        keep_longest(&mut self.examples, &other.examples);
    }
}
