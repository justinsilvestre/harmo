//! What leaving each hypothesis in the beam scores, and which of them a new segment opens after.

use super::chord_hooks::{ChordCloseContext, ChordTransitionContext};
use super::key_contexts::{KeyCloseContext, KeyDepartureContext, KeyTransitionContext, Rival, RivalContext};
use super::model::{ChordHypothesisId, ChordSegment, KeySegment, NONE};
use super::moment::Moment;
use super::proposals::{Opening, ProposedChord};
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
}

/// What leaving a hypothesis on the moment's slice scores in the key layer.
/// Its departure scores count only for new readings in another key.
pub struct KeyExit {
    /// The hypothesis, or `NONE` at the start of the piece.
    pub hypothesis: u32,
    /// The position in `Exits::chords` of the exit of the chord hypothesis that the hypothesis interprets.
    pub chord_exit: usize,
    /// The hypothesis's score before it is left.
    pub score: f32,
    /// The hypothesis's last segment, or `None` at the start of the piece.
    pub segment: Option<KeySegment>,
    /// The key close rules' score of the last segment.
    pub close: f32,
    pub departures: Vec<f32>,
    /// The rival hooks' groups for the last segment, in rule order.
    pub rivals: Vec<Option<Rival>>,
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
    /// The score of the path up to the new segment, closing and transition scores included.
    pub score: f32,
}

impl Exits {
    /// Returns the exits of the hypotheses in the beam,
    /// or the exit from the start of the piece when the beam is empty.
    pub fn of(moment: &Moment, beam: &[u32], proposed: &[ProposedChord], rules: &RuleSet) -> Exits {
        if beam.is_empty() {
            return Exits::from_start(moment, proposed, rules);
        }
        let (mut chords, mut keys) = (Vec::new(), Vec::with_capacity(beam.len()));
        let mut positions: HashMap<ChordHypothesisId, usize> = HashMap::new();
        for &hypothesis in beam {
            let chord_hypothesis = moment.layers.key_hypotheses[hypothesis as usize].chord_hypothesis;
            let chord_exit = *positions.entry(chord_hypothesis).or_insert_with(|| {
                chords.push(chord_exit(moment, chord_hypothesis, proposed, rules));
                chords.len() - 1
            });
            keys.push(key_exit(moment, hypothesis, chord_exit, rules));
        }
        Exits { chords, keys }
    }

    fn from_start(moment: &Moment, proposed: &[ProposedChord], rules: &RuleSet) -> Exits {
        let transitions = chord_transitions(moment, None, proposed, rules);
        let chord =
            ChordExit { chord_hypothesis: ChordHypothesisId::NONE, score: 0.0, close: 0.0, transitions };
        let (departures, rivals) = (Vec::new(), Vec::new());
        let key = KeyExit {
            hypothesis: NONE,
            chord_exit: 0,
            score: 0.0,
            segment: None,
            close: 0.0,
            departures,
            rivals,
        };
        Exits { chords: vec![chord], keys: vec![key] }
    }

    /// Finds the hypotheses that a new reading opens after: the one that leads into it best and,
    /// for each rival hook that puts that hypothesis's last segment in a group,
    /// the best one whose last segment is another member of the group. Returns them best first.
    pub fn predecessors(&self, opening: &Opening, moment: &Moment, rules: &RuleSet) -> Vec<Predecessor> {
        let lead_in = |position: usize| self.lead_in(position, opening, moment, rules);
        let scored: Vec<Predecessor> = (0..self.keys.len()).filter_map(lead_in).collect();
        let Some(best) = best_of(&scored, 0..scored.len()) else {
            return vec![];
        };
        let exit_of = |i: usize| &self.keys[scored[i].exit];
        let mut found = vec![best];
        for (rule, best_rival) in exit_of(best).rivals.iter().enumerate() {
            let Some(best_rival) = *best_rival else {
                continue;
            };
            let is_rival = |&i: &usize| exit_of(i).rivals[rule].is_some_and(|r| r.is_rival_of(best_rival));
            if let Some(rival) =
                best_of(&scored, (0..scored.len()).filter(is_rival)).filter(|r| !found.contains(r))
            {
                found.push(rival);
            }
        }
        found.into_iter().map(|i| scored[i]).collect()
    }

    /// Scores opening a new reading after the hypothesis of one exit,
    /// or returns `None` if an admit rule rules the transition out.
    fn lead_in(
        &self,
        position: usize,
        opening: &Opening,
        moment: &Moment,
        rules: &RuleSet,
    ) -> Option<Predecessor> {
        let exit = &self.keys[position];
        let chord_exit = &self.chords[exit.chord_exit];
        let chord_transition = chord_exit.transitions[opening.chord_index];
        let (slice, history) = (moment.slice(), moment.history(exit.hypothesis));
        let context =
            KeyTransitionContext { previous: exit.segment.as_ref(), next: &opening.reading, slice, history };
        if !rules.key.admits(&context) {
            return None;
        }
        let transition = chord_transition + rules.key.score_transition(&context, &exit.departures);
        let score = exit.score + (chord_exit.close + exit.close) + transition;
        Some(Predecessor { exit: position, score })
    }
}

/// Returns the best of the predecessors at the given positions, the last of any that tie.
fn best_of(scored: &[Predecessor], positions: impl Iterator<Item = usize>) -> Option<usize> {
    positions.max_by(|&a, &b| scored[a].score.total_cmp(&scored[b].score))
}

fn chord_exit(
    moment: &Moment,
    chord_hypothesis: ChordHypothesisId,
    proposed: &[ProposedChord],
    rules: &RuleSet,
) -> ChordExit {
    let heard = moment.layers.chord_hypothesis(chord_hypothesis);
    let close = chord_close(moment.slices, &heard.segment, moment.index, rules);
    let transitions = chord_transitions(moment, Some(&heard.segment), proposed, rules);
    ChordExit { chord_hypothesis, score: heard.score, close, transitions }
}

/// Scores, in the chord layer, opening each proposed chord after a segment,
/// or after nothing at the start of the piece.
fn chord_transitions(
    moment: &Moment,
    previous: Option<&ChordSegment>,
    proposed: &[ProposedChord],
    rules: &RuleSet,
) -> Vec<f32> {
    let (slice, slices) = (moment.slice(), moment.slices);
    let score = |next: &ProposedChord| {
        let context = ChordTransitionContext { previous, next: next.chord, slice, slices };
        rules.chord.score_transition(&context)
    };
    proposed.iter().map(score).collect()
}

fn key_exit(moment: &Moment, hypothesis: u32, chord_exit: usize, rules: &RuleSet) -> KeyExit {
    let segment = moment.layers.segment(hypothesis);
    let departure = KeyDepartureContext {
        previous: &segment,
        slice: moment.slice(),
        history: moment.history(hypothesis),
    };
    let start = segment.heard.start as usize;
    let rival = RivalContext {
        segment: &segment,
        slices: &moment.slices[start..moment.index],
        previous_slice: start.checked_sub(1).map(|i| &moment.slices[i]),
    };
    KeyExit {
        hypothesis,
        chord_exit,
        score: moment.layers.key_hypotheses[hypothesis as usize].score,
        segment: Some(segment),
        close: key_close(moment.slices, &segment, moment.index, rules),
        departures: rules.key.score_departures(&departure),
        rivals: rules.key.rival_groups(&rival),
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
