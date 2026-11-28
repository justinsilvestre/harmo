//! Records of the hypotheses that pruning drops from the beam, and why.
//!
//! A hypothesis that is kept always yields a continuation at the next slice, so it can only leave
//! the search by being pruned. A reading that the beam holds for some slice therefore leaves it
//! exactly when pruning drops the last hypotheses that hold it.

use super::model::KeySegment;
use serde::Serialize;

/// Why pruning dropped a hypothesis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PruneCause {
    /// The beam was already full of better hypotheses.
    Width,
    /// The beam already held `Config::hearing_width` hearings of the chords, each interpreted by a
    /// better hypothesis, and this hypothesis interprets another.
    HearingWidth,
    /// A better hypothesis whose last `Config::distinct_depth` segments the distinguish rules count
    /// as the same was kept instead. `keeper` is its index in the hypotheses.
    Merged { keeper: u32 },
}

impl PruneCause {
    pub fn name(self) -> &'static str {
        match self {
            PruneCause::Width => "beam_width",
            PruneCause::HearingWidth => "hearing_width",
            PruneCause::Merged { .. } => "distinct_depth",
        }
    }
}

/// A hypothesis dropped by pruning at the slice it reached.
#[derive(Clone, Copy, Debug)]
pub struct Pruned {
    /// Index in the hypotheses of the hypothesis that it extended or opened after.
    pub parent: u32,
    pub score: f32,
    /// Its last segment.
    pub segment: KeySegment,
    /// Its rank (1 = best) among all the hypotheses that competed for the beam at that slice.
    pub rank: usize,
    pub cause: PruneCause,
}

/// How a reading left the beam: the best of the hypotheses that held it, as it was pruned.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Departure {
    /// The reading's rank in the beam one slice earlier.
    pub last_rank: usize,
    /// Its best hypothesis's rank among all the hypotheses that competed for the beam.
    pub competing_rank: usize,
    /// How far that hypothesis's score fell below the best competing hypothesis.
    pub score_gap: f32,
    /// `beam_width`, `hearing_width` or `distinct_depth`.
    pub cause: &'static str,
    /// For `distinct_depth`, the rank in the new beam of the hypothesis kept in its place.
    pub merged_into_rank: Option<usize>,
    /// The index of that hypothesis in the hypotheses.
    #[serde(skip)]
    pub merged_into: Option<u32>,
}
