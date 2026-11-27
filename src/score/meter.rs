//! Metric strength of positions within a measure.

use super::model::{Measure, Offset};

const EPSILON: f64 = 1e-6;

/// The metric level of a position: 4 = downbeat, 3 = mid-measure beat in duple meters,
/// 2 = other beats, 1 = beat subdivisions, 0 = anything weaker.
pub fn metric_level(measure: &Measure, offset_in_measure: Offset) -> u8 {
    let position = effective_position(measure, offset_in_measure);
    let (beat, beats) = beat_unit(measure);
    let subdivision = if is_compound(measure) { beat / 3.0 } else { beat / 2.0 };
    if is_multiple(position, measure.nominal_duration()) {
        4
    } else if beats % 2 == 0 && beats > 2 && is_multiple(position, measure.nominal_duration() / 2.0) {
        3
    } else if is_multiple(position, beat) {
        2
    } else if is_multiple(position, subdivision) {
        1
    } else {
        0
    }
}

/// Measures positions in a pickup measure from the end, as if the measure were complete.
fn effective_position(measure: &Measure, offset_in_measure: Offset) -> Offset {
    let shortfall = measure.nominal_duration() - measure.duration;
    if shortfall > EPSILON { offset_in_measure + shortfall } else { offset_in_measure }
}

fn is_compound(measure: &Measure) -> bool {
    measure.beat_type >= 8 && measure.beats.is_multiple_of(3) && measure.beats > 3
}

/// Returns the length of the measure's beat in quarter notes: a dotted quarter in 6/8, for example.
pub fn beat_length(measure: &Measure) -> Offset {
    beat_unit(measure).0
}

/// Returns the beat length in quarter notes and the number of beats per measure.
fn beat_unit(measure: &Measure) -> (Offset, u32) {
    let unit = 4.0 / measure.beat_type as f64;
    if is_compound(measure) { (unit * 3.0, measure.beats / 3) } else { (unit, measure.beats) }
}

fn is_multiple(position: Offset, unit: Offset) -> bool {
    let ratio = position / unit;
    (ratio - ratio.round()).abs() < EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(beats: u32, beat_type: u32, duration: f64) -> Measure {
        Measure { number: "1".into(), start: 0.0, duration, beats, beat_type }
    }

    #[test]
    fn third_beat_of_common_time_is_stronger_than_second() {
        let m = measure(4, 4, 4.0);
        assert!(metric_level(&m, 2.0) > metric_level(&m, 1.0));
    }

    #[test]
    fn compound_meter_beats_are_dotted_quarters() {
        assert_eq!(metric_level(&measure(6, 8, 3.0), 1.5), 2);
    }

    #[test]
    fn pickup_positions_are_measured_from_the_barline() {
        assert_eq!(metric_level(&measure(3, 4, 1.0), 0.0), 2);
    }

    #[test]
    fn offbeat_eighth_is_a_subdivision() {
        assert_eq!(metric_level(&measure(3, 4, 3.0), 0.5), 1);
    }
}
