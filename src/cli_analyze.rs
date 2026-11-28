//! The `analyze` and `rules` commands.

use crate::cli_args::Args;
use crate::cli_overrides::load_overrides;
use crate::engine::ruleset::RuleSet;
use crate::engine::search::{Config, analyze};
use crate::output::analysis::{OutputOptions, Status, build_output};
use crate::rules::default_rules;
use crate::rules::sources::SOURCES;
use crate::score::musicxml::read_score;
use crate::score::slicing::slice_score;
use std::path::Path;

pub fn run(args: &Args) -> Result<(), String> {
    let path = args.positional(0).ok_or("analyze needs a score path")?;
    let rules = RuleSet::new(default_rules(), &load_overrides(args)?)?;
    let score = read_score(Path::new(path))?;
    let slices = slice_score(&score);
    let config = Config { record_pruning: true, ..Config::default() };
    let analysis = analyze(&slices, &rules, &config);
    let output =
        build_output(&score, &slices, &analysis, &rules, &OutputOptions { alternatives: 8, margin: 4.0 });
    if args.has("json") {
        println!("{}", serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?);
        return Ok(());
    }
    for segment in &output.segments {
        let marker = if segment.status == Status::Interpretive { "?" } else { " " };
        let rivals: Vec<String> = segment
            .alternatives
            .iter()
            .take(3)
            .map(|a| format!("{}: {} (-{:.1})", a.reading.key, a.reading.figure, a.score_gap))
            .collect();
        println!(
            "m{:<5} {:>7.2}  {:<4} {:<10}{marker} {}",
            segment.measure,
            segment.start,
            segment.reading.key,
            segment.reading.figure,
            rivals.join(", ")
        );
    }
    Ok(())
}

pub fn print_rules() {
    for rule in default_rules() {
        println!("{} [{:?}, {}] weight {}", rule.id, rule.topic, rule.hook.kind(), rule.default_weight);
        println!("    {}", rule.summary);
        for citation in rule.citations {
            println!("    - {} {}", citation.source, citation.locator);
        }
        for param in rule.params {
            println!("    param {}.{} = {} {}", rule.id, param.name, param.default, param.unit);
            println!("        {}", param.provenance);
        }
    }
    println!("\nsources:");
    for source in SOURCES {
        println!("  {}: {}", source.id, source.reference);
    }
}
