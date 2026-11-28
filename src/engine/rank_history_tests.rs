use super::*;
use crate::engine::hypothesis::Hypothesis;
use std::cell::Cell;

/// A layer whose unit is a letter, over three slices:
/// slice 0 holds a, b and a; slice 1 holds the b and then the first a extended;
/// slice 2 holds that a extended again.
fn store() -> Store<char, ()> {
    let hypothesis = |parent: u32, unit: char| Hypothesis {
        parent: HypothesisId::new(parent),
        previous: HypothesisId::NONE,
        reads: (),
        unit,
        score: 0.0,
    };
    let none = HypothesisId::<char>::NONE.0;
    let hypotheses = vec![
        hypothesis(none, 'a'),
        hypothesis(none, 'b'),
        hypothesis(none, 'a'),
        hypothesis(1, 'b'),
        hypothesis(0, 'a'),
        hypothesis(4, 'a'),
    ];
    Store { hypotheses, slice_starts: vec![0, 3, 5] }
}

/// The readings of slice 0, each the unit of the hypothesis stored at slice 0 that it descends from.
fn reading_of_slice_0(store: &Store<char, ()>) -> impl Fn(HypothesisId<char>) -> char + '_ {
    |hypothesis| store.get(hypothesis).unit
}

/// Each history's ranks, as (slice, rank) pairs.
fn ranks(
    departure: impl Fn(usize, usize, &Holders<char>) -> Option<Departure>,
) -> Vec<Vec<(usize, Option<usize>)>> {
    let store = store();
    let histories = follow(&store, 0, reading_of_slice_0(&store), departure);
    histories.iter().map(|h| h.changes.iter().map(|c| (c.slice, c.rank)).collect()).collect()
}

fn departure(last_rank: usize) -> Departure {
    Departure { last_rank, competing_rank: 0, score_gap: 0.0, cause: "test" }
}

#[test]
fn a_reading_held_twice_starts_at_the_rank_of_its_best_holder() {
    let store = store();
    let histories = follow(&store, 0, reading_of_slice_0(&store), |_, _, _| None);
    assert_eq!(histories.iter().map(|h| h.reading).collect::<Vec<_>>(), vec!['a', 'b']);
}

#[test]
fn a_history_follows_its_holders_by_their_parents() {
    assert_eq!(ranks(|_, _, _| None)[0], vec![(0, Some(1)), (1, Some(2)), (2, Some(1))]);
}

#[test]
fn a_history_ends_when_no_hypothesis_holds_the_reading() {
    assert_eq!(ranks(|_, _, _| None)[1], vec![(0, Some(2)), (1, Some(1)), (2, None)]);
}

#[test]
fn a_reading_that_leaves_the_ranking_is_explained_at_its_last_rank() {
    let store = store();
    let histories =
        follow(&store, 0, reading_of_slice_0(&store), |_, last_rank, _| Some(departure(last_rank)));
    assert_eq!(histories[1].changes[2].left_beam, Some(departure(1)));
}

#[test]
fn the_holders_of_a_departing_reading_are_its_hypotheses_in_the_ranking_before() {
    let held = Cell::new((false, false));
    ranks(|_, _, holders| {
        held.set((holders.holds(HypothesisId::new(3)), holders.holds(HypothesisId::new(4))));
        None
    });
    assert_eq!(held.get(), (true, false));
}

#[test]
fn a_slice_the_store_does_not_hold_has_no_histories() {
    let store = store();
    assert!(follow(&store, 3, reading_of_slice_0(&store), |_, _, _| None).is_empty());
}
