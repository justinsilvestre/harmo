//! The bass of a chord segment, from which its inversion is read.
//!
//! The engine hands every chord tone a segment hears to the `Bass` rules,
//! which keep the evidence below and decide the bass note. With no bass rule a segment has no bass,
//! and its chord is read in root position.

use crate::pitch::Tpc;

/// The bass evidence gathered while a segment is extended slice by slice, kept by the bass rules.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bass {
    /// The lowest chord tone heard so far, as (MIDI number, spelling).
    pub lowest: Option<(i32, Tpc)>,
    /// The chord member of the lowest chord tone: 0 = root, 1 = third, 2 = fifth, 3 = seventh.
    pub lowest_member: usize,
    /// Whether the lowest chord tone has been an accented bass note.
    pub lowest_accented: bool,
    /// The lowest root that has been an accented bass note.
    pub accented_root: Option<(i32, Tpc)>,
    /// The bass note that gives the segment's inversion, as the bass rules decided it.
    pub note: Option<(i32, Tpc)>,
}

/// A chord tone sounding in a slice, with where it falls.
pub struct BassNote {
    pub midi: i32,
    pub tpc: Tpc,
    /// The chord member: 0 = root, 1 = third, 2 = fifth, 3 = seventh.
    pub member: usize,
    /// Whether the note is the slice's lowest note.
    pub is_slice_bass: bool,
    /// The metric level at which the slice starts.
    pub metric_level: u8,
    /// Whether the slice opens the segment.
    pub opens_segment: bool,
}
