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
        summary: "Notes outside the chord cost less when they move by step into the next slice and when \
                  they are not attacked on a beat.",
        citations: &[
            Citation { source: TEMPERLEY_2001, locator: "pp. 153–154 (HPR 4, Ornamental Dissonance Rule)" },
            Citation { source: KOSTKA_PAYNE_ALMEN_2023, locator: "pp. 185–216 (nonchord tones)" },
        ],
        default_weight: 1.0,
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

fn dissonance_cost(note: &SliceNote, slice: &Slice, next: Option<&Slice>) -> f32 {
    let resolves_by_step =
        next.is_some_and(|n| n.notes.iter().any(|m| (1..=2).contains(&(m.midi - note.midi).abs())));
    let is_accented = note.attacked && slice.metric_level >= 2;
    let resolution = if resolves_by_step { 0.3 } else { 1.0 };
    let accent = if is_accented { 1.5 } else { 1.0 };
    resolution * accent
}
