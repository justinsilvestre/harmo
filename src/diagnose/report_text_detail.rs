//! Prints the detailed diagnosis of one set: error kinds, disagreements, beam diversity and examples.

use super::report_text::percent;
use super::report_text_disagreements::print_disagreements;
use super::summary::SetDiagnosis;
use super::tally::{Example, KindTally};

/// Kinds that get example spans in the text report.
const KINDS_WITH_EXAMPLES: usize = 8;
const EXAMPLES_PER_KIND: usize = 3;
const PAIRS_PER_KIND: usize = 3;
const LARGEST_SPANS: usize = 5;

pub fn print_detail(set: &SetDiagnosis) {
    let wrong = set.frames - set.correct;
    println!(
        "\n== {} ({} pieces, {} frames, rn {:.1}%, {wrong} wrong frames) ==",
        set.name,
        set.pieces,
        set.frames,
        percent(set.correct, set.frames)
    );
    println!(
        "\nerror kinds, ranked by frames lost ({:>5} = % of all frames, {:>5} = % of the kind that rn eq credits)",
        "%", "eq"
    );
    for (name, kind) in &set.kinds {
        println!(
            "  {name:<58} {:>5.1} {:>8} {:>5.1}{}",
            percent(kind.frames, set.frames),
            kind.frames,
            percent(kind.equivalent, kind.frames),
            common_pairs(kind)
        );
    }
    let bars = set.bars.max(1) as f64;
    println!(
        "\nchord changes per bar: expert {:.2}, engine {:.2}",
        set.expert_changes as f64 / bars,
        set.engine_changes as f64 / bars
    );
    print_disagreements(&set.disagreements);
    let beam = &set.beam;
    let per_slice = |sum: u64| sum as f64 / beam.slices.max(1) as f64;
    println!(
        "\nbeam diversity per slice: {:.1} hypotheses, {:.1} distinct readings, {:.1} distinct chord-and-key pairs; \
         {:.1} distinct readings among the leading --offer-count hypotheses",
        per_slice(beam.hypotheses),
        per_slice(beam.readings),
        per_slice(beam.chord_keys),
        per_slice(beam.leading_readings)
    );
    print_examples(set);
}

fn common_pairs(kind: &KindTally) -> String {
    let mut pairs: Vec<(&String, &u64)> = kind.pairs.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1));
    let named: Vec<String> =
        pairs.iter().take(PAIRS_PER_KIND).map(|(pair, frames)| format!("{pair} {frames}")).collect();
    if named.is_empty() { String::new() } else { format!("  [{}]", named.join(", ")) }
}

fn print_examples(set: &SetDiagnosis) {
    println!("\nexamples (piece m<measure> frames: expert / engine)");
    for (name, kind) in set.kinds.iter().filter(|(_, k)| k.group != "disagreement").take(KINDS_WITH_EXAMPLES)
    {
        println!("  {name}");
        kind.examples.iter().take(EXAMPLES_PER_KIND).for_each(|e| println!("    {}", format_example(e)));
    }
    for (name, class) in set.disagreements.iter().take(KINDS_WITH_EXAMPLES) {
        println!("  disagreement: {name} (analyst | analyst / engine)");
        class.examples.iter().take(EXAMPLES_PER_KIND).for_each(|e| println!("    {}", format_example(e)));
    }
    println!("\nlargest wrong spans where the analysts agree");
    let errors = set.kinds.iter().filter(|(_, k)| k.group != "disagreement");
    let mut spans: Vec<(&String, &Example)> =
        errors.flat_map(|(name, k)| k.examples.iter().map(move |e| (name, e))).collect();
    spans.sort_by_key(|(_, e)| std::cmp::Reverse(e.frames));
    for (name, example) in spans.into_iter().take(LARGEST_SPANS) {
        println!("  {}  [{name}]", format_example(example));
    }
}

fn format_example(example: &Example) -> String {
    format!(
        "{} m{} {}: {} / {}",
        example.piece, example.measure, example.frames, example.expert, example.engine
    )
}
