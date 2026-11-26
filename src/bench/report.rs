//! Summarizes evaluation results by benchmark set, prints them, and saves them as run records.

use super::metrics::{Accuracy, Agreement, AgreementSummary, FEATURES};
use super::runner::PieceResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// AugmentedNet's published accuracies on the subsets of its test set that we can also evaluate.
/// Source: AugmentedNet README, table "AugmentedNet vs. other models" (Nápoles López, Gotham and
/// Fujinaga 2021), model "AugN" trained on the full dataset.
pub const AUGMENTEDNET_REFERENCE: &[(&str, [f64; 6])] = &[
    ("anet-test-wir", [0.818, 0.692, 0.859, 0.903, 0.903, 0.564]),
    ("anet-test-abc_dcml", [0.836, 0.656, 0.780, 0.769, 0.789, 0.445]),
    ("anet-test-wirwtc", [0.772, 0.697, 0.750, 0.744, 0.827, 0.462]),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetRecord {
    pub name: String,
    pub pieces: usize,
    pub frames: u64,
    pub rates: [f64; 6],
    pub agreement: Option<AgreementSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PieceRecord {
    pub id: String,
    pub frames: u64,
    pub rates: [f64; 6],
    pub seconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunRecord {
    pub label: String,
    pub timestamp: u64,
    pub seconds: f64,
    pub settings: BTreeMap<String, f64>,
    pub weights: BTreeMap<String, f32>,
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
    for result in &members {
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
        agreement: agreement.map(|a| a.summary()),
    }
}

pub fn piece_records(results: &[PieceResult]) -> Vec<PieceRecord> {
    let record = |r: &PieceResult| PieceRecord {
        id: r.id.clone(),
        frames: r.accuracy.frames,
        rates: r.accuracy.rates(),
        seconds: r.seconds,
    };
    results.iter().map(record).collect()
}

/// Prints accuracy by set, with differences from a previous run and AugmentedNet's reference where known.
pub fn print_accuracy(sets: &[SetRecord], previous: Option<&RunRecord>) {
    println!("\n{:<28} {:>6} {:>8}  {}", "accuracy (%)", "pieces", "frames", header());
    for set in sets {
        println!("{:<28} {:>6} {:>8}  {}", set.name, set.pieces, set.frames, percentages(&set.rates));
        if let Some(old) = previous.and_then(|p| p.sets.iter().find(|s| s.name == set.name)) {
            println!(
                "{:<28} {:>6} {:>8}  {}",
                "  change vs previous",
                "",
                "",
                differences(&set.rates, &old.rates)
            );
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
        println!("{:<40} {}", "", header());
        let rows: [(&str, &[f64; 6]); 5] = [
            ("analysts agree", &summary.analyst_agreement),
            ("target: top matches consensus", &summary.target_consensus_rate),
            ("top matches consensus", &summary.consensus_rate),
            ("disagreements: both readings offered", &summary.disagreement_coverage),
            ("disagreements: top matches either", &summary.disagreement_top_matches_either),
        ];
        for (name, values) in rows {
            println!("{name:<40} {}", percentages(values));
        }
    }
}

fn header() -> String {
    FEATURES.iter().map(|f| format!("{f:>9}")).collect()
}

fn percentages(values: &[f64; 6]) -> String {
    values.iter().map(|v| format!("{:>9.1}", v * 100.0)).collect()
}

fn differences(values: &[f64; 6], old: &[f64; 6]) -> String {
    values.iter().zip(old).map(|(v, o)| format!("{:>+9.1}", (v - o) * 100.0)).collect()
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
