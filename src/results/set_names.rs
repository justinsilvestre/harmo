//! Readable names for benchmark sets, and the order in which their rows are listed.

/// Corpus names for the AugmentedNet modules and the When in Rome categories outside them.
const CORPUS_NAMES: &[(&str, &str)] = &[
    ("wir", "WiR"),
    ("abc_dcml", "ABC"),
    ("mps", "MPS"),
    ("wirwtc", "WTC"),
    ("haydnsun", "HaydnSun"),
    ("tavern", "TAVERN"),
    ("bps", "BPS"),
    ("early_choral", "Early choral"),
    ("keyboard_other", "Other keyboard"),
    ("openscore-liedercorpus", "OpenScore Lieder"),
    ("textbooks", "Textbooks"),
    ("variations_and_grounds", "Variations and grounds"),
];

/// Sets whose subsets are named `<set>-<corpus>`.
const PARENT_SETS: &[(&str, &str)] = &[("anet-test", "all of test"), ("tune", "all of tune")];

/// Returns the name of a set's row in a table, such as "WiR" for "tune-wir".
pub fn row_name(set: &str) -> String {
    if let Some((_, name)) = PARENT_SETS.iter().find(|(parent, _)| *parent == set) {
        return name.to_string();
    }
    let corpus = PARENT_SETS.iter().find_map(|(parent, _)| set.strip_prefix(&format!("{parent}-")));
    let named = corpus.and_then(|c| CORPUS_NAMES.iter().find(|(id, _)| *id == c));
    named.map_or_else(|| set.to_string(), |(_, name)| name.to_string())
}

/// Lists the subsets of `parent` in their given order, then `parent` itself.
pub fn family<'a, T>(sets: &'a [T], parent: &str, name: fn(&T) -> &str) -> Vec<&'a T> {
    let prefix = format!("{parent}-");
    let subsets = sets.iter().filter(|s| name(s).starts_with(&prefix));
    subsets.chain(sets.iter().filter(|s| name(s) == parent)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_a_tune_subset_by_its_corpus() {
        assert_eq!(row_name("tune-wirwtc"), "WTC");
    }

    #[test]
    fn keeps_the_name_of_a_set_without_a_parent() {
        assert_eq!(row_name("agreement-tune"), "agreement-tune");
    }

    #[test]
    fn lists_subsets_before_their_parent() {
        let sets = ["tune", "agreement", "tune-wir"];
        assert_eq!(family(&sets, "tune", |s| s), vec![&"tune-wir", &"tune"]);
    }
}
