//! The official themes, embedded in the binary: a presentation builds without a `themes/` folder
//! of its own. They are written once into the cache (`~/.cache/glitu/themes-<hash>/`, the hash
//! of their content), where the build reads them like any theme on disk. A `themes/<name>`
//! folder next to the presentation, or above it, still wins.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

include!(concat!(env!("OUT_DIR"), "/themes.rs"));

/// Names of the embedded themes, sorted
pub fn names() -> Vec<&'static str> {
    let mut names: Vec<&str> = THEMES.iter().filter_map(|(path, _)| path.split_once('/').map(|(n, _)| n)).collect();
    names.dedup();
    names
}

/// The folder of the embedded theme `name`, written into the cache if needed; `None` if there
/// is no such theme
pub fn dir(name: &str) -> Result<Option<PathBuf>, String> {
    if !names().contains(&name) {
        return Ok(None);
    }
    Ok(Some(root()?.join(name)))
}

/// The folder holding every embedded theme, written into the cache if needed
pub fn root() -> Result<PathBuf, String> {
    let root = crate::online::cache_dir().join(format!("themes-{}", hash()));
    if !root.is_dir() {
        extract(&root)?;
    }
    Ok(root)
}

/// Short hash of the embedded files: a binary with other themes uses another folder
fn hash() -> String {
    let mut h = Sha256::new();
    for (path, bytes) in THEMES {
        h.update(path.as_bytes());
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(bytes);
    }
    h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// Writes the themes into a temporary folder, then renames it: a build running at the same time
/// never reads a half-written theme
fn extract(root: &Path) -> Result<(), String> {
    let tmp = root.with_extension(format!("tmp-{}", std::process::id()));
    let err = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    for (path, bytes) in THEMES {
        let dest = tmp.join(path);
        fs::create_dir_all(dest.parent().unwrap_or(&tmp)).map_err(|e| err(&dest, e))?;
        fs::write(&dest, bytes).map_err(|e| err(&dest, e))?;
    }
    // another process may have won the race: its copy is the same
    if fs::rename(&tmp, root).is_err() {
        let _ = fs::remove_dir_all(&tmp);
        if !root.is_dir() {
            return Err(format!("{}: cannot write the embedded themes", root.display()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_official_themes_are_embedded() {
        let names = names();
        for theme in ["crystal", "black", "paper", "blueprint"] {
            assert!(names.contains(&theme), "{names:?}");
        }
        assert!(THEMES.iter().any(|(p, _)| *p == "crystal/glitu.json"));
    }
}
