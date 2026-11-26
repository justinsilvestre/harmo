//! Spelled pitch primitives on the line of fifths.
//!
//! A tonal pitch class (TPC) is a position on the line of fifths, with C = 0, G = 1 and F = -1.
//! Spelling is preserved: C# (7) and Db (-5) are different TPCs that share pitch class 1.

pub type Tpc = i32;

const LETTERS_BY_FIFTH: &[u8; 7] = b"FCGDAEB";

/// Returns the TPC for a letter name (A to G) and a chromatic alteration in semitones.
pub fn tpc_from_step(step: char, alter: i32) -> Option<Tpc> {
    let upper = step.to_ascii_uppercase() as u8;
    let index = LETTERS_BY_FIFTH.iter().position(|&letter| letter == upper)?;
    Some(index as i32 - 1 + 7 * alter)
}

/// Returns the pitch class (0 to 11, C = 0) of a TPC.
pub fn pitch_class(tpc: Tpc) -> u8 {
    (7 * tpc).rem_euclid(12) as u8
}

/// Returns the diatonic letter index of a TPC (C = 0, D = 1, ..., B = 6).
pub fn letter_index(tpc: Tpc) -> i32 {
    (4 * tpc).rem_euclid(7)
}

/// Returns the number of sharps (positive) or flats (negative) in a TPC's spelling.
pub fn alteration(tpc: Tpc) -> i32 {
    (tpc + 1).div_euclid(7)
}

/// Returns a name such as "F#" or "Bb".
pub fn tpc_name(tpc: Tpc) -> String {
    let letter = LETTERS_BY_FIFTH[(tpc + 1).rem_euclid(7) as usize] as char;
    let accidental = alteration(tpc);
    let symbol = if accidental > 0 { "#" } else { "b" };
    format!("{letter}{}", symbol.repeat(accidental.unsigned_abs() as usize))
}

/// Parses names such as "F#", "Bb", "B-" (music21 style) or "Cx".
pub fn tpc_from_name(name: &str) -> Option<Tpc> {
    let mut chars = name.trim().chars();
    let letter = chars.next()?;
    let rest: String = chars.collect();
    let alter = match rest.as_str() {
        "x" => 2,
        _ if rest.chars().all(|c| c == '#') => rest.len() as i32,
        _ if rest.chars().all(|c| c == 'b' || c == '-') => -(rest.len() as i32),
        _ => return None,
    };
    tpc_from_step(letter, alter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enharmonic_spellings_share_a_pitch_class() {
        assert_eq!(pitch_class(tpc_from_name("C#").unwrap()), pitch_class(tpc_from_name("Db").unwrap()));
    }

    #[test]
    fn names_round_trip() {
        let names = ["C", "F#", "Bb", "Ebb", "G##", "Fb"];
        let round_tripped: Vec<String> = names.iter().map(|n| tpc_name(tpc_from_name(n).unwrap())).collect();
        assert_eq!(round_tripped, names);
    }

    #[test]
    fn music21_flat_syntax_is_accepted() {
        assert_eq!(tpc_from_name("B-"), tpc_from_step('B', -1));
    }

    #[test]
    fn letter_index_ignores_accidentals() {
        assert_eq!(letter_index(tpc_from_name("F#").unwrap()), 3);
    }

    #[test]
    fn alteration_counts_flats_as_negative() {
        assert_eq!(alteration(tpc_from_name("Ebb").unwrap()), -2);
    }
}
