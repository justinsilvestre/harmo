//! Rules about which chord members a segment must contain.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, SCHULTZ_ET_AL_2018};
use crate::engine::rule::{Citation, CloseContext, Hook, Layer, Rule};
use crate::pitch::Tpc;
use crate::score::slicing::Slice;

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
                  (half or more costs nothing; a passing seventh is better read as a nonchord tone), \
                  or when it is struck together with the root, as part of the sonority rather than \
                  a melodic figure.",
        citations: &[
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 247 (a 7th approached as a passing tone may be an NCT; relative duration decides)",
            },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 219 (the 7th originated as a suspension or passing tone)",
            },
            Citation {
                source: SCHULTZ_ET_AL_2018,
                locator: "pp. 68–69 (a nonchord tone must fit a passing, neighbor, suspension or similar figure)",
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
    let Some(seventh) = chord.members().nth(3) else {
        return 0.0;
    };
    if context.slices.iter().any(|s| attacks(s, seventh) && attacks(s, chord.root)) {
        return 0.0;
    }
    let sounds_seventh = |slice: &&Slice| slice.notes.iter().any(|n| n.tpc == seventh);
    let heard: f64 = context.slices.iter().filter(sounds_seventh).map(|s| s.duration).sum();
    let share = heard / context.segment.duration.max(1e-9);
    ((share / SEVENTH_SHARE_TARGET).min(1.0) - 1.0) as f32
}

fn attacks(slice: &Slice, tpc: Tpc) -> bool {
    slice.notes.iter().any(|n| n.attacked && n.tpc == tpc)
}

fn completeness(context: &CloseContext) -> f32 {
    let segment = context.segment;
    let missing_root = if segment.has_heard_member(0) { 0.0 } else { 1.0 };
    let missing_third = if segment.has_heard_member(1) { 0.0 } else { 0.5 };
    -(missing_root + missing_third)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Category, NONE, Reading, Segment};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn note(name: &str, attacked: bool) -> SliceNote {
        SliceNote { tpc: tpc_from_name(name).unwrap(), midi: 0, attacked, onset_level: 0 }
    }

    fn slice(notes: Vec<SliceNote>, duration: f64) -> Slice {
        Slice { start: 0.0, duration, notes, measure: 0, metric_level: 0 }
    }

    /// Scores a G7 segment: G and B for three beats, then F for one beat over a G that may be restruck.
    fn g7_seventh_evidence(root_restruck: bool) -> f32 {
        let slices = [
            slice(vec![note("G", true), note("B", true)], 3.0),
            slice(vec![note("G", root_restruck), note("F", true)], 1.0),
        ];
        let chord = Chord::new(tpc_from_name("G").unwrap(), Quality::DominantSeventh);
        let key = Key::parse("C", "major").unwrap();
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        let segment =
            Segment { reading, start: 0, previous: NONE, duration: 4.0, members_heard: 0b1011, bass: None };
        seventh_evidence(&CloseContext { segment: &segment, slices: &slices, next_slice: None })
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
