//! Tracks how a reading's standing in the beam changes as the music unfolds.

use super::model::NONE;
use super::model::{Reading, Segment};
use super::pruning::{Departure, PruneCause};
use super::search::Analysis;
use serde::Serialize;

/// The rank (1 = best) at which a reading was held after a given slice, or `None` once no
/// hypothesis in the beam holds it. Only slices where the rank changed are listed.
/// A reading that has left the beam never returns, so a `None` rank ends the history.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RankChange {
    pub slice: usize,
    pub rank: Option<usize>,
    /// Why the reading left the beam, when the analysis recorded pruning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_beam: Option<Departure>,
}

/// The rank history of one reading of a slice.
pub struct ReadingHistory {
    pub reading: Reading,
    /// The best hypothesis holding the reading in the beam of the slice itself.
    pub node: u32,
    pub changes: Vec<RankChange>,
}

/// Follows every reading that the beam holds for slice `at` through the later beam snapshots,
/// in order of their rank at `at`. Requires an analysis run with `Config::record_snapshots`.
///
/// Every hypothesis in a later beam descends from one in the previous beam and keeps its reading of
/// `at`, so each beam's readings are found from its parents' and no path is walked.
pub fn rank_histories(analysis: &Analysis, at: usize) -> Vec<ReadingHistory> {
    let Some(first_beam) = analysis.snapshots.get(at) else {
        return Vec::new();
    };
    let mut histories: Vec<ReadingHistory> = Vec::new();
    let mut held: Vec<usize> = Vec::with_capacity(first_beam.len());
    for (position, &node) in first_beam.iter().enumerate() {
        let reading = reading_at(analysis, node, at).expect("a hypothesis covers its own slice");
        let id = histories.iter().position(|h| h.reading == reading).unwrap_or_else(|| {
            let changes = vec![RankChange { slice: at, rank: Some(position + 1), left_beam: None }];
            histories.push(ReadingHistory { reading, node, changes });
            histories.len() - 1
        });
        held.push(id);
    }
    let mut alive = histories.len();
    for slice in at + 1..analysis.snapshots.len() {
        if alive <= 1 {
            break;
        }
        let (previous, beam) = (&analysis.snapshots[slice - 1], &analysis.snapshots[slice]);
        let held_by_parent = |parent: u32| held[(parent - previous[0]) as usize];
        let next: Vec<usize> =
            beam.iter().map(|&node| held_by_parent(analysis.hypotheses[node as usize].parent)).collect();
        let mut ranks: Vec<Option<usize>> = vec![None; histories.len()];
        for (position, &id) in next.iter().enumerate() {
            ranks[id].get_or_insert(position + 1);
        }
        for (id, history) in histories.iter_mut().enumerate() {
            let last = history.changes.last().expect("histories start with a rank");
            let Some(last_rank) = last.rank.filter(|_| last.rank != ranks[id]) else {
                continue;
            };
            let left_beam = ranks[id]
                .is_none()
                .then(|| departure(analysis, slice, last_rank, |parent| held_by_parent(parent) == id));
            alive -= left_beam.is_some() as usize;
            history.changes.push(RankChange { slice, rank: ranks[id], left_beam: left_beam.flatten() });
        }
        held = next;
    }
    histories
}

/// Explains why a reading left the beam at `slice`, from the best pruned hypothesis that held it
/// (`holds` tests a pruned hypothesis's parent). Returns `None` unless the analysis recorded pruning.
fn departure(
    analysis: &Analysis,
    slice: usize,
    last_rank: usize,
    holds: impl Fn(u32) -> bool,
) -> Option<Departure> {
    let pruned = analysis.pruned.get(slice)?.iter().find(|p| holds(p.node.parent))?;
    let beam = &analysis.snapshots[slice];
    let best = beam.first().map_or(pruned.node.score, |&b| analysis.hypotheses[b as usize].score);
    let keeper = match pruned.cause {
        PruneCause::Merged { keeper } => Some(keeper),
        PruneCause::Width | PruneCause::HearingWidth => None,
    };
    Some(Departure {
        last_rank,
        competing_rank: pruned.rank,
        score_gap: best - pruned.node.score,
        cause: pruned.cause.name(),
        merged_into_rank: keeper.map(|k| (k - beam[0]) as usize + 1),
        merged_into: keeper,
    })
}

/// Returns the reading that the hypothesis ending at `node` assigns to slice `at`.
pub fn reading_at(analysis: &Analysis, node: u32, at: usize) -> Option<Reading> {
    segment_at(analysis, node, at).map(|s| s.reading)
}

/// Returns the segment that the hypothesis ending at `node` assigns to slice `at`.
pub fn segment_at(analysis: &Analysis, node: u32, at: usize) -> Option<Segment> {
    let mut current = node;
    while current != NONE {
        let segment = analysis.segment(current);
        if segment.start as usize <= at {
            return Some(segment);
        }
        current = segment.previous;
    }
    None
}
