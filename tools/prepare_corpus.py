# /// script
# requires-python = ">=3.12"
# dependencies = ["music21==10.5.2"]
# ///
"""Downloads the benchmark corpora and converts their RomanText analyses to JSON ground truth.

This script is the only part of the project that uses Python. It runs once, offline, and writes:
- data/sources/: pinned clones of When in Rome, the Chorale Corpus and AugmentedNet (for its splits),
  plus the AugmentedNet submodules that hold the HaydnSun, TAVERN and BPS scores.
- data/scores/: scores converted to MusicXML.
- data/groundtruth/<piece>/<analyst>.json: one harmony list per analysis file.
- data/manifest.json: every piece with its score, its analyses and the benchmark sets it belongs to.

Usage: uv run tools/prepare_corpus.py [--jobs N] [--manifest PATH]
"""

import argparse
import json
import multiprocessing
import os
import re
import subprocess
import unicodedata
from pathlib import Path

from prepare_corpus_anet import discover_anet_pieces, load_anet_module

SOURCES = {
    "When-in-Rome": (
        "https://github.com/MarkGotham/When-in-Rome.git",
        "1c61fe41b8c2910296d7d2bcbf6476c7c1f2fe35",
    ),
    "AugmentedNet": (
        "https://github.com/napulen/AugmentedNet.git",
        "46d3475651346fd9053db29bc2bfb7943a869b74",
    ),
    "Chorale-Corpus": ("https://github.com/MarkGotham/Chorale-Corpus.git", None),
}
# Machine-generated analyses are not ground truth.
EXCLUDED_ANALYSES = {"analysis_automatic.rntxt"}
# Spans of ground truth that are known to be wrong, by piece and analyst: the first and last excluded
# measures (None for the end of the piece) and why. The bench skips their frames.
EXCLUDED_SPANS = {
    ("quartets-beethoven-ludwig-van-op059-no3-1", "main"): (
        175,
        None,
        "The DCML source keeps the local key in C minor through the C-major recapitulation and coda.",
    ),
    ("quartets-beethoven-ludwig-van-op059-no3-4", "main"): (
        326,
        None,
        "The local key stays C minor over the closing C-major cadences (I, ii, IV in the score).",
    ),
    ("quartets-beethoven-ludwig-van-op132-1", "main"): (
        231,
        None,
        "The local key stays A major for the A-minor close, so VI is read as F# major over F major.",
    ),
    ("quartets-beethoven-ludwig-van-op135-4", "main"): (
        13,
        44,
        "The local key stays F minor through the F-major Allegro.",
    ),
}
# Analyses whose measure numbers disagree with their score from some measure on, by piece and analyst:
# the first measure to move and how many bars to add, applied in order.
MEASURE_SHIFTS = {
    # The analysis folds the repeat that the score writes out.
    ("early-choral-bach-johann-sebastian-chorales-253", "main"): [(6, 2)],
    ("early-choral-bach-johann-sebastian-chorales-272", "main"): [(7, 2)],
    ("early-choral-bach-johann-sebastian-chorales-333", "main"): [(7, 1)],
    # The analysis marks m208a-m209a but no second ending, and its numbering skips the score's
    # four-bar first ending (mm. 211-214).
    ("quartets-beethoven-ludwig-van-op059-no2-1", "main"): [(210, 4)],
    # The main analysis has m174a and m174b; this one writes the repeat as "m175-194 = m1-20" without them.
    ("piano-sonatas-mozart-wolfgang-amadeus-k310-3", "DT"): [(175, 1)],
    # The score numbers its first full bar 0; the analysis calls it m1.
    ("openscore-liedercorpus-schumann-robert-dichterliebe-op-48-10-hor-ich-das-liedchen-klingen", "main"): [(1, -1)],
}
# Corrections to the RomanText of analyses, by piece and analyst: each replaces text that occurs once.
KEY_CORRECTIONS = {
    # The Humdrum source moves to G, but the figures and the music stay in C (m137 is E-G-B, "iii").
    ("quartets-haydn-franz-joseph-op20-no2-4", "main"): [("m137 G: iii", "m137 C: iii")],
    # Titled "C Major to A- minor"; the chords are those of A-flat minor a fifth above the labels.
    ("textbooks-reger-max-modulation-38", "main"): [("db: IV6", "ab: IV6")],
    # Titled "C# Major to G- minor"; the target key is written as G-flat major yet ends on i.
    ("textbooks-reger-max-modulation-45b", "main"): [("Gb: V7", "gb: V7")],
    # The figures are those of G minor (V43/V is A-C#-E-G over E), and the repeat at m283 is labeled "g: i".
    ("quartets-beethoven-ludwig-van-op074-3", "main"): [("m114 c: i", "m114 g: i")],
    # "m32-34 = m16-18" copies E-major labels; the score repeats mm. 16-18 a major third higher, in G-sharp.
    ("openscore-liedercorpus-schumann-robert-dichterliebe-op-48-16-die-alten-bosen-lieder", "main"): [
        ("m32-34 = m16-18", "m32 G#: I\nm34 V")
    ],
    # The conversion folds the second ending (score m67) into "m66a I I", so later labels sit a bar early.
    ("quartets-beethoven-ludwig-van-op135-2", "main"): [("m66a I I b2 V2/IV", "m66a I b2 V2/IV\nm66b I")],
}
ANET_SPLIT_MODULES = ["wir", "abc_dcml", "mps", "wirwtc"]
ANET_SPLITS = ["training", "validation", "test"]
# A minor triad or seventh chord on the seventh degree: "vii" without a diminished sign or an accidental.
MINOR_SEVENTH_DEGREE = re.compile(r"(?<![#b-])vii(?![o%ø+])")
EIGHTH_NOTE_METER = re.compile(r"^(Time Signature:\s*)(3/8)", re.IGNORECASE | re.MULTILINE)
ADDED_TONES = re.compile(r"\[(?:no|add)[^\]]*\]")

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--jobs", type=int, default=os.cpu_count())
    parser.add_argument("--manifest", type=Path, default=DATA / "manifest.json")
    args = parser.parse_args()
    fetch_sources()
    wir_pieces = discover_pieces()
    attach_anet_splits(wir_pieces)
    anet_pieces = discover_anet_pieces()
    for piece in anet_pieces:
        piece["id"] = slugify(piece["wir_path"])
    with multiprocessing.Pool(args.jobs) as pool:
        wir_results = pool.map(convert_piece, wir_pieces, chunksize=4)
        anet_results = pool.map(convert_piece, anet_pieces, chunksize=4)
    # The agreement set predates the TAVERN pieces and keeps to When in Rome's own scores.
    for piece in wir_results:
        if len(piece["analyses"]) >= 2:
            piece["sets"].append("agreement")
    manifest = [piece for piece in wir_results + anet_results if piece["analyses"]]
    attach_tune_sets(manifest)
    write_manifest(manifest, args.manifest)
    print_summary(manifest)


