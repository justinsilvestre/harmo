//! Rules about the bass line within a chord segment.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::rule::{Citation, CloseContext, Hook, Layer, Rule};
use crate::score::slicing::{Slice, SliceNote};
use crate::theory::chord::Chord;

pub const RULES: &[Rule] = &[Rule {
    id: "chord.bass_change",
    layer: Layer::Chord,
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
    hook: Hook::Close(bass_change),
}];

/// The weakest metric level at which a bass change counts (2 = any beat).
const BEAT: u8 = 2;
/// The largest upward leap, in semitones, that still counts as the same bass voice.
const BASS_REGISTER: i32 = 12;

/// Returns minus the number of bass changes inside the segment.
///
/// The bass is established by the segment's first bass note and by every later bass note that is
/// attacked on a beat and held until the next beat. A change is an established bass on another
/// chord member than the one before it.
fn bass_change(context: &CloseContext) -> f32 {
    let chord = context.segment.reading.chord;
    let slices = context.slices;
    let mut established: Option<(usize, i32)> = None;
    let mut changes = 0;
    for (index, slice) in slices.iter().enumerate() {
        let Some((member, note)) = bass_member(slice, chord) else { continue };
        let is_held = is_beat_attack(slice) && holds_until_next_beat(&slices[index + 1..], chord, member);
        if established.is_some() && !is_held {
            continue;
        }
        if established.is_some_and(|(old, midi)| old != member && note.midi - midi <= BASS_REGISTER) {
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

fn is_beat_attack(slice: &Slice) -> bool {
    slice.metric_level >= BEAT && slice.bass().is_some_and(|note| note.attacked)
}

/// Returns whether the bass stays on `member` (or is silent) until the next beat.
fn holds_until_next_beat(following: &[Slice], chord: Chord, member: usize) -> bool {
    let stays =
        |slice: &Slice| slice.notes.is_empty() || bass_member(slice, chord).map(|b| b.0) == Some(member);
    following.iter().take_while(|slice| slice.metric_level < BEAT).all(stays)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Reading, Segment};
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Quality;
    use crate::theory::key::Key;

    /// A slice of one bass note, attacked, at the given metric level.
    fn slice(midi: i32, name: &str, metric_level: u8) -> Slice {
        let tpc = tpc_from_name(name).unwrap();
        let note = SliceNote { tpc, midi, attacked: true, onset_level: metric_level };
        Slice { start: 0.0, duration: 1.0, notes: vec![note], measure: 0, metric_level, phrase: 0 }
    }

    /// Scores a C major segment over the given slices.
    fn score(slices: &[Slice]) -> f32 {
        let chord = Chord::new(tpc_from_name("C").unwrap(), Quality::Major);
        let key = Key::parse("C", "major").unwrap();
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        let segment = Segment {
            reading,
            start: 0,
            previous: NONE,
            duration: 1.0,
            members_heard: 0,
            bass: Bass::default(),
        };
        bass_change(&CloseContext { segment: &segment, slices, next_slice: None })
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
