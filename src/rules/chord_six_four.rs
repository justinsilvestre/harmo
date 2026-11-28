//! Rules about triads in second inversion (six-four chords).

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::chord_hooks::{ChordCloseContext, ChordHook};
use crate::engine::model::ChordSegment;
use crate::engine::rule::{Citation, Hook, Rule, Topic};
use crate::pitch::pitch_class;
use crate::score::slicing::{Slice, SliceNote};
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[Rule {
    id: "chord.six_four",
    topic: Topic::Chord,
    summary: "Avoid second-inversion triads, in proportion to how long the fifth is the lowest note. \
              A six-four chord costs nothing when its bass is held or repeated into the next chord \
              (cadential and pedal six-fours) or moves on by step (passing six-fours).",
    citations: &[
        Citation { source: KOSTKA_PAYNE_ALMEN_2023, locator: "p. 134 (six-fours from bass arpeggiation)" },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 135–141 (cadential, passing and pedal six-four chords)",
        },
    ],
    default_weight: 0.5,
    params: &[],
    hook: Hook::Chord(ChordHook::Close(six_four)),
}];

/// The largest bass motion, in semitones, that counts as a step.
const STEP: i32 = 2;

fn six_four(context: &ChordCloseContext, _: &[f64]) -> f32 {
    let segment = context.segment;
    let Some((midi, tpc)) = segment.bass.note.filter(|_| is_six_four(segment)) else {
        return 0.0;
    };
    let next_bass = context.next_slice.and_then(Slice::bass);
    let is_held = |next: &SliceNote| pitch_class(next.tpc) == pitch_class(tpc);
    let is_step = |next: &SliceNote| (next.midi - midi).abs() <= STEP;
    if next_bass.is_some_and(|next| is_held(next) || is_step(next)) {
        return 0.0;
    }
    let has_fifth_in_bass =
        |slice: &&Slice| slice.bass().is_some_and(|n| pitch_class(n.tpc) == pitch_class(tpc));
    let heard: f64 = context.slices.iter().filter(has_fifth_in_bass).map(|s| s.duration).sum();
    -(heard / segment.duration.max(1e-9)) as f32
}

fn is_six_four(segment: &ChordSegment) -> bool {
    let chord = segment.chord;
    let is_triad =
        matches!(chord.quality, Quality::Major | Quality::Minor | Quality::Diminished | Quality::Augmented);
    is_triad && chord.member_index(segment.bass_tpc()) == Some(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::ChordSegment;
    use crate::engine::test_layers::chord_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Chord;

    fn slice(notes: &[(i32, &str)]) -> Slice {
        let note = |&(midi, name): &(i32, &str)| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi,
            attacked: true,
            onset_level: 0,
        };
        Slice {
            start: 0.0,
            duration: 1.0,
            notes: notes.iter().map(note).collect(),
            measure: 0,
            metric_level: 4,
            phrase: 0,
            key_signature: None,
        }
    }

    /// Scores a C major tonic six-four over G3 followed by the given slice.
    fn score_before(next: &Slice) -> f32 {
        let chord = Chord::new(tpc_from_name("C").unwrap(), Quality::Major);
        let bass = Bass { note: Some((55, tpc_from_name("G").unwrap())), ..Bass::default() };
        let segment = ChordSegment { members_heard: 0b111, bass, ..chord_segment(chord) };
        let slices = [slice(&[(55, "G"), (60, "C"), (64, "E")])];
        six_four(&ChordCloseContext { segment: &segment, slices: &slices, next_slice: Some(next) }, &[])
    }

    #[test]
    fn held_bass_costs_nothing() {
        assert_eq!(score_before(&slice(&[(55, "G"), (59, "B"), (62, "D")])), 0.0);
    }

    #[test]
    fn stepwise_bass_costs_nothing() {
        assert_eq!(score_before(&slice(&[(53, "F"), (62, "D"), (65, "F")])), 0.0);
    }

    #[test]
    fn leaping_bass_costs_the_full_penalty() {
        assert_eq!(score_before(&slice(&[(48, "C"), (64, "E"), (67, "G")])), -1.0);
    }
}
