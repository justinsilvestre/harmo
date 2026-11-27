//! Rules about the home key of a piece as evidence for the key of each passage.
//!
//! A tonal piece begins and ends in its home key, so the note on which it ends names its tonic.
//! The rule reads that note from the end of the score, which a listener hearing the piece for the
//! first time cannot do. It stands in for what a listener usually knows beforehand: the key a
//! piece is named for in its title or program, or the key of a familiar style of piece. Rank
//! histories therefore show the home key favored from the first slice on.

use super::sources::{BURSTEIN_STRAUS_2016, KOSTKA_PAYNE_ALMEN_2023};
use crate::engine::rule::{Citation, Hook, Layer, Rule, SliceContext};
use crate::pitch::{Tpc, pitch_class};
use crate::score::slicing::Slice;

pub const RULES: &[Rule] = &[Rule {
    id: "key.final_tonic",
    layer: Layer::Key,
    summary: "Prefer keys whose tonic is the home tonic of the piece, the bass note of its last \
              sounding slice. Each slice in such a key earns its duration. A piece begins and ends \
              in its home key, in either mode, so other keys must be supported by the music \
              itself.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 298 (almost all tonal compositions begin and end in the same key; the mode may change but the tonic note remains)",
        },
        Citation {
            source: BURSTEIN_STRAUS_2016,
            locator: "p. 368 (pieces usually begin and end in the same key, the main key)",
        },
    ],
    default_weight: 0.05,
    hook: Hook::Slice(final_tonic),
}];

fn final_tonic(context: &SliceContext) -> f32 {
    let is_home =
        final_bass(context.slices).is_some_and(|bass| is_same_pitch_class(bass, context.reading.key.tonic));
    if is_home { context.slice.duration as f32 } else { 0.0 }
}

/// Returns the lowest note of the last slice in which anything sounds.
fn final_bass(slices: &[Slice]) -> Option<Tpc> {
    slices.iter().rev().find_map(|slice| slice.bass()).map(|note| note.tpc)
}

fn is_same_pitch_class(a: Tpc, b: Tpc) -> bool {
    pitch_class(a) == pitch_class(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Category, Reading};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    fn slice(start: f64, names: &[&str]) -> Slice {
        let note = |(i, name): (usize, &&str)| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi: 48 + 12 * i as i32,
            attacked: true,
            onset_level: 0,
        };
        let notes = names.iter().enumerate().map(note).collect();
        Slice { start, duration: 2.0, notes, measure: 0, metric_level: 0, phrase: 0 }
    }

    /// Scores the first slice of a G-to-C passage that ends in silence, read in the given key.
    fn score(tonic: &str, mode: Mode) -> f32 {
        let slices = [slice(0.0, &["G", "B", "D"]), slice(2.0, &["C", "E", "G"]), slice(4.0, &[])];
        let key = Key::new(tpc_from_name(tonic).unwrap(), mode);
        let chord = Chord::new(tpc_from_name("G").unwrap(), Quality::Major);
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        final_tonic(&SliceContext {
            slice: &slices[0],
            next_slice: Some(&slices[1]),
            slices: &slices,
            index: 0,
            reading: &reading,
            is_opening: true,
        })
    }

    #[test]
    fn key_of_the_final_bass_earns_the_slice_duration() {
        assert_eq!(score("C", Mode::Major), 2.0);
    }

    #[test]
    fn parallel_minor_of_the_final_bass_also_earns_it() {
        assert_eq!(score("C", Mode::Minor), 2.0);
    }

    #[test]
    fn other_keys_earn_nothing() {
        assert_eq!(score("G", Mode::Major), 0.0);
    }
}
