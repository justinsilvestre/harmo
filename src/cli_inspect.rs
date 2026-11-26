//! The `inspect` command: prints a piece's ground truth beside the engine's favored reading.

use crate::bench::groundtruth::load_labels;
use crate::bench::manifest::Manifest;
use crate::cli_args::Args;
use crate::engine::path::best_segments;
use crate::engine::ruleset::{RuleSet, Weights};
use crate::engine::search::{Config, analyze};
use crate::pitch::tpc_name;
use crate::rules::default_rules;
use crate::score::musicxml::read_score;
use crate::score::slicing::{measure_at, slice_score};
use crate::theory::label::Label;
use std::path::Path;

pub fn run(args: &Args, weights: &Weights) -> Result<(), String> {
    let query = args.positional(0).ok_or("inspect needs a piece id (or part of one)")?;
    let manifest = Manifest::load(Path::new(args.value("data").unwrap_or("data")))?;
    let entry = manifest.pieces.iter().find(|p| p.id.contains(query)).ok_or("no piece matches")?;
    let analyst = args.value("analyst").unwrap_or("main");
    let truth_path = entry.analyses.get(analyst).ok_or("no such analyst for this piece")?;
    let score = read_score(&manifest.root.join(&entry.score))?;
    let truth = load_labels(&manifest.root.join(truth_path), &score)?;
    let slices = slice_score(&score);
    let rules = RuleSet::new(default_rules(), weights)?;
    let analysis = analyze(&slices, &rules, &Config::default());
    let predicted = best_segments(&analysis, &slices);
    println!("{} ({} vs engine)", entry.id, analyst);
    for (time, label) in &truth {
        let engine = predicted.iter().rev().find(|s| s.start <= time + 1e-6).map(|s| s.label);
        let measure = &score.measures[measure_at(&score.measures, *time)];
        let matches = match (label, engine) {
            (Some(a), Some(b)) => {
                if *a == b {
                    "  "
                } else {
                    "≠ "
                }
            }
            _ => "? ",
        };
        println!(
            "m{:<5} {:>8.2} {matches}{:<28} {}",
            measure.number,
            time,
            describe(label.as_ref()),
            describe(engine.as_ref())
        );
    }
    Ok(())
}

fn describe(label: Option<&Label>) -> String {
    match label {
        Some(l) => format!("{}: {} [{}{}]", l.key.name(), l.figure(), tpc_name(l.root), l.quality.label()),
        None => "-".into(),
    }
}
