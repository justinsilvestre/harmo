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
              arpeggiations in the bass, and leaps above the bass's register, do not count, except \
              that a move at the half beat right after a beat on which four or more voices were struck \
              together counts: in such a texture the moving note is the bass voice itself, not an \
              accompaniment figure.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 115 (inversions from bass arpeggiation; faster accompaniment figures are inner voices)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 134 (the real bass note depends on metric placement, duration and register)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 200 (a one-to-one framework, a chord struck in all voices on each beat, is \
                      embellished by weak-beat arpeggiations in single voices)",
        },
    ],
    default_weight: 0.5,
    hook: Hook::Close(bass_change),
}];

/// The weakest metric level at which a bass change counts (2 = any beat).
const BEAT: u8 = 2;
/// The metric level of a beat subdivision.
const SUBDIVISION: u8 = 1;
/// The largest upward leap, in semitones, that still counts as the same bass voice.
const BASS_REGISTER: i32 = 12;
/// The fewest voices struck together on a beat for a bass move at the half beat after it to count.
const FULL_TEXTURE_NOTES: usize = 4;

/// Returns minus the number of bass changes inside the segment.
///
/// The bass is established by the segment's first bass note and by every later bass note that is
/// attacked on a beat, or at the subdivision right after a beat struck by the full texture, and held
/// until the next beat. A change is an established bass on another chord member than the one before it.
fn bass_change(context: &CloseContext) -> f32 {
    let chord = context.segment.reading.chord;
    let slices = context.slices;
    let mut established: Option<(usize, i32)> = None;
    let mut changes = 0;
    for (index, slice) in slices.iter().enumerate() {
        let Some((member, note)) = bass_member(slice, chord) else { continue };
        let is_attack =
            is_beat_attack(slice) || index > 0 && is_full_texture_follow_up(&slices[index - 1], slice);
        let is_held = is_attack && holds_until_next_beat(&slices[index + 1..], chord, member);
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

/// Returns whether the slice attacks a bass note at the subdivision right after a beat on which the
/// whole texture of four or more voices was struck together.
fn is_full_texture_follow_up(previous: &Slice, slice: &Slice) -> bool {
    let is_full_beat = previous.metric_level >= BEAT && previous.is_struck_together(FULL_TEXTURE_NOTES);
    is_full_beat && slice.metric_level == SUBDIVISION && slice.bass().is_some_and(|note| note.attacked)
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

    /// A slice of a C major chord struck in the given number of voices over C3, with the bass attacked.
    fn struck_chord(voices: usize, metric_level: u8) -> Slice {
        let pitches = [(48, "C"), (55, "G"), (64, "E"), (72, "C")];
        let notes = pitches[..voices].iter().map(|&(midi, name)| slice(midi, name, metric_level).notes[0]);
        Slice { notes: notes.collect(), ..slice(48, "C", metric_level) }
    }

    /// A slice in which only the bass, on E2, is attacked under the upper voices of `chord` held over.
    fn lone_bass_e(chord: &Slice, metric_level: u8) -> Slice {
        let held = chord.notes[1..].iter().map(|note| SliceNote { attacked: false, ..*note });
        let bass = slice(40, "E", metric_level).notes[0];
        Slice { notes: std::iter::once(bass).chain(held).collect(), ..slice(40, "E", metric_level) }
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
    fn a_bass_moving_at_the_half_beat_after_four_voices_struck_together_counts_as_a_change() {
        let chord = struck_chord(4, 2);
        let bass = lone_bass_e(&chord, 1);
        assert_eq!(score(&[chord, bass]), -1.0);
    }

    #[test]
    fn a_bass_moving_at_the_half_beat_after_three_voices_is_no_change() {
        let chord = struck_chord(3, 2);
        let bass = lone_bass_e(&chord, 1);
        assert_eq!(score(&[chord, bass]), 0.0);
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
