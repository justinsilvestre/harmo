//! How the key of an engine reading relates to the key of an expert label.

use crate::theory::key::Mode;
use crate::theory::label::Label;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub enum KeyRelation {
    /// One label applies a chord to another degree of its key, and the other is in that degree's key.
    Tonicization,
    /// Major and minor keys that share a key signature, such as C and a.
    Relative,
    /// The engine's key lies a fifth above the expert's.
    Dominant,
    /// The engine's key lies a fifth below the expert's.
    Subdominant,
    /// The same tonic in the other mode, such as C and c.
    Parallel,
    Other,
}

impl KeyRelation {
    pub fn name(self) -> &'static str {
        match self {
            KeyRelation::Tonicization => "tonicization vs modulation",
            KeyRelation::Relative => "relative",
            KeyRelation::Dominant => "dominant",
            KeyRelation::Subdominant => "subdominant",
            KeyRelation::Parallel => "parallel",
            KeyRelation::Other => "other",
        }
    }
}

/// Relates the key of the engine's label to that of the expert's, which must differ.
pub fn relate_keys(expert: &Label, engine: &Label) -> KeyRelation {
    if is_tonicization_of(expert, engine) || is_tonicization_of(engine, expert) {
        return KeyRelation::Tonicization;
    }
    let (from, to) = (expert.key, engine.key);
    match (from.mode == to.mode, to.tonic - from.tonic) {
        (false, 0) => KeyRelation::Parallel,
        (false, 3) if from.mode == Mode::Major => KeyRelation::Relative,
        (false, -3) if from.mode == Mode::Minor => KeyRelation::Relative,
        (_, 1) => KeyRelation::Dominant,
        (_, -1) => KeyRelation::Subdominant,
        _ => KeyRelation::Other,
    }
}

/// Reports whether `applied` tonicizes the tonic of the key that `local` is read in.
pub fn is_tonicization_of(applied: &Label, local: &Label) -> bool {
    applied.key != local.key && applied.tonicized.is_some_and(|t| t.tonic == local.key.tonic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn label(key: &str, mode: &str, root: &str) -> Label {
        let root = tpc_from_name(root).unwrap();
        Label::new(Key::parse(key, mode).unwrap(), None, Chord::new(root, Quality::Major), root)
    }

    fn applied(key: &str, target: &str, root: &str) -> Label {
        let (key, target) = (Key::parse(key, "major").unwrap(), Key::parse(target, "major").unwrap());
        let root = tpc_from_name(root).unwrap();
        Label::new(key, Some(target), Chord::new(root, Quality::Major), root)
    }

    #[test]
    fn a_minor_is_relative_to_c_major() {
        assert_eq!(relate_keys(&label("C", "major", "C"), &label("A", "minor", "A")), KeyRelation::Relative);
    }

    #[test]
    fn e_flat_major_is_relative_to_c_minor() {
        let relation = relate_keys(&label("C", "minor", "C"), &label("E-", "major", "E-"));
        assert_eq!(relation, KeyRelation::Relative);
    }

    #[test]
    fn g_major_is_the_dominant_of_c_major() {
        assert_eq!(relate_keys(&label("C", "major", "C"), &label("G", "major", "G")), KeyRelation::Dominant);
    }

    #[test]
    fn f_major_is_the_subdominant_of_c_major() {
        let relation = relate_keys(&label("C", "major", "C"), &label("F", "major", "F"));
        assert_eq!(relation, KeyRelation::Subdominant);
    }

    #[test]
    fn c_minor_is_parallel_to_c_major() {
        assert_eq!(relate_keys(&label("C", "major", "C"), &label("C", "minor", "G")), KeyRelation::Parallel);
    }

    #[test]
    fn d_major_is_unrelated_to_c_major() {
        assert_eq!(relate_keys(&label("C", "major", "C"), &label("D", "major", "D")), KeyRelation::Other);
    }

    #[test]
    fn v_of_v_against_v_in_the_dominant_is_tonicization() {
        let relation = relate_keys(&applied("C", "G", "D"), &label("G", "major", "D"));
        assert_eq!(relation, KeyRelation::Tonicization);
    }

    #[test]
    fn engine_tonicizing_where_the_expert_modulates_is_tonicization() {
        let relation = relate_keys(&label("G", "major", "D"), &applied("C", "G", "D"));
        assert_eq!(relation, KeyRelation::Tonicization);
    }
}
