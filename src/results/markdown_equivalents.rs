//! Renders what the equivalents RN measure counts as right,
//! and how many frames each equivalence credits in each corpus.

use super::markdown_table::{percent, share, table};
use super::set_names::{family, row_name};
use crate::bench::equivalence::EQUIVALENCES;
use crate::bench::report::SetRecord;

const INTRODUCTION: &str = "## Equivalents RN

RN counts a frame as right only when the engine's label is the expert's exactly. Some differences are only notational conventions, two ways of writing the same reading, and **RN eq** also counts those as right. A frame counts when the engine's label is the expert's, or when one of these holds against the expert's label as given:
- **Seventh:** the expert labels a triad and then its own seventh chord on the same bass, or the reverse (V then V7), and the engine gives either of those two labels anywhere over the pair. Analysts differ on whether a seventh that enters partway through a chord starts a new one (Kostka, Payne and Almén 2023, ch. 15). A seventh that enters in the bass (V then V42) changes the bass, so it does not count.
- **Inversion:** the expert relabels one chord with another inversion as its bass moves (I6 then I), and the engine gives either of those two labels anywhere over the pair. Both labels name the same chord: Kostka, Payne and Almén (2023, ch. 8, p. 115) mark such a bass arpeggiation with figures alone, or leave it unmarked.
- **Applied:** an applied chord X/Y against X in the key that Y stands for: V/V in C against V in G, or viio7/ii in C against viio7 in d. This also covers TAVERN's key-area labels such as I/V. Only the frames that carry the slash count, not the rest of a key area. A chord applied to the tonic of the other mode (VI/i in a major key) does not count, since that would credit the parallel key, and neither does the same chord applied from two keys (V/V in C against V/IV in D), which reads it in a different key.
- **Rootless:** V7, in any inversion, against viio of the same key and tonicization over the same bass (or with the V7 written in root position, which then names its silent root rather than a sounding bass), while the dominant's root is not sounding. With the root absent, the two name the same pitches (Burstein and Straus 2016, ch. 13; Kostka, Payne and Almén 2023, ch. 7). The seventh chords viio7 and viiø7 add the dominant's ninth, so they are different chords and do not count.
- **Pedal:** where the lowest pitch class is held or restruck through two or more of the expert's labels, at least one of which leaves it out, the engine counts if it gives the chord the expert names above the pedal in any inversion, or the expert's chord at the start or end of the pedal whose bass is the pedal note. Kostka, Payne and Almén (2023, ch. 13) leave inversions above a pedal unmarked and hear the pedal as prolonging its own harmony. The same holds where the expert writes a pedal six-four between two statements of one root-position chord (I–IV64–I), which the engine may read as that chord throughout: Kostka, Payne and Almén (2023, ch. 9, pp. 139–140) hear it as an elaboration of that chord and put its numeral in parentheses. Their other pedal six-four, V–I64–V, does not count, since a tonic six-four over the dominant may be a cadential six-four.
- **Timing:** the engine gives the expert's label, or one of the equivalents above, at a frame within the timing tolerance (given above with the search settings) of this one.

Readings that genuinely differ never count: a cadential six-four read as V, a leading-tone seventh chord read as V7, an applied chord read from another key, a relative or parallel key, a tonicized area read as a modulation beyond the applied chord itself, or a root a third away.

Each figure below is the percentage of the set's frames that the equivalence credits and RN does not. A frame that several equivalences credit is given to the first in the order listed, and timing comes last, so it counts only frames that no other equivalence credits at the frame itself.
";

pub fn render_equivalents(sets: &[SetRecord]) -> String {
    let mut rows = family(sets, "anet-test", |s| &s.name);
    rows.extend(family(sets, "tune", |s| &s.name));
    let mut header = vec!["set", "RN", "RN eq"];
    header.extend(EQUIVALENCES.iter().map(|e| e.name()));
    let row = |set: &&SetRecord| {
        let credits = set.equivalence_credits.as_ref();
        let credited = |name: &str| credits.and_then(|c| c.get(name)).copied().unwrap_or(0);
        let rate = |index: usize| set.rates.get(index).map_or(String::new(), |r| percent(*r));
        let mut cells = vec![set_label(&set.name), rate(5), rate(11)];
        cells.extend(EQUIVALENCES.iter().map(|e| share(credited(e.name()), set.frames)));
        cells
    };
    format!("{INTRODUCTION}\n{}", table(&header, &rows.iter().map(row).collect::<Vec<_>>()))
}

/// Names a corpus row with its split, such as "WiR (test)", since the table lists both splits.
fn set_label(name: &str) -> String {
    if name == "tune" || name == "anet-test" {
        return row_name(name);
    }
    let split = if name.starts_with("anet-test") { "test" } else { "tune" };
    format!("{} ({split})", row_name(name))
}
