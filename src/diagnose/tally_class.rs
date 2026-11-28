//! Counts for one class of disagreement frames, merged per benchmark set.

use super::beam_absence::Absence;
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
    /// Readings no longer held in the beam, by how the search lost them.
    pub missing_cut_by_width: u64,
    pub missing_cut_by_hearings: u64,
    pub missing_merged: u64,
    pub missing_never_held: u64,
    /// Sum of how many quarter notes after the frame's slice began the readings cut by width,
    /// cut by the hearing limit or merged were pruned.
    pub pruned_after: f64,
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
        self.missing_cut_by_width += other.missing_cut_by_width;
        self.missing_cut_by_hearings += other.missing_cut_by_hearings;
        self.missing_merged += other.missing_merged;
        self.missing_never_held += other.missing_never_held;
        self.pruned_after += other.pruned_after;
        self.below_cutoff_gap += other.below_cutoff_gap;
        self.below_cutoff_rank += other.below_cutoff_rank;
        keep_longest(&mut self.examples, &other.examples);
    }

    pub fn add_absence(&mut self, absence: Absence) {
        match absence {
            Absence::Width { after } => {
                self.missing_cut_by_width += 1;
                self.pruned_after += after;
            }
            Absence::Hearings { after } => {
                self.missing_cut_by_hearings += 1;
                self.pruned_after += after;
            }
            Absence::Merged { after } => {
                self.missing_merged += 1;
                self.pruned_after += after;
            }
            Absence::NeverHeld => self.missing_never_held += 1,
        }
    }

    /// The mean of `pruned_after` over the readings it sums.
    pub fn mean_pruned_after(&self) -> f64 {
        self.pruned_after
            / (self.missing_cut_by_width + self.missing_cut_by_hearings + self.missing_merged).max(1) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reading_cut_by_the_hearing_limit_is_not_counted_as_cut_by_width() {
        let mut tally = ClassTally::default();
        tally.add_absence(Absence::Hearings { after: 0.0 });
        assert_eq!((tally.missing_cut_by_width, tally.missing_cut_by_hearings), (0, 1));
    }
}
