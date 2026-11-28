//! Command-line entry point: `harmo analyze | bench | diagnose | results | sweep | inspect | labelfit | rules`.

use harmo::bench::manifest::Manifest;
use harmo::bench::report::{self, RunRecord};
use harmo::bench::runner::{BenchPiece, EvalConfig, evaluate, load_pieces};
use harmo::bench::sweep::{SweepSpec, run_sweep};
use harmo::bench::{bootstrap, bootstrap_coverage};
use harmo::cli_args::Args;
use harmo::cli_overrides::load_overrides;
use harmo::diagnose::report_text::print_report;
use harmo::diagnose::summary as diagnosis;
use harmo::engine::ruleset::RuleSet;
use harmo::engine::search::Config;
use harmo::results::markdown::render;
use harmo::results::record::{ResultsRecord, RunHeader};
use harmo::rules::default_rules;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const USAGE: &str = "usage:
  harmo analyze <score.mxl|musicxml> [--json]
  harmo bench [--set NAME]... [--label TEXT] [--compare RUN.json] [--limit N] [--hindsight]
  harmo diagnose [--set NAME]... [--json] [--limit N]
  harmo results [--out FILE] [--limit N]
  harmo sweep <sweep.json> [--limit N]
  harmo inspect <piece-id> [--analyst NAME]
  harmo labelfit [--set NAME]... [--piece TEXT [--measures A-B]] [--min-gain FRAMES] [--top N] [--json]
  harmo rules
common options: --data DIR (default: data) --beam N --candidates N --distinct-depth N
  --offer-count N --offer-margin X --offer-lag QUARTERS
  --key-offer-margin X --chord-offer-margin X --numeral-offer-margin X (the margins of readings
    offered one layer at a time; each defaults to --offer-margin)
  --timing-tolerance QUARTERS (how far the equivalents RN measure looks for a shifted label)
  --weights FILE, --params FILE: JSON objects of rule weights by rule id and of rule
    parameters by <rule id>.<parameter name> (listed by harmo rules)
set aliases: test (AugmentedNet test split), dev (AugmentedNet validation split),
  tune (every piece outside the test split), each with its per-corpus subsets";

/// A piece that could not be loaded: its id and the reason.
type LoadFailure = (String, String);

/// The AugmentedNet modules, which the test and dev aliases report one by one.
const ANET_MODULES: &[&str] = &["wir", "abc_dcml", "mps", "wirwtc", "haydnsun", "tavern", "bps"];
/// The When in Rome categories whose pieces belong to no AugmentedNet module.
const OTHER_CATEGORIES: &[&str] =
    &["early_choral", "keyboard_other", "openscore-liedercorpus", "textbooks", "variations_and_grounds"];

