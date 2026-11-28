//! Scores how well the notes of a slice fit the chord of a reading.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, TEMPERLEY_2001};
use crate::engine::chord_hooks::{ChordHook, ChordSliceContext};
use crate::engine::rule::{Citation, Hook, Param, Rule, Topic};
use crate::score::slicing::{Slice, SliceNote};

pub const RULES: &[Rule] = &[
    Rule {
        id: "chord.compatibility",
        topic: Topic::Chord,
        summary: "Prefer chords whose members account for the sounding notes, valuing chord members in \
                  the order root, fifth, third, seventh. Each slice scores the mean value of its chord \
                  tones, times its duration.",
        citations: &[Citation {
            source: TEMPERLEY_2001, locator: "pp. 149–154 (HPR 1, Compatibility Rule)"
        }],
        default_weight: 1.0,
        params: &[THIRD_VALUE, FIFTH_VALUE, SEVENTH_VALUE, ENHARMONIC_SHARE],
        hook: Hook::Chord(ChordHook::Slice(compatibility)),
    },
    Rule {
        id: "chord.ornamental_dissonance",
        topic: Topic::Chord,
        summary: "Notes outside the chord cost less when a note a step away is attacked in the next \
                  slice, and more when they are attacked on a beat or held into a stronger beat \
                  (suspensions).",
        citations: &[
            Citation { source: TEMPERLEY_2001, locator: "pp. 153–154 (HPR 4, Ornamental Dissonance Rule)" },
            Citation { source: KOSTKA_PAYNE_ALMEN_2023, locator: "pp. 185–216 (nonchord tones)" },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 195 (suspensions almost always fall on accented beats)",
            },
        ],
        default_weight: 2.625,
        params: &[STEP_RESOLUTION, ACCENT],
        hook: Hook::Chord(ChordHook::Slice(ornamental_dissonance)),
    },
];

/// The values of the third, fifth and seventh of the chord, against 1 for the root.
/// HPR 1 orders them root, fifth, third, seventh.
const THIRD_VALUE: Param = Param {
    index: 0,
    name: "third_value",
    default: 0.8,
    unit: "ratio to the root",
    provenance: "chosen: 4e0fbe2, round 1 (Temperley 2001, p. 149, gives the order root, fifth, third, seventh but no values); measured: sweeps/round7-constants.json, round 7 (0.6 / 0.7 / 0.85: tune RN -0.37 / -0.17 / -0.00; kept)",
};
const FIFTH_VALUE: Param = Param {
    index: 1,
    name: "fifth_value",
    default: 0.9,
    unit: "ratio to the root",
    provenance: "chosen: 4e0fbe2, round 1 (Temperley 2001, p. 149, gives the order root, fifth, third, seventh but no values); measured: sweeps/round7-constants.json, round 7 (0.85 / 0.95 / 1: tune RN -0.02 / -0.01 / -0.08; kept)",
};
const SEVENTH_VALUE: Param = Param {
    index: 2,
    name: "seventh_value",
    default: 0.5,
    unit: "ratio to the root",
    provenance: "chosen: 4e0fbe2, round 1 (Temperley 2001, p. 149, gives the order root, fifth, third, seventh but no values); measured: sweeps/round7-constants.json, round 7 (0.25 / 0.375 / 0.625 / 0.75: tune RN -0.09 / -0.06 / -0.09 / -0.20; kept)",
};
/// The share of a chord tone's value earned by a note that matches it only enharmonically,
/// since the spelling disagrees.
const ENHARMONIC_SHARE: Param = Param {
    index: 3,
    name: "enharmonic_share",
    default: 0.5,
    unit: "ratio",
    provenance: "chosen: 4e0fbe2, round 1 (Temperley 2001, p. 149, scores spelled pitches); measured: sweeps/round7-constants.json, round 7 (0 / 0.25 / 0.75 / 1: tune RN +0.01 / +0.02 / -0.02 / -0.03; kept)",
};

fn compatibility(context: &ChordSliceContext, params: &[f64]) -> f32 {
    let notes = &context.slice.notes;
    if notes.is_empty() {
        return 0.0;
    }
    let chord = context.chord;
    let (third, fifth, seventh) =
        (THIRD_VALUE.read(params), FIFTH_VALUE.read(params), SEVENTH_VALUE.read(params));
    let member_values = [1.0, third as f32, fifth as f32, seventh as f32];
    let enharmonic_share = ENHARMONIC_SHARE.read(params) as f32;
    let value =
        |note: &SliceNote| match (chord.member_index(note.tpc), chord.member_index_enharmonic(note.tpc)) {
            (Some(member), _) => member_values[member],
            (None, Some(member)) => member_values[member] * enharmonic_share,
            (None, None) => 0.0,
        };
    notes.iter().map(value).sum::<f32>() / notes.len() as f32 * context.slice.duration as f32
}

