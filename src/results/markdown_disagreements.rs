//! Renders the classes of frames where the analysts disagree, and why readings are missing there.

use super::markdown_table::{share, table};
use crate::diagnose::summary::SetDiagnosis;
use crate::diagnose::tally_class::ClassTally;

const DISAGREEMENT_HEADER: [&str; 9] = [
    "class",
    "frames",
    "top matches either",
    "both offered",
    "missing: cannot produce",
    "missing: in the beam, not offered",
    "its mean rank",
    "its mean score gap",
    "missing: not in the beam",
];

const DISAGREEMENT_INTRODUCTION: &str = "### Where the analysts disagree, by class

Frames where two analysts differ are classed by what they differ on. **Frames** counts each pair of analysts once. **Top matches either** and **both offered** are percentages of the class's frames, as in the agreement tables. The other columns explain the readings that are not offered. They count readings rather than frames, since a frame can miss both of its analysts' readings:
- **Cannot produce:** the engine has no way to produce the reading.
- **In the beam, not offered:** a hypothesis in the beam holds it, but below the offer cut-off. Its mean rank (1 is the best hypothesis) and how far its score lies below the best are given beside it.
- **Not in the beam:** no hypothesis in the beam holds it.
";

pub fn render_disagreements(sets: &[&SetDiagnosis]) -> String {
    let mut sections = vec![DISAGREEMENT_INTRODUCTION.to_string()];
    let mut with_classes: Vec<&&SetDiagnosis> = sets.iter().filter(|s| !s.disagreements.is_empty()).collect();
    with_classes.sort_by_key(|s| s.name != "agreement-tune");
    for set in with_classes {
        let mut total = ClassTally::default();
        let mut rows: Vec<Vec<String>> = Vec::new();
        for (name, class) in &set.disagreements {
            rows.push(class_row(name, class));
            total.merge(class);
        }
        rows.push(class_row("all", &total));
        sections.push(format!("**`{}`**\n\n{}", set.name, table(&DISAGREEMENT_HEADER, &rows)));
    }
    sections.join("\n")
}

fn class_row(name: &str, class: &ClassTally) -> Vec<String> {
    let below = class.missing_below_cutoff.max(1) as f64;
    vec![
        name.to_string(),
        class.frames.to_string(),
        share(class.top_matches_either, class.frames),
        share(class.both_offered, class.frames),
        class.missing_unproducible.to_string(),
        class.missing_below_cutoff.to_string(),
        format!("{:.1}", class.below_cutoff_rank as f64 / below),
        format!("{:.1}", class.below_cutoff_gap / below),
        class.missing_outside_beam.to_string(),
    ]
}