fn main() {
    let args = Args::parse(std::env::args().skip(1));
    if args.wants_help() {
        println!("{USAGE}");
        return;
    }
    let result = match args.command() {
        Some("analyze") => harmo::cli_analyze::run(&args),
        Some("bench") => bench(&args),
        Some("diagnose") => diagnose(&args),
        Some("results") => results(&args),
        Some("sweep") => sweep(&args),
        Some("inspect") => load_overrides(&args).and_then(|w| harmo::cli_inspect::run(&args, &w)),
        Some("labelfit") => harmo::cli_labelfit::run(&args, &expand_sets(&args.values("set"), &[])),
        Some("rules") => {
            harmo::cli_analyze::print_rules();
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn bench(args: &Args) -> Result<(), String> {
    let sets = expand_sets(&args.values("set"), &["test"]);
    let rules = RuleSet::new(default_rules(), &load_overrides(args)?)?;
    let config = eval_config(args)?;
    let pieces = load(args, &sets)?;
    let started = Instant::now();
    let results = evaluate(&pieces, &rules, &config);
    let seconds = started.elapsed().as_secs_f64();
    let sets_summary = report::summarize(&results, &sets);
    let previous_path =
        args.value("compare").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("runs/latest.json"));
    let previous = report::load(&previous_path).ok();
    report::print_accuracy(&sets_summary, previous.as_ref());
    report::print_equivalence_credits(&sets_summary);
    if let Some(previous) = &previous {
        bootstrap::print_intervals(&sets_summary, &results, previous);
    }
    report::print_agreement(&sets_summary);
    if let Some(previous) = &previous {
        println!();
        bootstrap_coverage::print_coverage_intervals(&sets_summary, &results, previous);
        bootstrap_coverage::print_layer_coverage_intervals(&sets_summary, &results, previous);
    }
    let slices: usize = results.iter().map(|r| r.slices).sum();
    println!("\nanalyzed {} pieces ({slices} slices) in {seconds:.2}s", results.len());
    report::print_timing(&results);
    let record = RunRecord {
        label: args.value("label").unwrap_or("run").to_string(),
        timestamp: now(),
        seconds,
        settings: settings_of(&config),
        weights: rules.rules.iter().zip(&rules.weights).map(|(r, w)| (r.id.to_string(), *w)).collect(),
        params: rules.overridden_params().into_iter().collect(),
        sets: sets_summary,
        pieces: report::piece_records(&results),
    };
    let path = PathBuf::from(format!("runs/{}-{}.json", record.timestamp, record.label));
    report::save(&record, &path)?;
    report::save(&record, Path::new("runs/latest.json"))?;
    println!("saved {}", path.display());
    Ok(())
}

fn diagnose(args: &Args) -> Result<(), String> {
    let sets = expand_sets(&args.values("set"), &["tune"]);
    let rules = RuleSet::new(default_rules(), &load_overrides(args)?)?;
    let config = eval_config(args)?;
    let pieces = load(args, &sets)?;
    let diagnoses = diagnosis::diagnose_pieces(&pieces, &rules, &config);
    let summaries = diagnosis::summarize(&diagnoses, &sets);
    if args.has("json") {
        println!("{}", serde_json::to_string_pretty(&summaries).map_err(|e| e.to_string())?);
    } else {
        print_report(&summaries);
    }
    Ok(())
}

/// Runs the standard benchmark and diagnosis, and writes every figure to a Markdown document.
fn results(args: &Args) -> Result<(), String> {
    let started = Instant::now();
    let rules = RuleSet::new(default_rules(), &load_overrides(args)?)?;
    let config = EvalConfig { measure_offer_delays: true, measure_hindsight: true, ..eval_config(args)? };
    let names = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let bench_sets = expand_sets(&names(&["test", "agreement", "tune", "agreement-tune"]), &[]);
    let diagnose_sets = expand_sets(&names(&["tune", "agreement-tune"]), &[]);
    let (pieces, failures) = load_with_failures(args, &bench_sets)?;
    let bench_started = Instant::now();
    let results = evaluate(&pieces, &rules, &config);
    let bench_seconds = bench_started.elapsed().as_secs_f64();
    let accuracy = report::summarize(&results, &bench_sets).into_iter().filter(|s| s.name != "all").collect();
    let piece_count = pieces.len();
    let diagnosed: Vec<_> =
        pieces.into_iter().filter(|p| p.sets.iter().any(|s| diagnose_sets.contains(s))).collect();
    let diagnose_started = Instant::now();
    let diagnoses =
        diagnosis::summarize(&diagnosis::diagnose_pieces(&diagnosed, &rules, &config), &diagnose_sets);
    let diagnose_seconds = diagnose_started.elapsed().as_secs_f64();
    let header = RunHeader {
        commit: command_output("git", &["rev-parse", "--short", "HEAD"]),
        has_uncommitted_changes: has_uncommitted_changes(),
        date: command_output("date", &["-u", "+%Y-%m-%d %H:%M UTC"]),
        settings: settings_of(&config),
        weights: override_files(args),
        pieces: piece_count,
        failures,
        threads: rayon::current_num_threads(),
        bench_seconds,
        diagnose_seconds,
        total_seconds: started.elapsed().as_secs_f64(),
    };
    let path = args.value("out").unwrap_or("RESULTS.md");
    std::fs::write(path, render(&ResultsRecord { header, accuracy, diagnoses }))
        .map_err(|e| format!("{path}: {e}"))?;
    println!("wrote {path} in {:.1}s", started.elapsed().as_secs_f64());
    Ok(())
}

/// Names the weights and parameters files given on the command line, if any.
fn override_files(args: &Args) -> Option<String> {
    let files: Vec<&str> = [args.value("weights"), args.value("params")].into_iter().flatten().collect();
    (!files.is_empty()).then(|| files.join("` and `"))
}

/// Reports whether the engine's source differs from the commit.
fn has_uncommitted_changes() -> bool {
    !command_output("git", &["status", "--porcelain", "--", "src", "Cargo.toml", "Cargo.lock"]).is_empty()
}

/// Runs a command and returns its trimmed standard output, or "unknown" if it cannot run.
fn command_output(program: &str, arguments: &[&str]) -> String {
    let output = std::process::Command::new(program).args(arguments).output();
    output
        .ok()
        .filter(|o| o.status.success())
        .map_or("unknown".to_string(), |o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn sweep(args: &Args) -> Result<(), String> {
    let spec_path = args.positional(0).ok_or(USAGE)?;
    let text = std::fs::read_to_string(spec_path).map_err(|e| format!("{spec_path}: {e}"))?;
    let mut spec: SweepSpec = serde_json::from_str(&text).map_err(|e| format!("{spec_path}: {e}"))?;
    spec.sets = expand_sets(&spec.sets, &[]);
    let config = eval_config(args)?;
    let pieces = load(args, &spec.sets)?;
    let started = Instant::now();
    let mut progress = |done: usize, total: usize, result: &harmo::bench::sweep::SweepResult| {
        let elapsed = started.elapsed().as_secs_f64();
        println!("[{done}/{total} {elapsed:.1}s] objective {:.4}  {:?}", result.objective, result.weights);
    };
    let results = run_sweep(&spec, &default_rules(), &pieces, &config, &mut progress)?;
    println!("\nbest settings:");
    for result in results.iter().take(10) {
        println!("{:.4}  {:?}", result.objective, result.weights);
    }
    let path = PathBuf::from(format!("runs/sweep-{}.json", now()));
    let text = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
    std::fs::create_dir_all("runs").map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    println!("saved {}", path.display());
    Ok(())
}

fn load(args: &Args, sets: &[String]) -> Result<Vec<BenchPiece>, String> {
    load_with_failures(args, sets).map(|(pieces, _)| pieces)
}

/// Loads the pieces of the given sets, and also returns the ids of those that failed, with the reason.
fn load_with_failures(args: &Args, sets: &[String]) -> Result<(Vec<BenchPiece>, Vec<LoadFailure>), String> {
    let manifest = Manifest::load(Path::new(args.value("data").unwrap_or("data")))?;
    let mut selected = manifest.select(sets);
    if let Some(limit) = args.number("limit")? {
        selected.truncate(limit as usize);
    }
    let started = Instant::now();
    let (pieces, errors) = load_pieces(&manifest, &selected);
    for (id, error) in &errors {
        eprintln!("could not load {id}: {error}");
    }
    eprintln!(
        "loaded {} pieces in {:.2}s ({} failed)",
        pieces.len(),
        started.elapsed().as_secs_f64(),
        errors.len()
    );
    Ok((pieces, errors))
}

/// Expands the aliases "test", "dev" and "tune" into a set and its per-corpus subsets.
fn expand_sets(names: &[String], default: &[&str]) -> Vec<String> {
    let names: Vec<String> =
        if names.is_empty() { default.iter().map(|s| s.to_string()).collect() } else { names.to_vec() };
    let with_subsets = |set: &str, subsets: &[&[&str]]| {
        let subsets = subsets.iter().flat_map(|group| group.iter()).map(|s| format!("{set}-{s}"));
        std::iter::once(set.to_string()).chain(subsets).collect::<Vec<_>>()
    };
    let expand = |name: &String| match name.as_str() {
        "test" => with_subsets("anet-test", &[ANET_MODULES]),
        "dev" => with_subsets("anet-validation", &[ANET_MODULES]),
        "tune" => with_subsets("tune", &[ANET_MODULES, OTHER_CATEGORIES]),
        _ => vec![name.clone()],
    };
    names.iter().flat_map(expand).collect()
}

fn eval_config(args: &Args) -> Result<EvalConfig, String> {
    let defaults = EvalConfig::default();
    let number = |name: &str, default: f64| args.number(name).map(|n| n.unwrap_or(default));
    let offer_margin = number("offer-margin", defaults.offer_margin as f64)?;
    Ok(EvalConfig {
        engine: Config {
            beam_width: number("beam", defaults.engine.beam_width as f64)? as usize,
            candidate_limit: number("candidates", defaults.engine.candidate_limit as f64)? as usize,
            distinct_depth: number("distinct-depth", defaults.engine.distinct_depth as f64)? as usize,
            record_snapshots: false,
            record_pruning: false,
        },
        offer_count: number("offer-count", defaults.offer_count as f64)? as usize,
        offer_margin: offer_margin as f32,
        offer_lag: number("offer-lag", defaults.offer_lag)?,
        timing_tolerance: number("timing-tolerance", defaults.timing_tolerance)?,
        measure_offer_delays: false,
        key_offer_margin: number("key-offer-margin", offer_margin)? as f32,
        chord_offer_margin: number("chord-offer-margin", offer_margin)? as f32,
        numeral_offer_margin: number("numeral-offer-margin", offer_margin)? as f32,
        measure_hindsight: args.has("hindsight"),
    })
}

fn settings_of(config: &EvalConfig) -> std::collections::BTreeMap<String, f64> {
    [
        ("beam", config.engine.beam_width as f64),
        ("candidates", config.engine.candidate_limit as f64),
        ("distinct_depth", config.engine.distinct_depth as f64),
        ("offer_count", config.offer_count as f64),
        ("offer_margin", config.offer_margin as f64),
        ("offer_lag", config.offer_lag),
        ("key_offer_margin", config.key_offer_margin as f64),
        ("chord_offer_margin", config.chord_offer_margin as f64),
        ("numeral_offer_margin", config.numeral_offer_margin as f64),
        ("timing_tolerance", config.timing_tolerance),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}
