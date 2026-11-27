//! Frame-level accuracy and expert-agreement counts.

use crate::theory::label::Label;
use serde::{Deserialize, Serialize};

/// The reported features. The first six are AugmentedNet's, in its order, and "rn" requires the
/// five before it.
/// "chord" requires quality, inversion and root, whatever the key.
/// "rn|key" is the rn accuracy over only the frames whose key is correct.
/// "offered" is the share of frames whose full Roman numeral is among the readings the engine offers.
/// "key now" and "rn now" score the reading held at the moment rather than the one traced back from
/// the end of the piece.
pub const FEATURES: [&str; 11] = [
    "key",
    "degree",
    "quality",
    "inversion",
    "root",
    "rn",
    "chord",
    "rn|key",
    "offered",
    "key now",
    "rn now",
];

/// The number of leading features that AugmentedNet reports.
pub const AUGMENTEDNET_FEATURES: usize = 6;

/// The number of leading features that are compared frame by frame.
/// Of the remaining features, "rn|key" is derived from the counts for "rn" and "key", "offered" is
/// counted against the offered readings, and "key now" and "rn now" against the in-the-moment reading.
pub const MATCHED_FEATURES: usize = 7;

const KEY: usize = 0;
const RN: usize = 5;

pub type FeatureCounts = [u64; MATCHED_FEATURES];

type FeatureFlags = [bool; MATCHED_FEATURES];

pub fn feature_matches(a: &Label, b: &Label) -> FeatureFlags {
    let key = a.key == b.key;
    let degree = a.degree == b.degree && a.secondary == b.secondary;
    let quality = a.quality == b.quality;
    let inversion = a.inversion == b.inversion;
    let root = a.root == b.root;
    let chord = quality && inversion && root;
    [key, degree, quality, inversion, root, key && degree && chord, chord]
}

/// Reports whether two labels agree on the full Roman numeral, the benchmark's "rn" feature.
pub fn is_rn_match(a: &Label, b: &Label) -> bool {
    feature_matches(a, b)[RN]
}

fn add_where(counts: &mut FeatureCounts, flags: FeatureFlags) {
    for (count, flag) in counts.iter_mut().zip(flags) {
        *count += flag as u64;
    }
}

/// Accuracy of the top-ranked reading against one analyst, how often that analyst's reading is among
/// the readings offered, and how often the in-the-moment reading matches it.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Accuracy {
    pub frames: u64,
    pub correct: FeatureCounts,
    /// Frames whose full Roman numeral is among the offered readings.
    pub offered: u64,
    /// Sum over frames of the number of distinct full readings offered.
    pub offered_readings: u64,
    /// Frames whose in-the-moment reading has the right key.
    pub now_key: u64,
    /// Frames whose in-the-moment reading has the right full Roman numeral.
    pub now_rn: u64,
}

impl Accuracy {
    /// Adds a frame, given the top reading, the offered readings and the in-the-moment reading.
    pub fn add_frame(&mut self, top: Option<&Label>, offered: &[Label], now: Option<&Label>, truth: &Label) {
        self.frames += 1;
        if let Some(top) = top {
            add_where(&mut self.correct, feature_matches(top, truth));
        }
        self.offered += offered.iter().any(|o| is_rn_match(o, truth)) as u64;
        self.offered_readings += offered.len() as u64;
        let now_matches = now.map_or([false; MATCHED_FEATURES], |n| feature_matches(n, truth));
        self.now_key += now_matches[KEY] as u64;
        self.now_rn += now_matches[RN] as u64;
    }

    pub fn merge(&mut self, other: &Accuracy) {
        self.frames += other.frames;
        add_where_counts(&mut self.correct, &other.correct);
        self.offered += other.offered;
        self.offered_readings += other.offered_readings;
        self.now_key += other.now_key;
        self.now_rn += other.now_rn;
    }

    /// Returns the rate for each of `FEATURES`.
    pub fn rates(&self) -> Vec<f64> {
        let mut rates: Vec<f64> = self.correct.iter().map(|&c| ratio(c, self.frames)).collect();
        rates.push(ratio(self.correct[RN], self.correct[KEY]));
        rates.push(ratio(self.offered, self.frames));
        rates.push(ratio(self.now_key, self.frames));
        rates.push(ratio(self.now_rn, self.frames));
        rates
    }

    /// Returns the mean number of distinct readings offered per frame.
    pub fn mean_offered(&self) -> f64 {
        ratio(self.offered_readings, self.frames)
    }
}

