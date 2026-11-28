//! Renders the expert-agreement tables, one for each set with pieces that have two analyses.

use super::markdown_table::{percent, table};
use super::set_names::row_name;
use crate::bench::layer_coverage::{LAYERS, LayerSummary};
use crate::bench::metrics::AgreementSummary;
use crate::bench::report::{SetRecord, agreement_features};

/// Sets listed first, because they are the ones that hold two analyses for every piece.
const LEADING_SETS: [&str; 2] = ["agreement", "agreement-tune"];

const INTRODUCTION: &str = "## Agreement with two experts

These sets include pieces with two expert analyses. `agreement` holds the 150 When in Rome pieces with two or more: 100 Bach chorales and the Mozart piano sonatas. `agreement-tune` leaves out those in the test split. TAVERN's two analyses per piece count under `tune-tavern` and `anet-test-tavern`, but not under `agreement`. Every other set listed here mixes such pieces with others, and its figures cover only the pieces with two analyses.

Each row is a percentage of frames.
- **Where the analysts agree,** the target is for the top reading to match them as often as a third expert would. If each expert matched an ideal analysis at rate *p*, two experts would agree at about *p*², and a third would match their consensus at about *p*, so the target is the square root of the analysts' agreement rate.
- **Where they disagree,** the measure is how often both readings are among those the engine offers. How often the top reading matches either analyst is given beside it.
- **Offered by layer** lists keys and chords offered on their own rather than as parts of whole readings. A key's or chord's **shortfall** is the best score in the beam two quarter notes later minus the best score of a hypothesis holding it. Up to four keys and, separately, up to four chords are offered, those whose shortfall is within the offer margin, smallest first. A chord here is its root, quality and inversion, as in the chord column. The RN row pairs each offered key with each offered chord and offers up to four of these numerals, ranked by the sum of the two shortfalls, each with the tonicization of the best hypothesis holding that key and chord. Both offered is a share of the frames where the analysts differ in that layer, and **per frame** is how many keys, chords or numerals are offered on average.
- **In hindsight** ranks the same beam's readings by the best final score that a hypothesis holding each one led to, and offers whole readings in the RN row. A reading whose hypotheses all left the beam before the end has no such score and is never offered. This is a report on the finished analysis; no rule and no other figure uses it.
- **RN eq** counts two labels that are notational equivalents of each other as the same, for the analysts as for the engine. Its rows for where the analysts agree and disagree therefore cover different frames from the RN column's: analysts whose labels are equivalents count as agreeing.
";

pub fn render_agreement(sets: &[SetRecord]) -> String {
    let leading = LEADING_SETS.iter().filter_map(|name| sets.iter().find(|s| s.name == *name));
    let rest = sets.iter().filter(|s| !LEADING_SETS.contains(&s.name.as_str()));
    let sections: Vec<String> = leading.chain(rest).filter_map(render_set).collect();
    format!("{INTRODUCTION}\n{}", sections.join("\n"))
}

fn render_set(set: &SetRecord) -> Option<String> {
    let summary = set.agreement.as_ref()?;
    let name = row_name(&set.name);
    let title = if name == set.name { format!("`{name}`") } else { format!("`{}` ({name})", set.name) };
    Some(format!(
        "### {title}\n\n{}\nThe engine offers {:.1} readings per frame on average here.\n{}",
        agreement_table(summary),
        summary.mean_offered,
        layer_table(summary).map_or(String::new(), |t| format!("\nOffered by layer:\n\n{t}"))
    ))
}

fn layer_table(summary: &AgreementSummary) -> Option<String> {
    let now = summary.layers.as_ref()?;
    let header =
        ["layer", "both offered", "per frame", "both offered in hindsight", "per frame in hindsight"];
    let hindsight = |values: fn(&LayerSummary) -> &Vec<f64>, layer: usize, format: fn(f64) -> String| {
        summary.hindsight.as_ref().map_or("".to_string(), |h| format(values(h)[layer]))
    };
    let count = |v: f64| format!("{v:.2}");
    let rows: Vec<Vec<String>> = LAYERS
        .iter()
        .enumerate()
        .map(|(layer, name)| {
            vec![
                if *name == "rn" { "RN".to_string() } else { name.to_string() },
                percent(now.coverage[layer]),
                count(now.mean_offered[layer]),
                hindsight(|h| &h.coverage, layer, percent),
                hindsight(|h| &h.mean_offered, layer, count),
            ]
        })
        .collect();
    Some(table(&header, &rows))
}

fn agreement_table(summary: &AgreementSummary) -> String {
    let rows: [(&str, &[f64]); 5] = [
        ("The two analysts give the same label", &summary.analyst_agreement),
        ("Where they agree, the engine's top reading matches them", &summary.consensus_rate),
        ("Target: how often a third expert would be expected to match them", &summary.target_consensus_rate),
        (
            "Where they disagree, the top reading matches one of them",
            &summary.disagreement_top_matches_either,
        ),
        ("Where they disagree, both readings are among those offered", &summary.disagreement_coverage),
    ];
    let row = |(name, values): &(&str, &[f64])| {
        std::iter::once(name.to_string()).chain(values.iter().map(|v| percent(*v))).collect()
    };
    let capitalized = |h: &'static str| match h {
        "rn" => "RN",
        "rn eq" => "RN eq",
        _ => h,
    };
    let header: Vec<&str> =
        std::iter::once("").chain(agreement_features().into_iter().map(capitalized)).collect();
    table(&header, &rows.iter().map(row).collect::<Vec<_>>())
}
