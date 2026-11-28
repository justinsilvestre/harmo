//! The `analyze` and `rules` commands.

use crate::bench::runner::EvalConfig;
use crate::cli_args::Args;
use crate::cli_overrides::load_overrides;
use crate::engine::ruleset::RuleSet;
use crate::engine::search::{Config, analyze};
use crate::output::analysis::{SegmentOutput, Status, build_output};
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
    let config = offer_config(args)?;
    let analysis = analyze(&slices, &rules, &config.engine);
    let output = build_output(&score, &slices, &analysis, &rules, &config);
    if args.has("json") {
        println!("{}", serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?);
        return Ok(());
    }
    for segment in &output.segments {
        println!("{}", segment_line(segment));
    }
    Ok(())
}

/// Returns the default settings with the offer settings given on the command line,
/// recording pruning for the rank histories.
fn offer_config(args: &Args) -> Result<EvalConfig, String> {
    let defaults = EvalConfig::default();
    let number = |name: &str, default: f64| args.number(name).map(|n| n.unwrap_or(default));
    let margin = number("offer-margin", defaults.offer_margin as f64)?;
    Ok(EvalConfig {
        engine: Config { record_pruning: true, ..Config::default() },
        offer_count: number("offer-count", defaults.offer_count as f64)? as usize,
        offer_margin: margin as f32,
        offer_lag: number("offer-lag", defaults.offer_lag)?,
        key_offer_margin: number("key-offer-margin", margin)? as f32,
        chord_offer_margin: number("chord-offer-margin", margin)? as f32,
        numeral_offer_margin: number("numeral-offer-margin", margin)? as f32,
        ..defaults
    })
}

/// Formats a span with a `?` when it is interpretive, then up to three other readings offered for it.
fn segment_line(segment: &SegmentOutput) -> String {
    let marker = if segment.status == Status::Interpretive { "?" } else { " " };
    let others = segment.offered.joint.iter().filter(|offer| offer.reading != segment.reading);
    let rivals: Vec<String> = others
        .take(3)
        .map(|o| format!("{}: {} (-{:.1})", o.reading.key, o.reading.figure, o.score_gap))
        .collect();
    format!(
        "m{:<5} {:>7.2}  {:<4} {:<10}{marker} {}",
        segment.measure,
        segment.start,
        segment.reading.key,
        segment.reading.figure,
        rivals.join(", ")
    )
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
