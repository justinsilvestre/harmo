//! Tracks how a reading's standing in the beam changes as the music unfolds.

use super::model::NONE;
use super::model::{Reading, Segment};
use super::search::Analysis;
use serde::Serialize;

/// The rank (1 = best) at which a reading was held after a given slice, or `None` if no
/// hypothesis in the beam held it. Only slices where the rank changed are listed.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RankChange {
    pub slice: usize,
    pub rank: Option<usize>,
}

/// Follows a reading of the slice `at` through every later beam snapshot.
/// Requires an analysis run with `Config::record_snapshots`.
pub fn rank_history(analysis: &Analysis, at: usize, reading: &Reading) -> Vec<RankChange> {
    let mut changes: Vec<RankChange> = Vec::new();
    for (slice, beam) in analysis.snapshots.iter().enumerate().skip(at) {
        let rank =
            beam.iter().position(|&node| reading_at(analysis, node, at) == Some(*reading)).map(|r| r + 1);
        if changes.last().is_none_or(|c| c.rank != rank) {
            changes.push(RankChange { slice, rank });
        }
    }
    changes
}

/// Returns the reading that the hypothesis ending at `node` assigns to slice `at`.
pub fn reading_at(analysis: &Analysis, node: u32, at: usize) -> Option<Reading> {
    segment_at(analysis, node, at).map(|s| s.reading)
}

/// Returns the segment that the hypothesis ending at `node` assigns to slice `at`.
pub fn segment_at(analysis: &Analysis, node: u32, at: usize) -> Option<&Segment> {
    let mut current = node;
    while current != NONE {
        let segment = &analysis.arena[current as usize].segment;
        if segment.start as usize <= at {
            return Some(segment);
        }
        current = segment.previous;
    }
    None
}