def fetch_sources():
    for name, (url, commit) in SOURCES.items():
        target = DATA / "sources" / name
        if target.exists():
            continue
        if commit is None:
            subprocess.run(["git", "clone", "-q", "--depth", "1", url, str(target)], check=True)
            continue
        subprocess.run(["git", "clone", "-q", "--filter=blob:none", url, str(target)], check=True)
        subprocess.run(["git", "-C", str(target), "checkout", "-q", commit], check=True)


def discover_pieces():
    corpus = DATA / "sources" / "When-in-Rome" / "Corpus"
    pieces = []
    for directory, _, files in sorted(os.walk(corpus)):
        analyses = sorted(
            f
            for f in files
            if f.startswith("analysis") and f.endswith((".txt", ".rntxt")) and f not in EXCLUDED_ANALYSES
        )
        if not analyses:
            continue
        score = find_score(Path(directory), files)
        if score is None:
            continue
        relative = Path(directory).relative_to(corpus)
        directory_in_data = Path(directory).relative_to(DATA)
        pieces.append(
            {
                "id": slugify("/".join(relative.parts)),
                "wir_path": str(relative),
                "score": str(score.relative_to(DATA)),
                "analysis_files": {analyst_name(f): str(directory_in_data / f) for f in analyses},
                "sets": [relative.parts[0].lower()],
            }
        )
    return pieces


def find_score(directory, files):
    if "score.mxl" in files:
        return directory / "score.mxl"
    if "remote.json" not in files:
        return None
    remote = json.loads((directory / "remote.json").read_text())
    url = remote.get("remote_score_mxl", "")
    if "MarkGotham/Chorale-Corpus/" in url:
        return find_chorale_score(url.split("MarkGotham/Chorale-Corpus/", 1)[1])
    if "remote_score_music21" in remote:
        import music21

        path = music21.corpus.getWork(remote["remote_score_music21"])
        return copy_into_data(Path(path), remote["remote_score_music21"])
    return None


