//! Pruning: which of the hypotheses proposed at a slice the beam keeps,
//! and records of those it drops, with the reason.
//!
//! A hypothesis that is kept always yields a continuation at the next slice,
//! so it can only leave the search by being pruned.
//! A reading that the beam holds for some slice therefore leaves it exactly
//! when pruning drops the last hypotheses that hold it.

use super::model::{KeyHypothesisId, KeySegment};
use super::pending::{Pending, PendingChordHypothesisId, PendingKeyHypothesis};
use super::rule::Layer;
use super::search::Config;
use serde::Serialize;
use std::collections::HashSet;
use std::hash::Hash;

/// Why pruning dropped a hypothesis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PruneCause {
    /// Keeping the hypothesis would take a layer past its width (see `Config::width`):
    /// for the top layer, the beam was already full of better hypotheses;
    /// for a lower layer, the hypotheses kept already read as many of its hypotheses as it may hold,
    /// and this hypothesis reads another.
    Width { layer: Layer },
}

impl PruneCause {
    pub fn name(self) -> &'static str {
        match self {
            PruneCause::Width { layer: Layer::Key } => "beam_width",
            PruneCause::Width { layer: Layer::Chord } => "chord_hypothesis_width",
        }
    }
}

/// A hypothesis dropped by pruning at the slice it reached.
#[derive(Clone, Copy, Debug)]
pub struct Pruned {
    /// The hypothesis that it extended or opened after.
    pub parent: KeyHypothesisId,
    /// The last hypothesis of the segment before its last segment, or `KeyHypothesisId::NONE`.
    pub previous: KeyHypothesisId,
    /// The score of its complete interpretation.
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
    /// `beam_width` or `chord_hypothesis_width`.
    pub cause: &'static str,
}

/// Keeps the best of the pending hypotheses, in order,
/// skipping any that would take the beam past its limit of chord hypotheses. The sort is stable,
/// so ties are broken by the order in which the hypotheses were proposed.
/// Also returns the dropped hypotheses when `Config::record_pruning` is set.
pub fn prune(pending: &Pending, config: &Config) -> (Vec<PendingKeyHypothesis>, Vec<Pruned>) {
    let mut ranked: Vec<(f32, PendingKeyHypothesis)> =
        pending.key.hypotheses.iter().map(|h| (pending.score(h), *h)).collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut kept = Kept::new(config);
    let mut dropped = Vec::new();
    for (rank, (score, hypothesis)) in ranked.into_iter().enumerate() {
        match kept.judge(&hypothesis, config) {
            Ok(()) => kept.add(hypothesis),
            Err(PruneCause::Width { layer: Layer::TOP }) if !config.record_pruning => break,
            Err(cause) if config.record_pruning => {
                dropped.push(record(pending, &hypothesis, score, rank + 1, cause))
            }
            Err(_) => {}
        }
    }
    (kept.key_hypotheses, dropped)
}

/// The hypotheses kept so far, best first.
struct Kept {
    key_hypotheses: Vec<PendingKeyHypothesis>,
    /// The chord hypotheses that the kept hypotheses read.
    chord_hypotheses: Held<PendingChordHypothesisId>,
}

impl Kept {
    fn new(config: &Config) -> Kept {
        let key_hypotheses = Vec::with_capacity(config.width(Layer::Key));
        Kept { key_hypotheses, chord_hypotheses: Held::new(config.width(Layer::Chord)) }
    }

    /// Returns why the hypothesis cannot join the beam, if it cannot.
    fn judge(&self, hypothesis: &PendingKeyHypothesis, config: &Config) -> Result<(), PruneCause> {
        if self.key_hypotheses.len() == config.width(Layer::Key) {
            return Err(PruneCause::Width { layer: Layer::Key });
        }
        if !self.chord_hypotheses.admits(hypothesis.reads) {
            return Err(PruneCause::Width { layer: Layer::Chord });
        }
        Ok(())
    }

    fn add(&mut self, hypothesis: PendingKeyHypothesis) {
        self.chord_hypotheses.add(hypothesis.reads);
        self.key_hypotheses.push(hypothesis);
    }
}

/// Records a dropped hypothesis, given the score of its complete interpretation.
fn record(
    pending: &Pending,
    hypothesis: &PendingKeyHypothesis,
    score: f32,
    rank: usize,
    cause: PruneCause,
) -> Pruned {
    let (parent, previous) = (hypothesis.parent, hypothesis.previous);
    Pruned { parent, previous, score, segment: pending.segment(hypothesis), rank, cause }
}

/// The hypotheses of a lower layer that the kept hypotheses read, and how many the layer may hold.
struct Held<Id> {
    ids: HashSet<Id>,
    /// Zero sets no limit.
    width: usize,
}

impl<Id: Copy + Eq + Hash> Held<Id> {
    fn new(width: usize) -> Held<Id> {
        Held { ids: HashSet::new(), width }
    }

    /// Reports whether a kept hypothesis may read the hypothesis without exceeding the layer's width.
    fn admits(&self, id: Id) -> bool {
        self.width == 0 || self.ids.len() < self.width || self.ids.contains(&id)
    }

    fn add(&mut self, id: Id) {
        self.ids.insert(id);
    }
}

#[cfg(test)]
#[path = "pruning_tests.rs"]
mod tests;
