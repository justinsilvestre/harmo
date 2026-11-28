//! How often both analysts' readings are offered where they disagree, with each layer offered on
//! its own (see `layer_offers`).

use super::layer_offers::{ChordName, LayerOffers};
use super::metrics::is_rn_match;
use crate::theory::label::Label;
use serde::{Deserialize, Serialize};

/// The layers, in the order of every array below: key, chord, and Roman numeral.
pub const LAYERS: [&str; 3] = ["key", "chord", "rn"];

/// For each of `LAYERS`, the index of the matching agreement feature.
pub const LAYER_FEATURES: [usize; 3] = [0, 6, 5];

/// Counts over the frames of a pair of analysts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LayerCoverage {
    pub frames: u64,
    /// Frames where the analysts disagree on the layer.
    pub disagreements: [u64; 3],
    /// Those frames where both analysts' readings of the layer are offered.
    pub offered_both: [u64; 3],
    /// Sum over frames of the number of readings offered in the layer.
    pub offered_readings: [u64; 3],
}

/// The rates of one set.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LayerSummary {
    /// For each of `LAYERS`, the share of disagreeing frames where both readings are offered.
    pub coverage: Vec<f64>,
    /// For each of `LAYERS`, the mean number of readings offered per frame.
    pub mean_offered: Vec<f64>,
}

impl LayerCoverage {
    /// Adds a frame on which the analysts read `first` and `second`.
    pub fn add_frame(&mut self, offers: &LayerOffers, (first, second): (&Label, &Label)) {
        self.frames += 1;
        let has_key = |l: &Label| offers.keys.contains(&l.key);
        let has_chord = |l: &Label| offers.chords.contains(&ChordName::of(l));
        let has_numeral = |l: &Label| offers.numerals.iter().any(|n| is_rn_match(n, l));
        let differs = [
            first.key != second.key,
            ChordName::of(first) != ChordName::of(second),
            !is_rn_match(first, second),
        ];
        let covered = [
            has_key(first) && has_key(second),
            has_chord(first) && has_chord(second),
            has_numeral(first) && has_numeral(second),
        ];
        let counts = [offers.keys.len(), offers.chords.len(), offers.numerals.len()];
        for layer in 0..LAYERS.len() {
            self.disagreements[layer] += differs[layer] as u64;
            self.offered_both[layer] += (differs[layer] && covered[layer]) as u64;
            self.offered_readings[layer] += counts[layer] as u64;
        }
    }

    pub fn merge(&mut self, other: &LayerCoverage) {
        self.frames += other.frames;
        for layer in 0..LAYERS.len() {
            self.disagreements[layer] += other.disagreements[layer];
            self.offered_both[layer] += other.offered_both[layer];
            self.offered_readings[layer] += other.offered_readings[layer];
        }
    }

    pub fn summary(&self) -> LayerSummary {
        let ratio = |a: u64, b: u64| if b == 0 { 0.0 } else { a as f64 / b as f64 };
        LayerSummary {
            coverage: (0..3).map(|l| ratio(self.offered_both[l], self.disagreements[l])).collect(),
            mean_offered: (0..3).map(|l| ratio(self.offered_readings[l], self.frames)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn label(key: &str, root: &str) -> Label {
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        Label::new(Key::parse(key, "major").unwrap(), None, chord, chord.root)
    }

    fn offers(keys: &[&str], roots: &[&str]) -> LayerOffers {
        LayerOffers {
            keys: keys.iter().map(|k| Key::parse(k, "major").unwrap()).collect(),
            chords: roots.iter().map(|r| ChordName::of(&label("C", r))).collect(),
            numerals: vec![label("C", "G")],
        }
    }

    /// Analysts read G in C and in G; the engine offers both keys but only one chord.
    fn coverage() -> LayerCoverage {
        let mut coverage = LayerCoverage::default();
        coverage.add_frame(&offers(&["C", "G"], &["G"]), (&label("C", "G"), &label("G", "G")));
        coverage
    }

    #[test]
    fn both_keys_offered_count_where_the_keys_differ() {
        assert_eq!(coverage().summary().coverage[0], 1.0);
    }

    #[test]
    fn a_chord_both_analysts_share_is_no_disagreement() {
        assert_eq!(coverage().disagreements[1], 0);
    }

    #[test]
    fn a_numeral_needs_both_analysts_readings_offered() {
        assert_eq!(coverage().offered_both[2], 0);
    }

    #[test]
    fn mean_offered_counts_each_layer() {
        assert_eq!(coverage().summary().mean_offered, vec![2.0, 1.0, 1.0]);
    }
}
