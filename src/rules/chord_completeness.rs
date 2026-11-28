//! Rules about which chord members a segment must contain.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, SCHULTZ_ET_AL_2018};
use crate::engine::chord_hooks::{ChordCloseContext, ChordHook};
use crate::engine::rule::{Citation, Hook, Param, Rule, Topic};
use crate::pitch::Tpc;
use crate::score::slicing::{Slice, SliceNote};

pub const RULES: &[Rule] = &[
    Rule {
        id: "chord.completeness",
        topic: Topic::Chord,
        summary: "Prefer segments in which the root and third are heard. The fifth is often omitted; \
              the root and third rarely are.",
        citations: &[
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 82 (complete and incomplete triads in root position)",
            },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "pp. 121–122 (incomplete triads in first inversion)",
            },
        ],
        default_weight: 1.5,
        params: &[MISSING_THIRD],
        hook: Hook::Chord(ChordHook::Close(completeness)),
    },
    Rule {
        id: "chord.seventh_evidence",
        topic: Topic::Chord,
        summary: "Read a seventh chord only when the seventh sounds for a substantial share of the span \
                  (half or more costs nothing; a passing seventh is better read as a nonchord tone), \
                  or when it is struck together with the root, as part of the sonority rather than \
                  a melodic figure. A brief seventh left by leap, as in an arpeggiated chord, costs \
                  half as much, since it fits neither the suspension nor the passing figure.",
        citations: &[
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 247 (a 7th approached as a passing tone may be an NCT; relative duration decides)",
            },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 219 (the 7th originated as a downward-resolving suspension or descending passing tone)",
            },
            Citation {
                source: SCHULTZ_ET_AL_2018,
                locator: "pp. 68–69 (a nonchord tone must fit a passing, neighbor, suspension or similar figure; only the escape tone is left by leap)",
            },
        ],
        default_weight: 1.0,
        params: &[SEVENTH_SHARE_TARGET, LEAPING_SEVENTH_FACTOR],
        hook: Hook::Chord(ChordHook::Close(seventh_evidence)),
    },
];

/// The cost of a segment whose third is never heard, against 1 for one whose root is never heard.
const MISSING_THIRD: Param = Param {
    index: 0,
    name: "missing_third",
    default: 0.5,
    unit: "ratio to a missing root",
    provenance: "chosen: 4e0fbe2, round 1 (Kostka, Payne and Almen 2023, p. 82, rank the third's omission after the fifth's and the root's); measured: sweeps/round7-constants.json, round 7 (0.25 / 0.375 / 0.75 / 1: tune RN -0.16 / -0.02 / -0.15 / -0.35; kept)",
};
/// Share of the segment's duration at which the seventh costs nothing.
const SEVENTH_SHARE_TARGET: Param = Param {
    index: 0,
    name: "share_target",
    default: 0.5,
    unit: "share of the segment",
    provenance: "chosen: 4e0fbe2, round 1 (Kostka, Payne and Almen 2023, p. 247, let relative duration decide but give no share); measured: sweeps/round7-constants.json and round7-constants-refine.json, round 7 (0.25 / 0.375 / 0.625 / 0.75 / 1: tune RN -0.25 / +0.01 / +0.17 / +0.12 / +0.08; 0.625 and 0.75 lower agreement-tune both offered by 1.0; kept)",
};
/// The share of the cost charged for a brief seventh left by leap.
/// Such a seventh fits neither the suspension nor the passing figure it originated as,
/// but may still be an escape tone.
const LEAPING_SEVENTH_FACTOR: Param = Param {
    index: 1,
    name: "leaping_seventh_factor",
    default: 0.5,
    unit: "ratio",
    provenance: "measured: a3b7bff, round 3 (0: tune RN +0.10 but both offered -0.14; 0.5: +0.06, both offered +0.01); measured: sweeps/round7-constants.json, round 7 (0 / 0.25 / 0.375 / 0.75 / 1: tune RN -0.03 / +0.01 / +0.01 / -0.03 / -0.07; kept)",
};

fn seventh_evidence(context: &ChordCloseContext, params: &[f64]) -> f32 {
    let chord = context.segment.chord;
    if !chord.quality.is_seventh() {
        return 0.0;
    }
    let Some(seventh) = chord.members().nth(3) else {
        return 0.0;
    };
    if context.slices.iter().any(|s| attacks(s, seventh) && attacks(s, chord.root)) {
        return 0.0;
    }
    let factor =
        if is_left_by_leap(context, seventh) { LEAPING_SEVENTH_FACTOR.read(params) as f32 } else { 1.0 };
    let sounds_seventh = |slice: &&Slice| slice.notes.iter().any(|n| n.tpc == seventh);
    let heard: f64 = context.slices.iter().filter(sounds_seventh).map(|s| s.duration).sum();
    let share = heard / context.segment.duration.max(1e-9);
    factor * ((share / SEVENTH_SHARE_TARGET.read(params)).min(1.0) - 1.0) as f32
}

