//! Scores how well the notes of a slice fit the chord of a reading.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, TEMPERLEY_2001};
use crate::engine::rule::{Citation, Hook, Layer, Rule, SliceContext};
use crate::score::slicing::{Slice, SliceNote};

pub const RULES: &[Rule] = &[
    Rule {
        id: "chord.compatibility",
        layer: Layer::Chord,
        summary: "Prefer chords whose members account for the sounding notes, valuing chord members in \
                  the order root, fifth, third, seventh. Each slice scores the mean value of its chord \
                  tones, times its duration.",
        citations: &[Citation {
            source: TEMPERLEY_2001, locator: "pp. 149–154 (HPR 1, Compatibility Rule)"
        }],
        default_weight: 1.0,
        hook: Hook::Slice(compatibility),
    },
    Rule {
        id: "chord.ornamental_dissonance",
        layer: Layer::Chord,
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
        default_weight: 2.0,
        hook: Hook::Slice(ornamental_dissonance),
    },
];

/// Values of root, third, fifth and seventh, following HPR 1's ordering (^1, ^5, ^3, ^7).
const MEMBER_VALUES: [f32; 4] = [1.0, 0.8, 0.9, 0.5];

/// Enharmonically equivalent chord tones count for half, since the spelling disagrees.
fn compatibility(context: &SliceContext) -> f32 {
    let notes = &context.slice.notes;
    if notes.is_empty() {
        return 0.0;
    }
    let chord = context.reading.chord;
    let value =
        |note: &SliceNote| match (chord.member_index(note.tpc), chord.member_index_enharmonic(note.tpc)) {
            (Some(member), _) => MEMBER_VALUES[member],
            (None, Some(member)) => MEMBER_VALUES[member] / 2.0,
            (None, None) => 0.0,
        };
    notes.iter().map(value).sum::<f32>() / notes.len() as f32 * context.slice.duration as f32
}

/// Returns minus the mean cost of the slice's non-chord tones, times the slice's duration.
fn ornamental_dissonance(context: &SliceContext) -> f32 {
    let notes = &context.slice.notes;
    if notes.is_empty() {
        return 0.0;
    }
    let chord = context.reading.chord;
    let is_ornamental = |note: &&SliceNote| chord.member_index_enharmonic(note.tpc).is_none();
    let cost: f32 = notes
        .iter()
        .filter(is_ornamental)
        .map(|n| dissonance_cost(n, context.slice, context.next_slice))
        .sum();
    -cost / notes.len() as f32 * context.slice.duration as f32
}

/// Cost factor for a nonchord tone followed by a note a step away, as in passing and neighbor figures.
const STEP_RESOLUTION: f32 = 0.3;
/// Cost factor for a nonchord tone on a beat.
const ACCENT: f32 = 1.5;

fn dissonance_cost(note: &SliceNote, slice: &Slice, next: Option<&Slice>) -> f32 {
    let resolution = if is_followed_by_step(note, next) { STEP_RESOLUTION } else { 1.0 };
    let accent = if is_accented(note, slice) { ACCENT } else { 1.0 };
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
        Slice { start: 0.0, duration: 1.0, notes, measure: 0, metric_level }
    }

    #[test]
    fn a_step_to_an_attacked_note_discounts_the_cost() {
        let next = slice(vec![note(60, true, 0)], 0);
        assert_eq!(dissonance_cost(&note(62, true, 0), &slice(vec![], 0), Some(&next)), STEP_RESOLUTION);
    }

    #[test]
    fn a_step_to_a_held_note_is_no_resolution() {
        let next = slice(vec![note(60, false, 0)], 0);
        assert_eq!(dissonance_cost(&note(62, true, 0), &slice(vec![], 0), Some(&next)), 1.0);
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
