//! Renders the expert-agreement tables, one for each set with pieces that have two analyses.

use super::markdown_table::{percent, table};
use super::set_names::row_name;
use crate::bench::metrics::{AgreementSummary, FEATURES, MATCHED_FEATURES};
use crate::bench::report::SetRecord;

/// Sets listed first, because they are the ones that hold two analyses for every piece.
const LEADING_SETS: [&str; 2] = ["agreement", "agreement-tune"];

const INTRODUCTION: &str = "## Agreement with two experts

These sets include pieces with two expert analyses. `agreement` holds the 150 When in Rome pieces with two or more: 100 Bach chorales and the Mozart piano sonatas. `agreement-tune` leaves out those in the test split. TAVERN's two analyses per piece count under `tune-tavern` and `anet-test-tavern`, but not under `agreement`. Every other set listed here mixes such pieces with others, and its figures cover only the pieces with two analyses.

Each row is a percentage of frames.
- **Where the analysts agree,** the target is for the top reading to match them as often as a third expert would. If each expert matched an ideal analysis at rate *p*, two experts would agree at about *p*², and a third would match their consensus at about *p*, so the target is the square root of the analysts' agreement rate.
- **Where they disagree,** the measure is how often both readings are among those the engine offers. How often the top reading matches either analyst is given beside it.
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
        "### {title}\n\n{}\nThe engine offers {:.1} readings per frame on average here.\n",
        agreement_table(summary),
        summary.mean_offered
    ))
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
    let header: Vec<&str> = std::iter::once("").chain(FEATURES[..MATCHED_FEATURES].iter().copied()).collect();
    let header: Vec<&str> = header.into_iter().map(|h| if h == "rn" { "RN" } else { h }).collect();
    table(&header, &rows.iter().map(row).collect::<Vec<_>>())
}
