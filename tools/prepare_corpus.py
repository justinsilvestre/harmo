# /// script
# requires-python = ">=3.12"
# dependencies = ["music21==10.5.2"]
# ///
"""Downloads the benchmark corpora and converts their RomanText analyses to JSON ground truth.

This script is the only part of the project that uses Python. It runs once, offline, and writes:
- data/sources/: pinned clones of When in Rome, the Chorale Corpus and AugmentedNet (for its splits).
- data/groundtruth/<piece>/<analyst>.json: one harmony list per analysis file.
- data/manifest.json: every piece with its score, its analyses and the benchmark sets it belongs to.

Usage: uv run tools/prepare_corpus.py [--jobs N]
"""

import argparse
import ast
import json
import multiprocessing
import os
import re
import subprocess
import unicodedata
from pathlib import Path

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
ANET_SPLIT_MODULES = ["wir", "abc_dcml", "mps", "wirwtc"]

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--jobs", type=int, default=os.cpu_count())
    args = parser.parse_args()
    fetch_sources()
    pieces = discover_pieces()
    attach_anet_splits(pieces)
    with multiprocessing.Pool(args.jobs) as pool:
        results = pool.map(convert_piece, pieces, chunksize=4)
    manifest = [piece for piece in results if piece["analyses"]]
    for piece in manifest:
        if len(piece["analyses"]) >= 2:
            piece["sets"].append("agreement")
    (DATA / "manifest.json").write_text(json.dumps(manifest, indent=1, ensure_ascii=False))
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
        pieces.append(
            {
                "id": slugify("/".join(relative.parts)),
                "wir_path": str(relative),
                "score": str(score.relative_to(DATA)),
                "analysis_files": [str((Path(directory) / f).relative_to(DATA)) for f in analyses],
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


def load_anet_module(module):
    path = DATA / "sources" / "AugmentedNet" / "AugmentedNet" / "data" / f"{module}.py"
    # The module is downloaded code, so its two dictionaries are read as literals rather than executed.
    tree = ast.parse(path.read_text())
    values = {
        node.targets[0].id: ast.literal_eval(node.value)
        for node in tree.body
        if isinstance(node, ast.Assign)
        and getattr(node.targets[0], "id", None) in ("annotation_score_duples", "splits")
    }
    return values["annotation_score_duples"], values["splits"]


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
    for analysis_file in piece.pop("analysis_files"):
        analyst = analyst_name(analysis_file)
        try:
            harmonies = extract_harmonies(DATA / analysis_file)
        except Exception as error:  # noqa: BLE001 - malformed files are reported and skipped.
            print(f"skipping {analysis_file}: {error}")
            continue
        target = DATA / "groundtruth" / piece["id"] / f"{analyst}.json"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps({"source": analysis_file, "harmonies": harmonies}))
        analyses[analyst] = str(target.relative_to(DATA))
    piece["analyses"] = analyses
    return piece


def analyst_name(analysis_file):
    stem = Path(analysis_file).stem
    return "main" if stem == "analysis" else stem.removeprefix("analysis_")


def extract_harmonies(path):
    import music21

    score = music21.converter.parse(str(path), format="romantext", forceSource=True)
    return [
        harmony_record(measure, rn)
        for measure in score.parts[0].getElementsByClass("Measure")
        for rn in measure.getElementsByClass("RomanNumeral")
    ]


def harmony_record(measure, rn):
    key = rn.key
    tonicized = rn.secondaryRomanNumeralKey
    return {
        "measure": measure.number,
        "measure_suffix": measure.numberSuffix or "",
        "offset": float(rn.offset),
        "figure": rn.figure,
        "key": {"tonic": key.tonic.name, "mode": key.mode},
        "tonicized_key": {"tonic": tonicized.tonic.name, "mode": tonicized.mode} if tonicized else None,
        "root": rn.root().name,
        "bass": rn.bass().name,
        "pitches": [p.name for p in rn.pitches],
    }


def print_summary(manifest):
    counts = {}
    for piece in manifest:
        for name in piece["sets"]:
            counts[name] = counts.get(name, 0) + 1
    for name, count in sorted(counts.items()):
        print(f"{name:40} {count}")


if __name__ == "__main__":
    main()
