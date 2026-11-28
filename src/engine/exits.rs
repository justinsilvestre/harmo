//! What leaving each hypothesis in the beam scores, and which of them a new segment opens after.

use super::chord_hooks::{ChordCloseContext, ChordRivalContext, ChordTransitionContext, Rival};
use super::key_contexts::{KeyCloseContext, KeyDepartureContext, KeyTransitionContext};
use super::model::{ChordHypothesisId, ChordSegment, KeyHypothesisId, KeySegment};
use super::moment::Moment;
use super::proposals::{LayerScores, Opening, ProposedChord};
use super::ruleset::RuleSet;
use crate::score::slicing::Slice;
use std::collections::HashMap;

/// What leaving a chord hypothesis on the moment's slice scores in the chord layer.
pub struct ChordExit {
    /// The chord hypothesis, or `ChordHypothesisId::NONE` at the start of the piece.
    pub chord_hypothesis: ChordHypothesisId,
    /// The chord hypothesis's score before it is left.
    pub score: f32,
    /// The chord close rules' score of the chord hypothesis's last segment.
    pub close: f32,
    /// The chord transition rules' score of opening each proposed chord after the chord hypothesis,
    /// in the order of `Proposals::chords`.
    pub transitions: Vec<f32>,
    /// The chord rival hooks' groups for the last segment, in rule order.
    pub rivals: Vec<Option<Rival>>,
}

/// What leaving a hypothesis on the moment's slice scores in the key layer.
/// Its departure scores count only for new readings in another key.
pub struct KeyExit {
    /// The hypothesis, or `KeyHypothesisId::NONE` at the start of the piece.
    pub hypothesis: KeyHypothesisId,
    /// The position in `Exits::chords` of the exit of the chord hypothesis that the hypothesis interprets.
    pub chord_exit: usize,
    /// The hypothesis's own score before it is left, in the key layer alone.
    pub score: f32,
    /// The hypothesis's last segment, or `None` at the start of the piece.
    pub segment: Option<KeySegment>,
    /// The key close rules' score of the last segment.
    pub close: f32,
    pub departures: Vec<f32>,
}

/// The exits of every hypothesis in the beam, and one exit for each chord hypothesis that they interpret.
pub struct Exits {
    pub chords: Vec<ChordExit>,
    pub keys: Vec<KeyExit>,
}

/// A hypothesis that a new reading opens after.
#[derive(Clone, Copy)]
pub struct Predecessor {
    /// The position of the hypothesis's exit in `Exits::keys`.
    pub exit: usize,
    /// The score of the path up to the new segment in each layer, closing and transition scores included.
    pub score: LayerScores,
}

impl Exits {
    /// Returns the exits of the hypotheses in the beam,
    /// or the exit from the start of the piece when the beam is empty.
    pub fn of(
        moment: &Moment,
        beam: &[KeyHypothesisId],
        proposed: &[ProposedChord],
        rules: &RuleSet,
    ) -> Exits {
        if beam.is_empty() {
            return Exits::from_start(moment, proposed, rules);
        }
        let (mut chords, mut keys) = (Vec::new(), Vec::with_capacity(beam.len()));
        let mut positions: HashMap<ChordHypothesisId, usize> = HashMap::new();
        for &hypothesis in beam {
            let chord_hypothesis = moment.layers.key.get(hypothesis).reads;
            let chord_exit = *positions.entry(chord_hypothesis).or_insert_with(|| {
                chords.push(chord_exit(moment, chord_hypothesis, proposed, rules));
                chords.len() - 1
            });
            keys.push(key_exit(moment, hypothesis, chord_exit, rules));
        }
        Exits { chords, keys }
    }

    fn from_start(moment: &Moment, proposed: &[ProposedChord], rules: &RuleSet) -> Exits {
        let transitions = chord_transitions(moment, ChordHypothesisId::NONE, proposed, rules);
        let chord = ChordExit {
            chord_hypothesis: ChordHypothesisId::NONE,
            score: 0.0,
            close: 0.0,
            transitions,
            rivals: Vec::new(),
        };
        let departures = Vec::new();
        let key = KeyExit {
            hypothesis: KeyHypothesisId::NONE,
            chord_exit: 0,
            score: 0.0,
            segment: None,
            close: 0.0,
            departures,
        };
        Exits { chords: vec![chord], keys: vec![key] }
    }

    /// Finds the hypotheses that a new reading opens after: the one that leads into it best and,
    /// for each rival hook that puts that hypothesis's last segment in a group,
    /// the best one whose last segment is another member of the group. Returns them best first.
    pub fn predecessors(&self, opening: &Opening, moment: &Moment, rules: &RuleSet) -> Vec<Predecessor> {
        let lead_in = |position: usize| self.lead_in(position, opening, moment, rules);
        let scored: Vec<Predecessor> = (0..self.keys.len()).map(lead_in).collect();
        let Some(best) = best_of(&scored, 0..scored.len()) else {
            return vec![];
        };
        let rival = |i: usize, rule: usize| self.rivals_of(scored[i].exit)[rule];
        let mut found = vec![best];
        for (rule, best_rival) in self.rivals_of(scored[best].exit).iter().enumerate() {
            let Some(best_rival) = *best_rival else {
                continue;
            };
            let is_rival = |&i: &usize| rival(i, rule).is_some_and(|r| r.is_rival_of(best_rival));
            if let Some(rival) =
                best_of(&scored, (0..scored.len()).filter(is_rival)).filter(|r| !found.contains(r))
            {
                found.push(rival);
            }
        }
        found.into_iter().map(|i| scored[i]).collect()
    }

