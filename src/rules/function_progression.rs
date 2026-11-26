//! Rules about how chords function in succession.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::model::Category;
use crate::engine::rule::{Citation, CloseContext, Hook, Layer, Rule, TransitionContext};
use crate::pitch::pitch_class;
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[
    Rule {
        id: "function.diatonic_preference",
        layer: Layer::Function,
        summary: "Prefer diatonic readings. Altered chords (applied chords, mixture, the Neapolitan and \
                  augmented sixths) are read only when the notes call for them.",
        citations: &[Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 255 (chromaticism and altered chords)",
        }],
        default_weight: 1.0,
        hook: Hook::Transition(diatonic_preference),
    },
    Rule {
        id: "function.applied_resolution",
        layer: Layer::Function,
        summary: "An applied chord should be followed by the chord it tonicizes.",
        citations: &[Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 256–260, 273–276 (secondary functions in context)",
        }],
        default_weight: 1.0,
        hook: Hook::Transition(applied_resolution),
    },
    Rule {
        id: "function.dominant_resolution",
        layer: Layer::Function,
        summary: "Prefer readings in which V or V7 is followed by I (or i) of the same key.",
        citations: &[Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 95–98 (harmonic progression; the I and V chords)",
        }],
        default_weight: 1.0,
        hook: Hook::Transition(dominant_resolution),
    },
    Rule {
        id: "function.six_four",
        layer: Layer::Function,
        summary: "Avoid second-inversion triads unless the bass is held or repeated into the next chord, \
                  as in cadential and pedal six-four chords.",
        citations: &[Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 133–141 (triads in second inversion)",
        }],
        default_weight: 1.0,
        hook: Hook::Close(six_four),
    },
    Rule {
        id: "function.augmented_sixth_bass",
        layer: Layer::Function,
        summary: "Prefer augmented sixth chords with the lowered sixth degree in the bass.",
        citations: &[Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 383–386, 391 (augmented sixths and other bass positions)",
        }],
        default_weight: 1.0,
        hook: Hook::Transition(augmented_sixth_bass),
    },
];

fn diatonic_preference(context: &TransitionContext) -> f32 {
    if context.next.category == Category::Diatonic { 0.0 } else { -1.0 }
}

fn applied_resolution(context: &TransitionContext) -> f32 {
    let Some(previous) = context.previous else {
        return 0.0;
    };
    match previous.reading.tonicized {
        Some(target) if target.tonic == context.next.chord.root => 1.0,
        Some(_) => -1.0,
        None => 0.0,
    }
}

fn dominant_resolution(context: &TransitionContext) -> f32 {
    let Some(previous) = context.previous else {
        return 0.0;
    };
    let reading = previous.reading;
    let is_dominant = reading.tonicized.is_none()
        && reading.chord.root == reading.key.tonic + 1
        && matches!(reading.chord.quality, Quality::Major | Quality::DominantSeventh);
    let resolves = context.next.key == reading.key && context.next.chord.root == reading.key.tonic;
    if is_dominant && resolves { 1.0 } else { 0.0 }
}

fn six_four(context: &CloseContext) -> f32 {
    let segment = context.segment;
    let chord = segment.reading.chord;
    let is_triad =
        matches!(chord.quality, Quality::Major | Quality::Minor | Quality::Diminished | Quality::Augmented);
    let is_six_four = is_triad && chord.member_index(segment.bass_tpc()) == Some(2);
    let next_bass = context.next_slice.and_then(|s| s.bass()).map(|n| pitch_class(n.tpc));
    let bass_continues = next_bass == Some(pitch_class(segment.bass_tpc()));
    if is_six_four && !bass_continues { -1.0 } else { 0.0 }
}

fn augmented_sixth_bass(context: &TransitionContext) -> f32 {
    let chord = context.next.chord;
    let lowered_sixth = match chord.quality {
        Quality::ItalianSixth | Quality::GermanSixth => chord.root - 10,
        Quality::FrenchSixth => chord.root - 6,
        _ => return 0.0,
    };
    let bass = context.slice.bass().map(|n| pitch_class(n.tpc));
    if bass == Some(pitch_class(lowered_sixth)) { 0.0 } else { -1.0 }
}
