//! Renders where the engine's errors are, from the diagnosis of each set.

use super::markdown_disagreements::render_disagreements;
use super::markdown_errors_examples::{render_examples, render_spans};
use super::markdown_kinds::{render_corpus_kinds, render_kinds, render_unproducible};
use super::markdown_set_figures::{render_beam, render_key_dependence};
use super::markdown_table::{share, table};
use super::set_names::{family, row_name};
use crate::diagnose::summary::SetDiagnosis;

/// Error groups in the order of the overview table.
const GROUPS: [&str; 9] = [
    "key",
    "root",
    "quality",
    "inversion",
    "function",
    "unproducible",
    "timing",
    "disagreement",
    "no reading",
];

const INTRODUCTION: &str = "## Where the errors are

These figures come from `diagnose` on `tune` and `agreement-tune`. Every frame that the top reading gets wrong is assigned exactly one error kind:
- **Where the analysts agree, or there is only one,** a wrong key is classed by how the two keys relate, and as sustained or brief. With the key right, the error is in the root (a third, fifth or step away), the function (applied or not), the quality (a seventh added or missing) or the inversion.
- **Unproducible** labels are ones the engine cannot produce at all, whatever it hears. **Timing** means the label is right but its change falls up to a quarter note away from the expert's.
- **Disagreement** frames are those where a piece's analysts differ. They are set apart here and broken down further below.

### Frames lost, by group

Each figure is the percentage of the set's frames lost to that group of errors. RN is the share right, so a row adds up to 100.
";

pub fn render_errors(sets: &[SetDiagnosis], offer_count: usize) -> String {
    let ordered = ordered(sets);
    let top_level: Vec<&SetDiagnosis> = sets.iter().filter(|s| !s.name.starts_with("tune-")).collect();
    let mut sections = vec![format!("{INTRODUCTION}\n{}", groups_table(&ordered))];
    sections.extend(top_level.iter().map(|s| render_kinds(s)));
    sections.push(render_corpus_kinds(&family(sets, "tune", |s| &s.name)));
    sections.extend(sets.iter().find(|s| s.name == "tune").map(render_unproducible));
    sections.push(render_disagreements(&top_level));
    sections.push(render_key_dependence(&ordered));
    sections.push(render_spans(&top_level));
    sections.push(render_beam(&ordered, offer_count));
    sections.push(render_examples(&top_level));
    sections.join("\n")
}

/// Lists the subsets of `tune`, then `tune`, then the other sets.
fn ordered(sets: &[SetDiagnosis]) -> Vec<&SetDiagnosis> {
    let mut ordered = family(sets, "tune", |s| &s.name);
    ordered.extend(sets.iter().filter(|s| s.name != "tune" && !s.name.starts_with("tune-")));
    ordered
}

fn groups_table(sets: &[&SetDiagnosis]) -> String {
    let mut header = vec!["set", "frames", "RN"];
    header.extend(GROUPS);
    let row = |set: &&SetDiagnosis| {
        let frames = |group: &str| set.groups.iter().find(|(g, _)| g == group).map_or(0, |(_, f)| *f);
        let mut cells = vec![row_name(&set.name), set.frames.to_string(), share(set.correct, set.frames)];
        cells.extend(GROUPS.iter().map(|g| share(frames(g), set.frames)));
        cells
    };
    table(&header, &sets.iter().map(row).collect::<Vec<_>>())
}
