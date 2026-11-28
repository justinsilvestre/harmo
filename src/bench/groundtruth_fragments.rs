//! Decides whether short fragments of bars count as measures of their own.
//!
//! Some scores split a bar into a whole measure and a fragment,
//! for example a 5/4 bar written as a 4/4 measure and a 1/4 measure,
//! or leave the last beat of an ending as a measure of its own.
//! An analysis counts such a bar once.
//! When a score's numbering is broken and its bars are numbered by position,
//! a fragment would take a number and push every later analysis measure one bar early.

use crate::score::model::Measure;

const EPSILON: f64 = 1e-6;
/// How much better, summed over all harmonies, the fit must become before a fragment loses its number.
/// It amounts to about one harmony that fits its bar fully instead of not at all.
const REQUIRED_GAIN: f64 = 1.0;

/// Removes the numbers of fragments (short bars between two whole bars)
/// when the analysis fits the score better without them.
/// `total_fit` rates how well the analysis fits the score under a numbering.
pub fn skip_unfitting_fragments(
    bars: &[Measure],
    mut numbers: Vec<Option<i64>>,
    total_fit: &dyn Fn(&[Option<i64>]) -> f64,
) -> Vec<Option<i64>> {
    for index in fragment_indices(bars) {
        if numbers[index].is_none() {
            continue;
        }
        let skipped = without_number(&numbers, index);
        if total_fit(&skipped) > total_fit(&numbers) + REQUIRED_GAIN {
            numbers = skipped;
        }
    }
    numbers
}

fn fragment_indices(bars: &[Measure]) -> Vec<usize> {
    let is_whole = |i: usize| bars[i].duration >= bars[i].nominal_duration() - EPSILON;
    let is_fragment = |i: &usize| !is_whole(*i) && is_whole(i - 1) && is_whole(i + 1);
    (1..bars.len().saturating_sub(1)).filter(is_fragment).collect()
}

/// Unnumbers the bar at `index` and moves every later number down by one.
fn without_number(numbers: &[Option<i64>], index: usize) -> Vec<Option<i64>> {
    let mut result = numbers.to_vec();
    result[index] = None;
    for number in result[index + 1..].iter_mut().flatten() {
        *number -= 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(durations: &[f64]) -> Vec<Measure> {
        let to_measure = |duration: &f64| Measure {
            number: String::new(),
            start: 0.0,
            duration: *duration,
            beats: 4,
            beat_type: 4,
        };
        durations.iter().map(to_measure).collect()
    }

    fn numbers() -> Vec<Option<i64>> {
        vec![Some(1), Some(2), Some(3), Some(4)]
    }

    #[test]
    fn fragment_that_fits_better_unnumbered_loses_its_number() {
        let fit = |numbers: &[Option<i64>]| if numbers[2].is_none() { 5.0 } else { 0.0 };
        let result = skip_unfitting_fragments(&bars(&[4.0, 4.0, 1.0, 4.0]), numbers(), &fit);
        assert_eq!(result, vec![Some(1), Some(2), None, Some(3)]);
    }

    #[test]
    fn fragment_keeps_its_number_without_a_clear_gain() {
        let fit = |numbers: &[Option<i64>]| if numbers[2].is_none() { 0.5 } else { 0.0 };
        let result = skip_unfitting_fragments(&bars(&[4.0, 4.0, 1.0, 4.0]), numbers(), &fit);
        assert_eq!(result, numbers());
    }

    #[test]
    fn short_bar_beside_another_short_bar_is_no_fragment() {
        assert_eq!(fragment_indices(&bars(&[4.0, 3.0, 1.0, 4.0])), Vec::<usize>::new());
    }
}
