//! Prints the disagreement table: for each class, how often the engine matches or offers the
//! analysts' readings, and why missing readings are missing.

use super::report_text::percent;
use super::tally_class::ClassTally;

pub fn print_disagreements(classes: &[(String, ClassTally)]) {
    let frames: u64 = classes.iter().map(|(_, c)| c.frames).sum();
    if frames == 0 {
        return;
    }
    println!(
        "\ndisagreements ({frames} frames, per pair of analysts): % of the class's frames, then missing readings by reason"
    );
    println!(
        "  {:<32} {:>7} {:>10} {:>6} | {:>7} {:>20} {:>7}",
        "class", "frames", "top∈either", "both", "unprod", "below (rank, gap)", "outside"
    );
    let mut total = ClassTally::default();
    for (name, class) in classes {
        print_class(name, class);
        total.merge(class);
    }
    print_class("all", &total);
}

fn print_class(name: &str, class: &ClassTally) {
    let below = class.missing_below_cutoff.max(1) as f64;
    println!(
        "  {name:<32} {:>7} {:>10.1} {:>6.1} | {:>7} {:>7} ({:>4.1}, {:>4.1}) {:>7}",
        class.frames,
        percent(class.top_matches_either, class.frames),
        percent(class.both_offered, class.frames),
        class.missing_unproducible,
        class.missing_below_cutoff,
        class.below_cutoff_rank as f64 / below,
        class.below_cutoff_gap / below,
        class.missing_outside_beam
    );
}
