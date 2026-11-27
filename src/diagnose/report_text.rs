//! Prints a compact text report of set diagnoses.

use super::key_dependence::ChordLayerCounts;
use super::report_text_detail::print_detail;
use super::summary::SetDiagnosis;
use crate::bench::equivalence::EQUIVALENCES;

/// Error groups in the order of the overview table, with short column names.
const GROUPS: [(&str, &str); 9] = [
    ("key", "key"),
    ("root", "root"),
    ("quality", "qual"),
    ("inversion", "inv"),
    ("function", "func"),
    ("unproducible", "unprod"),
    ("timing", "timing"),
    ("disagreement", "disagr"),
    ("no reading", "none"),
];

/// Prints an overview of every set, then the details of each set that is not a subset of another.
pub fn print_report(sets: &[SetDiagnosis]) {
    print_overview(sets);
    for set in sets.iter().filter(|s| !is_subset(&s.name, sets)) {
        print_detail(set);
    }
}

fn print_overview(sets: &[SetDiagnosis]) {
    println!("\nframes lost, by group (% of frames)");
    let columns: String = GROUPS.iter().map(|(_, short)| format!("{short:>7}")).collect();
    println!("{:<28} {:>8} {:>6} {:>6}{columns}", "set", "frames", "rn", "rn eq");
    for set in sets {
        let shares: String =
            GROUPS.iter().map(|(group, _)| format!("{:>7.1}", group_share(set, group))).collect();
        let (rn, rn_eq) = (percent(set.correct, set.frames), percent(set.equivalent, set.frames));
        println!("{:<28} {:>8} {rn:>6.1} {rn_eq:>6.1}{shares}", set.name, set.frames);
    }
    print_credits(sets);
    println!("\nlargest kinds by set (% of frames)");
    for set in sets {
        let top: Vec<String> = set
            .kinds
            .iter()
            .take(4)
            .map(|(name, k)| format!("{name} {:.1}", percent(k.frames, set.frames)))
            .collect();
        println!("{:<28} {}", set.name, top.join(" · "));
    }
    print_key_dependence(sets);
    print_monophony(sets);
}

fn print_credits(sets: &[SetDiagnosis]) {
    println!("\nframes that rn eq credits and rn does not, by equivalence (% of frames)");
    let columns: String = EQUIVALENCES.iter().map(|e| format!("{:>9}", e.name())).collect();
    println!("{:<28}{columns}", "set");
    for set in sets {
        let credited = |name: &str| set.credits.get(name).copied().unwrap_or(0);
        let shares: String = EQUIVALENCES
            .iter()
            .map(|e| format!("{:>9.1}", percent(credited(e.name()), set.frames)))
            .collect();
        println!("{:<28}{shares}", set.name);
    }
}

fn print_key_dependence(sets: &[SetDiagnosis]) {
    println!(
        "\nchord layer with the key right and wrong (% of frames; last column: % of chord-wrong frames with the key right)"
    );
    println!("{:<29}{:<38}key wrong", "", "key right");
    let side = format!("{:>8} {:>6} {:>6} {:>6} {:>6}", "frames", "qual", "inv", "root", "chord");
    println!("{:<28} {side}  {side} {:>8}", "set", "share");
    for set in sets {
        let counts = &set.key_dependence;
        let (right, wrong) = (format_layer(&counts.key_right), format_layer(&counts.key_wrong));
        let share = 100.0 * counts.chord_wrong_key_right_share();
        println!("{:<28} {right}  {wrong} {share:>8.1}", set.name);
    }
}

fn print_monophony(sets: &[SetDiagnosis]) {
    println!("\nframes where one pitch class sounds alone (% of those frames; kinds: % of all frames)");
    println!("{:<28} {:>8} {:>6} {:>6} {:>6}  largest kinds", "set", "frames", "share", "rn", "rn eq");
    for set in sets {
        let mono = &set.monophony;
        let mut kinds: Vec<(&String, &u64)> = mono.kinds.iter().collect();
        kinds.sort_by(|a, b| b.1.cmp(a.1));
        let top: Vec<String> =
            kinds.iter().take(4).map(|(name, f)| format!("{name} {:.2}", percent(**f, set.frames))).collect();
        println!(
            "{:<28} {:>8} {:>6.1} {:>6.1} {:>6.1}  {}",
            set.name,
            mono.frames,
            percent(mono.frames, set.frames),
            percent(mono.correct, mono.frames),
            percent(mono.equivalent, mono.frames),
            top.join(" · ")
        );
    }
}

fn format_layer(counts: &ChordLayerCounts) -> String {
    let rate = |part: u64| percent(part, counts.frames);
    format!(
        "{:>8} {:>6.1} {:>6.1} {:>6.1} {:>6.1}",
        counts.frames,
        rate(counts.quality),
        rate(counts.inversion),
        rate(counts.root),
        rate(counts.chord)
    )
}

fn group_share(set: &SetDiagnosis, group: &str) -> f64 {
    let frames = set.groups.iter().find(|(name, _)| name == group).map_or(0, |(_, frames)| *frames);
    percent(frames, set.frames)
}

/// Reports whether the set is a per-corpus subset of another listed set, such as "tune-wir" of "tune".
fn is_subset(name: &str, sets: &[SetDiagnosis]) -> bool {
    sets.iter().any(|s| name.strip_prefix(s.name.as_str()).is_some_and(|rest| rest.starts_with('-')))
}

pub fn percent(part: u64, whole: u64) -> f64 {
    if whole == 0 { 0.0 } else { 100.0 * part as f64 / whole as f64 }
}
