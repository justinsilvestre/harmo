//! Scores how well the notes of a slice fit the key of a reading.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, TEMPERLEY_2001, TEMPERLEY_2007};
use crate::engine::rule::{Citation, Hook, Layer, Rule, SliceContext};
use crate::pitch::pitch_class;
use crate::theory::key::{Key, Mode};
use std::sync::LazyLock;

pub const RULES: &[Rule] = &[Rule {
    id: "key.profile",
    layer: Layer::Key,
    summary: "Prefer keys whose key profile favors the sounding pitch classes. Each slice scores the mean \
              log profile value of its distinct pitch classes, relative to the profile's average, times \
              its duration. The profiles are Temperley's counts from the Kostka-Payne corpus. An \
              applied chord is scored half against the key it tonicizes, so that its accidentals \
              count less against the main key.",
    citations: &[
        Citation { source: TEMPERLEY_2001, locator: "p. 188 (KPR 1, Key-Profile Rule)" },
        Citation {
            source: TEMPERLEY_2007,
            locator: "pp. 84–86 (Kostka-Payne key profiles, figure 6.4 and table 6.1)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 256 (a secondary function belongs more closely to another key)",
        },
    ],
    default_weight: 0.375,
    hook: Hook::Slice(profile_fit),
}];

/// How much of an applied chord's score comes from the key it tonicizes rather than the main key.
const TONICIZED_SHARE: f32 = 0.5;

/// Proportion of segments containing each scale degree, by semitones above the tonic.
const MAJOR_PROFILE: [f32; 12] =
    [0.748, 0.060, 0.488, 0.082, 0.670, 0.460, 0.096, 0.715, 0.104, 0.366, 0.057, 0.400];
const MINOR_PROFILE: [f32; 12] =
    [0.712, 0.084, 0.474, 0.618, 0.049, 0.460, 0.105, 0.747, 0.404, 0.067, 0.133, 0.330];

static CENTERED_LOGS: LazyLock<[[f32; 12]; 2]> =
    LazyLock::new(|| [centered_logs(&MAJOR_PROFILE), centered_logs(&MINOR_PROFILE)]);

fn centered_logs(profile: &[f32; 12]) -> [f32; 12] {
    let logs = profile.map(f32::ln);
    let mean = logs.iter().sum::<f32>() / 12.0;
    logs.map(|value| value - mean)
}

/// Returns the log key profile of a mode, centered on its mean, by semitones above the tonic.
pub fn profile_logs(mode: Mode) -> &'static [f32; 12] {
    &CENTERED_LOGS[if mode == Mode::Major { 0 } else { 1 }]
}

fn profile_fit(context: &SliceContext) -> f32 {
    let home = key_fit(context.reading.key, context);
    match context.reading.tonicized {
        Some(local) => TONICIZED_SHARE * key_fit(local, context) + (1.0 - TONICIZED_SHARE) * home,
        None => home,
    }
}

fn key_fit(key: Key, context: &SliceContext) -> f32 {
    let profile = profile_logs(key.mode);
    let mut seen = [false; 12];
    let (mut total, mut count) = (0.0, 0);
    for note in &context.slice.notes {
        let pc = pitch_class(note.tpc - key.tonic) as usize;
        if !std::mem::replace(&mut seen[pc], true) {
            total += profile[pc];
            count += 1;
        }
    }
    if count == 0 { 0.0 } else { total / count as f32 * context.slice.duration as f32 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Category, Reading};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::{Slice, SliceNote};
    use crate::theory::chord::{Chord, Quality};

    fn tpc(name: &str) -> i32 {
        tpc_from_name(name).unwrap()
    }

    /// Scores a D7 chord in C major, read either as V7/V or with no tonicization.
    fn secondary_dominant_fit(tonicized: Option<Key>) -> f32 {
        let notes = ["D", "F#", "A", "C"].map(|name| SliceNote {
            tpc: tpc(name),
            midi: 60,
            attacked: true,
            onset_level: 0,
        });
        let slice = Slice {
            start: 0.0,
            duration: 1.0,
            notes: notes.to_vec(),
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        };
        let chord = Chord::new(tpc("D"), Quality::DominantSeventh);
        let reading =
            Reading { key: Key::new(tpc("C"), Mode::Major), chord, tonicized, category: Category::Applied };
        let slices = [slice];
        let context = SliceContext {
            slice: &slices[0],
            next_slice: None,
            slices: &slices,
            index: 0,
            reading: &reading,
            is_opening: true,
        };
        profile_fit(&context)
    }

    #[test]
    fn applied_chord_is_partly_scored_against_the_key_it_tonicizes() {
        let dominant_key = Some(Key::new(tpc("G"), Mode::Major));
        assert!(secondary_dominant_fit(dominant_key) > secondary_dominant_fit(None));
    }
}
