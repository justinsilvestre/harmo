//! Rules that keep a dominant triad and its dominant seventh chord as rival readings of one span.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::key_hooks::{KeyHook, RivalContext};
use crate::engine::rule::{Citation, Hook, Layer, Rule};
use crate::score::slicing::{Slice, SliceNote};
use crate::theory::chord::{Chord, Quality};
use std::hash::{DefaultHasher, Hash, Hasher};

pub const RULES: &[Rule] = &[Rule {
    id: "chord.seventh_rival",
    layer: Layer::Chord,
    summary: "Keep a dominant triad and its dominant seventh chord as rival readings of a span in which \
              the seventh sounds for only part of the span and is approached like a nonchord tone: from \
              the same pitch (a suspension figure) or from a step above or below (a passing or neighbor \
              figure). Either reading is then correct, so when the next chord opens, it opens after the \
              best hypothesis with each reading.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 219 (a 7th that occupies only a portion of the chord's duration may be labeled \
                      as a 7th chord or as a triad with an NCT; either analysis is correct)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 220 (the approach to the 7th as a suspension, passing tone or neighbor tone \
                      figure, or else an appoggiatura figure)",
        },
    ],
    // Off by default: with four readings offered, its rival readings take the places of readings in
    // other keys, so both keys are offered less often where analysts disagree on the key.
    default_weight: 0.0,
    params: &[],
    hook: Hook::Key(KeyHook::Rival(seventh_rival)),
}];

/// Groups a segment read as a dominant triad or dominant seventh chord, by its key, tonicized key
/// and root, when its seventh sounds for part of the segment and is approached like a nonchord tone.
fn seventh_rival(context: &RivalContext, _: &[f64]) -> Option<u64> {
    let reading = context.segment.reading();
    let chord = reading.chord;
    let is_dominant = reading.function.tonicized.unwrap_or(reading.key).degree_of(chord.root).number == 5;
    if !matches!(chord.quality, Quality::Major | Quality::DominantSeventh) || !is_dominant {
        return None;
    }
    let seventh = Chord::new(chord.root, Quality::DominantSeventh).members().nth(3)?;
    let note_in = |slice: &Slice| slice.notes.iter().find(|note| note.tpc == seventh).copied();
    let slices = context.slices;
    let entry = slices.iter().position(|slice| note_in(slice).is_some())?;
    let is_partial = slices.iter().any(|slice| note_in(slice).is_none());
    let before = if entry == 0 { context.previous_slice } else { Some(&slices[entry - 1]) };
    let note = note_in(&slices[entry])?;
    if !is_partial || !is_approached_by_step(&note, before) {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    (reading.key, reading.function.tonicized, chord.root).hash(&mut hasher);
    Some(hasher.finish())
}

/// Reports whether the note is held over from the slice before, or that slice sounds the same pitch
/// class or a pitch a step above or below it.
fn is_approached_by_step(note: &SliceNote, before: Option<&Slice>) -> bool {
    let is_step = |other: &SliceNote| other.tpc == note.tpc || (other.midi - note.midi).abs() <= 2;
    !note.attacked || before.is_some_and(|slice| slice.notes.iter().any(is_step))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{ChordSegment, Function, KeySegment, Reading};
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::key::Key;

    /// A one-beat slice sounding the named notes at the given MIDI numbers, all attacked.
    fn slice(notes: &[(&str, i32)]) -> Slice {
        let note = |&(name, midi): &(&str, i32)| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi,
            attacked: true,
            onset_level: 0,
        };
        let notes = notes.iter().map(note).collect();
        Slice {
            start: 0.0,
            duration: 1.0,
            notes,
            measure: 0,
            metric_level: 2,
            phrase: 0,
            key_signature: None,
        }
    }

    /// Groups a segment on G in C major over the given slices, after the slice `before`.
    fn group(root: &str, quality: Quality, before: &Slice, slices: &[Slice]) -> Option<u64> {
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        let key = Key::parse("C", "major").unwrap();
        let segment = key_segment(Reading { key, chord, function: Function::DIATONIC });
        let segment =
            KeySegment { chord: ChordSegment { start: 1, duration: 2.0, ..segment.chord }, ..segment };
        let context = RivalContext { segment: &segment, slices, previous_slice: Some(before) };
        seventh_rival(&context, &[])
    }

    /// G major, then G with a passing F reached from G above it.
    fn passing_seventh() -> Vec<Slice> {
        vec![slice(&[("G", 43), ("B", 59), ("G", 67)]), slice(&[("G", 43), ("B", 59), ("F", 65)])]
    }

    fn opening() -> Slice {
        slice(&[("C", 48), ("E", 64), ("G", 67)])
    }

    #[test]
    fn a_dominant_triad_and_its_seventh_chord_share_a_group() {
        let triad = group("G", Quality::Major, &opening(), &passing_seventh());
        let seventh = group("G", Quality::DominantSeventh, &opening(), &passing_seventh());
        assert!(triad.is_some() && triad == seventh);
    }

    #[test]
    fn a_seventh_reached_by_leap_has_no_rival() {
        let slices = [slice(&[("G", 43), ("B", 59), ("D", 62)]), slice(&[("G", 43), ("B", 59), ("F", 65)])];
        assert_eq!(group("G", Quality::Major, &opening(), &slices), None);
    }

    #[test]
    fn a_seventh_sounding_throughout_has_no_rival() {
        let slices = [slice(&[("G", 43), ("F", 65)]), slice(&[("G", 43), ("F", 65)])];
        assert_eq!(group("G", Quality::Major, &slice(&[("G", 67)]), &slices), None);
    }

    #[test]
    fn a_chord_that_is_not_the_dominant_has_no_rival() {
        let slices =
            [slice(&[("F", 41), ("A", 57)]), slice(&[("F", 41), ("E", 64)]), slice(&[("F", 41), ("Eb", 63)])];
        assert_eq!(group("F", Quality::Major, &opening(), &slices), None);
    }
}