    /// Returns the rival groups of the last segment of the chord hypothesis that a hypothesis reads.
    fn rivals_of(&self, position: usize) -> &[Option<Rival>] {
        &self.chords[self.keys[position].chord_exit].rivals
    }

    /// Scores opening a new reading after the hypothesis of one exit.
    fn lead_in(&self, position: usize, opening: &Opening, moment: &Moment, rules: &RuleSet) -> Predecessor {
        let exit = &self.keys[position];
        let chord_exit = &self.chords[exit.chord_exit];
        let chord_transition = chord_exit.transitions[opening.chord_index];
        let (slice, history) = (moment.slice(), moment.history(exit.hypothesis));
        let context =
            KeyTransitionContext { previous: exit.segment.as_ref(), next: &opening.reading, slice, history };
        let chord = chord_exit.score + chord_exit.close + chord_transition;
        let key = exit.score + exit.close + rules.key.score_transition(&context, &exit.departures);
        Predecessor { exit: position, score: LayerScores { chord, key } }
    }
}

/// Returns the best of the predecessors at the given positions, the last of any that tie.
fn best_of(scored: &[Predecessor], positions: impl Iterator<Item = usize>) -> Option<usize> {
    positions.max_by(|&a, &b| scored[a].score.total().total_cmp(&scored[b].score.total()))
}

fn chord_exit(
    moment: &Moment,
    chord_hypothesis: ChordHypothesisId,
    proposed: &[ProposedChord],
    rules: &RuleSet,
) -> ChordExit {
    let heard = moment.layers.chord.get(chord_hypothesis);
    let close = chord_close(moment.slices, &heard.unit, moment.index, rules);
    let transitions = chord_transitions(moment, chord_hypothesis, proposed, rules);
    let start = heard.unit.start as usize;
    let rival = ChordRivalContext {
        segment: &heard.unit,
        slices: &moment.slices[start..moment.index],
        previous_slice: start.checked_sub(1).map(|i| &moment.slices[i]),
    };
    let rivals = rules.chord.rival_groups(&rival);
    ChordExit { chord_hypothesis, score: heard.score, close, transitions, rivals }
}

/// Scores, in the chord layer, opening each proposed chord after a chord hypothesis,
/// or after nothing at the start of the piece.
fn chord_transitions(
    moment: &Moment,
    chord_hypothesis: ChordHypothesisId,
    proposed: &[ProposedChord],
    rules: &RuleSet,
) -> Vec<f32> {
    let previous = (chord_hypothesis != ChordHypothesisId::NONE)
        .then(|| &moment.layers.chord.get(chord_hypothesis).unit);
    let (slice, history) = (moment.slice(), moment.history(chord_hypothesis));
    let score = |next: &ProposedChord| {
        let context = ChordTransitionContext { previous, next: next.chord, slice, history };
        rules.chord.score_transition(&context)
    };
    proposed.iter().map(score).collect()
}

fn key_exit(moment: &Moment, hypothesis: KeyHypothesisId, chord_exit: usize, rules: &RuleSet) -> KeyExit {
    let segment = moment.layers.segment(hypothesis);
    let departure = KeyDepartureContext {
        previous: &segment,
        slice: moment.slice(),
        history: moment.history(hypothesis),
    };
    KeyExit {
        hypothesis,
        chord_exit,
        score: moment.layers.key.get(hypothesis).score,
        segment: Some(segment),
        close: key_close(moment.slices, &segment, moment.index, rules),
        departures: rules.key.score_departures(&departure),
    }
}

/// Returns the chord close rules' score of a segment whose last slice is the one before `end`.
pub fn chord_close(slices: &[Slice], segment: &ChordSegment, end: usize, rules: &RuleSet) -> f32 {
    let (span, next_slice) = (&slices[segment.start as usize..end], slices.get(end));
    rules.chord.score_close(&ChordCloseContext { segment, slices: span, next_slice })
}

/// Returns the key close rules' score of a segment whose last slice is the one before `end`.
pub fn key_close(slices: &[Slice], segment: &KeySegment, end: usize, rules: &RuleSet) -> f32 {
    let (span, next_slice) = (&slices[segment.heard.start as usize..end], slices.get(end));
    rules.key.score_close(&KeyCloseContext { segment, slices: span, next_slice })
}

#[cfg(test)]
#[path = "exits_tests.rs"]
mod tests;
