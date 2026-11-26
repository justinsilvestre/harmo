//! Rules about changes of key.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, TEMPERLEY_2001};
use crate::engine::rule::{Citation, Hook, Layer, Rule, TransitionContext};

pub const RULES: &[Rule] = &[
    Rule {
        id: "key.modulation",
        layer: Layer::Key,
        summary: "Prefer to stay in the same key from one chord to the next.",
        citations: &[
            Citation { source: TEMPERLEY_2001, locator: "p. 188 (KPR 2, Modulation Rule)" },
            Citation { source: KOSTKA_PAYNE_ALMEN_2023, locator: "p. 298 (modulation and tonicization)" },
        ],
        default_weight: 1.0,
        hook: Hook::Transition(modulation),
    },
    Rule {
        id: "key.area_duration",
        layer: Layer::Key,
        summary: "Prefer key areas that last. Leaving a key that has lasted less than four quarter-note \
                  measures costs in proportion to the shortfall; briefer areas are better heard as \
                  tonicizations.",
        citations: &[Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 299 (the length of time spent in the new key area as evidence of modulation)",
        }],
        default_weight: 0.25,
        hook: Hook::Transition(area_duration),
    },
];

/// Duration in quarter notes at which a key area is long enough to cost nothing when it ends.
const ESTABLISHED_AREA: f64 = 16.0;

fn modulation(context: &TransitionContext) -> f32 {
    match context.previous {
        Some(previous) if previous.reading.key != context.next.key => -1.0,
        _ => 0.0,
    }
}

fn area_duration(context: &TransitionContext) -> f32 {
    let Some(previous) = context.previous else { return 0.0 };
    let key = previous.reading.key;
    if key == context.next.key {
        return 0.0;
    }
    let area: f64 = context.history.segments().take_while(|s| s.reading.key == key).map(|s| s.duration).sum();
    -((1.0 - area / ESTABLISHED_AREA).max(0.0) as f32)
}
