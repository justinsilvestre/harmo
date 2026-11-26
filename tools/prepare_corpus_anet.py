"""Collects the AugmentedNet modules whose scores are not in When in Rome: HaydnSun, TAVERN and BPS.

AugmentedNet pairs each analysis with a score through paths into its rawdata/ submodules.
This module checks out those submodules at the commits AugmentedNet pins, fetching only the folders it needs.
Humdrum scores are converted to MusicXML, because the engine reads only MusicXML.
"""

import ast
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data"
ANET = DATA / "sources" / "AugmentedNet"

SUBMODULE_URLS = {
    "When-in-Rome": "https://github.com/MarkGotham/When-in-Rome.git",
    "haydn_op20_harm": "https://github.com/napulen/haydn_op20_harm.git",
    "functional-harmony-micchi": "https://github.com/napulen/functional-harmony-micchi.git",
}
SCORE_MODULES = ["haydnsun", "tavern", "bps"]
# TAVERN has two analyses of every piece, which AugmentedNet lists as separate entries ending in -a and -b.
TAVERN_ANALYSTS = {"a": "main", "b": "B"}


def discover_anet_pieces():
    """Returns the pieces of the score modules, with their AugmentedNet split labels."""
    modules = {module: load_anet_module(module) for module in SCORE_MODULES}
    fetch_submodules([path for duples, _ in modules.values() for pair in duples.values() for path in pair])
    pieces = {}
    for module, (duples, splits) in modules.items():
        for split, anet_ids in splits.items():
            for anet_id in anet_ids:
                add_entry(pieces, module, split, anet_id, duples[anet_id])
    return list(pieces.values())


def load_anet_module(module):
    """Reads a module's annotation-score pairs and its splits."""
    path = ANET / "AugmentedNet" / "data" / f"{module}.py"
    # The module is downloaded code, so its two dictionaries are read as literals rather than executed.
    tree = ast.parse(path.read_text())
    values = {
        node.targets[0].id: ast.literal_eval(node.value)
        for node in tree.body
        if isinstance(node, ast.Assign)
        and getattr(node.targets[0], "id", None) in ("annotation_score_duples", "splits")
    }
    return values["annotation_score_duples"], values["splits"]


def fetch_submodules(anet_paths):
    """Checks out the folders that the paths need, in each rawdata/ submodule at AugmentedNet's pinned commit."""
    folders = {}
    for path in anet_paths:
        _, submodule, *rest = Path(path).parts
        if submodule in SUBMODULE_URLS:
            folders.setdefault(submodule, set()).add("/" + "/".join(rest[:-1]) + "/")
    for submodule, patterns in folders.items():
        target = ANET / "rawdata" / submodule
        if (target / ".git").exists():
            continue
        commit = pinned_commit(submodule)
        target.rmdir()
        git("clone", "-q", "--filter=blob:none", "--no-checkout", SUBMODULE_URLS[submodule], str(target))
        git("-C", str(target), "sparse-checkout", "set", "--no-cone", *sorted(patterns))
        git("-C", str(target), "checkout", "-q", commit)


def pinned_commit(submodule):
    return git("-C", str(ANET), "ls-tree", "HEAD", f"rawdata/{submodule}").split()[2]


def git(*arguments):
    return subprocess.run(["git", *arguments], check=True, capture_output=True, text=True).stdout


def add_entry(pieces, module, split, anet_id, duple):
    """Adds one AugmentedNet entry, merging TAVERN's two analyses of a score into one piece."""
    analysis, score = (ANET / path for path in duple)
    folders = analysis.parent.parts
    wir_path = Path(*folders[folders.index("Corpus") + 1 :])
    analyst = TAVERN_ANALYSTS[anet_id[-1]] if module == "tavern" else "main"
    piece = pieces.setdefault(
        str(score),
        {
            "wir_path": str(wir_path),
            "score": str(convert_score(module, score).relative_to(DATA)),
            "analysis_files": {},
            "sets": [wir_path.parts[0].lower(), f"anet-{split}", f"anet-{split}-{module}"],
            "anet_id": re.sub(r"-[ab]$", "", anet_id) if module == "tavern" else anet_id,
        },
    )
    piece["analysis_files"][analyst] = str(analysis.relative_to(DATA))


def convert_score(module, score):
    """Returns a MusicXML version of the score, converting Humdrum files once."""
    if score.suffix != ".krn":
        return score
    target = DATA / "scores" / module / f"{score.stem}.musicxml"
    if not target.exists():
        import music21

        stream = music21.converter.parse(str(score), format="humdrum", forceSource=True)
        mark_short_measures(stream)
        target.parent.mkdir(parents=True, exist_ok=True)
        stream.write("musicxml", fp=str(target))
    return target


def mark_short_measures(stream):
    """Marks measures shorter than their meter, such as halves of a bar split by a repeat sign.

    Without the mark, music21 fills them with hidden rests when it writes MusicXML, which shifts later music.
    """
    for part in stream.parts:
        for measure in part.getElementsByClass("Measure"):
            shortfall = (
                measure.barDuration.quarterLength - measure.duration.quarterLength - measure.paddingLeft
            )
            if shortfall > 0:
                measure.paddingRight = shortfall
