//! Formats Markdown tables and the numbers in them.

/// Formats a table, right-aligning the columns after the first whose cells all hold numbers.
pub fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let is_numeric = |column: usize| rows.iter().all(|r| r.get(column).is_none_or(|c| is_number(c)));
    let alignment: String =
        (0..header.len()).map(|c| if c > 0 && is_numeric(c) { "---:|" } else { "---|" }).collect();
    let mut lines = vec![row(header.iter().map(|h| h.to_string())), format!("|{alignment}")];
    lines.extend(rows.iter().map(|r| row(r.iter().cloned())));
    lines.join("\n") + "\n"
}

/// Reports whether a cell holds a number, perhaps followed by another in brackets, or nothing.
fn is_number(cell: &str) -> bool {
    cell.split_whitespace().next().is_none_or(|first| first.parse::<f64>().is_ok())
}

fn row(cells: impl Iterator<Item = String>) -> String {
    let cells: Vec<String> = cells.map(|c| c.replace('|', "\\|")).collect();
    format!("| {} |", cells.join(" | "))
}

/// Formats a rate as a percentage with one decimal.
pub fn percent(rate: f64) -> String {
    format!("{:.1}", rate * 100.0)
}

/// Formats `part` as a percentage of `whole` with one decimal.
pub fn share(part: u64, whole: u64) -> String {
    percent(if whole == 0 { 0.0 } else { part as f64 / whole as f64 })
}

/// Formats a length in frames (thirty-second notes) as quarter notes.
pub fn quarters(frames: u64) -> String {
    format!("{:.1}", frames as f64 / 8.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_aligns_every_column_but_the_first() {
        assert_eq!(table(&["a", "b"], &[]), "| a | b |\n|---|---:|\n");
    }

    #[test]
    fn left_aligns_a_column_of_words() {
        assert!(table(&["a", "b"], &[vec!["1".into(), "x".into()]]).contains("|---|---|"));
    }

    #[test]
    fn escapes_pipes_inside_cells() {
        assert!(table(&["a"], &[vec!["x | y".into()]]).contains("x \\| y"));
    }

    #[test]
    fn shares_of_nothing_are_zero() {
        assert_eq!(share(3, 0), "0.0");
    }
}