/// Reports whether a note of the given spelling is attacked and then left by leap:
/// neither held into the next slice nor followed there by a note a step away.
fn is_left_by_leap(context: &ChordCloseContext, tpc: Tpc) -> bool {
    let slices = context.slices;
    (0..slices.len()).any(|i| {
        let next = slices.get(i + 1).or(context.next_slice.filter(|_| i + 1 == slices.len()));
        let is_left_by_step = |note: &&SliceNote| next.is_none_or(|n| continues_by_step(note, n));
        slices[i].notes.iter().filter(|n| n.attacked && n.tpc == tpc).any(|n| !is_left_by_step(&n))
    })
}

/// Reports whether the next slice holds the note over or attacks a note a step away from it.
fn continues_by_step(note: &SliceNote, next: &Slice) -> bool {
    let continues = |other: &SliceNote| {
        let interval = (other.midi - note.midi).abs();
        if other.attacked { (1..=2).contains(&interval) } else { interval == 0 }
    };
    next.notes.iter().any(continues)
}

fn attacks(slice: &Slice, tpc: Tpc) -> bool {
    slice.notes.iter().any(|n| n.attacked && n.tpc == tpc)
}

fn completeness(context: &ChordCloseContext, params: &[f64]) -> f32 {
    let segment = context.segment;
    let missing_root = if segment.has_heard_member(0) { 0.0 } else { 1.0 };
    let missing_third = if segment.has_heard_member(1) { 0.0 } else { MISSING_THIRD.read(params) as f32 };
    -(missing_root + missing_third)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::ChordSegment;
    use crate::engine::test_layers::chord_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};

    fn note(name: &str, attacked: bool) -> SliceNote {
        SliceNote { tpc: tpc_from_name(name).unwrap(), midi: 0, attacked, onset_level: 0 }
    }

    fn slice(notes: Vec<SliceNote>, duration: f64) -> Slice {
        Slice { start: 0.0, duration, notes, measure: 0, metric_level: 0, phrase: 0, key_signature: None }
    }

    /// A G7 segment four beats long that has heard its root, third and seventh.
    fn g7_segment() -> ChordSegment {
        let chord = Chord::new(tpc_from_name("G").unwrap(), Quality::DominantSeventh);
        ChordSegment { duration: 4.0, members_heard: 0b1011, ..chord_segment(chord) }
    }

    /// Scores a G7 segment: G and B for three beats, then F for one beat over a G that may be restruck.
    fn g7_seventh_evidence(root_restruck: bool) -> f32 {
        let slices = [
            slice(vec![note("G", true), note("B", true)], 3.0),
            slice(vec![note("G", root_restruck), note("F", true)], 1.0),
        ];
        let segment = g7_segment();
        seventh_evidence(
            &ChordCloseContext { segment: &segment, slices: &slices, next_slice: None },
            &RULES[1].default_params(),
        )
    }

    /// Scores a G7 segment with a held G and B, in which F5 (MIDI 77) is attacked
    /// for one beat of four and followed by a slice attacking the given MIDI note.
    fn g7_with_seventh_left_to(next_midi: i32) -> f32 {
        let tpc = |name: &str| tpc_from_name(name).unwrap();
        let pitch = |name: &str, midi: i32, attacked: bool| SliceNote {
            tpc: tpc(name),
            midi,
            attacked,
            onset_level: 0,
        };
        let slices = [
            slice(vec![pitch("G", 55, true), pitch("B", 71, true)], 3.0),
            slice(vec![pitch("G", 55, false), pitch("F", 77, true)], 1.0),
        ];
        let next = slice(vec![pitch("C", next_midi, true)], 1.0);
        let segment = g7_segment();
        seventh_evidence(
            &ChordCloseContext { segment: &segment, slices: &slices, next_slice: Some(&next) },
            &RULES[1].default_params(),
        )
    }

    #[test]
    fn a_brief_seventh_left_by_step_costs_the_full_shortfall() {
        assert_eq!(g7_with_seventh_left_to(76), -0.5);
    }

    #[test]
    fn a_brief_seventh_left_by_leap_costs_half() {
        assert_eq!(g7_with_seventh_left_to(72), -0.25);
    }

    #[test]
    fn a_seventh_struck_with_the_root_costs_nothing() {
        assert_eq!(g7_seventh_evidence(true), 0.0);
    }

    #[test]
    fn a_brief_seventh_over_a_held_root_costs() {
        assert!(g7_seventh_evidence(false) < 0.0);
    }
}
