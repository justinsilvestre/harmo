//! A minimal MusicXML (partwise) reader that extracts sounding notes and measures.

use super::model::{Measure, Note, Offset, Score};
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
    let notes = readings.into_iter().flat_map(|r| realign_notes(r, &measures)).collect();
    Ok(Score { notes, measures })
}

struct PartReading {
    notes: Vec<Note>,
    /// The index of the measure in which each note starts.
    notes_measures: Vec<usize>,
    measures: Vec<Measure>,
}

/// Moves a part's notes onto the merged measure grid, in case the part's measures were shorter.
fn realign_notes(reading: PartReading, measures: &[Measure]) -> Vec<Note> {
    let shift = |index: usize| match (measures.get(index), reading.measures.get(index)) {
        (Some(merged), Some(own)) => merged.start - own.start,
        _ => 0.0,
    };
    let shifts: Vec<f64> = reading.notes_measures.iter().map(|&i| shift(i)).collect();
    reading.notes.into_iter().zip(shifts).map(|(note, s)| Note { onset: note.onset + s, ..note }).collect()
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
        measure_index: 0,
        open_ties: HashMap::new(),
    };
    let mut measures = Vec::new();
    for (index, measure) in part.children().filter(|n| n.has_tag_name("measure")).enumerate() {
        state.measure_index = index;
        measures.push(read_measure(measure, &mut state));
    }
    PartReading { notes: state.notes, notes_measures: state.notes_measures, measures }
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
    let duration = if reached - start > EPSILON { reached - start } else { nominal };
    state.position = start + duration;
    Measure {
        number: measure.attribute("number").unwrap_or("").to_string(),
        start,
        duration,
        beats: state.beats,
        beat_type: state.beat_type,
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
        add_note(state, Note { onset, duration, tpc, midi, part: state.part, tied_from_previous: ties_back });
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
