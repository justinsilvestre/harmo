//! Rules about where a dominant seventh chord begins when its seventh enters after the chord.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, SCHULTZ_ET_AL_2018};
use crate::engine::rule::{Citation, CloseContext, Hook, Layer, Param, Rule};
use crate::pitch::Tpc;
use crate::score::slicing::Slice;
use crate::theory::chord::{Chord, Quality};

pub const RULES: &[Rule] = &[Rule {
    id: "chord.seventh_arrival",
    layer: Layer::Chord,
    summary: "Start a new segment where the minor seventh above a major triad's root enters at the half \
              beat right after a beat on which four or more voices sound, attacked alone and held to \
              the end of the chord. In such a texture the annotators of the Bach chorales label the \
              chord as a triad until the seventh enters and as a dominant seventh chord from there on. \
              A triad that lets the seventh enter inside it costs; a dominant seventh chord that does \
              costs half as much, since reading the whole span as a seventh chord is also correct.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 200 (a one-to-one framework, a chord struck in all voices on each beat, is \
                      embellished by weak-beat passing tones and arpeggiations in single voices)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 219 (the 7th originated as a downward-resolving suspension or descending passing \
                      tone; a 7th that occupies part of the chord may be read either way, and the book \
                      calls labeling both V and V7 unnecessary, so the split follows the chorale \
                      annotators rather than the book)",
        },
        Citation {
            source: SCHULTZ_ET_AL_2018,
            locator: "pp. 68–69 (passing tones in chorales may each be read as sevenths or not; chordal \
                      7ths are nearly always treated like dissonances)",
        },
    ],
    default_weight: 0.5,
    params: &[FULL_TEXTURE_NOTES, SEVENTH_CHORD_FACTOR],
    hook: Hook::Close(seventh_arrival),
}];

/// The weakest metric level of a beat.
const BEAT: u8 = 2;
/// The metric level of a beat subdivision.
const SUBDIVISION: u8 = 1;
/// The fewest voices sounding on the beat before the seventh enters.
const FULL_TEXTURE_NOTES: Param = Param {
    index: 0,
    name: "full_texture_notes",
    default: 4.0,
    unit: "notes",
    provenance: "measured: 8feac94, round 5 (four notes sounding against four struck together: tune RN +0.08 against +0.05)",
};
/// The minor seventh above the root, in fifths.
const MINOR_SEVENTH: i32 = -2;
/// The cost for a dominant seventh chord, against 1 for a major triad. A dominant seventh chord
/// over the whole span is a correct reading too.
const SEVENTH_CHORD_FACTOR: Param = Param {
    index: 1,
    name: "seventh_chord_factor",
    default: 0.5,
    unit: "ratio",
    provenance: "measured: 8feac94, round 5 (0 / 0.25 / 0.5 / 0.75 / 1: tune RN +0.08 / +0.08 / +0.08 / +0.22 / +0.22, both offered 20.67 / 20.77 / 20.80 / 20.04 / 20.01)",
};

/// Returns minus the cost of a segment inside which the seventh arrives, or 0 when it arrives where
/// the segment opens or not at all.
fn seventh_arrival(context: &CloseContext, params: &[f64]) -> f32 {
    let chord = context.segment.reading.chord;
    let Some(factor) = cost_factor(chord, SEVENTH_CHORD_FACTOR.read(params) as f32) else { return 0.0 };
    let full_texture = FULL_TEXTURE_NOTES.read(params) as usize;
    let seventh = chord.root + MINOR_SEVENTH;
    let slices = context.slices;
    let arrives = (1..slices.len()).any(|index| is_arrival(&slices[index - 1..], seventh, full_texture));
    if arrives { -factor } else { 0.0 }
}

/// Returns how much an arrival inside the segment costs for the chord, if the rule judges it.
fn cost_factor(chord: Chord, seventh_chord_factor: f32) -> Option<f32> {
    match chord.quality {
        Quality::Major => Some(1.0),
        Quality::DominantSeventh => Some(seventh_chord_factor),
        _ => None,
    }
}

/// Reports whether `slices` open with a beat on which the full texture sounds without the seventh,
/// followed at the subdivision by the seventh attacked alone and held through the remaining slices.
fn is_arrival(slices: &[Slice], seventh: Tpc, full_texture: usize) -> bool {
    let [beat, arrival, held @ ..] = slices else { return false };
    let is_full_beat = beat.metric_level >= BEAT && beat.notes.len() >= full_texture;
    let is_entry = !beat.notes.iter().any(|note| note.tpc == seventh) && arrival.metric_level == SUBDIVISION;
    let Some((midi, _)) = lone_attack(arrival).filter(|&(_, tpc)| tpc == seventh) else {
        return false;
    };
    let holds = |slice: &Slice| slice.notes.iter().any(|note| !note.attacked && note.midi == midi);
    is_full_beat && is_entry && held.iter().all(holds)
}

