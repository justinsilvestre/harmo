//! The bass note of a chord segment, from which its inversion is read.
//!
//! The bass is the lowest chord tone, except when the lowest tone is a fifth that is never accented
//! while the root, at most an octave above, is. A bass note is accented when it is the lowest note
//! on a downbeat or where the chord begins. The fifth then belongs to a bass arpeggiation over the
//! root rather than making a six-four chord (Kostka, Payne and Almen 2023, p. 115: the bass note
//! struck with the chord is the primary one; p. 134: the real bass note depends on metric placement,
//! duration and register).

use crate::pitch::Tpc;
use crate::theory::chord::Chord;

/// The metric level of a downbeat.
const DOWNBEAT: u8 = 4;
/// The largest distance, in semitones, at which a root above the lowest fifth still holds the bass.
const BASS_REGISTER: i32 = 12;
/// The chord member index of the fifth.
const FIFTH: usize = 2;

/// The bass evidence gathered while a segment is extended slice by slice.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bass {
    /// The lowest chord tone heard so far, as (MIDI number, spelling).
    pub lowest: Option<(i32, Tpc)>,
    /// Whether the lowest chord tone has been an accented bass note.
    pub lowest_accented: bool,
    /// The lowest root that has been an accented bass note.
    pub accented_root: Option<(i32, Tpc)>,
}

/// A chord tone sounding in a slice.
pub struct BassNote {
    pub midi: i32,
    pub tpc: Tpc,
    /// The chord member: 0 = root, 1 = third, 2 = fifth, 3 = seventh.
    pub member: usize,
    /// Whether the note is the lowest note of a slice that starts on a downbeat or opens the segment.
    pub is_accented_bass: bool,
}

impl Bass {
    /// Returns the bass evidence after hearing one more chord tone.
    pub fn hear(mut self, note: &BassNote) -> Bass {
        if self.lowest.is_none_or(|(midi, _)| note.midi < midi) {
            self.lowest = Some((note.midi, note.tpc));
            self.lowest_accented = note.is_accented_bass;
        } else if self.lowest.is_some_and(|(midi, _)| midi == note.midi) && note.is_accented_bass {
            self.lowest_accented = true;
        }
        let is_lower_root = self.accented_root.is_none_or(|(midi, _)| note.midi < midi);
        if note.member == 0 && note.is_accented_bass && is_lower_root {
            self.accented_root = Some((note.midi, note.tpc));
        }
        self
    }

    /// Returns the bass note that gives the segment's inversion.
    pub fn note(&self, chord: Chord) -> Option<(i32, Tpc)> {
        let (Some((low, low_tpc)), Some(root)) = (self.lowest, self.accented_root) else {
            return self.lowest;
        };
        let is_offbeat_fifth = !self.lowest_accented && chord.member_index(low_tpc) == Some(FIFTH);
        if is_offbeat_fifth && root.0 - low <= BASS_REGISTER { Some(root) } else { self.lowest }
    }
}

/// Returns whether a slice's lowest note is accented: the slice starts on a downbeat or opens the segment.
pub fn is_accented(metric_level: u8, opens_segment: bool) -> bool {
    opens_segment || metric_level >= DOWNBEAT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Quality;

    fn c_major() -> Chord {
        Chord::new(tpc_from_name("C").unwrap(), Quality::Major)
    }

    fn note(midi: i32, name: &str, is_accented_bass: bool) -> BassNote {
        let tpc = tpc_from_name(name).unwrap();
        BassNote { midi, tpc, member: c_major().member_index(tpc).unwrap(), is_accented_bass }
    }

    fn bass_of(notes: &[BassNote]) -> Option<i32> {
        let bass = notes.iter().fold(Bass::default(), |bass, n| bass.hear(n));
        bass.note(c_major()).map(|(midi, _)| midi)
    }

    #[test]
    fn the_slice_opening_a_segment_is_accented_off_the_downbeat() {
        assert!(is_accented(2, true));
    }

    #[test]
    fn a_later_slice_off_the_downbeat_is_not_accented() {
        assert!(!is_accented(3, false));
    }

    #[test]
    fn an_offbeat_fifth_below_a_accented_root_yields_to_the_root() {
        assert_eq!(bass_of(&[note(48, "C", true), note(43, "G", false)]), Some(48));
    }

    #[test]
    fn a_fifth_on_a_downbeat_is_the_bass() {
        assert_eq!(bass_of(&[note(48, "C", true), note(43, "G", true)]), Some(43));
    }

    #[test]
    fn a_fifth_is_the_bass_when_no_root_falls_on_a_downbeat() {
        assert_eq!(bass_of(&[note(48, "C", false), note(43, "G", false)]), Some(43));
    }

    #[test]
    fn a_root_more_than_an_octave_above_does_not_hold_the_bass() {
        assert_eq!(bass_of(&[note(60, "C", true), note(43, "G", false)]), Some(43));
    }

    #[test]
    fn an_offbeat_third_stays_the_bass() {
        assert_eq!(bass_of(&[note(48, "C", true), note(40, "E", false)]), Some(40));
    }
}
