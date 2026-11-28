//! Rules that keep a major triad and the dominant seventh chord on its root as rival chords of one span.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::chord_hooks::{ChordHook, ChordRivalContext, Rival};
use crate::engine::rule::{Citation, Hook, Rule, Topic};
use crate::score::slicing::{Slice, SliceNote};
use crate::theory::chord::{Chord, Quality};
use std::hash::{DefaultHasher, Hash, Hasher};

pub const RULES: &[Rule] = &[Rule {
    id: "chord.seventh_rival",
    topic: Topic::Chord,
    summary: "Keep a major triad and the dominant seventh chord on its root as rival chords of a span in which \
              the seventh sounds for only part of the span and is approached like a nonchord tone: from \
              the same pitch (a suspension figure) or from a step above or below (a passing or neighbor \
              figure). Either chord is then correct, so when the next chord opens, it opens after the \
              best hypothesis with each chord.",
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
    default_weight: 1.0,
    params: &[],
    hook: Hook::Chord(ChordHook::Rival(seventh_rival)),
}];

/// Groups a segment heard as a major triad or dominant seventh chord, by its root,
/// when its seventh sounds for part of the segment and is approached like a nonchord tone.
fn seventh_rival(context: &ChordRivalContext, _: &[f64]) -> Option<Rival> {
    let chord = context.segment.chord;
    if !matches!(chord.quality, Quality::Major | Quality::DominantSeventh) {
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
    chord.root.hash(&mut hasher);
    Some(Rival { group: hasher.finish(), member: chord.quality as u64 })
}

/// Reports whether the note is held over from the slice before,
/// or that slice sounds the same pitch class or a pitch a step above or below it.
fn is_approached_by_step(note: &SliceNote, before: Option<&Slice>) -> bool {
    let is_step = |other: &SliceNote| other.tpc == note.tpc || (other.midi - note.midi).abs() <= 2;
    !note.attacked || before.is_some_and(|slice| slice.notes.iter().any(is_step))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::ChordSegment;
    use crate::engine::test_layers::chord_segment;
    use crate::pitch::tpc_from_name;

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

    /// Groups a segment of the given chord, opening on slice 1, over the given slices,
    /// after the slice `before`.
    fn group(root: &str, quality: Quality, before: &Slice, slices: &[Slice]) -> Option<u64> {
        rival(root, quality, before, slices).map(|rival| rival.group)
    }

    /// Places a segment of the given chord, opening on slice 1, over the given slices,
    /// after the slice `before`.
    fn rival(root: &str, quality: Quality, before: &Slice, slices: &[Slice]) -> Option<Rival> {
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        let segment = ChordSegment { start: 1, duration: 2.0, ..chord_segment(chord) };
        let context = ChordRivalContext { segment: &segment, slices, previous_slice: Some(before) };
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
    fn a_major_triad_and_its_dominant_seventh_chord_share_a_group() {
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
    fn a_major_triad_and_its_dominant_seventh_chord_are_rivals() {
        let rival = |quality: Quality| rival("G", quality, &opening(), &passing_seventh());
        let (triad, seventh) = (rival(Quality::Major), rival(Quality::DominantSeventh));
        assert!(triad.zip(seventh).is_some_and(|(t, s)| t.is_rival_of(s)));
    }

    #[test]
    fn a_minor_triad_has_no_rival() {
        let slices = [slice(&[("G", 43), ("Bb", 58), ("G", 67)]), slice(&[("G", 43), ("Bb", 58), ("F", 65)])];
        assert_eq!(group("G", Quality::Minor, &opening(), &slices), None);
    }
}
