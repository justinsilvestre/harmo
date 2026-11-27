//! A minimal MusicXML (partwise) reader that extracts sounding notes and measures.

use super::model::{KeySignature, Measure, Note, Offset, Score};
use crate::pitch::{Tpc, tpc_from_step};
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

const EPSILON: f64 = 1e-6;

/// Reads a compressed (.mxl) or uncompressed MusicXML file.
pub fn read_score(path: &Path) -> Result<Score, String> {
    let is_compressed = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mxl"));
    let text = if is_compressed { read_mxl(path)? } else { read_text(path)? };
    parse_musicxml(&text)
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read(path).map(decode_text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Decodes UTF-8 or UTF-16 (with a byte-order mark); other encodings are read leniently as UTF-8.
fn decode_text(bytes: Vec<u8>) -> String {
    let utf16 = |chunk: &[u8], little_endian: bool| match little_endian {
        true => u16::from_le_bytes([chunk[0], chunk[1]]),
        false => u16::from_be_bytes([chunk[0], chunk[1]]),
    };
    match bytes.get(..2) {
        Some([0xFF, 0xFE]) | Some([0xFE, 0xFF]) => {
            let little_endian = bytes[0] == 0xFF;
            let units: Vec<u16> =
                bytes[2..].as_chunks::<2>().0.iter().map(|c| utf16(c, little_endian)).collect();
            String::from_utf16_lossy(&units)
        }
        _ => String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
    }
}

fn read_mxl(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let root = root_file_name(&mut archive)?;
    let mut entry = archive.by_name(&root).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Ok(decode_text(bytes))
}

fn root_file_name(archive: &mut zip::ZipArchive<std::fs::File>) -> Result<String, String> {
    if let Ok(mut container) = archive.by_name("META-INF/container.xml") {
        let mut text = String::new();
        container.read_to_string(&mut text).map_err(|e| e.to_string())?;
        let doc = Document::parse(&text).map_err(|e| e.to_string())?;
        let rootfile = doc.descendants().find(|n| n.has_tag_name("rootfile"));
        if let Some(path) = rootfile.and_then(|n| n.attribute("full-path")) {
            return Ok(path.to_string());
        }
    }
    let names: Vec<String> = archive.file_names().filter_map(|n| n.ok()).map(|n| n.to_string()).collect();
    names
        .into_iter()
        .find(|n| !n.starts_with("META-INF") && (n.ends_with(".xml") || n.ends_with(".musicxml")))
        .ok_or_else(|| "no MusicXML file inside the archive".to_string())
}

/// Parses MusicXML text into notes at sounding pitch and the measure layout of the first part.
pub fn parse_musicxml(text: &str) -> Result<Score, String> {
    let options = ParsingOptions { allow_dtd: true, ..ParsingOptions::default() };
    let doc = Document::parse_with_options(text, options).map_err(|e| e.to_string())?;
    let parts: Vec<Node> = doc.root_element().children().filter(|n| n.has_tag_name("part")).collect();
    if parts.is_empty() {
        return Err("no <part> elements (only score-partwise is supported)".into());
    }
    let readings: Vec<PartReading> = parts.iter().enumerate().map(|(i, p)| read_part(*p, i as u16)).collect();
    let measures = merge_measures(&readings);
    let key_signatures = realign_key_signatures(&readings[0], &measures);
    let notes: Vec<Note> = readings.into_iter().flat_map(|r| realign_notes(r, &measures)).collect();
    let fermata_ends = fermata_ends(&notes);
    Ok(Score { notes, measures, fermata_ends, key_signatures })
}

/// Returns the distinct end times of notes marked with a fermata.
fn fermata_ends(notes: &[Note]) -> Vec<Offset> {
    let mut ends: Vec<Offset> =
        notes.iter().filter(|n| n.has_fermata).map(|n| n.onset + n.duration).collect();
    ends.sort_by(f64::total_cmp);
    ends.dedup_by(|a, b| (*a - *b).abs() < EPSILON);
    ends
}

struct PartReading {
    notes: Vec<Note>,
    /// The index of the measure in which each note starts.
    notes_measures: Vec<usize>,
    measures: Vec<Measure>,
    /// Each key signature with the index of the measure in which it appears.
    key_signatures: Vec<(usize, KeySignature)>,
}

/// Moves a part's notes onto the merged measure grid, in case the part's measures were shorter.
fn realign_notes(reading: PartReading, measures: &[Measure]) -> Vec<Note> {
    let shifts: Vec<f64> =
        reading.notes_measures.iter().map(|&i| measure_shift(&reading, measures, i)).collect();
    reading.notes.into_iter().zip(shifts).map(|(note, s)| Note { onset: note.onset + s, ..note }).collect()
}

/// Moves a part's key signatures onto the merged measure grid, keeping only actual changes.
fn realign_key_signatures(reading: &PartReading, measures: &[Measure]) -> Vec<KeySignature> {
    let mut signatures: Vec<KeySignature> = Vec::new();
    for &(index, signature) in &reading.key_signatures {
        let start = signature.start + measure_shift(reading, measures, index);
        let is_change = signatures.last().is_none_or(|last| last.fifths != signature.fifths);
        if is_change {
            signatures.push(KeySignature { start, ..signature });
        }
    }
    signatures
}

/// Returns how far the merged grid moves the start of a part's measure.
fn measure_shift(reading: &PartReading, measures: &[Measure], index: usize) -> Offset {
    match (measures.get(index), reading.measures.get(index)) {
        (Some(merged), Some(own)) => merged.start - own.start,
        _ => 0.0,
    }
}

#[derive(Default, Clone, Copy)]
struct Transposition {
    diatonic: i32,
    chromatic: i32,
    octaves: i32,
}

struct PartState {
    part: u16,
    divisions: f64,
    position: Offset,
    last_onset: Offset,
    beats: u32,
    beat_type: u32,
    transposition: Transposition,
    notes: Vec<Note>,
    notes_measures: Vec<usize>,
    key_signatures: Vec<(usize, KeySignature)>,
    measure_index: usize,
    open_ties: HashMap<i32, usize>,
}

fn read_part(part: Node, index: u16) -> PartReading {
    let mut state = PartState {
        part: index,
        divisions: 1.0,
        position: 0.0,
        last_onset: 0.0,
        beats: 4,
        beat_type: 4,
        transposition: Transposition::default(),
        notes: Vec::new(),
        notes_measures: Vec::new(),
        key_signatures: Vec::new(),
        measure_index: 0,
        open_ties: HashMap::new(),
    };
    let mut measures = Vec::new();
    for (index, measure) in part.children().filter(|n| n.has_tag_name("measure")).enumerate() {
        state.measure_index = index;
        measures.push(read_measure(measure, &mut state));
    }
    PartReading {
        notes: state.notes,
        notes_measures: state.notes_measures,
        measures,
        key_signatures: state.key_signatures,
    }
}

fn read_measure(measure: Node, state: &mut PartState) -> Measure {
    let start = state.position;
    let mut reached = start;
    for child in measure.children().filter(Node::is_element) {
        match child.tag_name().name() {
            "attributes" => read_attributes(child, state),
            "note" => read_note(child, state),
            "backup" => state.position -= duration_of(child, state),
            "forward" => state.position += duration_of(child, state),
            _ => {}
        }
        reached = reached.max(state.position);
    }
    let nominal = state.beats as f64 * 4.0 / state.beat_type as f64;
    let duration = measure_duration(measure, reached - start, nominal);
    state.position = start + duration;
    Measure {
        number: measure.attribute("number").unwrap_or("").to_string(),
        start,
        duration,
        beats: state.beats,
        beat_type: state.beat_type,
    }
}

/// Returns how long a part's measure lasts, given how far its contents reach.
/// - A measure with nothing in it takes no time: converters leave such measures at stray barlines.
/// - A measure of rests lasts at most a full bar, since some files write a whole-bar rest twice.
/// - A measure whose contents take no time, such as one holding only grace notes, lasts a full bar.
fn measure_duration(measure: Node, reached: Offset, nominal: Offset) -> Offset {
    let mut events = measure.children().filter(|c| c.has_tag_name("note") || c.has_tag_name("forward"));
    let is_rest = |event: Node| event.has_tag_name("forward") || child(event, "rest").is_some();
    if reached > EPSILON {
        let is_rests_only = events.all(is_rest);
        if is_rests_only { reached.min(nominal) } else { reached }
    } else if events.next().is_none() {
        0.0
    } else {
        nominal
    }
}

fn read_attributes(attributes: Node, state: &mut PartState) {
    if let Some(divisions) = child_number(attributes, "divisions") {
        state.divisions = divisions;
    }
    if let Some(time) = child(attributes, "time") {
        let beats = child_text(time, "beats")
            .map(|t| t.split('+').filter_map(|b| b.trim().parse::<u32>().ok()).sum());
        state.beats = beats.filter(|&b| b > 0).unwrap_or(state.beats);
        state.beat_type = child_number(time, "beat-type").map(|b| b as u32).unwrap_or(state.beat_type);
    }
    if let Some(transpose) = child(attributes, "transpose") {
        state.transposition = Transposition {
            diatonic: child_number(transpose, "diatonic").unwrap_or(0.0) as i32,
            chromatic: child_number(transpose, "chromatic").unwrap_or(0.0) as i32,
            octaves: child_number(transpose, "octave-change").unwrap_or(0.0) as i32,
        };
    }
    if let Some(signature) = child(attributes, "key").and_then(|key| key_signature(key, state)) {
        state.key_signatures.push((state.measure_index, signature));
    }
}

/// Reads a traditional key signature at sounding pitch. Signatures without `<fifths>` are skipped,
/// and so is `<mode>`, which exports often fill in as "major" whatever the key.
fn key_signature(key: Node, state: &PartState) -> Option<KeySignature> {
    let written = child_number(key, "fifths")? as i32;
    let Transposition { diatonic, chromatic, .. } = state.transposition;
    Some(KeySignature { start: state.position, fifths: written + 7 * chromatic - 12 * diatonic })
}

fn read_note(note: Node, state: &mut PartState) {
    if child(note, "grace").is_some() || child(note, "cue").is_some() {
        return;
    }
    let duration = duration_of(note, state);
    let is_chord_member = child(note, "chord").is_some();
    let onset = if is_chord_member { state.last_onset } else { state.position };
    if !is_chord_member {
        state.last_onset = onset;
        state.position += duration;
    }
    if let Some((tpc, midi)) = child(note, "pitch").and_then(|p| sounding_pitch(p, state.transposition)) {
        let ties_back = note.children().any(|n| n.has_tag_name("tie") && n.attribute("type") == Some("stop"));
        let has_fermata = note.descendants().any(|n| n.has_tag_name("fermata"));
        let note =
            Note { onset, duration, tpc, midi, part: state.part, tied_from_previous: ties_back, has_fermata };
        add_note(state, note);
    }
}

/// Adds a note, merging it into the note it is tied from when that note ends where this one starts.
fn add_note(state: &mut PartState, note: Note) {
    if note.tied_from_previous
        && let Some(&index) = state.open_ties.get(&note.midi)
    {
        let previous = &mut state.notes[index];
        if (previous.onset + previous.duration - note.onset).abs() < EPSILON {
            previous.duration += note.duration;
            previous.has_fermata |= note.has_fermata;
            return;
        }
    }
    state.open_ties.insert(note.midi, state.notes.len());
    state.notes.push(note);
    state.notes_measures.push(state.measure_index);
}

fn sounding_pitch(pitch: Node, transposition: Transposition) -> Option<(Tpc, i32)> {
    let step = child_text(pitch, "step")?.trim().chars().next()?;
    let alter = child_number(pitch, "alter").unwrap_or(0.0).round() as i32;
    let octave = child_number(pitch, "octave")? as i32;
    let written_tpc = tpc_from_step(step, alter)?;
    let step_semitones = [0, 2, 4, 5, 7, 9, 11]["CDEFGAB".find(step.to_ascii_uppercase())?];
    let written_midi = 12 * (octave + 1) + step_semitones + alter;
    let Transposition { diatonic, chromatic, octaves } = transposition;
    Some((written_tpc + 7 * chromatic - 12 * diatonic, written_midi + chromatic + 12 * octaves))
}

fn duration_of(node: Node, state: &PartState) -> Offset {
    child_number(node, "duration").unwrap_or(0.0) / state.divisions
}

/// Uses the first part's measure numbers and time signatures, and the longest part's durations.
fn merge_measures(readings: &[PartReading]) -> Vec<Measure> {
    let mut measures = readings[0].measures.clone();
    let mut start = 0.0;
    for (index, measure) in measures.iter_mut().enumerate() {
        let longest = readings.iter().filter_map(|r| r.measures.get(index)).map(|m| m.duration);
        measure.duration = longest.fold(measure.duration, f64::max);
        measure.start = start;
        start += measure.duration;
    }
    measures
}

fn child<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    node.children().find(|n| n.has_tag_name(name))
}

fn child_text<'a>(node: Node<'a, 'a>, name: &str) -> Option<&'a str> {
    child(node, name).and_then(|n| n.text())
}

fn child_number(node: Node, name: &str) -> Option<f64> {
    child_text(node, name).and_then(|t| t.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FERMATA_SCORE: &str = r#"<score-partwise><part id="P1"><measure number="1">
        <attributes><divisions>1</divisions></attributes>
        <note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration></note>
        <note><pitch><step>D</step><octave>4</octave></pitch><duration>2</duration>
            <notations><fermata/></notations></note>
        </measure></part></score-partwise>"#;

    fn measure_starts(measures_xml: &str) -> Vec<Offset> {
        let text = format!(
            r#"<score-partwise><part id="P1"><measure number="1">
            <attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>
            <note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note>
            </measure>{measures_xml}</part></score-partwise>"#
        );
        parse_musicxml(&text).unwrap().measures.iter().map(|m| m.start).collect()
    }

    #[test]
    fn empty_measure_takes_no_time() {
        let next = r#"<measure number="2"><note><rest/><duration>4</duration></note></measure>"#;
        assert_eq!(measure_starts(&format!(r#"<measure number="0"/>{next}"#)), vec![0.0, 4.0, 4.0]);
    }

    #[test]
    fn doubled_whole_bar_rest_lasts_one_bar() {
        let rests = r#"<measure number="2"><note><rest/><duration>4</duration></note>
            <note><rest/><duration>4</duration></note></measure><measure number="3"/>"#;
        assert_eq!(measure_starts(rests), vec![0.0, 4.0, 8.0]);
    }

    fn key_signatures(measures_xml: &str) -> Vec<KeySignature> {
        let text = format!(r#"<score-partwise><part id="P1">{measures_xml}</part></score-partwise>"#);
        parse_musicxml(&text).unwrap().key_signatures
    }

    const KEY_CHANGE: &str = r#"<measure number="1">
        <attributes><divisions>1</divisions><key><fifths>-3</fifths><mode>minor</mode></key></attributes>
        <note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure>
        <measure number="2"><attributes><key><fifths>0</fifths></key></attributes>
        <note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure>"#;

    #[test]
    fn key_signature_reads_fifths() {
        let first = KeySignature { start: 0.0, fifths: -3 };
        assert_eq!(key_signatures(KEY_CHANGE)[0], first);
    }

    #[test]
    fn key_signature_change_starts_at_its_measure() {
        assert_eq!(key_signatures(KEY_CHANGE)[1].start, 4.0);
    }

    #[test]
    fn key_signature_of_a_transposing_part_is_read_at_sounding_pitch() {
        let clarinet = r#"<measure number="1"><attributes><divisions>1</divisions><key><fifths>2</fifths></key>
            <transpose><diatonic>-1</diatonic><chromatic>-2</chromatic></transpose></attributes></measure>"#;
        assert_eq!(key_signatures(clarinet)[0].fifths, 0);
    }

    #[test]
    fn fermata_ends_are_read_from_notations() {
        assert_eq!(parse_musicxml(FERMATA_SCORE).unwrap().fermata_ends, vec![4.0]);
    }
}