/// Counts comparing the engine with a pair of analysts, frame by frame.
///
/// Where the analysts agree, the engine's top reading should match them about as often as a third
/// expert would, estimated as the square root of the analysts' agreement rate.
/// Where they disagree, the engine's offered readings should include both analysts' readings.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Agreement {
    pub frames: u64,
    pub analysts_agree: FeatureCounts,
    pub top_matches_consensus: FeatureCounts,
    pub offered_first_analyst: FeatureCounts,
    pub offered_second_analyst: FeatureCounts,
    pub offered_both: FeatureCounts,
    pub top_matches_either: FeatureCounts,
    /// Sum over frames of the number of distinct full readings offered.
    pub offered_readings: u64,
}

impl Agreement {
    pub fn add_frame(&mut self, top: Option<&Label>, offered: &[Label], first: &Label, second: &Label) {
        self.frames += 1;
        self.offered_readings += offered.len() as u64;
        let agree = feature_matches(first, second);
        let none = [false; MATCHED_FEATURES];
        let top_first = top.map_or(none, |t| feature_matches(t, first));
        let top_second = top.map_or(none, |t| feature_matches(t, second));
        let offers = |truth: &Label| offered.iter().fold(none, |acc, o| or(acc, feature_matches(o, truth)));
        let (offers_first, offers_second) = (offers(first), offers(second));
        let disagree = agree.map(|a| !a);
        add_where(&mut self.analysts_agree, agree);
        add_where(&mut self.top_matches_consensus, and(agree, top_first));
        add_where(&mut self.offered_first_analyst, and(disagree, offers_first));
        add_where(&mut self.offered_second_analyst, and(disagree, offers_second));
        add_where(&mut self.offered_both, and(disagree, and(offers_first, offers_second)));
        add_where(&mut self.top_matches_either, and(disagree, or(top_first, top_second)));
    }

    pub fn merge(&mut self, other: &Agreement) {
        self.frames += other.frames;
        self.offered_readings += other.offered_readings;
        let pairs = [
            (&mut self.analysts_agree, &other.analysts_agree),
            (&mut self.top_matches_consensus, &other.top_matches_consensus),
            (&mut self.offered_first_analyst, &other.offered_first_analyst),
            (&mut self.offered_second_analyst, &other.offered_second_analyst),
            (&mut self.offered_both, &other.offered_both),
            (&mut self.top_matches_either, &other.top_matches_either),
        ];
        for (mine, theirs) in pairs {
            add_where_counts(mine, theirs);
        }
    }

    pub fn summary(&self) -> AgreementSummary {
        let per_feature = |f: &dyn Fn(usize) -> f64| (0..MATCHED_FEATURES).map(f).collect();
        let disagreements = |i: usize| self.frames - self.analysts_agree[i];
        AgreementSummary {
            analyst_agreement: per_feature(&|i| ratio(self.analysts_agree[i], self.frames)),
            target_consensus_rate: per_feature(&|i| ratio(self.analysts_agree[i], self.frames).sqrt()),
            consensus_rate: per_feature(&|i| ratio(self.top_matches_consensus[i], self.analysts_agree[i])),
            disagreement_coverage: per_feature(&|i| ratio(self.offered_both[i], disagreements(i))),
            disagreement_top_matches_either: per_feature(&|i| {
                ratio(self.top_matches_either[i], disagreements(i))
            }),
            mean_offered: ratio(self.offered_readings, self.frames),
        }
    }
}

/// Agreement rates for each of the first `MATCHED_FEATURES` features.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgreementSummary {
    pub analyst_agreement: Vec<f64>,
    pub target_consensus_rate: Vec<f64>,
    pub consensus_rate: Vec<f64>,
    pub disagreement_coverage: Vec<f64>,
    pub disagreement_top_matches_either: Vec<f64>,
    pub mean_offered: f64,
}

fn add_where_counts(counts: &mut FeatureCounts, other: &FeatureCounts) {
    for (count, extra) in counts.iter_mut().zip(other) {
        *count += extra;
    }
}

fn and(a: FeatureFlags, b: FeatureFlags) -> FeatureFlags {
    std::array::from_fn(|i| a[i] && b[i])
}

