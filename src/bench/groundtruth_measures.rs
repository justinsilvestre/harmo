//! Matches the measure numbers of a RomanText analysis to the measures of a score.
//!
//! RomanText counts whole bars, while scores often number them differently:
//! - A bar split in two around a repeat sign appears in the score as two short measures.
//! - Some scores restart or skip their numbering, for example at a trio.
//! - First and second endings are written `m12a` and `m12b` in the analysis,
//!   but a score either numbers them in sequence or leaves the second ending unnumbered ("X1").

use crate::score::model::Measure;

const EPSILON: f64 = 1e-6;

/// A measure reference in an analysis: number and suffix, such as (12, "b").
pub type AnalysisMeasure<'a> = (i64, &'a str);

/// Groups the score's measures into whole bars and finds the bar each analysis measure refers to.
/// The references are given in the order they appear in the analysis.
/// `fit` rates how well the harmony of a reference matches a bar's notes, from 0 to 1;
/// it decides how long a first ending is when the analysis leaves this unclear.
pub fn locate_measures(
    measures: &[Measure],
    references: &[AnalysisMeasure],
    fit: impl Fn(usize, &Measure) -> f64,
) -> (Vec<Measure>, Vec<Option<usize>>) {
    let bars = merge_split_bars(measures);
    let mut numbers: Vec<Option<i64>> = bars.iter().map(|b| leading_number(&b.number)).collect();
    if is_numbering_broken(&numbers) {
        numbers = numbers_by_position(&numbers);
    }
    let pickup_shift = i64::from(!numbers.contains(&Some(0)) && references.iter().any(|r| r.0 == 0));
    let resolve = |index: usize, target: Target| match target {
        Target::Bar(bar) => Some(bar),
        Target::Number(number) => find_bar(&bars, &numbers, number + pickup_shift, references[index].1),
    };
    let rate =
        |index: usize, target: Target| resolve(index, target).map_or(0.0, |bar| fit(index, &bars[bar]));
    let targets = volta_targets(&bars, &numbers, references, &rate);
    let located = targets.into_iter().enumerate().map(|(index, target)| resolve(index, target)).collect();
    (bars, located)
}

/// Joins two consecutive short measures that together fill one bar, unless the first is the opening pickup.
fn merge_split_bars(measures: &[Measure]) -> Vec<Measure> {
    let mut bars: Vec<Measure> = Vec::with_capacity(measures.len());
    let mut index = 0;
    while index < measures.len() {
        let measure = &measures[index];
        match measures.get(index + 1) {
            Some(next) if index > 0 && is_split_bar(measure, next) => {
                bars.push(Measure { duration: measure.duration + next.duration, ..measure.clone() });
                index += 2;
            }
            _ => {
                bars.push(measure.clone());
                index += 1;
            }
        }
    }
    bars
}

fn is_split_bar(first: &Measure, second: &Measure) -> bool {
    let is_short = |m: &Measure| m.duration < m.nominal_duration() - EPSILON;
    let fills_bar = (first.duration + second.duration - first.nominal_duration()).abs() < EPSILON;
    is_short(first) && is_short(second) && fills_bar
}

