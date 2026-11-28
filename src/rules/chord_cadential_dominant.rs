//! Rules that read a six-four over the dominant both as a tonic six-four and as the dominant it
//! delays.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::chord_hooks::ChordHook;
use crate::engine::key_hooks::KeyHook;
use crate::engine::model::Reading;
use crate::engine::rule::{Citation, Hook, Rule, Topic};
use crate::pitch::Tpc;
use crate::score::slicing::Slice;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::Mode;

const CADENTIAL_SIX_FOUR: Citation = Citation {
    source: KOSTKA_PAYNE_ALMEN_2023,
    locator: "p. 135 (the cadential I64 does not represent a tonic triad, and some label it as the V it \
              delays, V 6/4–5/3)",
};

pub const RULES: &[Rule] = &[
    Rule {
        id: "chord.propose_cadential_dominant",
        topic: Topic::Chord,
        summary: "Where a slice sounds only a six-four (the bass with a perfect fourth above it, and \
                  perhaps a sixth), also propose the major triad and the dominant seventh chord on the \
                  bass. A cadential six-four delays the dominant on its bass, and it can be labeled as \
                  that dominant, with its sixth and fourth resolving to the fifth and third.",
        citations: &[CADENTIAL_SIX_FOUR],
        default_weight: 1.0,
        params: &[],
        hook: Hook::Chord(ChordHook::Propose(propose_cadential_dominant)),
    },
    Rule {
        id: "function.cadential_rival",
        topic: Topic::Function,
        summary: "Where a slice sounds only a six-four, keep the tonic six-four and the dominant on its \
                  bass, in the same key, as rival candidates for a chord opening there, whichever of \
                  the two fits the slice better. Either reading of a cadential six-four is a correct \
                  label, so each is weighed in the search on its own merits.",
        citations: &[CADENTIAL_SIX_FOUR],
        default_weight: 1.0,
        params: &[],
        hook: Hook::Key(KeyHook::RivalCandidate(cadential_rival)),
    },
];

/// A perfect fourth above a pitch, on the line of fifths.
const FOURTH: Tpc = -1;
/// A major and a minor sixth above a pitch, on the line of fifths.
const SIXTHS: [Tpc; 2] = [3, -4];

/// Returns the slice's bass if the slice sounds only that bass, a perfect fourth above it, and
/// perhaps a sixth above it.
fn six_four_bass(slice: &Slice) -> Option<Tpc> {
    let bass = slice.bass()?.tpc;
    let fits = |tpc: Tpc| tpc == bass || tpc == bass + FOURTH || SIXTHS.iter().any(|&s| tpc == bass + s);
    let has_fourth = slice.notes.iter().any(|note| note.tpc == bass + FOURTH);
    (has_fourth && slice.notes.iter().all(|note| fits(note.tpc))).then_some(bass)
}

fn propose_cadential_dominant(slice: &Slice) -> Vec<Chord> {
    let Some(bass) = six_four_bass(slice) else {
        return vec![];
    };
    vec![Chord::new(bass, Quality::Major), Chord::new(bass, Quality::DominantSeventh)]
}

/// Returns the dominant on the bass for a tonic triad over a six-four in its key, and the tonic
/// triad for that dominant. The tonic triad's quality follows the key's mode.
fn cadential_rival(reading: &Reading, slice: &Slice, _: &[f64]) -> Option<Reading> {
    let bass = six_four_bass(slice)?;
    let key = reading.function.key;
    if reading.function.tonicized.is_some() || bass != key.tonic + 1 {
        return None;
    }
    let tonic_quality = match key.mode {
        Mode::Major => Quality::Major,
        Mode::Minor => Quality::Minor,
    };
    let tonic = Chord::new(key.tonic, tonic_quality);
    let dominant = Chord::new(bass, Quality::Major);
    let rival = match reading.chord {
        chord if chord == tonic => dominant,
        chord if chord.root == bass && matches!(chord.quality, Quality::Major | Quality::DominantSeventh) => {
            tonic
        }
        _ => return None,
    };
    Some(Reading { chord: rival, ..*reading })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::Function;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::key::Key;

    /// A slice sounding the named notes, lowest first.
    fn slice(names: &[&str]) -> Slice {
        let note = |(i, name): (usize, &&str)| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi: 40 + 5 * i as i32,
            attacked: true,
            onset_level: 0,
        };
        let notes = names.iter().enumerate().map(note).collect();
        Slice {
            start: 0.0,
            duration: 1.0,
            notes,
            measure: 0,
            metric_level: 4,
            phrase: 0,
            key_signature: None,
        }
    }

    #[test]
    fn a_six_four_proposes_the_dominant_on_its_bass() {
        let proposed = propose_cadential_dominant(&slice(&["G", "C", "E", "G"]));
        assert!(proposed.contains(&Chord::new(tpc_from_name("G").unwrap(), Quality::Major)));
    }

    #[test]
    fn a_minor_six_four_proposes_the_dominant_on_its_bass() {
        let proposed = propose_cadential_dominant(&slice(&["G", "C", "Eb"]));
        assert!(proposed.contains(&Chord::new(tpc_from_name("G").unwrap(), Quality::Major)));
    }

    #[test]
    fn a_root_position_triad_proposes_nothing() {
        assert!(propose_cadential_dominant(&slice(&["C", "E", "G"])).is_empty());
    }

    #[test]
    fn a_six_four_with_the_dominant_third_proposes_nothing() {
        assert!(propose_cadential_dominant(&slice(&["G", "C", "E", "B"])).is_empty());
    }

    fn in_c(root: &str, quality: Quality) -> Reading {
        let key = Key::parse("C", "major").unwrap();
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        Reading { chord, function: Function::diatonic(key) }
    }

    #[test]
    fn the_tonic_six_four_has_the_dominant_as_its_rival() {
        let rival = cadential_rival(&in_c("C", Quality::Major), &slice(&["G", "C", "E"]), &[]);
        assert_eq!(rival.map(|r| r.chord), Some(in_c("G", Quality::Major).chord));
    }

    #[test]
    fn the_dominant_over_a_six_four_has_the_tonic_six_four_as_its_rival() {
        let rival = cadential_rival(&in_c("G", Quality::Major), &slice(&["G", "C", "E"]), &[]);
        assert_eq!(rival.map(|r| r.chord), Some(in_c("C", Quality::Major).chord));
    }

    #[test]
    fn a_six_four_over_another_degree_has_no_rival() {
        assert_eq!(cadential_rival(&in_c("F", Quality::Major), &slice(&["C", "F", "A"]), &[]), None);
    }
}