fn or(a: FeatureFlags, b: FeatureFlags) -> FeatureFlags {
    std::array::from_fn(|i| a[i] || b[i])
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 { 0.0 } else { numerator as f64 / denominator as f64 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn label(key: &str, root: &str, bass: &str) -> Label {
        let key = Key::parse(key, "major").unwrap();
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        Label::new(key, None, chord, tpc_from_name(bass).unwrap())
    }

    fn index(feature: &str) -> usize {
        FEATURES.iter().position(|f| *f == feature).unwrap()
    }

    fn rate(accuracy: &Accuracy, feature: &str) -> f64 {
        accuracy.rates()[index(feature)]
    }

    /// Two frames in C: one wholly right, one with the right chord read in the wrong key.
    /// One frame in G with the right key and the wrong inversion.
    fn accuracy() -> Accuracy {
        let mut accuracy = Accuracy::default();
        accuracy.add_frame(Some(&label("C", "G", "G")), &[], None, &label("C", "G", "G"));
        accuracy.add_frame(Some(&label("G", "G", "G")), &[], None, &label("C", "G", "G"));
        accuracy.add_frame(Some(&label("G", "D", "F#")), &[], None, &label("G", "D", "D"));
        accuracy
    }

    /// One frame whose reading is offered second, one offered only in another key, and one with
    /// nothing offered: five readings in all.
    fn offers() -> Accuracy {
        let mut accuracy = Accuracy::default();
        let truth = label("C", "G", "G");
        accuracy.add_frame(None, &[label("C", "G", "B"), truth], None, &truth);
        accuracy.add_frame(
            None,
            &[label("C", "F", "F"), label("G", "G", "G"), label("C", "C", "C")],
            None,
            &truth,
        );
        accuracy.add_frame(None, &[], None, &truth);
        accuracy
    }

    #[test]
    fn chord_ignores_the_key() {
        assert!(feature_matches(&label("G", "G", "G"), &label("C", "G", "G"))[index("chord")]);
    }

    #[test]
    fn chord_requires_the_inversion() {
        assert!(!feature_matches(&label("C", "G", "B"), &label("C", "G", "G"))[index("chord")]);
    }

    #[test]
    fn chord_rate_counts_frames_with_the_wrong_key() {
        assert_eq!(rate(&accuracy(), "chord"), 2.0 / 3.0);
    }

    #[test]
    fn rn_given_key_counts_only_frames_with_the_right_key() {
        assert_eq!(rate(&accuracy(), "rn|key"), 0.5);
    }

    #[test]
    fn rn_given_key_is_zero_without_a_right_key() {
        assert_eq!(rate(&Accuracy::default(), "rn|key"), 0.0);
    }

    #[test]
    fn offered_counts_a_reading_offered_below_the_top() {
        assert_eq!(rate(&offers(), "offered"), 1.0 / 3.0);
    }

    #[test]
    fn offered_requires_the_key() {
        let mut accuracy = Accuracy::default();
        accuracy.add_frame(None, &[label("G", "G", "G")], None, &label("C", "G", "G"));
        assert_eq!(rate(&accuracy, "offered"), 0.0);
    }

    #[test]
    fn offered_requires_the_inversion() {
        let mut accuracy = Accuracy::default();
        accuracy.add_frame(None, &[label("C", "G", "B")], None, &label("C", "G", "G"));
        assert_eq!(rate(&accuracy, "offered"), 0.0);
    }

    #[test]
    fn offered_is_independent_of_the_top_reading() {
        let mut accuracy = Accuracy::default();
        accuracy.add_frame(Some(&label("C", "G", "G")), &[], None, &label("C", "G", "G"));
        assert_eq!(rate(&accuracy, "offered"), 0.0);
    }

    #[test]
    fn mean_offered_averages_over_frames() {
        assert_eq!(offers().mean_offered(), 5.0 / 3.0);
    }

    #[test]
    fn merge_adds_offered_counts() {
        let mut merged = offers();
        merged.merge(&offers());
        assert_eq!(rate(&merged, "offered"), 1.0 / 3.0);
    }

    /// One frame whose in-the-moment reading is wholly right but whose top reading is in another key.
    fn revised() -> Accuracy {
        let mut accuracy = Accuracy::default();
        let truth = label("C", "G", "G");
        accuracy.add_frame(Some(&label("G", "G", "G")), &[], Some(&truth), &truth);
        accuracy
    }

    #[test]
    fn rn_now_counts_the_in_the_moment_reading() {
        assert_eq!(rate(&revised(), "rn now"), 1.0);
    }

    #[test]
    fn rn_ignores_the_in_the_moment_reading() {
        assert_eq!(rate(&revised(), "rn"), 0.0);
    }

    #[test]
    fn key_now_counts_the_in_the_moment_key() {
        assert_eq!(rate(&revised(), "key now"), 1.0);
    }

    #[test]
    fn key_now_is_zero_without_an_in_the_moment_reading() {
        assert_eq!(rate(&accuracy(), "key now"), 0.0);
    }

    #[test]
    fn rn_now_requires_the_inversion() {
        let mut accuracy = Accuracy::default();
        accuracy.add_frame(None, &[], Some(&label("C", "G", "B")), &label("C", "G", "G"));
        assert_eq!(rate(&accuracy, "rn now"), 0.0);
    }

    #[test]
    fn merge_adds_in_the_moment_counts() {
        let mut merged = revised();
        merged.merge(&accuracy());
        assert_eq!(rate(&merged, "rn now"), 0.25);
    }

    #[test]
    fn rates_cover_every_feature() {
        assert_eq!(accuracy().rates().len(), FEATURES.len());
    }
}
