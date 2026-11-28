//! Summarizes evaluation results by benchmark set, prints them, and saves them as run records.

use super::equivalence::EQUIVALENCES;
use super::layer_coverage::{LAYERS, LayerCoverage, LayerSummary};
use super::metrics::{Accuracy, Agreement, AgreementSummary, FEATURES, MATCHED_FEATURES};
use super::offer_delay::{DELAY_LAYERS, DelaySummary, OfferDelays};
use super::runner::PieceResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// AugmentedNet's published accuracies on the subsets of its test set that we can also evaluate.
/// Source: AugmentedNet README, table "AugmentedNet vs. other models" (Nápoles López, Gotham and
/// Fujinaga 2021), model "AugN" trained on the full dataset.
/// The TAVERN figures cover both of its analyses, while the benchmark scores only the first.
pub const AUGMENTEDNET_REFERENCE: &[(&str, [f64; 6])] = &[
    ("anet-test-wir", [0.818, 0.692, 0.859, 0.903, 0.903, 0.564]),
    ("anet-test-abc_dcml", [0.836, 0.656, 0.780, 0.769, 0.789, 0.445]),
    ("anet-test-wirwtc", [0.772, 0.697, 0.750, 0.744, 0.827, 0.462]),
    ("anet-test-haydnsun", [0.812, 0.629, 0.802, 0.827, 0.865, 0.486]),
    ("anet-test-tavern", [0.887, 0.600, 0.774, 0.788, 0.815, 0.426]),
    ("anet-test-bps", [0.850, 0.734, 0.790, 0.734, 0.844, 0.454]),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetRecord {
    pub name: String,
    pub pieces: usize,
    pub frames: u64,
    /// Rates for each of `FEATURES`. Runs saved before a feature was added lack its rate.
    pub rates: Vec<f64>,
    /// The mean number of distinct readings offered per frame. Older runs lack it.
    #[serde(default)]
    pub mean_offered: Option<f64>,
    pub agreement: Option<AgreementSummary>,
    /// Frames that each equivalence credits under the equivalents measure but the exact measure
    /// does not, by the equivalence's name. Older runs lack it.
    #[serde(default)]
    pub equivalence_credits: Option<BTreeMap<String, u64>>,
    /// How soon each expert reading is offered, for each of `DELAY_LAYERS`, when it was measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offer_delays: Option<Vec<DelaySummary>>,
}

/// The column names of the expert-agreement figures: the matched features, then "rn eq".
pub fn agreement_features() -> Vec<&'static str> {
    FEATURES[..MATCHED_FEATURES].iter().copied().chain(["rn eq"]).collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PieceRecord {
    pub id: String,
    pub frames: u64,
    pub rates: Vec<f64>,
    pub seconds: f64,
    /// Where the piece's analysts disagree, per feature. Older runs and single-analysis pieces lack it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<PieceCoverage>,
}

/// Counts behind the "both readings offered" rates of one piece, for each of the first
/// `MATCHED_FEATURES` features.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PieceCoverage {
    /// Frames where the two analyses disagree on the feature.
    pub disagreements: Vec<u64>,
    /// Those frames where both analyses' readings are offered.
    pub offered_both: Vec<u64>,
    /// The same with each layer offered on its own. Older runs lack it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<LayerCoverage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunRecord {
    pub label: String,
    pub timestamp: u64,
    pub seconds: f64,
    pub settings: BTreeMap<String, f64>,
    pub weights: BTreeMap<String, f32>,
    /// The rule parameters set away from their defaults, by `<rule id>.<parameter name>`.
    /// Older runs lack them.
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
    pub sets: Vec<SetRecord>,
    pub pieces: Vec<PieceRecord>,
}

/// Aggregates results for each named set (frame-weighted, as AugmentedNet does) and for all pieces.
pub fn summarize(results: &[PieceResult], set_names: &[String]) -> Vec<SetRecord> {
    let mut names: Vec<String> = set_names.to_vec();
    names.push("all".into());
    names.iter().map(|name| summarize_set(results, name)).filter(|s| s.pieces > 0).collect()
}

