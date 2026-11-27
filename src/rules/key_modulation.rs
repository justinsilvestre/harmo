//! Rules about changes of key.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, TEMPERLEY_2001};
use crate::engine::rule::{Citation, History, Hook, Layer, Rule, TransitionContext};
use crate::theory::key::Key;

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
    -((1.0 - area_heard(context.history, key) / ESTABLISHED_AREA).max(0.0) as f32)
}

/// Sums the durations of the latest segments in the given key, stopping once the area is
/// established, since a longer area costs no more.
fn area_heard(history: History, key: Key) -> f64 {
    let mut area = 0.0;
    for segment in history.segments().take_while(|s| s.reading.key == key) {
        area += segment.duration;
        if area >= ESTABLISHED_AREA {
            break;
        }
    }
    area
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Node, Reading, Segment};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Mode;

    fn reading(tonic: &str) -> Reading {
        let tpc = tpc_from_name(tonic).unwrap();
        let chord = Chord::new(tpc, Quality::Major);
        Reading { key: Key::new(tpc, Mode::Major), chord, tonicized: None, category: Category::Diatonic }
    }

    /// Chains one tonic triad segment per (tonic, duration), each following the one before.
    fn arena(segments: &[(&str, f64)]) -> Vec<Node> {
        let node = |(index, &(tonic, duration)): (usize, &(&str, f64))| {
            let previous = if index == 0 { NONE } else { index as u32 - 1 };
            let segment = Segment {
                reading: reading(tonic),
                start: index as u32,
                previous,
                duration,
                members_heard: 0b111,
                bass: Bass::default(),
            };
            Node { parent: previous, slice: index as u32, segment, score: 0.0 }
        };
        segments.iter().enumerate().map(node).collect()
    }

    fn heard(segments: &[(&str, f64)]) -> f64 {
        let arena = arena(segments);
        area_heard(History { arena: &arena, slices: &[], last: arena.len() as u32 - 1 }, reading("C").key)
    }

    /// Scores moving to G major after the given segments.
    fn leaving(segments: &[(&str, f64)]) -> f32 {
        let arena = arena(segments);
        let last = arena.len() as u32 - 1;
        let slice = Slice {
            start: 0.0,
            duration: 1.0,
            notes: vec![],
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        };
        let context = TransitionContext {
            previous: Some(&arena[last as usize].segment),
            next: &reading("G"),
            slice: &slice,
            history: History { arena: &arena, slices: &[], last },
        };
        area_duration(&context)
    }

    #[test]
    fn area_heard_sums_the_latest_segments_in_the_key() {
        assert_eq!(heard(&[("G", 4.0), ("C", 2.0), ("C", 3.0)]), 5.0);
    }

    #[test]
    fn area_heard_stops_once_the_area_is_established() {
        assert_eq!(heard(&[("C", 10.0), ("C", 10.0), ("C", 10.0)]), 20.0);
    }

    #[test]
    fn leaving_a_brief_area_costs_its_shortfall() {
        assert_eq!(leaving(&[("C", 4.0)]), -0.75);
    }

    #[test]
    fn leaving_an_established_area_costs_nothing() {
        assert_eq!(leaving(&[("C", 10.0), ("C", 10.0)]), 0.0);
    }
}
