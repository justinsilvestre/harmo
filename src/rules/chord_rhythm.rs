//! Rules about where chord changes happen.

use super::sources::TEMPERLEY_2001;
use crate::engine::rule::{Citation, Hook, Layer, Rule, TransitionContext};
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
        hook: Hook::Transition(thin_texture_rhythm),
    },
];

/// The most pitch classes a slice may sound for its texture to count as thin.
const THIN_PITCH_CLASSES: usize = 2;
/// The strongest metric level at which a change in a thin texture costs anything (a beat subdivision).
const SUBDIVISION_LEVEL: u8 = 1;

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

fn thin_texture_rhythm(context: &TransitionContext) -> f32 {
    let level = context.slice.metric_level;
    if context.previous.is_none() || level > SUBDIVISION_LEVEL || !is_thin(context.slice) {
        return 0.0;
    }
    -f32::from(SUBDIVISION_LEVEL + 1 - level) / f32::from(SUBDIVISION_LEVEL + 1)
}

fn is_thin(slice: &Slice) -> bool {
    let mut classes: Vec<u8> = slice.notes.iter().map(|n| pitch_class(n.tpc)).collect();
    classes.sort();
    classes.dedup();
    classes.len() <= THIN_PITCH_CLASSES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Reading, Segment};
    use crate::engine::rule::History;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn reading(root: &str) -> Reading {
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        let key = Key::parse("C", "major").unwrap();
        Reading { key, chord, tonicized: None, category: Category::Diatonic }
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
        let previous = Segment {
            reading: reading("C"),
            start: 0,
            previous: NONE,
            duration: 1.0,
            members_heard: 0b111,
            bass: Bass::default(),
        };
        let context = TransitionContext {
            previous: Some(&previous),
            next: &reading("G"),
            slice: &slice,
            history: History { arena: &[], last: NONE },
        };
        thin_texture_rhythm(&context)
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
