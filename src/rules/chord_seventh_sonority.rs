//! Rules about when a seventh chord other than the dominant seventh is heard as one sonority.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::rule::{ChordCloseContext, Citation, Hook, Layer, Param, Rule};
use crate::pitch::Tpc;
use crate::score::slicing::Slice;
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[Rule {
    id: "chord.seventh_sonority",
    layer: Layer::Chord,
    summary: "Read a seventh chord other than the dominant and diminished sevenths only when its root \
              and seventh sound together for a substantial share of the span (five eighths or \
              more costs nothing). Otherwise the seventh, or the root, is better read as a nonchord tone.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 237 (incomplete seventh chords must contain at least the root and the 7th)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 247 (a 7th approached as a passing tone may be an NCT; relative duration decides)",
        },
    ],
    default_weight: 1.25,
    params: &[SHARE_TARGET],
    hook: Hook::ChordClose(seventh_sonority),
}];

/// Share of the segment's duration, with root and seventh sounding together, at which the rule costs nothing.
const SHARE_TARGET: Param = Param {
    index: 0,
    name: "share_target",
    default: 0.625,
    unit: "share of the segment",
    provenance: "measured: sweeps/round7-constants.json and round7-constants-refine.json, round 7 (0.25 / 0.375 / 0.5 / 0.5625 / 0.625 / 0.6875 / 0.75 / 0.875 / 1: tune RN -0.16 / -0.04 / 0 / +0.06 / +0.15 / +0.15 / +0.14 / +0.13 / +0.13; 0.625: +0.15 [+0.09, +0.22], agreement-tune both offered +0.03 [-0.05, +0.11]; taken); 34a3f45, round 2, had found 0.75 and 1.0 costing WTC and keyboard_other up to 2 points",
};

fn seventh_sonority(context: &ChordCloseContext, params: &[f64]) -> f32 {
    let chord = context.segment.chord;
    let Some(seventh) = chord.members().nth(3).filter(|_| is_judged(chord.quality)) else {
        return 0.0;
    };
    let sounds = |slice: &Slice, tpc: Tpc| slice.notes.iter().any(|n| n.tpc == tpc);
    let together = |slice: &&Slice| sounds(slice, seventh) && sounds(slice, chord.root);
    let heard: f64 = context.slices.iter().filter(together).map(|s| s.duration).sum();
    let share = heard / context.segment.duration.max(1e-9);
    ((share / SHARE_TARGET.read(params)).min(1.0) - 1.0) as f32
}

/// Reports whether the rule judges a quality: the seventh chords, leaving out the dominant and diminished
/// sevenths, whose sevenths are treated freely, and the augmented sixths, which have no seventh.
fn is_judged(quality: Quality) -> bool {
    let is_exempt = matches!(
        quality,
        Quality::DominantSeventh
            | Quality::DiminishedSeventh
            | Quality::ItalianSixth
            | Quality::FrenchSixth
            | Quality::GermanSixth
    );
    quality.is_seventh() && !is_exempt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Reading, Segment};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    fn slice(names: &[&str]) -> Slice {
        let note = |name: &&str| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi: 0,
            attacked: true,
            onset_level: 0,
        };
        Slice {
            start: 0.0,
            duration: 1.0,
            notes: names.iter().map(note).collect(),
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        }
    }

    /// Scores a two-beat segment on the given root and quality in C major.
    fn score(root: &str, quality: Quality, slices: &[Slice]) -> f32 {
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        let key = Key::parse("C", "major").unwrap();
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        let segment = Segment {
            reading,
            start: 0,
            previous: NONE,
            duration: 2.0,
            members_heard: 0b1111,
            bass: Bass::default(),
        };
        seventh_sonority(
            &ChordCloseContext { segment: &segment.chord_segment(NONE), slices, next_slice: None },
            &RULES[0].default_params(),
        )
    }

    #[test]
    fn a_supertonic_seventh_with_root_and_seventh_together_throughout_costs_nothing() {
        assert_eq!(
            score("D", Quality::MinorSeventh, &[slice(&["D", "F", "A", "C"]), slice(&["D", "C"])]),
            0.0
        );
    }

    #[test]
    fn a_supertonic_seventh_with_root_and_seventh_together_for_half_the_span_costs() {
        let cost = score("D", Quality::MinorSeventh, &[slice(&["D", "F", "A", "C"]), slice(&["F", "A"])]);
        assert!(cost < 0.0);
    }

    #[test]
    fn a_subdominant_seventh_whose_root_never_meets_the_seventh_costs() {
        assert_eq!(score("F", Quality::MajorSeventh, &[slice(&["F", "A"]), slice(&["A", "C", "E"])]), -1.0);
    }

    #[test]
    fn a_dominant_seventh_is_not_judged() {
        assert_eq!(score("G", Quality::DominantSeventh, &[slice(&["G", "B"]), slice(&["B", "D", "F"])]), 0.0);
    }
}
