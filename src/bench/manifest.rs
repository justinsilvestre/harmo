//! The corpus manifest written by tools/prepare_corpus.py.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
pub struct ManifestPiece {
    pub id: String,
    pub score: String,
    pub sets: Vec<String>,
    /// Ground-truth files by analyst; "main" is the corpus's primary analysis.
    pub analyses: BTreeMap<String, String>,
}

pub struct Manifest {
    pub root: PathBuf,
    pub pieces: Vec<ManifestPiece>,
}

impl Manifest {
    pub fn load(data_dir: &Path) -> Result<Manifest, String> {
        let path = data_dir.join("manifest.json");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("{}: {e} (run `uv run tools/prepare_corpus.py` first)", path.display()))?;
        let pieces = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        Ok(Manifest { root: data_dir.to_path_buf(), pieces })
    }

    /// Returns the pieces that belong to any of the sets (all pieces when no set is given).
    pub fn select(&self, sets: &[String]) -> Vec<&ManifestPiece> {
        let wanted = |p: &&ManifestPiece| sets.is_empty() || p.sets.iter().any(|s| sets.contains(s));
        self.pieces.iter().filter(wanted).collect()
    }
}
