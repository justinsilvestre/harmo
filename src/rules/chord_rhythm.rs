//! Rules about where chord changes happen.

use super::sources::TEMPERLEY_2001;
use crate::engine::chord_hooks::{ChordHook, ChordTransitionContext};
use crate::engine::rule::{Citation, Hook, Layer, Param, Rule};
use crate::pitch::pitch_class;
use crate::score::slicing::Slice;

pub const RULES: &[Rule] = &[
    Rule {
        id: "chord.strong_beat",
        layer: Layer::Chord,
        summary: "Prefer chord changes on strong beats. Every change costs something; a change on a \
                  downbeat costs nothing and one on the weakest positions costs the most.",
        citations: &[Citation { source: TEMPERLEY_2001, locator: "pp. 151, 157 (HPR 2, Strong Beat Rule)" }],
        default_weight: 0.5,
        params: &[],
        hook: Hook::Chord(ChordHook::Transition(strong_beat)),
    },
    Rule {
        id: "chord.harmonic_variance",
        layer: Layer::Chord,
        summary: "Prefer roots close to the previous root on the line of fifths.",
        citations: &[Citation {
            source: TEMPERLEY_2001,
            locator: "pp. 152, 157–158 (HPR 3, Harmonic Variance Rule)",
        }],
        default_weight: 0.1875,
        params: &[MAX_DISTANCE],
        hook: Hook::Chord(ChordHook::Transition(harmonic_variance)),
    },
    Rule {
        id: "chord.thin_texture_rhythm",
        layer: Layer::Chord,
        summary: "In a thin texture, where the slice sounds no more than two pitch classes (a single \
                  line, octaves or two voices), avoid chord changes between beats. Each note of a \
                  line fits several chords, so a change there is weakly supported; a change on a beat \
                  subdivision costs half as much as one on a weaker position.",
        citations: &[Citation {
            source: TEMPERLEY_2001,
            locator: "pp. 158–159 (in an unaccompanied melody the strong beat rule keeps the harmonic \
                      rhythm slow, so that single notes do not become chord spans of their own)",
        }],
        default_weight: 0.5,
        params: &[THIN_PITCH_CLASSES, SUBDIVISION_LEVEL],
        hook: Hook::Chord(ChordHook::Transition(thin_texture_rhythm)),
    },
];

/// The distance between roots on the line of fifths at which the cost stops growing; it costs 1 there.
const MAX_DISTANCE: Param = Param {
    index: 0,
    name: "max_distance",
    default: 6.0,
    unit: "fifths",
    provenance: "chosen: 4e0fbe2, round 1 (a tritone; Temperley 2001, p. 152, gives no cap); measured: sweeps/round7-constants.json, round 7 (3 / 4 / 9 / 12: tune RN -0.02 / +0.02 / -0.02 / -0.02; kept)",
};
/// The most pitch classes a slice may sound for its texture to count as thin.
const THIN_PITCH_CLASSES: Param = Param {
    index: 0,
    name: "thin_pitch_classes",
    default: 2.0,
    unit: "pitch classes",
    provenance: "chosen: 222f9f0, round 4 (a single line, octaves or two voices); measured: sweeps/round7-constants.json, round 7 (1 / 3: tune RN -0.06 / -0.20; kept)",
};
/// The strongest metric level at which a change in a thin texture costs anything (a beat subdivision).
/// A change at that level costs 1 / (level + 1) of one at level 0.
const SUBDIVISION_LEVEL: Param = Param {
    index: 1,
    name: "subdivision_level",
    default: 1.0,
    unit: "metric level",
    provenance: "measured: 222f9f0, round 4 (the strong-beat curve on every level instead: tune RN +0.1 with wir -0.2 to -0.4); measured: sweeps/round7-constants.json and round7-constants-refine.json, round 7 (0 / 2 / 3: tune RN -0.12 / +0.10 / +0.11; 2 gives +0.10 [+0.01, +0.20] but agreement-tune both offered -0.35 [-0.76, +0.04] while offering 0.014 more readings; kept)",
};

fn strong_beat(context: &ChordTransitionContext, _: &[f64]) -> f32 {
    match context.previous {
        Some(_) => (context.slice.metric_level as f32 - 4.0) / 4.0,
        None => 0.0,
    }
}

fn harmonic_variance(context: &ChordTransitionContext, params: &[f64]) -> f32 {
    let Some(previous) = context.previous else {
        return 0.0;
    };
    let cap = MAX_DISTANCE.read(params) as i32;
    let distance = (previous.chord.root - context.next.root).abs();
    -(distance.min(cap) as f32) / cap as f32
}

fn thin_texture_rhythm(context: &ChordTransitionContext, params: &[f64]) -> f32 {
    let level = context.slice.metric_level;
    let strongest = SUBDIVISION_LEVEL.read(params) as u8;
    let most_pitch_classes = THIN_PITCH_CLASSES.read(params) as usize;
    if context.previous.is_none() || level > strongest || !is_thin(context.slice, most_pitch_classes) {
        return 0.0;
    }
    -f32::from(strongest + 1 - level) / f32::from(strongest + 1)
}

fn is_thin(slice: &Slice, most_pitch_classes: usize) -> bool {
    let mut classes: Vec<u8> = slice.notes.iter().map(|n| pitch_class(n.tpc)).collect();
    classes.sort();
    classes.dedup();
    classes.len() <= most_pitch_classes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{ChordSegment, Function, Reading};
    use crate::engine::test_layers::chord_segment;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn reading(root: &str) -> Reading {
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        let key = Key::parse("C", "major").unwrap();
        Reading { key, chord, function: Function::DIATONIC }
    }

    /// Scores a change from C to G on a slice sounding the named notes at the given metric level.
    fn change_on(names: &[&str], metric_level: u8) -> f32 {
        let note = |name: &&str| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi: 0,
            attacked: true,
            onset_level: 0,
        };
        let slice = Slice {
            start: 0.0,
            duration: 0.5,
            notes: names.iter().map(note).collect(),
            measure: 0,
            metric_level,
            phrase: 0,
            key_signature: None,
        };
        let previous = ChordSegment { members_heard: 0b111, ..chord_segment(reading("C").chord) };
        let context = ChordTransitionContext {
            previous: Some(&previous),
            next: reading("G").chord,
            slice: &slice,
            slices: &[],
        };
        thin_texture_rhythm(&context, &RULES[2].default_params())
    }

    #[test]
    fn a_change_on_a_single_note_between_subdivisions_costs_the_most() {
        assert_eq!(change_on(&["D"], 0), -1.0);
    }

    #[test]
    fn a_change_on_a_bare_fifth_at_a_subdivision_costs_half() {
        assert_eq!(change_on(&["G", "D"], 1), -0.5);
    }

    #[test]
    fn a_change_on_a_beat_in_a_thin_texture_costs_nothing() {
        assert_eq!(change_on(&["D"], 2), 0.0);
    }

    #[test]
    fn a_change_on_a_full_triad_costs_nothing() {
        assert_eq!(change_on(&["G", "B", "D"], 0), 0.0);
    }
}
