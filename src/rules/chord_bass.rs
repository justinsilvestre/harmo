//! Rules about the bass line within a chord segment.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::chord_hooks::{ChordCloseContext, ChordHook};
use crate::engine::rule::{Citation, Hook, Param, Rule, Topic};
use crate::score::slicing::{Slice, SliceNote};
use crate::theory::chord::Chord;

pub const RULES: &[Rule] = &[Rule {
    id: "chord.bass_change",
    topic: Topic::Chord,
    summary: "Start a new segment when the bass moves to another chord tone on a beat and stays there \
              until the next beat, so that each bass position gets its own inversion. Faster \
              arpeggiations in the bass, and leaps above the bass's register, do not count.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 115 (inversions from bass arpeggiation; faster accompaniment figures are inner voices)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 134 (the real bass note depends on metric placement, duration and register)",
        },
    ],
    default_weight: 0.5,
    params: &[BEAT_LEVEL, BASS_REGISTER],
    hook: Hook::Chord(ChordHook::Close(bass_change)),
}];

/// The weakest metric level at which a bass change counts, and to which the new bass must be held
/// (2 = any beat).
const BEAT_LEVEL: Param = Param {
    index: 0,
    name: "beat_level",
    default: 2.0,
    unit: "metric level",
    provenance: "measured: 9d7b407, round 2 (beat: tune RN +0.67; beat subdivision: -0.0, TAVERN -1.9; half bar: +0.2); measured: sweeps/round7-constants.json, round 7 (1 / 3: tune RN -0.48 / -0.13; kept)",
};
/// The largest upward leap that still counts as the same bass voice.
const BASS_REGISTER: Param = Param {
    index: 1,
    name: "bass_register",
    default: 12.0,
    unit: "semitones",
    provenance: "chosen: 9d7b407, round 2 (an octave, after Kostka, Payne and Almen 2023, p. 134: the real bass depends on register); measured: sweeps/round7-constants.json, round 7 (7 / 9 / 19 / 24: tune RN -0.02 / -0.01 / -0.10 / -0.12; kept)",
};

/// Returns minus the number of bass changes inside the segment.
///
/// The bass is established by the segment's first bass note and by every later bass note that is
/// attacked on a beat and held until the next beat. A change is an established bass on another chord member than the one before it.
fn bass_change(context: &ChordCloseContext, params: &[f64]) -> f32 {
    let beat = BEAT_LEVEL.read(params) as u8;
    let register = BASS_REGISTER.read(params) as i32;
    let chord = context.segment.chord;
    let slices = context.slices;
    let mut established: Option<(usize, i32)> = None;
    let mut changes = 0;
    for (index, slice) in slices.iter().enumerate() {
        let Some((member, note)) = bass_member(slice, chord) else { continue };
        let is_held =
            is_beat_attack(slice, beat) && holds_until_next_beat(&slices[index + 1..], chord, member, beat);
        if established.is_some() && !is_held {
            continue;
        }
        if established.is_some_and(|(old, midi)| old != member && note.midi - midi <= register) {
            changes += 1;
        }
        established = Some((member, note.midi));
    }
    -(changes as f32)
}

/// Returns the chord member in the bass of a slice, with the note, when its lowest note is a chord tone.
fn bass_member(slice: &Slice, chord: Chord) -> Option<(usize, &SliceNote)> {
    let note = slice.bass()?;
    chord.member_index_enharmonic(note.tpc).map(|member| (member, note))
}

fn is_beat_attack(slice: &Slice, beat: u8) -> bool {
    slice.metric_level >= beat && slice.bass().is_some_and(|note| note.attacked)
}

/// Returns whether the bass stays on `member` (or is silent) until the next slice at the given metric level.
fn holds_until_next_beat(following: &[Slice], chord: Chord, member: usize, beat: u8) -> bool {
    let stays =
        |slice: &Slice| slice.notes.is_empty() || bass_member(slice, chord).map(|b| b.0) == Some(member);
    following.iter().take_while(|slice| slice.metric_level < beat).all(stays)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_layers::chord_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Quality;

    /// A slice of one bass note, attacked, at the given metric level.
    fn slice(midi: i32, name: &str, metric_level: u8) -> Slice {
        let tpc = tpc_from_name(name).unwrap();
        let note = SliceNote { tpc, midi, attacked: true, onset_level: metric_level };
        Slice {
            start: 0.0,
            duration: 1.0,
            notes: vec![note],
            measure: 0,
            metric_level,
            phrase: 0,
            key_signature: None,
        }
    }

    /// Scores a C major segment over the given slices.
    fn score(slices: &[Slice]) -> f32 {
        let segment = chord_segment(Chord::new(tpc_from_name("C").unwrap(), Quality::Major));
        bass_change(
            &ChordCloseContext { segment: &segment, slices, next_slice: None },
            &RULES[0].default_params(),
        )
    }

    #[test]
    fn a_bass_moving_to_the_third_on_a_beat_counts_as_a_change() {
        assert_eq!(score(&[slice(48, "C", 4), slice(52, "E", 2)]), -1.0);
    }

    #[test]
    fn a_bass_moving_off_a_beat_is_no_change() {
        assert_eq!(score(&[slice(48, "C", 4), slice(52, "E", 1)]), 0.0);
    }

    #[test]
    fn an_alberti_figure_is_no_change() {
        let figure = [slice(48, "C", 4), slice(55, "G", 1), slice(52, "E", 2), slice(55, "G", 1)];
        assert_eq!(score(&figure), 0.0);
    }

    #[test]
    fn a_leap_above_the_bass_register_is_no_change() {
        assert_eq!(score(&[slice(48, "C", 4), slice(64, "E", 2)]), 0.0);
    }
}