/// Returns minus the mean cost of the slice's non-chord tones, times the slice's duration.
fn ornamental_dissonance(context: &ChordSliceContext, params: &[f64]) -> f32 {
    let notes = &context.slice.notes;
    if notes.is_empty() {
        return 0.0;
    }
    let chord = context.chord;
    let is_ornamental = |note: &&SliceNote| chord.member_index_enharmonic(note.tpc).is_none();
    let cost: f32 = notes
        .iter()
        .filter(is_ornamental)
        .map(|n| dissonance_cost(n, context.slice, context.next_slice, params))
        .sum();
    -cost / notes.len() as f32 * context.slice.duration as f32
}

/// Cost factor for a nonchord tone followed by a note a step away, as in passing and neighbor figures.
const STEP_RESOLUTION: Param = Param {
    index: 0,
    name: "step_resolution",
    default: 0.3,
    unit: "ratio",
    provenance: "chosen: 4e0fbe2, round 1 (Temperley 2001, p. 153, prefers ornamental tones followed by a step but gives no value); measured: sweeps/round7-constants.json, round 7 (0.15 / 0.225 / 0.45 / 0.6: tune RN -0.05 / -0.07 / -0.19 / -0.37; kept)",
};
/// Cost factor for a nonchord tone on a beat.
const ACCENT: Param = Param {
    index: 1,
    name: "accent",
    default: 1.5,
    unit: "ratio",
    provenance: "chosen: 4e0fbe2, round 1 (Temperley 2001, p. 154, prefers metrically weak ornamental tones but gives no value); measured: sweeps/round7-constants.json, round 7 (1 / 1.25 / 2 / 3: tune RN -0.31 / -0.13 / -0.06 / -0.27; kept)",
};

fn dissonance_cost(note: &SliceNote, slice: &Slice, next: Option<&Slice>, params: &[f64]) -> f32 {
    let resolution = if is_followed_by_step(note, next) { STEP_RESOLUTION.read(params) as f32 } else { 1.0 };
    let accent = if is_accented(note, slice) { ACCENT.read(params) as f32 } else { 1.0 };
    resolution * accent
}

/// Returns whether a note attacked in the next slice lies a step from this one.
/// Notes held into the next slice are not a resolution.
fn is_followed_by_step(note: &SliceNote, next: Option<&Slice>) -> bool {
    let is_step = |other: &SliceNote| other.attacked && (1..=2).contains(&(other.midi - note.midi).abs());
    next.is_some_and(|n| n.notes.iter().any(is_step))
}

/// Returns whether the note is attacked on a beat or held into a beat stronger than its onset,
/// as a suspension is.
fn is_accented(note: &SliceNote, slice: &Slice) -> bool {
    let is_suspended = !note.attacked && slice.metric_level > note.onset_level;
    slice.metric_level >= 2 && (note.attacked || is_suspended)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(midi: i32, attacked: bool, onset_level: u8) -> SliceNote {
        SliceNote { tpc: 0, midi, attacked, onset_level }
    }

    fn slice(notes: Vec<SliceNote>, metric_level: u8) -> Slice {
        Slice { start: 0.0, duration: 1.0, notes, measure: 0, metric_level, phrase: 0, key_signature: None }
    }

    #[test]
    fn a_step_to_an_attacked_note_discounts_the_cost() {
        let next = slice(vec![note(60, true, 0)], 0);
        let cost =
            dissonance_cost(&note(62, true, 0), &slice(vec![], 0), Some(&next), &RULES[1].default_params());
        assert_eq!(cost, 0.3);
    }

    #[test]
    fn a_step_to_a_held_note_is_no_resolution() {
        let next = slice(vec![note(60, false, 0)], 0);
        let cost =
            dissonance_cost(&note(62, true, 0), &slice(vec![], 0), Some(&next), &RULES[1].default_params());
        assert_eq!(cost, 1.0);
    }

    #[test]
    fn a_suspension_counts_as_accented() {
        assert!(is_accented(&note(62, false, 1), &slice(vec![], 4)));
    }

    #[test]
    fn a_note_held_into_a_weaker_beat_is_unaccented() {
        assert!(!is_accented(&note(62, false, 4), &slice(vec![], 2)));
    }
}
