//! Rules about where chord changes happen.

use super::sources::TEMPERLEY_2001;
use crate::engine::rule::{Citation, Hook, Layer, Rule, TransitionContext};

pub const RULES: &[Rule] = &[
    Rule {
        id: "chord.strong_beat",
        layer: Layer::Chord,
        summary: "Prefer chord changes on strong beats. Every change costs something; a change on a \
                  downbeat costs nothing and one on the weakest positions costs the most.",
        citations: &[Citation { source: TEMPERLEY_2001, locator: "pp. 151, 157 (HPR 2, Strong Beat Rule)" }],
        default_weight: 0.5,
        hook: Hook::Transition(strong_beat),
    },
    Rule {
        id: "chord.harmonic_variance",
        layer: Layer::Chord,
        summary: "Prefer roots close to the previous root on the line of fifths.",
        citations: &[Citation {
            source: TEMPERLEY_2001,
            locator: "pp. 152, 157–158 (HPR 3, Harmonic Variance Rule)",
        }],
        default_weight: 0.25,
        hook: Hook::Transition(harmonic_variance),
    },
];

fn strong_beat(context: &TransitionContext) -> f32 {
    match context.previous {
        Some(_) => (context.slice.metric_level as f32 - 4.0) / 4.0,
        None => 0.0,
    }
}

fn harmonic_variance(context: &TransitionContext) -> f32 {
    let Some(previous) = context.previous else {
        return 0.0;
    };
    let distance = (previous.reading.chord.root - context.next.chord.root).abs();
    -(distance.min(6) as f32) / 6.0
}
