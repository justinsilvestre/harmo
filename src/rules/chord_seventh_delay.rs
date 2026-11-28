//! Rules about a dominant seventh chord whose seventh is first heard only in a later bar.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::rule::{ChordCloseContext, Citation, Hook, Layer, Param, Rule};
use crate::score::slicing::Slice;
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[Rule {
    id: "chord.seventh_delay",
    layer: Layer::Chord,
    summary: "Avoid a dominant seventh chord whose seventh first sounds in a later bar than the one in \
              which the chord begins. The dominant is then heard as a triad for a bar or more, and the \
              seventh as a new event, so the triad and the seventh chord are better read as two chords.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 219 (a 7th that occupies only a portion of the chord's duration may be read as \
                      a 7th chord or as a triad with an NCT; the book reads the whole span as one chord, \
                      so the bar limit follows the annotators rather than the book)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 219 (the 7th originated as a downward-resolving suspension or descending \
                      passing tone)",
        },
    ],
    default_weight: 1.0,
    params: &[MIN_DELAY],
    hook: Hook::ChordClose(seventh_delay),
}];

/// The least time, from the start of the chord, after which a seventh first heard in a later bar costs.
const MIN_DELAY: Param = Param {
    index: 0,
    name: "min_delay",
    default: 1.0,
    unit: "quarter notes",
    provenance: "measured: round 7 (0 / 1 / 2 / 3 / 4: tune RN +0.12 / +0.13 / +0.11 / +0.08 / +0.05, rn eq +0.10 / +0.11 / +0.09 / +0.06 / +0.03 against round7-base2)",
};

/// Returns -1 for a dominant seventh chord whose seventh first sounds in a later bar than the chord's
/// first slice, at least the least delay after the chord begins, and 0 otherwise.
fn seventh_delay(context: &ChordCloseContext, params: &[f64]) -> f32 {
    let chord = context.segment.chord;
    let Some(seventh) = chord.members().nth(3).filter(|_| chord.quality == Quality::DominantSeventh) else {
        return 0.0;
    };
    let slices = context.slices;
    let sounds = |slice: &&Slice| slice.notes.iter().any(|note| note.tpc == seventh);
    let (Some(opening), Some(entry)) = (slices.first(), slices.iter().find(sounds)) else {
        return 0.0;
    };
    let is_late = entry.start - opening.start >= MIN_DELAY.read(params) - 1e-6;
    if entry.measure != opening.measure && is_late { -1.0 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Function, NONE, Reading, Segment};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    /// A two-beat slice in the given bar, sounding the named notes.
    fn slice(start: f64, measure: usize, names: &[&str]) -> Slice {
        let note = |name: &&str| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi: 0,
            attacked: true,
            onset_level: 0,
        };
        let notes = names.iter().map(note).collect();
        Slice { start, duration: 2.0, notes, measure, metric_level: 2, phrase: 0, key_signature: None }
    }

    /// Scores a segment on G in C major over the given slices.
    fn score(quality: Quality, slices: &[Slice]) -> f32 {
        let chord = Chord::new(tpc_from_name("G").unwrap(), quality);
        let key = Key::parse("C", "major").unwrap();
        let reading = Reading { key, chord, function: Function::DIATONIC };
        let duration = slices.iter().map(|s| s.duration).sum();
        let (start, previous, members_heard) = (0, NONE, 0b1111);
        let segment = Segment { reading, start, previous, duration, members_heard, bass: Bass::default() };
        seventh_delay(
            &ChordCloseContext { segment: &segment.chord_segment(NONE), slices, next_slice: None },
            &RULES[0].default_params(),
        )
    }

    fn seventh_in_bar_two() -> Vec<Slice> {
        vec![
            slice(0.0, 0, &["G", "B", "D"]),
            slice(2.0, 0, &["G", "B", "D"]),
            slice(4.0, 1, &["G", "B", "F"]),
        ]
    }

    #[test]
    fn a_dominant_seventh_whose_seventh_waits_for_the_next_bar_costs() {
        assert_eq!(score(Quality::DominantSeventh, &seventh_in_bar_two()), -1.0);
    }

    #[test]
    fn a_dominant_seventh_whose_seventh_enters_within_the_first_bar_costs_nothing() {
        let slices =
            [slice(0.0, 0, &["G", "B", "D"]), slice(2.0, 0, &["G", "B", "F"]), slice(4.0, 1, &["G", "B"])];
        assert_eq!(score(Quality::DominantSeventh, &slices), 0.0);
    }

    #[test]
    fn a_major_triad_is_not_judged() {
        assert_eq!(score(Quality::Major, &seventh_in_bar_two()), 0.0);
    }

    #[test]
    fn a_dominant_seventh_whose_seventh_never_sounds_is_not_judged() {
        let slices = [slice(0.0, 0, &["G", "B", "D"]), slice(4.0, 1, &["G", "B", "D"])];
        assert_eq!(score(Quality::DominantSeventh, &slices), 0.0);
    }
}
