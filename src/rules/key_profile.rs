//! Scores how well the notes of a slice fit the key of a reading.

use super::sources::{TEMPERLEY_2001, TEMPERLEY_2007};
use crate::engine::rule::{Citation, Hook, Layer, Rule, SliceContext};
use crate::pitch::pitch_class;
use crate::theory::key::Mode;
use std::sync::LazyLock;

pub const RULES: &[Rule] = &[Rule {
    id: "key.profile",
    layer: Layer::Key,
    summary: "Prefer keys whose key profile favors the sounding pitch classes. Each slice scores the mean \
              log profile value of its distinct pitch classes, relative to the profile's average, times \
              its duration. The profiles are Temperley's counts from the Kostka-Payne corpus.",
    citations: &[
        Citation { source: TEMPERLEY_2001, locator: "p. 188 (KPR 1, Key-Profile Rule)" },
        Citation {
            source: TEMPERLEY_2007,
            locator: "pp. 84–86 (Kostka-Payne key profiles, figure 6.4 and table 6.1)",
        },
    ],
    default_weight: 0.5,
    hook: Hook::Slice(profile_fit),
}];

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

fn profile_fit(context: &SliceContext) -> f32 {
    let key = context.reading.key;
    let profile = &CENTERED_LOGS[if key.mode == Mode::Major { 0 } else { 1 }];
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
