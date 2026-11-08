//! The files a presentation may read (SEC-16 of `docs/design/diagram-engines.md`).
//!
//! A presentation is data received from anyone, and what it reads ends up in a page that is then
//! published: so it reads only inside its root folder (the folder of its main file, and of the
//! other files named on the command line), checked after the symbolic links are resolved. An
//! absolute path is refused; a path that leads outside is an error, unless the user allowed that
//! folder (`--allow-read DIR`, or `files.allow` in the user's configuration, see `settings`).

use std::path::{Component, Path, PathBuf};

/// Where a presentation may read
#[derive(Clone, Debug)]
pub struct Reads {
    /// The folders, canonical: the root first, then those the user allowed
    roots: Vec<PathBuf>,
    /// The root as given, for messages
    shown: PathBuf,
}

impl Reads {
    /// `root`: the presentation's folder; `allowed`: the folders the user allowed besides (one that
    /// does not exist is left out)
    pub fn new(root: &Path, allowed: &[PathBuf]) -> Reads {
        let roots = std::iter::once(root).chain(allowed.iter().map(PathBuf::as_path)).filter_map(canonical).collect();
        Reads { roots, shown: root.to_path_buf() }
    }

    /// A folder alone (a theme's files stay in the theme)
    pub fn only(dir: &Path) -> Reads {
        Reads::new(dir, &[])
    }

    /// The file `written` (a path as the presentation writes it), relative to `base`: the path to
    /// read, or why it is refused
    pub fn open(&self, base: &Path, written: &str) -> Result<PathBuf, String> {
        if is_absolute(written) {
            return Err(format!(
                "{written}: absolute path refused; a presentation reads only the files of its folder, \
                 write the path relative to it"
            ));
        }
        let path = base.join(written);
        self.check(&path, written)?;
        Ok(path)
    }

    /// `path` (already joined to its folder) is inside the allowed folders, once its links are
    /// resolved; `shown` names it in the message. A file that does not exist passes: reading it
    /// will say so, unless its path leads out (`img/../../x`, `img` missing)
    pub fn check(&self, path: &Path, shown: &str) -> Result<(), String> {
        let Some(real) = canonical(path).or_else(|| resolved(path)) else { return Ok(()) };
        if self.roots.iter().any(|r| real.starts_with(r)) {
            return Ok(());
        }
        let root = canonical(&self.shown).unwrap_or_else(|| self.shown.clone());
        Err(format!(
            "{shown}: outside the presentation's folder ({}){}; to read it anyway, allow its folder: \
             --allow-read DIR, or `files: allow: [DIR]` in {}",
            root.display(),
            if written_up(shown) { "" } else { " through a symbolic link" },
            crate::settings::config_file().map_or("the user's configuration".into(), |p| p.display().to_string())
        ))
    }

    /// Is `path` inside? (no message: for the folders copied whole, where a link leading out is
    /// left out with a warning)
    pub fn contains(&self, path: &Path) -> bool {
        self.check(path, "").is_ok()
    }
}

/// The path once its links are resolved, if it exists
fn canonical(path: &Path) -> Option<PathBuf> {
    let path = if path.as_os_str().is_empty() { Path::new(".") } else { path };
    path.canonicalize().ok()
}

/// A path that does not exist: its deepest existing folder resolved, the rest applied as written
fn resolved(path: &Path) -> Option<PathBuf> {
    let parts: Vec<Component> = path.components().collect();
    let found = (0..parts.len()).rev().find_map(|n| {
        let head: PathBuf = parts[..n].iter().collect();
        canonical(&head).map(|c| (n, c))
    });
    let (n, mut out) = found?;
    for c in &parts[n..] {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(p) => out.push(p),
            _ => {}
        }
    }
    Some(out)
}

/// `/x`, `C:\x`, `\\server\x`, `\x`: absolute on some system
fn is_absolute(written: &str) -> bool {
    let p = Path::new(written);
    p.is_absolute()
        || p.has_root()
        || written.starts_with(['/', '\\'])
        || written.as_bytes().get(1) == Some(&b':') && written.as_bytes()[0].is_ascii_alphabetic()
}

/// The written path climbs (`../x`): the message need not mention a link
fn written_up(written: &str) -> bool {
    Path::new(written).components().any(|c| c == Component::ParentDir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Dir(PathBuf);
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn dir(name: &str) -> Dir {
        let d = std::env::temp_dir().join(format!("glitu-confine-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("talk/sub")).unwrap();
        fs::create_dir_all(d.join("shared")).unwrap();
        fs::write(d.join("talk/sub/a.md"), "a").unwrap();
        fs::write(d.join("secret.txt"), "s").unwrap();
        fs::write(d.join("shared/b.md"), "b").unwrap();
        Dir(d)
    }

    #[test]
    fn inside_passes_outside_and_absolute_fail() {
        let d = dir("basic");
        let talk = d.0.join("talk");
        let reads = Reads::new(&talk, &[]);
        assert!(reads.open(&talk, "sub/a.md").is_ok());
        assert!(reads.open(&talk.join("sub"), "../sub/./a.md").is_ok());
        let e = reads.open(&talk, "../secret.txt").unwrap_err();
        assert!(e.starts_with("../secret.txt: outside the presentation's folder") && e.contains("--allow-read"), "{e}");
        let abs = d.0.join("secret.txt").display().to_string();
        assert!(reads.open(&talk, &abs).unwrap_err().contains("absolute path refused"));
        assert!(is_absolute("C:\\x") && is_absolute("\\\\srv\\x") && !is_absolute("a/b"));
        // a missing file is for the reader to report, unless its path leads out
        assert!(reads.open(&talk, "missing.md").is_ok());
        assert!(reads.open(&talk, "missing/../../secret.txt").is_err());
    }

    #[test]
    fn allowed_folders_open_the_way() {
        let d = dir("allow");
        let talk = d.0.join("talk");
        assert!(Reads::new(&talk, &[]).open(&talk, "../shared/b.md").is_err());
        assert!(Reads::new(&talk, &[d.0.join("shared")]).open(&talk, "../shared/b.md").is_ok());
        assert!(Reads::new(&talk, &[d.0.join("shared")]).open(&talk, "../secret.txt").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn links_are_followed_before_checking() {
        let d = dir("link");
        let talk = d.0.join("talk");
        std::os::unix::fs::symlink(d.0.join("secret.txt"), talk.join("innocent.txt")).unwrap();
        std::os::unix::fs::symlink(&d.0, talk.join("up")).unwrap();
        let reads = Reads::new(&talk, &[]);
        let e = reads.open(&talk, "innocent.txt").unwrap_err();
        assert!(e.contains("through a symbolic link"), "{e}");
        assert!(reads.open(&talk, "up/secret.txt").is_err());
        assert!(!reads.contains(&talk.join("innocent.txt")) && reads.contains(&talk.join("sub/a.md")));
    }
}
