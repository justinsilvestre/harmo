//! Renders the classes of frames where the analysts disagree, and why readings are missing there.

use super::markdown_table::{share, table};
use crate::diagnose::summary::SetDiagnosis;
use crate::diagnose::tally_class::ClassTally;

const DISAGREEMENT_HEADER: [&str; 13] = [
    "class",
    "frames",
    "top matches either",
    "both offered",
    "missing: cannot produce",
    "missing: in the beam, not offered",
    "its mean rank",
    "its mean score gap",
    "missing: cut by beam width",
    "missing: cut by the hearing limit",
    "missing: merged by distinct depth",
    "their mean time to pruning",
    "missing: never held",
];

const DISAGREEMENT_INTRODUCTION: &str = "### Where the analysts disagree, by class

Frames where two analysts differ are classed by what they differ on. **Frames** counts each pair of analysts once. **Top matches either** and **both offered** are percentages of the class's frames, as in the agreement tables. The other columns explain the readings that are not offered. They count readings rather than frames, since a frame can miss both of its analysts' readings:
- **Cannot produce:** the engine has no way to produce the reading.
- **In the beam, not offered:** a hypothesis in the beam holds it, but below the offer cut-off. Its mean rank (1 is the best hypothesis) and how far its score lies below the best are given beside it.
- **Not in the beam:** no hypothesis in the beam holds it. Either the hypotheses that held it were all pruned, because the beam was full of better ones (**cut by beam width**), because the beam already held as many hearings of the chords as it may, each read by a better hypothesis, and they read another (**cut by the hearing limit**), or because better ones gave their last segments the same readings with other boundaries (**merged by distinct depth**), or no hypothesis ever gave it to the frame (**never held**). The mean time to pruning is how many quarter notes after the frame's slice began the last of them was pruned; 0 means on arrival.
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
        class.missing_cut_by_width.to_string(),
        class.missing_cut_by_hearings.to_string(),
        class.missing_merged.to_string(),
        format!("{:.1}", class.mean_pruned_after()),
        class.missing_never_held.to_string(),
    ]
}
