//! The rule that decides which note is a chord segment's bass, and so its inversion.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::bass::{Bass, BassNote};
use crate::engine::chord_hooks::ChordHook;
use crate::engine::rule::{Citation, Hook, Layer, Param, Rule};
use crate::pitch::Tpc;

pub const RULES: &[Rule] = &[Rule {
    id: "chord.bass_note",
    layer: Layer::Chord,
    summary: "Take the lowest chord tone of a segment as its bass, except when that tone is the fifth, \
              never sounds as an accented bass note, and a root that does lies at most an octave above \
              it: the fifth then belongs to an arpeggiation over the root, and the root is the bass. \
              A bass note is accented when it is the lowest note of a slice that starts on a downbeat \
              or opens the segment.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "ch. 3, pp. 38–44 (a chord's inversion is named by the chord member in the bass)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 115 (the bass note struck with the chord is the primary one; later arpeggiated \
                      bass notes give variety)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 134 (the real bass note depends on metric placement, duration and register)",
        },
    ],
    default_weight: 1.0,
    params: &[ACCENT_LEVEL, ROOT_REGISTER],
    hook: Hook::Chord(ChordHook::Bass(hear)),
}];

/// The weakest metric level on which a slice's lowest note counts as an accented bass note.
const ACCENT_LEVEL: Param = Param {
    index: 0,
    name: "accent_level",
    default: 4.0,
    unit: "metric level",
    provenance: "measured: 4840b86, round 3 (downbeats against downbeats and mid-measure beats: keyboard \
                 other -3.2); measured: round 7 against round7-base2, see the commit adding this rule (3 / 2 / 1 / segment openings only: \
                 tune RN +0.01 / +0.01 / -0.26 / -0.03, inversion +0.02 / -0.04 / -0.42 / -0.04; kept)",
};
/// The largest distance, in semitones, at which an accented root above the lowest fifth holds the bass.
const ROOT_REGISTER: Param = Param {
    index: 1,
    name: "root_register",
    default: 12.0,
    unit: "semitones",
    provenance: "chosen: 4840b86, round 3 (an octave, after Kostka, Payne and Almen 2023, p. 134: the real \
                 bass depends on register); measured: round 7 against round7-base2, see the commit adding this rule (7 / 24: tune RN +0.00 \
                 / +0.05 [-0.00, +0.12], inversion +0.00 / +0.10; no exception: RN -0.44, inversion -0.77; kept)",
};
/// The chord member index of the root and of the fifth.
const ROOT: usize = 0;
const FIFTH: usize = 2;

/// Records one more chord tone and decides the bass from the evidence so far.
fn hear(mut bass: Bass, note: &BassNote, params: &[f64]) -> Bass {
    let accent_level = ACCENT_LEVEL.read(params) as u8;
    let register = ROOT_REGISTER.read(params) as i32;
    let is_accented = note.is_slice_bass && (note.opens_segment || note.metric_level >= accent_level);
    if bass.lowest.is_none_or(|(midi, _)| note.midi < midi) {
        bass.lowest = Some((note.midi, note.tpc));
        bass.lowest_member = note.member;
        bass.lowest_accented = is_accented;
    } else if bass.lowest.is_some_and(|(midi, _)| midi == note.midi) && is_accented {
        bass.lowest_accented = true;
    }
    let is_lower_root = bass.accented_root.is_none_or(|(midi, _)| note.midi < midi);
    if note.member == ROOT && is_accented && is_lower_root {
        bass.accented_root = Some((note.midi, note.tpc));
    }
    bass.note = bass_note(&bass, register);
    bass
}

/// Returns the lowest chord tone, or the accented root when the lowest tone is an unaccented fifth
/// within `register` semitones below it.
fn bass_note(bass: &Bass, register: i32) -> Option<(i32, Tpc)> {
    let (Some((low, _)), Some(root)) = (bass.lowest, bass.accented_root) else {
        return bass.lowest;
    };
    let is_offbeat_fifth = !bass.lowest_accented && bass.lowest_member == FIFTH;
    if is_offbeat_fifth && root.0 - low <= register { Some(root) } else { bass.lowest }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};

    fn c_major() -> Chord {
        Chord::new(tpc_from_name("C").unwrap(), Quality::Major)
    }

    /// A chord tone of C major that is its slice's bass, on a downbeat or not.
    fn note(midi: i32, name: &str, on_downbeat: bool) -> BassNote {
        let tpc = tpc_from_name(name).unwrap();
        let member = c_major().member_index(tpc).unwrap();
        let metric_level = if on_downbeat { 4 } else { 1 };
        BassNote { midi, tpc, member, is_slice_bass: true, metric_level, opens_segment: false }
    }

    fn bass_of(notes: &[BassNote]) -> Option<i32> {
        let params = RULES[0].default_params();
        let bass = notes.iter().fold(Bass::default(), |bass, n| hear(bass, n, &params));
        bass.note().map(|(midi, _)| midi)
    }

    #[test]
    fn the_note_opening_a_segment_is_accented_off_the_downbeat() {
        let opening = BassNote { opens_segment: true, ..note(48, "C", false) };
        assert_eq!(bass_of(&[opening, note(43, "G", false)]), Some(48));
    }

    #[test]
    fn an_offbeat_fifth_below_an_accented_root_yields_to_the_root() {
        assert_eq!(bass_of(&[note(48, "C", true), note(43, "G", false)]), Some(48));
    }

    #[test]
    fn a_fifth_on_a_downbeat_is_the_bass() {
        assert_eq!(bass_of(&[note(48, "C", true), note(43, "G", true)]), Some(43));
    }

    #[test]
    fn a_fifth_is_the_bass_when_no_root_is_accented() {
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

    #[test]
    fn a_note_above_the_slice_bass_is_not_accented() {
        let inner = BassNote { is_slice_bass: false, ..note(48, "C", true) };
        assert_eq!(bass_of(&[inner, note(43, "G", false)]), Some(43));
    }
}