/// Returns the one note attacked in a slice, as (MIDI number, spelling), when exactly one is.
fn lone_attack(slice: &Slice) -> Option<(i32, Tpc)> {
    let mut attacked = slice.notes.iter().filter(|note| note.attacked);
    let note = attacked.next()?;
    attacked.next().is_none().then_some((note.midi, note.tpc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Reading, Segment};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::key::Key;

    /// A G major chord over G2, in up to four voices.
    const G_MAJOR: [(i32, &str); 4] = [(43, "G"), (59, "B"), (62, "D"), (67, "G")];

    fn note(midi: i32, name: &str, attacked: bool) -> SliceNote {
        SliceNote { tpc: tpc_from_name(name).unwrap(), midi, attacked, onset_level: 0 }
    }

    fn slice(notes: Vec<SliceNote>, metric_level: u8) -> Slice {
        Slice { start: 0.0, duration: 0.5, notes, measure: 0, metric_level, phrase: 0, key_signature: None }
    }

    fn struck_g(voices: usize) -> Slice {
        slice(G_MAJOR[..voices].iter().map(|&(midi, name)| note(midi, name, true)).collect(), 2)
    }

    /// The chord of `beat` held over, with F4 attacked or held in place of its top voice.
    fn with_f(beat: &Slice, metric_level: u8, attacked: bool) -> Slice {
        let held = beat.notes[..beat.notes.len() - 1].iter().map(|n| SliceNote { attacked: false, ..*n });
        slice(held.chain(std::iter::once(note(65, "F", attacked))).collect(), metric_level)
    }

    /// The usual arrival: G major struck in `voices` voices, then F4 alone at the half beat, held on.
    fn arrival(voices: usize) -> Vec<Slice> {
        let beat = struck_g(voices);
        let (entry, held) = (with_f(&beat, 1, true), with_f(&beat, 2, false));
        vec![beat, entry, held]
    }

    fn score(quality: Quality, slices: &[Slice]) -> f32 {
        let chord = Chord::new(tpc_from_name("G").unwrap(), quality);
        let key = Key::parse("C", "major").unwrap();
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        let (start, previous, duration, members_heard) = (0, NONE, 1.5, 0b1111);
        let segment = Segment { reading, start, previous, duration, members_heard, bass: Bass::default() };
        seventh_arrival(
            &CloseContext { segment: &segment, slices, next_slice: None },
            &RULES[0].default_params(),
        )
    }

    #[test]
    fn a_dominant_seventh_whose_seventh_arrives_inside_it_costs_half() {
        assert_eq!(score(Quality::DominantSeventh, &arrival(4)), -0.5);
    }

    #[test]
    fn a_major_triad_whose_seventh_arrives_inside_it_costs() {
        assert_eq!(score(Quality::Major, &arrival(4)), -1.0);
    }

    #[test]
    fn a_seventh_arriving_after_a_beat_with_one_voice_held_over_costs() {
        let mut slices = arrival(4);
        slices[0].notes[3].attacked = false;
        assert_eq!(score(Quality::Major, &slices), -1.0);
    }

    #[test]
    fn a_minor_triad_is_not_judged() {
        assert_eq!(score(Quality::Minor, &arrival(4)), 0.0);
    }

    #[test]
    fn a_dominant_seventh_opening_where_its_seventh_arrives_costs_nothing() {
        assert_eq!(score(Quality::DominantSeventh, &arrival(4)[1..]), 0.0);
    }

    #[test]
    fn a_seventh_arriving_after_three_voices_costs_nothing() {
        assert_eq!(score(Quality::Major, &arrival(3)), 0.0);
    }

    #[test]
    fn a_seventh_left_before_the_segment_ends_costs_nothing() {
        let mut slices = arrival(4);
        slices.push(struck_g(4));
        assert_eq!(score(Quality::Major, &slices), 0.0);
    }

    #[test]
    fn a_seventh_attacked_with_another_voice_costs_nothing() {
        let mut slices = arrival(4);
        slices[1].notes[0].attacked = true;
        assert_eq!(score(Quality::Major, &slices), 0.0);
    }

    #[test]
    fn a_seventh_arriving_on_a_beat_costs_nothing() {
        let mut slices = arrival(4);
        slices[1].metric_level = 2;
        assert_eq!(score(Quality::Major, &slices), 0.0);
    }
}
