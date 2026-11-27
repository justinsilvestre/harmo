//! Renders a results record as the Markdown document RESULTS.md.

use super::markdown_accuracy::{render_test, render_tune};
use super::markdown_agreement::render_agreement;
use super::markdown_equivalents::render_equivalents;
use super::markdown_errors::render_errors;
use super::markdown_header::render_header;
use super::markdown_offer_delays::render_offer_delays;
use super::record::ResultsRecord;

const LEFT_OUT: &str = "## What this document leaves out

- **Changes against an earlier run,** with their 95% intervals: run `harmo bench --compare RUN.json` for those, since they need a second run to compare with.
- **Per-piece accuracy:** each `bench` run saves it in `runs/`.
- **Examples beyond the longest few,** and label pairs beyond the three most common of each kind: run `harmo diagnose` (or `--json`) for the full lists.
";

/// Renders every section, the most important first.
pub fn render(record: &ResultsRecord) -> String {
    let offer_count = record.header.settings.get("offer_count").copied().unwrap_or(0.0);
    let sections = [
        render_header(&record.header),
        render_test(&record.accuracy),
        render_tune(&record.accuracy),
        render_equivalents(&record.accuracy),
        render_agreement(&record.accuracy),
        render_offer_delays(&record.accuracy),
        render_errors(&record.diagnoses, offer_count as usize),
        LEFT_OUT.to_string(),
    ];
    sections.join("\n")
}
