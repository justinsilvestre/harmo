//! Scores the set of pitch classes heard in a slice's measure against the key of a reading.

use super::key_profile::{TONICIZED_SHARE, profile_proportions};
use super::sources::{CAPLIN_2024, TEMPERLEY_2001, TEMPERLEY_2007};
use crate::engine::rule::{Citation, Hook, Layer, Rule, SliceContext};
use crate::pitch::pitch_class;
use crate::score::slicing::Slice;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::{Key, Mode};
use std::cell::Cell;

pub const RULES: &[Rule] = &[Rule {
    id: "key.segment_profile",
    layer: Layer::Key,
    summary: "Prefer keys under which the set of pitch classes heard in a slice's measure is likely. \
              The set holds every pitch class sounding in the measure, from its start to at most two \
              quarter notes past the slice and within the phrase, each counted once however long or \
              often it sounds. A key scores the log probability of the set under its key profile: \
              the log profile value of each pitch class present and the log of one minus it for each \
              one absent. The score is taken relative to the average over all 24 keys and multiplied \
              by the slice's duration over a four-quarter-note measure. An applied chord is scored \
              half against the key it tonicizes. When the chord is a major triad or dominant seventh \
              on the fifth degree of the key it is scored against, an absent tonic costs nothing, \
              since a dominant chord does not contain the tonic: a measure that holds only the \
              dominant chord fits its key as well as it fits the key of the dominant.",
    citations: &[
        Citation {
            source: TEMPERLEY_2001,
            locator: "pp. 181–182, 189 (flat input: each pitch class present in a segment of about a \
                      measure counts once; scores scale with segment length)",
        },
        Citation {
            source: TEMPERLEY_2007,
            locator: "p. 87 (equation 6.3: the pitch classes present and absent both score)",
        },
        Citation {
            source: TEMPERLEY_2001,
            locator: "p. 188 (a measure in G major considered in isolation outlines a V chord in a C major section)",
        },
        Citation {
            source: CAPLIN_2024,
            locator: "p. 22 (the dominant, like the tonic, may be prolonged, for instance over a pedal point)",
        },
        Citation {
            source: CAPLIN_2024,
            locator: "p. 38 (after a half cadence, a standing on the dominant prolongs dominant harmony)",
        },
    ],
    default_weight: 0.25,
    hook: Hook::Slice(segment_fit),
}];

/// How far the set reaches past the end of the slice, in quarter notes, the lag the benchmark
/// allows offered readings.
const LOOK_AHEAD: f64 = 2.0;

/// The length of a segment in quarter notes, a measure of common time.
const SEGMENT: f64 = 4.0;

fn segment_fit(context: &SliceContext) -> f32 {
    let (present, scores) = cached_key_scores(context.slices, context.index);
    let chord = context.reading.chord;
    let fit_in = |key| key_score(&scores, key) + dominant_allowance(&present, key, chord);
    let home = fit_in(context.reading.key);
    let fit = match context.reading.tonicized {
        Some(local) => TONICIZED_SHARE * fit_in(local) + (1.0 - TONICIZED_SHARE) * home,
        None => home,
    };
    fit * (context.slice.duration / SEGMENT) as f32
}

/// Returns the charge for an absent tonic that a dominant chord of the key is spared. The amount
/// is the charge in a major key for either mode, so that it does not by itself favor one of the
/// two keys that share a dominant.
fn dominant_allowance(present: &[bool; 12], key: Key, chord: Chord) -> f32 {
    let is_dominant =
        chord.root == key.tonic + 1 && matches!(chord.quality, Quality::Major | Quality::DominantSeventh);
    if !is_dominant || present[pitch_class(key.tonic) as usize] {
        return 0.0;
    }
    -(1.0 - profile_proportions(Mode::Major)[0]).ln()
}

/// Looks up a key in scores listed by tonic pitch class, major keys first.
fn key_score(scores: &[f32; 24], key: Key) -> f32 {
    let offset = if key.mode == Mode::Major { 0 } else { 12 };
    scores[offset + pitch_class(key.tonic) as usize]
}

/// Identifies a slice of a particular score: the addresses of the slice list and of the slice's
/// notes, the list's length and the slice's index.
type SliceIdentity = (usize, usize, usize, usize);

