//! The data behind a results document: the run's circumstances and the summaries it produced.

use crate::bench::report::SetRecord;
use crate::diagnose::summary::SetDiagnosis;
use std::collections::BTreeMap;

pub struct ResultsRecord {
    pub header: RunHeader,
    /// Accuracy and expert agreement for each evaluated set.
    pub accuracy: Vec<SetRecord>,
    /// The error diagnosis for each diagnosed set.
    pub diagnoses: Vec<SetDiagnosis>,
}

/// What was run, when, and how long it took.
pub struct RunHeader {
    /// The short commit hash.
    pub commit: String,
    /// Whether the engine's source differs from the commit.
    pub has_uncommitted_changes: bool,
    pub date: String,
    pub settings: BTreeMap<String, f64>,
    /// The weights and parameters files, if the defaults were overridden.
    pub weights: Option<String>,
    pub pieces: usize,
    /// Pieces that could not be loaded, with the reason.
    pub failures: Vec<(String, String)>,
    pub threads: usize,
    pub bench_seconds: f64,
    pub diagnose_seconds: f64,
    pub total_seconds: f64,
}
