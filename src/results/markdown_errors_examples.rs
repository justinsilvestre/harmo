//! Renders the longest wrong spans and examples of each error kind and disagreement class.

use super::markdown_table::{quarters, table};
use crate::diagnose::summary::SetDiagnosis;
use crate::diagnose::tally::Example;

const LONGEST_SPANS: usize = 10;
const KINDS_WITH_EXAMPLES: usize = 8;
const EXAMPLES_PER_KIND: usize = 3;
const EXAMPLE_HEADER: [&str; 6] = ["kind", "piece", "measure", "quarter notes", "expert", "engine"];

pub fn render_spans(sets: &[&SetDiagnosis]) -> String {
    let tables = sets.iter().map(|set| format!("**`{}`**\n\n{}", set.name, spans_table(set)));
    format!(
        "### The longest wrong spans where the analysts agree\n\nEach row is a run of consecutive wrong frames of one kind, with the measure where it starts and its length in quarter notes.\n\n{}",
        tables.collect::<Vec<_>>().join("\n")
    )
}

fn spans_table(set: &SetDiagnosis) -> String {
    let errors = set.kinds.iter().filter(|(_, k)| k.group != "disagreement");
    let mut spans: Vec<(&String, &Example)> =
        errors.flat_map(|(name, k)| k.examples.iter().map(move |e| (name, e))).collect();
    spans.sort_by_key(|(_, e)| std::cmp::Reverse(e.frames));
    let rows: Vec<Vec<String>> =
        spans.into_iter().take(LONGEST_SPANS).map(|(n, e)| example_row(n, e)).collect();
    table(&EXAMPLE_HEADER, &rows)
}

pub fn render_examples(sets: &[&SetDiagnosis]) -> String {
    let blocks = sets.iter().map(|set| {
        format!(
            "<details>\n<summary>Examples on <code>{}</code></summary>\n\n{}\n</details>\n",
            set.name,
            examples_table(set)
        )
    });
    format!(
        "### Examples of the largest kinds and classes\n\nThe longest spans of each of the largest error kinds and disagreement classes. For a disagreement, the expert column gives both analysts' labels.\n\n{}",
        blocks.collect::<Vec<_>>().join("\n")
    )
}

fn examples_table(set: &SetDiagnosis) -> String {
    let errors =
        set.kinds.iter().filter(|(_, k)| k.group != "disagreement").map(|(n, k)| (n.clone(), &k.examples));
    let classes = set.disagreements.iter().map(|(n, c)| (format!("disagreement: {n}"), &c.examples));
    let mut rows = Vec::new();
    for (name, examples) in errors.take(KINDS_WITH_EXAMPLES).chain(classes.take(KINDS_WITH_EXAMPLES)) {
        rows.extend(examples.iter().take(EXAMPLES_PER_KIND).map(|e| example_row(&name, e)));
    }
    table(&EXAMPLE_HEADER, &rows)
}

fn example_row(kind: &str, example: &Example) -> Vec<String> {
    vec![
        kind.to_string(),
        example.piece.clone(),
        example.measure.clone(),
        quarters(example.frames),
        example.expert.clone(),
        example.engine.clone(),
    ]
}