def find_chorale_score(url_path):
    """Resolves a Chorale Corpus URL to the local clone, tolerating renamed folders."""
    relative = re.sub(r"^(refs/heads/)?main/", "", url_path)
    for candidate in (relative, relative.replace(",_", "_")):
        path = DATA / "sources" / "Chorale-Corpus" / candidate
        if path.exists():
            return path
    return None


def copy_into_data(source, name):
    target = DATA / "scores" / name
    target.parent.mkdir(parents=True, exist_ok=True)
    if not target.exists():
        target.write_bytes(source.read_bytes())
    return target


def slugify(text):
    ascii_text = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9]+", "-", ascii_text.lower()).strip("-")


def attach_anet_splits(pieces):
    """Tags pieces with the AugmentedNet split (training/validation/test) that contains them."""
    by_key = {piece_match_key(p["wir_path"]): p for p in pieces}
    for module in ANET_SPLIT_MODULES:
        duples, splits = load_anet_module(module)
        for split, ids in splits.items():
            for anet_id in ids:
                piece = by_key.get(anet_match_key(module, anet_id, duples))
                if piece is not None:
                    piece["sets"] += [f"anet-{split}", f"anet-{split}-{module}"]
                    piece["anet_id"] = anet_id


def piece_match_key(wir_path):
    """Builds a key from composer, collection and number that survives When in Rome's renames."""
    parts = Path(wir_path).parts[1:]
    return tuple(normalize_component(part) for part in parts)


def normalize_component(text):
    text = slugify(text).replace("-", "")
    return re.sub(r"(?<![0-9])0+(?=[0-9])", "", text)


def anet_match_key(module, anet_id, duples):
    if module == "abc_dcml":
        match = re.fullmatch(r"abc-op(\d+)(?:-no(\d+))?-(\d+)", anet_id)
        opus = f"op{match[1]}" + (f"no{match[2]}" if match[2] else "")
        return ("beethovenludwigvan", opus, match[3])
    if module == "mps":
        match = re.fullmatch(r"mps-k(\d+)-(\d+)", anet_id)
        return ("mozartwolfgangamadeus", f"k{match[1]}", match[2])
    analysis_path = Path(duples[anet_id][0]).parent
    corpus_parts = analysis_path.parts[analysis_path.parts.index("Corpus") + 2 :]
    return tuple(normalize_component(part) for part in corpus_parts)


def convert_piece(piece):
    analyses = {}
    for analyst, analysis_file in piece.pop("analysis_files").items():
        key = (piece["id"], analyst)
        try:
            harmonies = extract_harmonies(DATA / analysis_file, KEY_CORRECTIONS.get(key, []))
        except Exception as error:  # noqa: BLE001 - malformed files are reported and skipped.
            print(f"skipping {analysis_file}: {error}")
            continue
        shift_measures(harmonies, MEASURE_SHIFTS.get(key, []))
        exclude_span(harmonies, EXCLUDED_SPANS.get(key))
        target = DATA / "groundtruth" / piece["id"] / f"{analyst}.json"
        write_if_changed(target, json.dumps({"source": analysis_file, "harmonies": harmonies}))
        analyses[analyst] = str(target.relative_to(DATA))
    piece["analyses"] = analyses
    return piece


def shift_measures(harmonies, shifts):
    for first_measure, bars in shifts:
        for harmony in harmonies:
            if harmony["measure"] >= first_measure:
                harmony["measure"] += bars


def exclude_span(harmonies, span):
    if span is None:
        return
    first_measure, last_measure, reason = span
    last_measure = float("inf") if last_measure is None else last_measure
    for harmony in harmonies:
        if first_measure <= harmony["measure"] <= last_measure:
            harmony["excluded"] = reason


def write_if_changed(path, text):
    """Replaces a file in one step, or not at all when it is unchanged, so that running benchmarks are not disturbed."""
    if path.exists() and path.read_text() == text:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(text)
    os.replace(temporary, path)


def analyst_name(analysis_file):
    stem = Path(analysis_file).stem
    return "main" if stem == "analysis" else stem.removeprefix("analysis_")


