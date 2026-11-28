//! Rank histories: how one layer's reading of a slice rises and falls in that layer's later rankings.

use super::hypothesis::HypothesisId;
use super::layers::Store;
use super::pruning::Departure;
use super::ranking::Ranking;
use serde::Serialize;

/// The rank (1 = best) at which a reading was held after a given slice,
/// or `None` once no hypothesis in the layer's ranking holds it.
/// Only slices where the rank changed are listed.
/// A reading that has left the ranking never returns, so a `None` rank ends the history.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RankChange {
    pub slice: usize,
    pub rank: Option<usize>,
    /// Why the reading left the beam, when the analysis recorded pruning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_beam: Option<Departure>,
}

/// The rank history of one reading `T` of a slice, followed through the rankings of the layer whose unit is `U`.
pub struct RankHistory<T, U> {
    pub reading: T,
    /// The best hypothesis holding the reading in the layer's ranking at the slice itself.
    pub hypothesis: HypothesisId<U>,
    pub changes: Vec<RankChange>,
}

/// The hypotheses of a ranking that held one reading, as it left the ranking after it.
pub struct Holders<'a, U> {
    ranking: Ranking<U>,
    /// The history that each hypothesis of the ranking holds, by position.
    held: &'a [usize],
    history: usize,
}

impl<U> Holders<'_, U> {
    /// Tells whether a hypothesis of the ranking held the reading.
    pub fn holds(&self, hypothesis: HypothesisId<U>) -> bool {
        self.ranking.position(hypothesis).is_some_and(|position| self.held[position] == self.history)
    }
}

/// Follows every reading that a layer's ranking holds for slice `at` through its later rankings,
/// in order of their rank at `at`. `reading_of` gives the reading of `at` held by a hypothesis stored at `at`,
/// and `departure` explains why a reading left the ranking at a slice (given its last rank and its holders).
///
/// Every hypothesis in a later ranking descends from one in the ranking before and keeps its reading of `at`,
/// so each ranking's readings are found from its parents' and no path is walked.
pub fn follow<U, R, T: PartialEq>(
    store: &Store<U, R>,
    at: usize,
    reading_of: impl Fn(HypothesisId<U>) -> T,
    departure: impl Fn(usize, usize, &Holders<U>) -> Option<Departure>,
) -> Vec<RankHistory<T, U>> {
    let Some(first) = store.ranking(at) else {
        return Vec::new();
    };
    let (mut histories, mut held) = first_histories(first, at, reading_of);
    let mut alive = histories.len();
    let mut previous = first;
    for (slice, ranking) in store.rankings().enumerate().skip(at + 1) {
        if alive <= 1 {
            break;
        }
        let held_by_parent = |hypothesis: HypothesisId<U>| {
            held[previous.position(store.get(hypothesis).parent).expect("a parent is in the ranking before")]
        };
        let next: Vec<usize> = ranking.iter().map(held_by_parent).collect();
        let ranks = best_ranks(&next, histories.len());
        for (id, history) in histories.iter_mut().enumerate() {
            let last = history.changes.last().expect("histories start with a rank");
            let Some(last_rank) = last.rank.filter(|_| last.rank != ranks[id]) else {
                continue;
            };
            let holders = Holders { ranking: previous, held: &held, history: id };
            let left_beam = ranks[id].is_none().then(|| departure(slice, last_rank, &holders));
            alive -= left_beam.is_some() as usize;
            history.changes.push(RankChange { slice, rank: ranks[id], left_beam: left_beam.flatten() });
        }
        held = next;
        previous = ranking;
    }
    histories
}

/// Starts a history for each distinct reading of the first ranking, at the rank of its best holder.
/// Returns the histories and the history that each hypothesis of the ranking holds.
fn first_histories<U, T: PartialEq>(
    ranking: Ranking<U>,
    at: usize,
    reading_of: impl Fn(HypothesisId<U>) -> T,
) -> (Vec<RankHistory<T, U>>, Vec<usize>) {
    let mut histories: Vec<RankHistory<T, U>> = Vec::new();
    let mut held: Vec<usize> = Vec::with_capacity(ranking.len());
    for (position, hypothesis) in ranking.iter().enumerate() {
        let reading = reading_of(hypothesis);
        let id = histories.iter().position(|h| h.reading == reading).unwrap_or_else(|| {
            let changes = vec![RankChange { slice: at, rank: Some(position + 1), left_beam: None }];
            histories.push(RankHistory { reading, hypothesis, changes });
            histories.len() - 1
        });
        held.push(id);
    }
    (histories, held)
}

/// Returns the rank of each history's best holder among `held`, the history held at each position.
fn best_ranks(held: &[usize], histories: usize) -> Vec<Option<usize>> {
    let mut ranks: Vec<Option<usize>> = vec![None; histories];
    for (position, &id) in held.iter().enumerate() {
        ranks[id].get_or_insert(position + 1);
    }
    ranks
}

#[cfg(test)]
#[path = "rank_history_tests.rs"]
mod tests;
