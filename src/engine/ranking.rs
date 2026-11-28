//! A layer's ranking at one slice: the hypotheses it stored at that slice, best first.

use super::hypothesis::HypothesisId;
use std::marker::PhantomData;

/// The hypotheses of the layer whose unit is `U` that were stored at one slice, best first.
/// Each is ranked by the best complete interpretation that reads it,
/// so the key layer's ranking at a slice is the beam after it.
pub struct Ranking<U> {
    start: u32,
    end: u32,
    unit: PhantomData<fn() -> U>,
}

impl<U> Ranking<U> {
    /// A ranking that holds no hypotheses.
    pub const EMPTY: Ranking<U> = Ranking::new(0, 0);

    /// The ranking of the hypotheses with indices from `start` up to but not including `end`.
    pub const fn new(start: u32, end: u32) -> Ranking<U> {
        Ranking { start, end, unit: PhantomData }
    }

    pub fn len(&self) -> usize {
        (self.end - self.start) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns the hypothesis at a position (0 = best).
    pub fn get(&self, position: usize) -> HypothesisId<U> {
        assert!(position < self.len(), "position {position} is outside a ranking of {}", self.len());
        HypothesisId::new(self.start + position as u32)
    }

    /// Returns the best hypothesis, if any.
    pub fn first(&self) -> Option<HypothesisId<U>> {
        (!self.is_empty()).then(|| HypothesisId::new(self.start))
    }

    /// Returns the position of a hypothesis (0 = best), or `None` if the ranking does not hold it.
    pub fn position(&self, hypothesis: HypothesisId<U>) -> Option<usize> {
        (self.start..self.end).contains(&hypothesis.0).then(|| (hypothesis.0 - self.start) as usize)
    }

    /// Returns the hypotheses, best first.
    pub fn iter(&self) -> impl Iterator<Item = HypothesisId<U>> + use<U> {
        (self.start..self.end).map(HypothesisId::new)
    }
}

// These are written out because deriving them would require them of `U` as well.
impl<U> Clone for Ranking<U> {
    fn clone(&self) -> Ranking<U> {
        *self
    }
}

impl<U> Copy for Ranking<U> {}

impl<U> std::fmt::Debug for Ranking<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Ranking({}..{})", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranking() -> Ranking<()> {
        Ranking::new(3, 6)
    }

    #[test]
    fn a_ranking_holds_its_hypotheses() {
        assert_eq!(ranking().len(), 3);
    }

    #[test]
    fn a_ranking_gives_its_hypotheses_by_position() {
        assert_eq!(ranking().get(1), HypothesisId::new(4));
    }

    #[test]
    fn a_ranking_begins_with_its_best_hypothesis() {
        assert_eq!(ranking().first(), Some(HypothesisId::new(3)));
    }

    #[test]
    fn an_empty_ranking_has_no_best_hypothesis() {
        assert_eq!(Ranking::<()>::new(3, 3).first(), None);
    }

    #[test]
    fn a_ranking_finds_the_position_of_a_hypothesis() {
        assert_eq!(ranking().position(HypothesisId::new(5)), Some(2));
    }

    #[test]
    fn a_ranking_does_not_find_a_hypothesis_stored_at_another_slice() {
        assert_eq!(ranking().position(HypothesisId::new(6)), None);
    }

    #[test]
    fn a_ranking_lists_its_hypotheses_best_first() {
        assert_eq!(ranking().iter().map(|h| h.0).collect::<Vec<_>>(), vec![3, 4, 5]);
    }
}