fn summarize_set(results: &[PieceResult], name: &str) -> SetRecord {
    let members: Vec<&PieceResult> =
        results.iter().filter(|r| name == "all" || r.sets.iter().any(|s| s == name)).collect();
    let mut accuracy = Accuracy::default();
    let mut agreement: Option<Agreement> = None;
    let mut delays: Option<OfferDelays> = None;
    for result in &members {
        if let Some(piece_delays) = &result.offer_delays {
            let merged = delays.get_or_insert_with(Default::default);
            merged.iter_mut().zip(piece_delays).for_each(|(set, piece)| set.merge(piece));
        }
        accuracy.merge(&result.accuracy);
        if let Some(piece_agreement) = &result.agreement {
            agreement.get_or_insert_with(Agreement::default).merge(piece_agreement);
        }
    }
    SetRecord {
        name: name.to_string(),
        pieces: members.len(),
        frames: accuracy.frames,
        rates: accuracy.rates(),
        mean_offered: Some(accuracy.mean_offered()),
        agreement: agreement.map(|a| a.summary()),
        equivalence_credits: Some(credits_by_name(&accuracy)),
        offer_delays: delays
            .map(|d| d.iter().zip(DELAY_LAYERS).map(|(l, (name, _))| l.summary(name)).collect()),
    }
}

fn credits_by_name(accuracy: &Accuracy) -> BTreeMap<String, u64> {
    EQUIVALENCES.iter().zip(accuracy.credits).map(|(e, frames)| (e.name().to_string(), frames)).collect()
}

pub fn piece_records(results: &[PieceResult]) -> Vec<PieceRecord> {
    let record = |r: &PieceResult| PieceRecord {
        id: r.id.clone(),
        frames: r.accuracy.frames,
        rates: r.accuracy.rates(),
        seconds: r.seconds,
        coverage: r.agreement.as_ref().map(|a| PieceCoverage {
            disagreements: a.disagreements(),
            offered_both: a.offered_both.to_vec(),
            layers: Some(a.layers.clone()),
        }),
    };
    results.iter().map(record).collect()
}

/// Prints accuracy by set, with differences from a previous run and AugmentedNet's reference where known.
/// The last column is the mean number of readings offered per frame, against which "offered" is read.
pub fn print_accuracy(sets: &[SetRecord], previous: Option<&RunRecord>) {
    let header = format!("{}{:>10}", header(&FEATURES), "n offered");
    println!("\n{:<28} {:>6} {:>8}  {header}", "accuracy (%)", "pieces", "frames");
    for set in sets {
        let row = format!("{}{}", percentages(&set.rates), count(set.mean_offered));
        println!("{:<28} {:>6} {:>8}  {row}", set.name, set.pieces, set.frames);
        if let Some(old) = previous.and_then(|p| p.sets.iter().find(|s| s.name == set.name)) {
            let change = format!(
                "{}{}",
                differences(&set.rates, &old.rates),
                count_difference(set.mean_offered, old.mean_offered)
            );
            println!("{:<28} {:>6} {:>8}  {change}", "  change vs previous", "", "");
        }
        if let Some((_, reference)) = AUGMENTEDNET_REFERENCE.iter().find(|(n, _)| *n == set.name) {
            println!("{:<28} {:>6} {:>8}  {}", "  AugmentedNet (published)", "", "", percentages(reference));
        }
    }
}

/// Prints the expert-agreement metrics for sets that have pieces with several analyses.
pub fn print_agreement(sets: &[SetRecord]) {
    for set in sets {
        let Some(summary) = &set.agreement else {
            continue;
        };
        println!(
            "\nexpert agreement: {} (mean offered readings per frame: {:.2})",
            set.name, summary.mean_offered
        );
        println!("{:<40} {}", "", header(&agreement_features()));
        let rows: [(&str, &[f64]); 5] = [
            ("analysts agree", &summary.analyst_agreement),
            ("target: top matches consensus", &summary.target_consensus_rate),
            ("top matches consensus", &summary.consensus_rate),
            ("disagreements: both readings offered", &summary.disagreement_coverage),
            ("disagreements: top matches either", &summary.disagreement_top_matches_either),
        ];
        for (name, values) in rows {
            println!("{name:<40} {}", percentages(values));
        }
        print_layers("offered by layer, at the moment", summary.layers.as_ref());
        print_layers("offered by layer, in hindsight", summary.hindsight.as_ref());
    }
}