thread_local! {
    /// The scores of the most recent slice. The engine scores many readings of one slice in a
    /// row, so one entry avoids nearly all recomputation without changing any result.
    static LAST_SCORES: Cell<(SliceIdentity, [bool; 12], [f32; 24])> =
        const { Cell::new(((0, 0, 0, usize::MAX), [false; 12], [0.0; 24])) };
}

/// Returns the pitch classes of the slice's measure and the score of each key for them.
fn cached_key_scores(slices: &[Slice], index: usize) -> ([bool; 12], [f32; 24]) {
    let identity = (slices.as_ptr() as usize, slices[index].notes.as_ptr() as usize, slices.len(), index);
    let (cached_identity, present, scores) = LAST_SCORES.get();
    if cached_identity == identity {
        return (present, scores);
    }
    let present = measure_pitch_classes(slices, index);
    let scores = key_scores(&present);
    LAST_SCORES.set((identity, present, scores));
    (present, scores)
}

/// Returns the log probability of a pitch-class set under each key, relative to the mean over the
/// 24 keys, listed by tonic pitch class, major keys first.
fn key_scores(present: &[bool; 12]) -> [f32; 24] {
    let mut scores = [0.0; 24];
    for (offset, mode) in [(0, Mode::Major), (12, Mode::Minor)] {
        let profile = profile_proportions(mode);
        for tonic in 0..12 {
            scores[offset + tonic] = (0..12)
                .map(|pc| {
                    let share = profile[(pc + 12 - tonic) % 12];
                    if present[pc] { share.ln() } else { (1.0 - share).ln() }
                })
                .sum();
        }
    }
    let mean = scores.iter().sum::<f32>() / 24.0;
    scores.map(|score| score - mean)
}