fn leading_number(number: &str) -> Option<i64> {
    let digits: String = number.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Reports numbering that goes back or skips a number. Repeated numbers are allowed.
fn is_numbering_broken(numbers: &[Option<i64>]) -> bool {
    let numbered: Vec<i64> = numbers.iter().flatten().copied().collect();
    numbered.windows(2).any(|pair| pair[1] < pair[0] || pair[1] > pair[0] + 1)
}

/// Numbers the bars one after another from the first number. Unnumbered bars stay unnumbered.
fn numbers_by_position(numbers: &[Option<i64>]) -> Vec<Option<i64>> {
    let mut next = numbers.iter().flatten().next().copied().unwrap_or(0);
    let mut renumber = |number: &Option<i64>| {
        number.map(|_| {
            next += 1;
            next - 1
        })
    };
    numbers.iter().map(&mut renumber).collect()
}

fn find_bar(bars: &[Measure], numbers: &[Option<i64>], number: i64, suffix: &str) -> Option<usize> {
    let is_number = |i: &usize| numbers[*i] == Some(number);
    let exact =
        (0..bars.len()).filter(is_number).find(|&i| bars[i].number.ends_with(suffix) && !suffix.is_empty());
    exact.or_else(|| (0..bars.len()).find(is_number))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Target {
    Number(i64),
    Bar(usize),
}

/// Resolves first and second endings. A second ending is either the unnumbered bar after the first ending,
/// or, when the score numbers both endings in sequence, the bars after the first ending,
/// in which case every later analysis measure lies that many bars further on.
fn volta_targets(
    bars: &[Measure],
    numbers: &[Option<i64>],
    references: &[AnalysisMeasure],
    rate: &dyn Fn(usize, Target) -> f64,
) -> Vec<Target> {
    let mut targets: Vec<Target> = references.iter().map(|r| Target::Number(r.0)).collect();
    let mut shift = 0;
    let mut index = 0;
    while let Some(volta) = find_volta(references, index) {
        shift_numbers(&mut targets[index..volta.second_start], shift);
        let start = volta.start_number + shift;
        let longest = volta.first_ending_lengths.iter().copied().max().unwrap_or(1);
        match unnumbered_bar_after(bars, numbers, start, longest) {
            Some(bar) => {
                for i in volta.second_start..volta.end {
                    let is_first_bar = references[i].0 == volta.start_number;
                    targets[i] =
                        if is_first_bar { Target::Bar(bar) } else { Target::Number(references[i].0 + shift) };
                }
            }
            None => {
                let next = find_volta(references, volta.end).map_or(references.len(), |v| v.second_start);
                let after_first_ending = |length: i64| {
                    let rating = |i: usize| rate(i, Target::Number(references[i].0 + shift + length));
                    (volta.second_start..next).map(rating).sum::<f64>()
                };
                let lengths = volta.first_ending_lengths.iter().copied();
                let length = lengths.max_by(|a, b| after_first_ending(*a).total_cmp(&after_first_ending(*b)));
                shift += length.unwrap_or(1);
                shift_numbers(&mut targets[volta.second_start..volta.end], shift);
            }
        }
        index = volta.end;
    }
    shift_numbers(&mut targets[index..], shift);
    targets
}

fn shift_numbers(targets: &mut [Target], shift: i64) {
    for target in targets {
        if let Target::Number(number) = target {
            *number += shift;
        }
    }
}

struct Volta {
    start_number: i64,
    /// The index of the first reference in the second ending.
    second_start: usize,
    /// The index after the last reference in the second ending.
    end: usize,
    /// Candidate lengths of the first ending, in bars: the span of either ending in the analysis,
    /// since an analysis may skip the last bars of an ending when the harmony does not change,
    /// or 0 when the score writes the repeated music only once.
    first_ending_lengths: Vec<i64>,
}

/// Finds the next second ending at or after `from`: a run of references marked "b",
/// usually preceded by a first ending marked "a" that starts at the same number.
fn find_volta(references: &[AnalysisMeasure], from: usize) -> Option<Volta> {
    let second_start = from + references[from..].iter().position(|r| r.1 == "b")?;
    let end = second_start + references[second_start..].iter().take_while(|r| r.1 == "b").count();
    let start_number = references[second_start].0;
    let first_ending = references[from..second_start].iter().position(|r| *r == (start_number, "a"));
    let first = first_ending.map_or(second_start, |i| from + i);
    let span = |part: &[AnalysisMeasure]| part.iter().map(|r| r.0 - start_number + 1).max().unwrap_or(1);
    let mut first_ending_lengths =
        vec![0, span(&references[first..second_start]), span(&references[second_start..end])];
    first_ending_lengths.dedup();
    Some(Volta { start_number, second_start, end, first_ending_lengths })
}

/// Returns the first unnumbered whole bar within the endings that start at `start`.
/// Short unnumbered bars are pickups rather than endings.
fn unnumbered_bar_after(bars: &[Measure], numbers: &[Option<i64>], start: i64, length: i64) -> Option<usize> {
    let first = numbers.iter().position(|n| *n == Some(start))?;
    let window = numbers.iter().enumerate().skip(first + 1).take(length as usize + 1);
    let is_whole = |i: usize| bars[i].duration >= bars[i].nominal_duration() - EPSILON;
    window
        .take_while(|(_, n)| n.is_none_or(|n| n < start + 2 * length))
        .find(|(i, n)| n.is_none() && is_whole(*i))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(layout: &[(&str, f64)]) -> Vec<Measure> {
        let mut start = 0.0;
        let to_measure = |(number, duration): &(&str, f64)| {
            let measure =
                Measure { number: number.to_string(), start, duration: *duration, beats: 4, beat_type: 4 };
            start += duration;
            measure
        };
        layout.iter().map(to_measure).collect()
    }

    fn located_starts(layout: &[(&str, f64)], references: &[AnalysisMeasure]) -> Vec<Option<f64>> {
        let (bars, located) = locate_measures(&bars(layout), references, |_, _| 0.0);
        located.into_iter().map(|i| i.map(|i| bars[i].start)).collect()
    }

    #[test]
    fn split_bar_with_stray_numbers_counts_as_one_bar() {
        let layout = [("0", 1.0), ("1", 4.0), ("283", 3.0), ("284", 1.0), ("4", 4.0)];
        assert_eq!(located_starts(&layout, &[(2, ""), (3, "")]), vec![Some(5.0), Some(9.0)]);
    }

    #[test]
    fn split_bar_is_one_measure_long() {
        let (bars, _) =
            locate_measures(&bars(&[("0", 1.0), ("1", 3.0), ("X1", 1.0), ("2", 4.0)]), &[], |_, _| 0.0);
        assert_eq!(bars[1].duration, 4.0);
    }

    #[test]
    fn restarted_numbering_continues_by_position() {
        let layout = [("1", 4.0), ("2", 4.0), ("1", 4.0), ("2", 4.0)];
        assert_eq!(located_starts(&layout, &[(4, "")]), vec![Some(12.0)]);
    }

    #[test]
    fn second_ending_numbered_in_sequence_shifts_later_bars() {
        let layout = [("1", 4.0), ("2", 4.0), ("3", 4.0), ("4", 4.0), ("5", 4.0), ("6", 4.0)];
        let references = [(1, ""), (2, "a"), (2, "b"), (3, "b"), (4, "")];
        let expected = vec![Some(0.0), Some(4.0), Some(12.0), Some(16.0), Some(20.0)];
        assert_eq!(located_starts(&layout, &references), expected);
    }

    #[test]
    fn first_ending_length_follows_the_better_fit() {
        let layout: Vec<(&str, f64)> = ["1", "2", "3", "4", "5", "6", "7", "8"].map(|n| (n, 4.0)).to_vec();
        let references = [(2, "a"), (3, ""), (2, "b"), (3, "b"), (4, "b"), (5, "")];
        let fit = |index: usize, bar: &Measure| f64::from(index == 2 && bar.start == 12.0);
        let (bars, located) = locate_measures(&bars(&layout), &references, fit);
        assert_eq!(located[2].map(|i| bars[i].start), Some(12.0));
    }

    #[test]
    fn unnumbered_second_ending_takes_the_first_second_ending_bar() {
        let layout = [("1", 4.0), ("2", 4.0), ("X1", 4.0), ("3", 4.0)];
        let references = [(2, "a"), (2, "b"), (3, "")];
        assert_eq!(located_starts(&layout, &references), vec![Some(4.0), Some(8.0), Some(12.0)]);
    }

    #[test]
    fn regular_numbering_is_kept() {
        let layout = [("0", 1.0), ("1", 4.0), ("2", 4.0)];
        assert_eq!(located_starts(&layout, &[(2, "")]), vec![Some(5.0)]);
    }
}