def extract_harmonies(path, corrections=()):
    import music21

    text = path.read_text(encoding="utf-8-sig")
    for old, new in corrections:
        assert text.count(old) == 1, f"{path}: expected one {old!r}"
        text = text.replace(old, new)
    try:
        score = music21.converter.parse(text, format="romantext", forceSource=True)
    except music21.romanText.translate.RomanTextTranslateException as error:
        if "too many notes" not in str(error):
            raise
        # Some files count the beats of 3/8 in eighth notes, which music21 reads only from "slow 3/8".
        slow_text = EIGHTH_NOTE_METER.sub(r"\1slow \2", text)
        score = music21.converter.parse(slow_text, format="romantext", forceSource=True)
    return [
        harmony_record(measure, rn, underlying_chord(rn))
        for measure in score.parts[0].getElementsByClass("Measure")
        for rn in measure.getElementsByClass("RomanNumeral")
    ]


def underlying_chord(rn):
    """Returns the chord a Roman numeral stands for, correcting two readings of music21's.

    - Suspended and added tones, such as [no3][add4], are dropped, so that the chord keeps its quality.
    - In minor, a minor triad on the seventh degree (vii, or a tonicized /vii) is built on the natural
      seventh. music21 builds it on the leading tone, which gives chords such as B# D# F## in C# minor.
    """
    from music21 import roman

    figure = natural_minor_sevenths(ADDED_TONES.sub("", rn.figure), rn.key.mode)
    if figure == rn.figure:
        return rn
    # CAUTIONARY is the convention music21 applies when it reads RomanText.
    convention = roman.Minor67Default.CAUTIONARY
    return roman.RomanNumeral(figure, rn.key, sixthMinor=convention, seventhMinor=convention)


def natural_minor_sevenths(figure, mode):
    """Marks minor triads on the seventh degree of a minor key as flat, part by part of a figure such as V/vii."""
    parts = figure.split("/")
    for index in reversed(range(len(parts))):
        if mode == "minor":
            parts[index] = MINOR_SEVENTH_DEGREE.sub("bvii", parts[index])
        mode = "minor" if re.search(r"[iv]", parts[index]) else "major"
    return "/".join(parts)


def harmony_record(measure, rn, chord):
    """Describes a Roman numeral, taking the root, bass and pitches from the chord it stands for."""
    key = rn.key
    tonicized = chord.secondaryRomanNumeralKey
    return {
        "measure": measure.number,
        "measure_suffix": measure.numberSuffix or "",
        "offset": float(rn.offset),
        "figure": rn.figure,
        "key": {"tonic": key.tonic.name, "mode": key.mode},
        "tonicized_key": {"tonic": tonicized.tonic.name, "mode": tonicized.mode} if tonicized else None,
        "root": chord.root().name,
        "bass": chord.bass().name,
        "pitches": [p.name for p in chord.pitches],
    }


def attach_tune_sets(manifest):
    """Adds the tuning sets, which hold every piece outside AugmentedNet's test split.

    - tune: all such pieces.
    - tune-<module>: the training and validation pieces of an AugmentedNet module.
    - tune-<category>: the pieces of a When in Rome category that belong to no AugmentedNet module.
    - agreement-tune: the agreement pieces outside the test split.
    """
    for piece in manifest:
        sets = piece["sets"]
        if "anet-test" in sets:
            continue
        modules = [
            name.split("-", 2)[2]
            for name in sets
            if name.startswith(tuple(f"anet-{s}-" for s in ANET_SPLITS))
        ]
        category = Path(piece["wir_path"]).parts[0].lower()
        extra = ["tune"] + [f"tune-{name}" for name in modules or [category]]
        if "agreement" in sets:
            extra.append("agreement-tune")
        sets += extra


def write_manifest(manifest, path):
    """Replaces the manifest in one step, so that readers never see a partly written file."""
    ids = [piece["id"] for piece in manifest]
    assert len(ids) == len(set(ids)), "duplicate piece ids"
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(manifest, indent=1, ensure_ascii=False))
    os.replace(temporary, path)


def print_summary(manifest):
    counts = {}
    for piece in manifest:
        for name in piece["sets"]:
            counts[name] = counts.get(name, 0) + 1
    for name, count in sorted(counts.items()):
        print(f"{name:40} {count}")


if __name__ == "__main__":
    main()