/// Marks the pitch classes sounding in the slice's measure and phrase, up to two quarter notes
/// past the slice.
fn measure_pitch_classes(slices: &[Slice], index: usize) -> [bool; 12] {
    let slice = &slices[index];
    let horizon = slice.start + slice.duration + LOOK_AHEAD;
    let together = |other: &&Slice| other.measure == slice.measure && other.phrase == slice.phrase;
    let before = slices[..index].iter().rev().take_while(together);
    let after = slices[index + 1..].iter().take_while(together).take_while(|s| s.start < horizon);
    let mut present = [false; 12];
    for other in before.chain(after).chain(std::iter::once(slice)) {
        for note in &other.notes {
            present[pitch_class(note.tpc) as usize] = true;
        }
    }
    present
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Category, Reading};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;

    fn tpc(name: &str) -> i32 {
        tpc_from_name(name).unwrap()
    }

    fn slice(start: f64, duration: f64, names: &[&str]) -> Slice {
        let notes = names
            .iter()
            .map(|name| SliceNote { tpc: tpc(name), midi: 60, attacked: true, onset_level: 0 })
            .collect();
        Slice {
            start,
            duration,
            notes,
            measure: start as usize / 4,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        }
    }

    fn set(names: &[&str]) -> [bool; 12] {
        let mut present = [false; 12];
        for name in names {
            present[pitch_class(tpc(name)) as usize] = true;
        }
        present
    }

    fn has(present: &[bool; 12], name: &str) -> bool {
        present[pitch_class(tpc(name)) as usize]
    }

    #[test]
    fn a_major_scale_fits_its_own_key_best() {
        let scores = key_scores(&set(&["C", "D", "E", "F", "G", "A", "B"]));
        let best = (0..24).max_by(|&a, &b| scores[a].total_cmp(&scores[b]));
        assert_eq!(best, Some(pitch_class(tpc("C")) as usize));
    }

    #[test]
    fn an_absent_tonic_counts_against_a_key() {
        let scores = key_scores(&set(&["E", "G", "B"]));
        let e_minor = Key::new(tpc("E"), Mode::Minor);
        assert!(key_score(&scores, e_minor) > key_score(&scores, Key::new(tpc("C"), Mode::Major)));
    }

    /// Repeated Cs in a D7 bar count once, so they do not pull the bar toward A minor.
    #[test]
    fn a_repeated_pitch_class_counts_once() {
        let slices = [slice(0.0, 1.0, &["D", "F#", "A"]), slice(1.0, 1.0, &["C"])];
        assert_eq!(measure_pitch_classes(&slices, 0), set(&["D", "F#", "A", "C"]));
    }

    #[test]
    fn the_set_holds_earlier_music_of_the_measure() {
        let slices = [slice(0.0, 1.0, &["C"]), slice(1.0, 2.0, &["E"])];
        assert!(has(&measure_pitch_classes(&slices, 1), "C"));
    }

    #[test]
    fn the_set_stops_at_the_start_of_the_measure() {
        let slices = [slice(0.0, 4.0, &["C"]), slice(4.0, 4.0, &["E"])];
        assert!(!has(&measure_pitch_classes(&slices, 1), "C"));
    }

    #[test]
    fn the_set_ignores_music_more_than_two_quarter_notes_ahead() {
        let slices = [slice(0.0, 1.0, &["C"]), slice(1.0, 2.0, &["E"]), slice(3.0, 1.0, &["G"])];
        assert!(!has(&measure_pitch_classes(&slices, 0), "G"));
    }

    #[test]
    fn the_set_stops_at_the_start_of_the_phrase() {
        let mut slices = [slice(0.0, 1.0, &["C"]), slice(1.0, 1.0, &["D"])];
        slices[1].phrase = 1;
        assert!(!has(&measure_pitch_classes(&slices, 1), "C"));
    }

    /// Scores a bar of D7 in C major, read either as V7/V or with no tonicization.
    fn secondary_dominant_fit(tonicized: Option<Key>) -> f32 {
        let slices = [slice(0.0, 4.0, &["D", "F#", "A", "C"])];
        let chord = Chord::new(tpc("D"), Quality::DominantSeventh);
        let reading =
            Reading { key: Key::new(tpc("C"), Mode::Major), chord, tonicized, category: Category::Applied };
        let context = SliceContext {
            slice: &slices[0],
            next_slice: None,
            slices: &slices,
            index: 0,
            reading: &reading,
            is_opening: true,
        };
        segment_fit(&context)
    }

    /// Scores a bar holding only the given notes against a key, read as a chord on the given root.
    fn bar_fit(names: &[&str], key: Key, root: &str, quality: Quality) -> f32 {
        let slices = [slice(0.0, 4.0, names)];
        let chord = Chord::new(tpc(root), quality);
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        let context = SliceContext {
            slice: &slices[0],
            next_slice: None,
            slices: &slices,
            index: 0,
            reading: &reading,
            is_opening: true,
        };
        segment_fit(&context)
    }

    fn major(name: &str) -> Key {
        Key::new(tpc(name), Mode::Major)
    }

    #[test]
    fn a_dominant_bar_read_as_v_is_spared_the_charge_for_the_absent_tonic() {
        let triad = ["G", "B", "D"];
        let charged = key_score(&key_scores(&set(&triad)), major("C"));
        assert!(bar_fit(&triad, major("C"), "G", Quality::Major) > charged);
    }

    #[test]
    fn a_chord_that_is_not_the_dominant_gets_no_allowance() {
        let chord = Chord::new(tpc("D"), Quality::Minor);
        assert_eq!(dominant_allowance(&set(&["D", "F", "A"]), major("C"), chord), 0.0);
    }

    #[test]
    fn the_allowance_is_the_same_in_major_and_minor() {
        let present = set(&["G", "B", "D"]);
        let chord = Chord::new(tpc("G"), Quality::Major);
        let minor = dominant_allowance(&present, Key::new(tpc("C"), Mode::Minor), chord);
        assert_eq!(minor, dominant_allowance(&present, major("C"), chord));
    }

    #[test]
    fn a_dominant_chord_with_the_tonic_present_gets_no_allowance() {
        let chord = Chord::new(tpc("G"), Quality::DominantSeventh);
        assert_eq!(dominant_allowance(&set(&["G", "B", "D", "F", "C"]), major("C"), chord), 0.0);
    }

    #[test]
    fn applied_chord_is_partly_scored_against_the_key_it_tonicizes() {
        let dominant_key = Some(Key::new(tpc("G"), Mode::Major));
        assert!(secondary_dominant_fit(dominant_key) > secondary_dominant_fit(None));
    }
}
