//! How a chord segment grows slice by slice: its duration, the chord members it has heard,
//! and the bass that the bass rules decide.

use super::bass::{Bass, BassNote};
use super::model::ChordSegment;
use super::moment::Moment;
use super::ruleset::RuleSet;
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;

/// Returns the segment that a chord opens on the moment's slice.
pub fn open_segment(chord: Chord, moment: &Moment, rules: &RuleSet) -> ChordSegment {
    let start = moment.index as u32;
    let empty = ChordSegment { chord, start, duration: 0.0, members_heard: 0, bass: Bass::default() };
    extend_segment(empty, moment.slice(), rules)
}

/// Returns a segment extended by one more slice.
pub fn extend_segment(mut segment: ChordSegment, slice: &Slice, rules: &RuleSet) -> ChordSegment {
    let lowest = slice.bass().map(|n| n.midi);
    let opens_segment = segment.duration == 0.0;
    segment.duration += slice.duration;
    for note in &slice.notes {
        let Some(member) = segment.chord.member_index(note.tpc) else {
            continue;
        };
        segment.members_heard |= 1 << member;
        let (midi, tpc, metric_level) = (note.midi, note.tpc, slice.metric_level);
        let is_slice_bass = lowest == Some(midi);
        let heard = BassNote { midi, tpc, member, is_slice_bass, metric_level, opens_segment };
        segment.bass = rules.chord.hear_bass(segment.bass, &heard);
    }
    segment
}
