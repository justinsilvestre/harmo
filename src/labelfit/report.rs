//! Summarizes label fits by set, piece, key span and measure, and prints the candidates for correction.

use super::candidates::{Alteration, Candidate, altered_fit};
use super::piece::{GroupFit, PieceFit, group_fits};
use super::sounding::Fit;
use crate::pitch::tpc_name;
use serde::Serialize;

#[derive(Serialize)]
pub struct PieceSummary {
    pub id: String,
    pub analyst: String,
    pub sets: Vec<String>,
    pub frames: f64,
    pub fit: Option<f64>,
    pub labels: usize,
    pub labels_beyond_bar: usize,
    pub key_spans: Vec<GroupFit>,
    pub measures: Vec<GroupFit>,
}

#[derive(Serialize)]
pub struct SetSummary {
    pub name: String,
    pub analyses: usize,
    pub frames: f64,
    pub fit: Option<f64>,
}

#[derive(Serialize)]
pub struct LabelFitReport {
    pub sets: Vec<SetSummary>,
    pub pieces: Vec<PieceSummary>,
    pub candidates: Vec<Candidate>,
}

pub fn summarize_piece(piece: &PieceFit) -> PieceSummary {
    PieceSummary {
        id: piece.id.clone(),
        analyst: piece.analyst.clone(),
        sets: piece.sets.clone(),
        frames: piece.frames(),
        fit: piece.total_fit().share(),
        labels: piece.spans.len(),
        labels_beyond_bar: piece.labels_beyond_bar,
        key_spans: group_fits(&piece.spans, |s| s.label.key.name()),
        measures: group_fits(&piece.spans, |s| s.measure_name()),
    }
}

/// Pools the fits of the analyses in each set, weighting each by its sounding time.
pub fn summarize_sets(pieces: &[PieceFit], names: &[String]) -> Vec<SetSummary> {
    let summarize = |name: &String| {
        let members: Vec<&PieceFit> = pieces.iter().filter(|p| p.sets.contains(name)).collect();
        SetSummary {
            name: name.clone(),
            analyses: members.len(),
            frames: members.iter().map(|p| p.frames()).sum(),
            fit: members.iter().fold(Fit::default(), |sum, p| sum + p.total_fit()).share(),
        }
    };
    names.iter().map(summarize).filter(|s| s.analyses > 0).collect()
}

pub fn print_report(report: &LabelFitReport, top: usize) {
    println!("chord-tone fit of the labels, by set (share of sounding time, weighted by duration)");
    println!("{:40} {:>8} {:>10} {:>6}", "set", "analyses", "frames", "fit");
    for set in &report.sets {
        println!("{:40} {:>8} {:>10.0} {:>6}", set.name, set.analyses, set.frames, percent(set.fit));
    }
    let mut lowest: Vec<&PieceSummary> = report.pieces.iter().filter(|p| p.frames >= 256.0).collect();
    lowest.sort_by(|a, b| a.fit.unwrap_or(0.0).total_cmp(&b.fit.unwrap_or(0.0)));
    println!("\nlowest-fitting analyses (at least 256 frames)");
    for piece in lowest.iter().take(top.min(20)) {
        println!("{:>6} {:>7.0}  {} ({})", percent(piece.fit), piece.frames, piece.id, piece.analyst);
    }
    println!("\nanalyses that place labels past the end of their bar (the meters may disagree)");
    for piece in report.pieces.iter().filter(|p| p.labels_beyond_bar > 0) {
        let share = format!("{}/{}", piece.labels_beyond_bar, piece.labels);
        println!("{share:>9}  {:>6}  {} ({})", percent(piece.fit), piece.id, piece.analyst);
    }
    println!("\ncandidates, ranked by gain (frames at stake times the gain in fit)");
    println!("{:>6} {:>6} {:>11} {:>13}  piece (analyst): change", "gain", "frames", "fit", "measures");
    for candidate in report.candidates.iter().take(top) {
        print_candidate(candidate);
    }
}

fn print_candidate(candidate: &Candidate) {
    let fit = format!("{} -> {}", percent(Some(candidate.fit_before)), percent(Some(candidate.fit_after)));
    let measures = format!("m{}-{}", candidate.first_measure, candidate.last_measure);
    println!(
        "{:>6.0} {:>6.0} {fit:>11} {measures:>13}  {} ({}): {}, {} labels",
        candidate.gain,
        candidate.frames,
        candidate.piece,
        candidate.analyst,
        candidate.description,
        candidate.labels
    );
}

/// Prints the fit of every key span and measure of one analysis,
/// with each measure's fit when its labels move by up to two bars either way.
pub fn print_piece_detail(piece: &PieceFit) {
    let summary = summarize_piece(piece);
    println!(
        "\n{} ({}): fit {} over {:.0} frames",
        piece.id,
        piece.analyst,
        percent(summary.fit),
        summary.frames
    );
    println!("key spans:");
    for span in &summary.key_spans {
        println!("  {:>6} m{}-{} {}", percent(span.fit), span.first_measure, span.last_measure, span.name);
    }
    println!("measures: fit as labeled, then with the labels 2 and 1 bars earlier and 1 and 2 bars later");
    let mut index = 0;
    for measure in &summary.measures {
        let mut members = Vec::new();
        while index < piece.spans.len() && piece.spans[index].measure_name() == measure.name {
            members.push(&piece.spans[index]);
            index += 1;
        }
        let shifted = [-2, -1, 1, 2].map(|bars| {
            let fits = members.iter().filter_map(|s| altered_fit(piece, s, Alteration::Bars(bars)));
            percent(fits.fold(Fit::default(), |sum, fit| sum + fit).share())
        });
        println!(
            "  {:>6} | {:>6} {:>6} {:>6} {:>6}  m{}",
            percent(measure.fit),
            shifted[0],
            shifted[1],
            shifted[2],
            shifted[3],
            measure.name
        );
    }
}

/// Prints each label from measure `first` to `last` of the analysis, with its chord tones,
/// its fit and the pitch classes that sound while it is in force, longest first.
pub fn print_labels(piece: &PieceFit, first: i64, last: i64) {
    println!("\n{} ({}): labels in m{first}-{last}", piece.id, piece.analyst);
    for span in piece.spans.iter().filter(|s| (first..=last).contains(&s.measure)) {
        let chord: Vec<String> = span.pitches.iter().map(|p| tpc_name(*p)).collect();
        let durations = piece.sounding.durations(span.start, span.end);
        let mut classes: Vec<usize> = (0..12).filter(|pc| durations[*pc] > 1e-9).collect();
        classes.sort_by(|a, b| durations[*b].total_cmp(&durations[*a]));
        let total: f64 = durations.iter().sum();
        let sounding: Vec<String> = classes
            .iter()
            .map(|pc| format!("{}{:.0}", PITCH_CLASS_NAMES[*pc], 100.0 * durations[*pc] / total))
            .collect();
        println!(
            "  m{:<6} {:>8.2} {:>6} {:>4}: {:<10} {:<14} | {}",
            span.measure_name(),
            span.start,
            percent(span.fit.share()),
            span.label.key.name(),
            span.label.figure(),
            chord.join(" "),
            sounding.join(" ")
        );
    }
}

const PITCH_CLASS_NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

fn percent(share: Option<f64>) -> String {
    share.map_or("-".to_string(), |s| format!("{:.1}", 100.0 * s))
}
