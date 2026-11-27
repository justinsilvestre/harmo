//! Renders the error kinds ranked by frames lost, the largest kinds in each corpus, and the labels
//! the engine cannot produce.

use super::markdown_table::{share, table};
use super::set_names::row_name;
use crate::diagnose::summary::SetDiagnosis;
use crate::diagnose::tally::KindTally;

const KINDS_PER_CORPUS: usize = 5;
const PAIRS_PER_KIND: usize = 3;

pub fn render_kinds(set: &SetDiagnosis) -> String {
    let row = |(name, kind): &(String, KindTally)| {
        let (frames, equivalent) = (kind.frames.to_string(), share(kind.equivalent, kind.frames));
        vec![name.clone(), share(kind.frames, set.frames), frames, equivalent, common_pairs(kind)]
    };
    let rows: Vec<Vec<String>> = set.kinds.iter().map(row).collect();
    format!(
        "### Error kinds on `{}`, by frames lost\n\nThe share is of all the set's frames. **RN eq** is the percentage of the kind's frames that the equivalents measure counts as right. The most common pairs are written expert first, then engine, with the number of frames each; a key is written as M (major) or m (minor) before the numeral.\n\n{}",
        set.name,
        table(&["kind", "% of frames", "frames", "RN eq", "most common (expert vs engine)"], &rows)
    )
}

fn common_pairs(kind: &KindTally) -> String {
    let mut pairs: Vec<(&String, &u64)> = kind.pairs.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1));
    let named = pairs.iter().take(PAIRS_PER_KIND).map(|(pair, frames)| format!("{pair} ({frames})"));
    named.collect::<Vec<_>>().join(", ")
}

pub fn render_corpus_kinds(sets: &[&SetDiagnosis]) -> String {
    let row = |set: &&SetDiagnosis| {
        let kinds = set.kinds.iter().take(KINDS_PER_CORPUS);
        let cells = kinds.map(|(name, k)| format!("{name} ({})", share(k.frames, set.frames)));
        std::iter::once(row_name(&set.name)).chain(cells).collect()
    };
    format!(
        "### The largest error kinds in each corpus of `tune`\n\nEach cell gives a kind and, in brackets, the percentage of the corpus's frames it loses.\n\n{}",
        table(
            &["corpus", "first", "second", "third", "fourth", "fifth"],
            &sets.iter().map(row).collect::<Vec<_>>()
        )
    )
}

pub fn render_unproducible(set: &SetDiagnosis) -> String {
    let kinds: Vec<&(String, KindTally)> =
        set.kinds.iter().filter(|(_, k)| k.group == "unproducible").collect();
    let total: u64 = kinds.iter().map(|(_, k)| k.frames).sum();
    let row = |(name, kind): &&(String, KindTally)| {
        let reason = name.strip_prefix("unproducible: ").unwrap_or(name).to_string();
        vec![
            reason,
            share(kind.frames, set.frames),
            kind.frames.to_string(),
            share(kind.frames, total),
            common_pairs(kind),
        ]
    };
    format!(
        "### Labels the engine cannot produce, on `tune`\n\nThese expert labels lie outside what the rule set's proposers and vocabulary can produce, so no weight can make the engine find them. They lose {}% of `tune`'s frames in all.\n\n{}",
        share(total, set.frames),
        table(
            &["reason", "% of frames", "frames", "% of unproducible", "most common labels"],
            &kinds.iter().map(row).collect::<Vec<_>>()
        )
    )
}
