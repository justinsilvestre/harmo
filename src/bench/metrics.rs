//! Frame-level accuracy and expert-agreement counts.

use crate::theory::label::Label;
use serde::{Deserialize, Serialize};

/// The features AugmentedNet reports, in its order. "rn" requires all of the others.
pub const FEATURES: [&str; 6] = ["key", "degree", "quality", "inversion", "root", "rn"];

pub type FeatureCounts = [u64; 6];

pub fn feature_matches(a: &Label, b: &Label) -> [bool; 6] {
    let key = a.key == b.key;
    let degree = a.degree == b.degree && a.secondary == b.secondary;
    let quality = a.quality == b.quality;
    let inversion = a.inversion == b.inversion;
    let root = a.root == b.root;
    [key, degree, quality, inversion, root, key && degree && quality && inversion && root]
}

/// Reports whether two labels agree on the full Roman numeral, the benchmark's "rn" feature.
pub fn is_rn_match(a: &Label, b: &Label) -> bool {
    feature_matches(a, b)[5]
}

fn add_where(counts: &mut FeatureCounts, flags: [bool; 6]) {
    for (count, flag) in counts.iter_mut().zip(flags) {
        *count += flag as u64;
    }
}

/// Accuracy of the top-ranked reading against one analyst.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Accuracy {
    pub frames: u64,
    pub correct: FeatureCounts,
}

impl Accuracy {
    pub fn add_frame(&mut self, predicted: Option<&Label>, truth: &Label) {
        self.frames += 1;
        if let Some(predicted) = predicted {
            add_where(&mut self.correct, feature_matches(predicted, truth));
        }
    }

    pub fn merge(&mut self, other: &Accuracy) {
        self.frames += other.frames;
        add_where_counts(&mut self.correct, &other.correct);
    }

    pub fn rates(&self) -> [f64; 6] {
        self.correct.map(|c| ratio(c, self.frames))
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
        let top_first = top.map_or([false; 6], |t| feature_matches(t, first));
        let top_second = top.map_or([false; 6], |t| feature_matches(t, second));
        let offers =
            |truth: &Label| offered.iter().fold([false; 6], |acc, o| or(acc, feature_matches(o, truth)));
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
        let per_feature = |f: &dyn Fn(usize) -> f64| std::array::from_fn(f);
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgreementSummary {
    pub analyst_agreement: [f64; 6],
    pub target_consensus_rate: [f64; 6],
    pub consensus_rate: [f64; 6],
    pub disagreement_coverage: [f64; 6],
    pub disagreement_top_matches_either: [f64; 6],
    pub mean_offered: f64,
}

fn add_where_counts(counts: &mut FeatureCounts, other: &FeatureCounts) {
    for (count, extra) in counts.iter_mut().zip(other) {
        *count += extra;
    }
}

fn and(a: [bool; 6], b: [bool; 6]) -> [bool; 6] {
    std::array::from_fn(|i| a[i] && b[i])
}

fn or(a: [bool; 6], b: [bool; 6]) -> [bool; 6] {
    std::array::from_fn(|i| a[i] || b[i])
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 { 0.0 } else { numerator as f64 / denominator as f64 }
}
