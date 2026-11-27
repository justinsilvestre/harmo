//! The `labelfit` command: checks every analysis's labels against the notes and lists likely errors.

use crate::bench::groundtruth::load_harmonies;
use crate::bench::manifest::{Manifest, ManifestPiece};
use crate::cli_args::Args;
use crate::labelfit::candidates::{MINIMUM_GAIN, find_candidates};
use crate::labelfit::piece::PieceFit;
use crate::labelfit::report::{
    LabelFitReport, print_labels, print_piece_detail, print_report, summarize_piece, summarize_sets,
};
use crate::score::musicxml::read_score;
use rayon::prelude::*;
use std::path::Path;

/// Runs the check on the pieces of the given sets (all pieces when none is given).
/// `--piece TEXT` restricts it to pieces whose id contains the text and prints their key spans and measures,
/// or, with `--measures FIRST-LAST`, each label in those measures.
pub fn run(args: &Args, sets: &[String]) -> Result<(), String> {
    let manifest = Manifest::load(Path::new(args.value("data").unwrap_or("data")))?;
    let query = args.value("piece");
    let selected: Vec<&ManifestPiece> =
        manifest.select(sets).into_iter().filter(|p| query.is_none_or(|q| p.id.contains(q))).collect();
    let analyses: Vec<(&ManifestPiece, &String)> =
        selected.iter().flat_map(|p| p.analyses.keys().map(move |analyst| (*p, analyst))).collect();
    let loaded: Vec<Result<PieceFit, String>> =
        analyses.par_iter().map(|(piece, analyst)| load(&manifest, piece, analyst)).collect();
    let mut pieces = Vec::new();
    for result in loaded {
        match result {
            Ok(piece) => pieces.push(piece),
            Err(error) => eprintln!("{error}"),
        }
    }
    let minimum_gain = args.number("min-gain")?.unwrap_or(MINIMUM_GAIN);
    let mut candidates: Vec<_> = pieces.par_iter().flat_map(|p| find_candidates(p, minimum_gain)).collect();
    candidates.sort_by(|a, b| b.gain.total_cmp(&a.gain));
    let set_names = if sets.is_empty() { all_set_names(&manifest) } else { sets.to_vec() };
    let report = LabelFitReport {
        sets: summarize_sets(&pieces, &set_names),
        pieces: pieces.iter().map(summarize_piece).collect(),
        candidates,
    };
    if args.has("json") {
        println!("{}", serde_json::to_string(&report).map_err(|e| e.to_string())?);
        return Ok(());
    }
    print_report(&report, args.number("top")?.map_or(40, |n| n as usize));
    match measure_range(args)? {
        Some((first, last)) => pieces.iter().for_each(|p| print_labels(p, first, last)),
        None if query.is_some() => pieces.iter().for_each(print_piece_detail),
        None => {}
    }
    Ok(())
}

/// Reads `--measures FIRST-LAST`, or a single measure.
fn measure_range(args: &Args) -> Result<Option<(i64, i64)>, String> {
    let Some(text) = args.value("measures") else {
        return Ok(None);
    };
    let parse = |part: &str| {
        part.trim().parse::<i64>().map_err(|_| format!("--measures expects FIRST-LAST, got {text}"))
    };
    let (first, last) = text.split_once('-').unwrap_or((text, text));
    Ok(Some((parse(first)?, parse(last)?)))
}

fn load(manifest: &Manifest, piece: &ManifestPiece, analyst: &str) -> Result<PieceFit, String> {
    let failure = |error: String| format!("could not load {} ({analyst}): {error}", piece.id);
    let score = read_score(&manifest.root.join(&piece.score)).map_err(failure)?;
    let harmonies = load_harmonies(&manifest.root.join(&piece.analyses[analyst]), &score).map_err(failure)?;
    let identity = (piece.id.clone(), analyst.to_string(), piece.sets.clone());
    Ok(PieceFit::new(identity, &score, &harmonies))
}

fn all_set_names(manifest: &Manifest) -> Vec<String> {
    let mut names: Vec<String> = manifest.pieces.iter().flat_map(|p| p.sets.iter().cloned()).collect();
    names.sort();
    names.dedup();
    names
}
