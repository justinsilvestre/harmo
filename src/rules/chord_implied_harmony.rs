//! Scores chord changes in a single melodic line by the harmony that the line's coming notes imply.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, TEMPERLEY_2001};
use crate::engine::chord_hooks::{ChordHook, ChordTransitionContext};
use crate::engine::rule::{Citation, Hook, Param, Rule, Topic};
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;

pub const RULES: &[Rule] = &[Rule {
    id: "chord.implied_harmony",
    topic: Topic::Chord,
    summary: "Where a single line sounds alone (one pitch class, in unison or octaves), keep the \
              harmony already heard unless the line's notes from here to shortly ahead fit the new \
              chord better than the old. A new segment, even one restating the same chord, costs 1, \
              less the share of the line's sounding time by which the new chord's tones exceed the \
              old chord's.",
    citations: &[
        Citation {
            source: TEMPERLEY_2001,
            locator: "pp. 158–159 (an unaccompanied melody implies harmonies that it does not fully \
                      state; the notes of a line fit several chords, so context decides)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 189 (an arpeggiation moves between two tones of the same chord)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 213 (the harmonies implied by an unaccompanied melodic line)",
        },
    ],
    default_weight: 0.25,
    params: &[LOOKAHEAD],
    hook: Hook::Chord(ChordHook::Transition(implied_harmony)),
}];

/// How far past the start of the slice the line is read.
/// A listener may look ahead about two quarter notes.
const LOOKAHEAD: Param = Param {
    index: 0,
    name: "lookahead",
    default: 2.0,
    unit: "quarter notes",
    provenance: "measured: r7-monody, round 7, on a first form of the rule at weight 0.5 (1 / 1.5 / 2 quarter notes: tune RN +0.12 / +0.14 / +0.16 against round7-base2; 2 is the most the listener may look ahead)",
};

const EPSILON: f64 = 1e-6;

fn implied_harmony(context: &ChordTransitionContext, params: &[f64]) -> f32 {
    let Some(previous) = context.previous else {
        return 0.0;
    };
    if !context.slice.is_single_line() {
        return 0.0;
    }
    let slices = context.history.slices;
    let index = slices.partition_point(|s| s.start < context.slice.start - EPSILON);
    if slices.get(index).is_none_or(|s| (s.start - context.slice.start).abs() > EPSILON) {
        return 0.0;
    }
    let line = line_ahead(slices, index, LOOKAHEAD.read(params));
    let support = line_fit(line.clone(), context.next) - line_fit(line, previous.chord);
    support.clamp(0.0, 1.0) as f32 - 1.0
}

/// Returns the slices of the single line that sounds without interruption from the given slice
/// until `lookahead` quarter notes past its start, each with the time it sounds within that span.
fn line_ahead<'a>(
    slices: &'a [Slice],
    index: usize,
    lookahead: f64,
) -> impl Iterator<Item = (&'a Slice, f64)> + Clone {
    let end = slices[index].start + lookahead;
    let sounds_in_span = move |s: &&Slice| s.start < end - EPSILON && s.is_single_line();
    let timed = move |s: &'a Slice| (s, (s.start + s.duration).min(end) - s.start);
    slices[index..].iter().take_while(sounds_in_span).map(timed)
}

/// Returns the share of the line's sounding time taken by tones of the chord.
fn line_fit<'a>(line: impl Iterator<Item = (&'a Slice, f64)>, chord: Chord) -> f64 {
    let (mut fit, mut total) = (0.0, 0.0);
    for (slice, time) in line {
        total += time;
        if chord.member_index_enharmonic(slice.notes[0].tpc).is_some() {
            fit += time;
        }
    }
    if total > 0.0 { fit / total } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::chord_hooks::ChordHistory;
    use crate::engine::model::ChordSegment;
    use crate::engine::test_layers::chord_segment;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::Quality;

    /// The major triad on a root.
    fn major(root: &str) -> Chord {
        Chord::new(tpc_from_name(root).unwrap(), Quality::Major)
    }

    /// Returns one slice per beat, each sounding the named notes.
    fn slices(beats: &[&[&str]]) -> Vec<Slice> {
        let note = |name: &&str| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi: 60,
            attacked: true,
            onset_level: 2,
        };
        let slice = |(i, names): (usize, &&[&str])| Slice {
            start: i as f64,
            duration: 1.0,
            notes: names.iter().map(note).collect(),
            measure: 0,
            metric_level: 2,
            phrase: 0,
            key_signature: None,
        };
        beats.iter().enumerate().map(slice).collect()
    }

    /// Scores a change from C major to G major on the second of the given beats.
    fn change_from_c_to_g(beats: &[&[&str]]) -> f32 {
        let slices = slices(beats);
        let previous = ChordSegment { members_heard: 0b111, ..chord_segment(major("C")) };
        let context = ChordTransitionContext {
            previous: Some(&previous),
            next: major("G"),
            slice: &slices[1],
            history: ChordHistory::empty(&slices),
        };
        implied_harmony(&context, &RULES[0].default_params())
    }

    #[test]
    fn a_change_the_line_outlines_costs_nothing() {
        assert_eq!(change_from_c_to_g(&[&["C", "E"], &["B"], &["D"], &["F"]]), 0.0);
    }

    #[test]
    fn a_change_the_line_fits_no_better_costs_the_most() {
        assert_eq!(change_from_c_to_g(&[&["C", "E"], &["G"], &["F"], &["A"]]), -1.0);
    }

    #[test]
    fn a_change_the_line_partly_supports_costs_part() {
        assert_eq!(change_from_c_to_g(&[&["C", "E"], &["B"], &["A"], &["C"]]), -0.5);
    }

    #[test]
    fn a_change_on_a_chord_costs_nothing() {
        assert_eq!(change_from_c_to_g(&[&["C", "E"], &["G", "B"], &["F"], &["A"]]), 0.0);
    }
}
