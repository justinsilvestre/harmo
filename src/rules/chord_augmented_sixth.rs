//! Rules about the bass of augmented sixth chords.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::rule::{ChordTransitionContext, Citation, Hook, Layer, Rule};
use crate::pitch::pitch_class;
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[Rule {
    id: "chord.augmented_sixth_bass",
    layer: Layer::Chord,
    summary: "Prefer augmented sixth chords with the lowered sixth degree in the bass.",
    citations: &[Citation {
        source: KOSTKA_PAYNE_ALMEN_2023,
        locator: "pp. 383–386, 391 (augmented sixths and other bass positions)",
    }],
    default_weight: 1.0,
    params: &[],
    hook: Hook::ChordTransition(augmented_sixth_bass),
}];

fn augmented_sixth_bass(context: &ChordTransitionContext, _: &[f64]) -> f32 {
    let chord = context.next;
    let lowered_sixth = match chord.quality {
        Quality::ItalianSixth | Quality::GermanSixth => chord.root - 10,
        Quality::FrenchSixth => chord.root - 6,
        _ => return 0.0,
    };
    let bass = context.slice.bass().map(|n| pitch_class(n.tpc));
    if bass == Some(pitch_class(lowered_sixth)) { 0.0 } else { -1.0 }
}
