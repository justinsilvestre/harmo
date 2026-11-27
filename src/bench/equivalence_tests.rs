use super::*;
use crate::pitch::{Tpc, tpc_from_name};
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::Key;

fn tpc(name: &str) -> Tpc {
    tpc_from_name(name).unwrap()
}

fn in_key(key: &str, tonicized: Option<&str>, (root, quality): (&str, Quality), bass: &str) -> Label {
    let key_named = |name: &str| Key::parse(name, "major").unwrap();
    Label::new(key_named(key), tonicized.map(key_named), Chord::new(tpc(root), quality), tpc(bass))
}

/// A label in C major.
fn label(chord: (&str, Quality), bass: &str) -> Label {
    in_key("C", None, chord, bass)
}

const I: (&str, Quality) = ("C", Quality::Major);
const IV: (&str, Quality) = ("F", Quality::Major);
const V: (&str, Quality) = ("G", Quality::Major);
const V7: (&str, Quality) = ("G", Quality::DominantSeventh);
const VIIO: (&str, Quality) = ("B", Quality::Diminished);

/// An expert analysis with the given labels, each lasting the given number of frames, over a
/// texture whose lowest note is `lowest` and which sounds `sounding` throughout.
struct Fixture {
    labels: Vec<Option<Label>>,
    sounds: FrameSounds,
}

impl Fixture {
    fn new(spans: &[(Label, usize)], lowest: &str, sounding: &[&str]) -> Fixture {
        let labels: Vec<Option<Label>> =
            spans.iter().flat_map(|&(label, frames)| std::iter::repeat_n(Some(label), frames)).collect();
        let mask = sounding.iter().fold(0, |mask, name| mask | 1 << pitch_class(tpc(name)));
        let lowest = Some(pitch_class(tpc(lowest)));
        let sounds = FrameSounds { sounding: vec![mask; labels.len()], lowest: vec![lowest; labels.len()] };
        Fixture { labels, sounds }
    }

    /// Returns how the label matches at the frame, with a timing reach of `reach` frames.
    fn credit(&self, label: Label, frame: usize, reach: usize) -> Option<Credit> {
        let equivalents = ExpertEquivalents::new(&self.labels, &self.sounds);
        let view =
            ExpertView { labels: &self.labels, equivalents: &equivalents, sounds: &self.sounds, reach };
        view.credit(&label, frame)
    }
}

fn dominant_then_seventh() -> Fixture {
    Fixture::new(&[(label(V, "G"), 4), (label(V7, "G"), 4)], "G", &["G", "B", "D", "F"])
}

#[test]
fn an_exact_match_is_credited_as_exact() {
    assert_eq!(dominant_then_seventh().credit(label(V, "G"), 1, 0), Some(Credit::Exact));
}

#[test]
fn the_seventh_chord_counts_over_the_triad_before_it() {
    assert_eq!(dominant_then_seventh().credit(label(V7, "G"), 1, 0), Some(Credit::Seventh));
}

#[test]
fn the_triad_counts_over_the_seventh_chord_after_it() {
    assert_eq!(dominant_then_seventh().credit(label(V, "G"), 6, 0), Some(Credit::Seventh));
}

#[test]
fn only_the_experts_own_labels_count_over_the_pair() {
    assert_eq!(dominant_then_seventh().credit(label(V7, "B"), 1, 0), None);
}

#[test]
fn a_seventh_chord_does_not_count_over_a_triad_on_another_root() {
    let fixture = Fixture::new(&[(label(V, "G"), 4), (label(I, "C"), 4)], "G", &["G", "B", "D"]);
    assert_eq!(fixture.credit(label(V7, "G"), 1, 0), None);
}

fn tonic_then_dominant() -> Fixture {
    Fixture::new(&[(label(I, "C"), 4), (label(V, "G"), 4)], "C", &["C", "E", "G"])
}

#[test]
fn a_label_within_the_reach_of_the_experts_counts_as_timing() {
    assert_eq!(tonic_then_dominant().credit(label(V, "G"), 3, 2), Some(Credit::Timing));
}

#[test]
fn a_label_beyond_the_reach_does_not_count() {
    assert_eq!(tonic_then_dominant().credit(label(V, "G"), 1, 2), None);
}

#[test]
fn the_timing_tolerance_applies_the_other_equivalences_nearby() {
    let applied = in_key("C", Some("G"), ("D", Quality::Major), "D");
    let fixture = Fixture::new(&[(label(I, "C"), 4), (applied, 4)], "D", &["D", "F#", "A"]);
    assert_eq!(fixture.credit(in_key("G", None, ("D", Quality::Major), "D"), 3, 2), Some(Credit::Timing));
}

#[test]
fn an_applied_chord_counts_against_the_same_chord_in_its_target_key() {
    let applied = in_key("C", Some("G"), ("D", Quality::Major), "D");
    let fixture = Fixture::new(&[(applied, 4)], "D", &["D", "F#", "A"]);
    assert_eq!(fixture.credit(in_key("G", None, ("D", Quality::Major), "D"), 1, 0), Some(Credit::Applied));
}

#[test]
fn a_rootless_dominant_seventh_counts_against_the_leading_tone_chord() {
    let fixture = Fixture::new(&[(label(VIIO, "D"), 4)], "D", &["D", "F", "B"]);
    assert_eq!(fixture.credit(label(V7, "D"), 1, 0), Some(Credit::Rootless));
}

#[test]
fn a_dominant_seventh_with_its_root_sounding_does_not_count_as_rootless() {
    let fixture = Fixture::new(&[(label(VIIO, "D"), 4)], "D", &["D", "F", "B", "G"]);
    assert_eq!(fixture.credit(label(V7, "D"), 1, 0), None);
}

/// I, then V7 and I over a held C, which V7 leaves out.
fn tonic_pedal() -> Fixture {
    let spans = [(label(I, "C"), 4), (label(V7, "D"), 4), (label(I, "C"), 4)];
    Fixture::new(&spans, "C", &["C", "D", "F", "G", "B"])
}

#[test]
fn the_pedal_chord_counts_over_the_chords_above_the_pedal() {
    assert_eq!(tonic_pedal().credit(label(I, "C"), 5, 0), Some(Credit::Pedal));
}

#[test]
fn a_chord_above_the_pedal_counts_in_any_inversion() {
    assert_eq!(tonic_pedal().credit(label(V7, "G"), 5, 0), Some(Credit::Pedal));
}

#[test]
fn a_chord_whose_bass_is_the_pedal_note_needs_its_own_inversion() {
    let spans = [(label(I, "E"), 4), (label(V7, "D"), 4), (label(I, "E"), 4)];
    let fixture = Fixture::new(&spans, "E", &["E", "D", "F", "G", "B"]);
    assert_eq!(fixture.credit(label(I, "C"), 1, 0), None);
}

#[test]
fn a_held_bass_that_every_chord_contains_is_not_a_pedal() {
    let spans = [(label(I, "C"), 4), (label(IV, "C"), 4)];
    let fixture = Fixture::new(&spans, "C", &["C", "F", "A"]);
    assert_eq!(fixture.credit(label(I, "C"), 5, 0), None);
}

#[test]
fn a_held_bass_under_a_single_label_is_not_a_pedal() {
    let fixture = Fixture::new(&[(label(V7, "D"), 8)], "C", &["C", "D", "F", "G", "B"]);
    assert_eq!(fixture.credit(label(V7, "G"), 1, 0), None);
}