/// Prints how often both readings are offered with each layer offered on its own, and the mean
/// number offered in each layer.
fn print_layers(title: &str, summary: Option<&LayerSummary>) {
    let Some(summary) = summary else {
        return;
    };
    let cells: Vec<String> = LAYERS
        .iter()
        .enumerate()
        .map(|(l, name)| {
            format!("{name} {:.1} (n {:.2})", 100.0 * summary.coverage[l], summary.mean_offered[l])
        })
        .collect();
    println!("{:<40} {}", format!("{title}: both offered"), cells.join("  "));
}

/// Prints the CPU time summed over pieces for each part of the evaluation.
pub fn print_timing(results: &[PieceResult]) {
    let sum = |part: fn(&PieceResult) -> f64| results.iter().map(part).sum::<f64>();
    println!(
        "CPU seconds summed over pieces: search {:.2}, offering by layer {:.2}, hindsight {:.2}, whole evaluation {:.2}",
        sum(|r| r.timing.search),
        sum(|r| r.timing.layers),
        sum(|r| r.timing.hindsight),
        sum(|r| r.seconds)
    );
    let slices = results.iter().map(|r| r.slices).sum::<usize>().max(1) as f64;
    let (hypotheses, hearings) = (sum(|r| r.timing.hypotheses as f64), sum(|r| r.timing.hearings as f64));
    println!(
        "search kept per slice: {:.1} hypotheses, {:.1} chord hearings",
        hypotheses / slices,
        hearings / slices
    );
}

/// Prints, for each set, the share of frames that each equivalence credits under the equivalents
/// measure but the exact measure does not.
pub fn print_equivalence_credits(sets: &[SetRecord]) {
    let names: Vec<&str> = EQUIVALENCES.iter().map(|e| e.name()).collect();
    println!("\n{:<28} {}{:>9}", "rn eq credits (% of frames)", header(&names), "total");
    for set in sets {
        let Some(credits) = &set.equivalence_credits else {
            continue;
        };
        let shares: Vec<f64> =
            names.iter().map(|n| *credits.get(*n).unwrap_or(&0) as f64 / set.frames.max(1) as f64).collect();
        println!("{:<28} {}{}", set.name, percentages(&shares), percentages(&[shares.iter().sum()]));
    }
}

fn header(features: &[&str]) -> String {
    features.iter().map(|f| format!("{f:>9}")).collect()
}

fn percentages(values: &[f64]) -> String {
    values.iter().map(|v| format!("{:>9.1}", v * 100.0)).collect()
}

/// Formats each change in percentage points, leaving blank the features the old run lacks.
fn differences(values: &[f64], old: &[f64]) -> String {
    let difference = |(i, v): (usize, &f64)| match old.get(i) {
        Some(o) => format!("{:>+9.1}", (v - o) * 100.0),
        None => format!("{:>9}", ""),
    };
    values.iter().enumerate().map(difference).collect()
}

fn count(value: Option<f64>) -> String {
    value.map_or(format!("{:>10}", ""), |v| format!("{v:>10.2}"))
}

fn count_difference(value: Option<f64>, old: Option<f64>) -> String {
    match (value, old) {
        (Some(v), Some(o)) => format!("{:>+10.2}", v - o),
        _ => format!("{:>10}", ""),
    }
}

pub fn save(record: &RunRecord, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(record).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn load(path: &Path) -> Result<RunRecord, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_difference_is_blank_for_an_old_run_without_the_count() {
        assert_eq!(count_difference(Some(2.5), None), format!("{:>10}", ""));
    }

    #[test]
    fn set_records_without_mean_offered_still_load() {
        let json = r#"{"name": "tune", "pieces": 1, "frames": 8, "rates": [0.5], "agreement": null}"#;
        let record: SetRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.mean_offered, None);
    }

    #[test]
    fn set_records_without_equivalence_credits_still_load() {
        let json = r#"{"name": "tune", "pieces": 1, "frames": 8, "rates": [0.5], "agreement": null}"#;
        let record: SetRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.equivalence_credits, None);
    }

    #[test]
    fn differences_leave_features_missing_from_the_old_run_blank() {
        assert_eq!(differences(&[0.5, 0.25], &[0.4]), format!("{:>9}{:>9}", "+10.0", ""));
    }
}
