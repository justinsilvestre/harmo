//! Rules about the raised fourth degree as evidence that the music has moved to the dominant key.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::key_contexts::KeyTransitionContext;
use crate::engine::key_hooks::KeyHook;
use crate::engine::model::Reading;
use crate::engine::rule::{Citation, Hook, Param, Rule, Topic};
use crate::pitch::Tpc;
use crate::score::slicing::Slice;
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[Rule {
    id: "key.raised_fourth",
    topic: Topic::Key,
    summary: "Prefer not to hear a dominant triad moving to its tonic as V–I of a key whose fourth \
              degree was last heard raised. The raised fourth degree is the leading tone of the \
              dominant key, so after it the same two chords are better heard as I–IV in that key, \
              as when G and C follow an F♯ in C major. The charge is waived when the key's own \
              fourth degree sounds within two quarter notes of the tonic's arrival, which shows the \
              old key still in force. A dominant seventh is not charged, since its seventh is the \
              fourth degree itself.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 299 (the consistent use of accidentals that distinguish the second key from the first as evidence of a modulation)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 304 (the point of modulation is often a chord with an accidental not found in the first key; F♯° functions more naturally in G than in C, and the C chord before it is I in C and IV in G)",
        },
    ],
    default_weight: 0.5,
    params: &[LOOK_AHEAD],
    hook: Hook::Key(KeyHook::Transition(raised_fourth)),
}];

/// How far past the tonic's arrival the key's own fourth degree may sound to waive the charge.
const LOOK_AHEAD: Param = Param {
    index: 0,
    name: "look_ahead",
    default: 2.0,
    unit: "quarter notes",
    provenance: "limit: the listener may hear at most two quarter notes ahead (README, How it works); measured: 7e35101, round 6 (no waiver: tune RN +0.09 against +0.11 at weight 0.5); measured: sweeps/round7-constants.json, round 7 (0 / 1: tune RN -0.01 / -0.00; kept)",
};

const EPSILON: f64 = 1e-6;

fn raised_fourth(context: &KeyTransitionContext, params: &[f64]) -> f32 {
    let Some(previous) = context.previous else { return 0.0 };
    let key = previous.function.key;
    if !is_dominant_triad_to_tonic(&previous.reading(), context.next) {
        return 0.0;
    }
    let slices = context.history.slices;
    let opening = slices.partition_point(|s| s.start < context.slice.start - EPSILON);
    let fourth = key.tonic - 1;
    let horizon = context.slice.start + LOOK_AHEAD.read(params) - EPSILON;
    let ahead = slices[opening..].iter().take_while(|s| s.start < horizon);
    if ahead.into_iter().any(|s| sounds(s, fourth)) {
        return 0.0;
    }
    if was_last_raised(&slices[..opening], fourth) { -1.0 } else { 0.0 }
}

/// Reports whether a reading is the major dominant triad of its key and the next is that key's tonic.
fn is_dominant_triad_to_tonic(dominant: &Reading, next: &Reading) -> bool {
    let key = dominant.function.key;
    let is_dominant = dominant.function.tonicized.is_none()
        && dominant.chord.root == key.tonic + 1
        && dominant.chord.quality == Quality::Major;
    is_dominant
        && next.function.key == key
        && next.function.tonicized.is_none()
        && next.chord.root == key.tonic
}

/// Reports whether, of a fourth degree and its raised form, the raised form sounded last.
fn was_last_raised(slices: &[Slice], fourth: Tpc) -> bool {
    let raised = fourth + 7;
    let last = slices.iter().rev().find(|s| sounds(s, fourth) || sounds(s, raised));
    last.is_some_and(|s| sounds(s, raised) && !sounds(s, fourth))
}

fn sounds(slice: &Slice, tpc: Tpc) -> bool {
    slice.notes.iter().any(|note| note.tpc == tpc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::key_contexts::KeyHistory;
    use crate::engine::model::Function;
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::Chord;
    use crate::theory::key::{Key, Mode};

    fn tpc(name: &str) -> i32 {
        tpc_from_name(name).unwrap()
    }

    fn reading(root: &str, quality: Quality) -> Reading {
        let key = Key::new(tpc("C"), Mode::Major);
        Reading { chord: Chord::new(tpc(root), quality), function: Function::diatonic(key) }
    }

    /// A one-beat slice starting at `start` that sounds the named pitches.
    fn slice(start: f64, names: &[&str]) -> Slice {
        let note = |name: &&str| SliceNote { tpc: tpc(name), midi: 60, attacked: true, onset_level: 2 };
        let notes = names.iter().map(note).collect();
        Slice { start, duration: 1.0, notes, measure: 0, metric_level: 2, phrase: 0, key_signature: None }
    }

    /// Scores a move from the given dominant reading, over the slices before the tonic,
    /// to C major arriving on a slice with the given pitches, followed by one more slice.
    fn score(dominant: Quality, before: &[&[&str]], tonic: &[&str], after: &[&str]) -> f32 {
        let mut slices: Vec<Slice> =
            before.iter().enumerate().map(|(i, names)| slice(i as f64, names)).collect();
        let opening = slices.len();
        slices.push(slice(opening as f64, tonic));
        slices.push(slice(opening as f64 + 1.0, after));
        let previous = key_segment(reading("G", dominant));
        let next = reading("C", Quality::Major);
        let history = KeyHistory::empty(&slices);
        raised_fourth(
            &KeyTransitionContext {
                previous: Some(&previous),
                next: &next,
                slice: &slices[opening],
                history,
            },
            &RULES[0].default_params(),
        )
    }

    #[test]
    fn a_dominant_after_the_raised_fourth_is_charged() {
        assert_eq!(
            score(Quality::Major, &[&["D", "F#", "A"], &["G", "B", "D"]], &["C", "E", "G"], &["C"]),
            -1.0
        );
    }

    #[test]
    fn a_dominant_after_the_fourth_degree_is_not_charged() {
        assert_eq!(score(Quality::Major, &[&["F#"], &["F", "G", "B"]], &["C", "E", "G"], &["C"]), 0.0);
    }

    #[test]
    fn the_fourth_degree_heard_right_after_the_tonic_waives_the_charge() {
        assert_eq!(
            score(Quality::Major, &[&["D", "F#", "A"], &["G", "B", "D"]], &["C", "E", "G"], &["F"]),
            0.0
        );
    }

    #[test]
    fn a_dominant_seventh_is_not_charged() {
        assert_eq!(
            score(Quality::DominantSeventh, &[&["D", "F#", "A"], &["G", "B", "D"]], &["C", "E", "G"], &["C"]),
            0.0
        );
    }

    #[test]
    fn a_dominant_with_neither_form_of_the_fourth_heard_is_not_charged() {
        assert_eq!(score(Quality::Major, &[&["G", "B", "D"]], &["C", "E", "G"], &["C"]), 0.0);
    }
}
