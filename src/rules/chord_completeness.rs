//! Rules about which chord members a segment must contain.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::rule::{Citation, CloseContext, Hook, Layer, Rule};

pub const RULES: &[Rule] = &[
    Rule {
        id: "chord.completeness",
        layer: Layer::Chord,
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
        default_weight: 1.0,
        hook: Hook::Close(completeness),
    },
    Rule {
        id: "chord.seventh_evidence",
        layer: Layer::Chord,
        summary: "Read a seventh chord only when the seventh sounds for a substantial share of the span \
                  (half or more costs nothing; a passing seventh is better read as a nonchord tone).",
        citations: &[
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 247 (a 7th approached as a passing tone may be an NCT; relative duration decides)",
            },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 219 (the 7th originated as a suspension or passing tone)",
            },
        ],
        default_weight: 1.0,
        hook: Hook::Close(seventh_evidence),
    },
];

/// Share of the segment's duration at which the seventh costs nothing.
const SEVENTH_SHARE_TARGET: f64 = 0.5;

fn seventh_evidence(context: &CloseContext) -> f32 {
    let chord = context.segment.reading.chord;
    if !chord.quality.is_seventh() {
        return 0.0;
    }
    let seventh = chord.members().nth(3);
    let sounds_seventh =
        |slice: &&crate::score::slicing::Slice| slice.notes.iter().any(|n| Some(n.tpc) == seventh);
    let heard: f64 = context.slices.iter().filter(sounds_seventh).map(|s| s.duration).sum();
    let share = heard / context.segment.duration.max(1e-9);
    ((share / SEVENTH_SHARE_TARGET).min(1.0) - 1.0) as f32
}

fn completeness(context: &CloseContext) -> f32 {
    let segment = context.segment;
    let missing_root = if segment.has_heard_member(0) { 0.0 } else { 1.0 };
    let missing_third = if segment.has_heard_member(1) { 0.0 } else { 0.5 };
    -(missing_root + missing_third)
}
