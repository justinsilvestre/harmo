//! Groups per-frame records into spans of consecutive frames with the same value.

/// Consecutive frames that share a value.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Span<T> {
    pub start: usize,
    pub frames: usize,
    pub value: T,
}

/// Groups (frame, value) records, sorted by frame, into spans of adjacent frames with equal values.
pub fn group_spans<T: PartialEq + Copy>(records: &[(usize, T)]) -> Vec<Span<T>> {
    let mut spans: Vec<Span<T>> = Vec::new();
    for &(frame, value) in records {
        match spans.last_mut() {
            Some(span) if span.start + span.frames == frame && span.value == value => span.frames += 1,
            _ => spans.push(Span { start: frame, frames: 1, value }),
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_equal_frames_form_one_span() {
        assert_eq!(group_spans(&[(3, 'a'), (4, 'a')]), vec![Span { start: 3, frames: 2, value: 'a' }]);
    }

    #[test]
    fn a_gap_between_frames_splits_the_span() {
        assert_eq!(group_spans(&[(3, 'a'), (5, 'a')]).len(), 2);
    }

    #[test]
    fn a_new_value_splits_the_span() {
        assert_eq!(group_spans(&[(3, 'a'), (4, 'b')]).len(), 2);
    }
}
