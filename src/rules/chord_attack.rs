//! Rules about chord changes and how many voices start together.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, SCHULTZ_ET_AL_2018};
use crate::engine::rule::{Citation, Hook, Layer, Param, Rule, TransitionContext};

pub const RULES: &[Rule] = &[Rule {
    id: "chord.full_attack",
    layer: Layer::Chord,
    summary: "Favor a chord change on a slice where every sounding note, three or more, starts together. \
              When all voices are struck at once the slice is very likely a new chord; embellishing \
              tones are added in single voices between such attacks.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 200 (a one-to-one framework, a chord struck in all voices on each beat, is \
                      embellished by passing tones and arpeggiations in single voices)",
        },
        Citation {
            source: SCHULTZ_ET_AL_2018,
            locator: "p. 68 (chorale textures are consistent and highly constrained, with few \
                      non-chord tones)",
        },
    ],
    default_weight: 0.25,
    params: &[FULL_ATTACK_NOTES],
    hook: Hook::Transition(full_attack),
}];

/// The fewest sounding notes for a slice struck all at once to count as a full attack.
const FULL_ATTACK_NOTES: Param = Param {
    index: 0,
    name: "full_attack_notes",
    default: 3.0,
    unit: "notes",
    provenance: "measured: b00871a, round 4 (3 or 4 notes; 3 gave tune RN +0.6 with the bass change, 4 gave +0.5)",
};

fn full_attack(context: &TransitionContext, params: &[f64]) -> f32 {
    let is_change = context.previous.is_some();
    if is_change && context.slice.is_struck_together(FULL_ATTACK_NOTES.read(params) as usize) {
        1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Reading, Segment};
    use crate::engine::rule::History;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::{Slice, SliceNote};
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn reading(root: &str) -> Reading {
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        let key = Key::parse("C", "major").unwrap();
        Reading { key, chord, tonicized: None, category: Category::Diatonic }
    }

    /// Scores a change from C to G on a slice of the named notes, attacked or held over.
    fn change_on(notes: &[(&str, bool)]) -> f32 {
        let note = |&(name, attacked): &(&str, bool)| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi: 0,
            attacked,
            onset_level: 2,
        };
        let slice = Slice {
            start: 0.0,
            duration: 1.0,
            notes: notes.iter().map(note).collect(),
            measure: 0,
            metric_level: 2,
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
            history: History { arena: &[], slices: &[], last: NONE },
        };
        full_attack(&context, &RULES[0].default_params())
    }

    #[test]
    fn a_change_where_all_voices_start_together_is_favored() {
        assert_eq!(change_on(&[("G", true), ("D", true), ("B", true), ("G", true)]), 1.0);
    }

    #[test]
    fn a_change_where_one_voice_is_held_over_is_not_favored() {
        assert_eq!(change_on(&[("G", true), ("D", true), ("B", true), ("C", false)]), 0.0);
    }

    #[test]
    fn a_change_on_two_notes_struck_together_is_not_favored() {
        assert_eq!(change_on(&[("G", true), ("D", true)]), 0.0);
    }
}
