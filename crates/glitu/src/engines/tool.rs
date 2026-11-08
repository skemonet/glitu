//! Finding a tool (§3 and SEC-3 of `docs/design/diagram-engines.md`): the path the user named
//! (`--tool`, `GLITU_TOOL_*`, `tools:` of the configuration), otherwise the `PATH`, resolved by
//! glitu to an absolute path before the tool starts. The `PATH` lookup skips its relative entries
//! and refuses a program inside the presentation's folder; on Windows, only `.exe` files.

use std::path::{Path, PathBuf};

use crate::settings::Settings;

/// Where a tool was found
#[derive(Clone, Debug, PartialEq)]
pub enum Found {
    /// Named by the user
    Named(PathBuf),
    /// In the `PATH`
    Path(PathBuf),
}

impl Found {
    pub fn path(&self) -> &Path {
        match self {
            Found::Named(p) | Found::Path(p) => p,
        }
    }
}

/// The tool `name`; `root`: the presentation's folder (a program there is not taken from the
/// `PATH`). `Ok(None)`: not found
pub fn find(name: &str, settings: &Settings, root: &Path) -> Result<Option<Found>, String> {
    if let Some(p) = settings.tools.get(name) {
        let p = absolute(p)?;
        if !p.is_file() {
            return Err(format!("{name}: {} is not a file (given by --tool, GLITU_TOOL_* or tools:)", p.display()));
        }
        return Ok(Some(Found::Named(p)));
    }
    let root = root.canonicalize().ok();
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        // `.`, an empty entry or `bin`: relative to wherever glitu runs, never trusted
        if !dir.is_absolute() {
            continue;
        }
        for file in candidates(&dir, name) {
            if !executable(&file) {
                continue;
            }
            let real = file.canonicalize().unwrap_or_else(|_| file.clone());
            if root.as_ref().is_some_and(|r| real.starts_with(r)) {
                return Err(format!(
                    "{name}: {} is inside the presentation's folder; a tool is never taken from there. \
                     If it is yours, name it: --tool {name}={}",
                    file.display(),
                    file.display()
                ));
            }
            return Ok(Some(Found::Path(file)));
        }
    }
    Ok(None)
}

/// The files that would be `name` in `dir`
fn candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    if cfg!(windows) {
        // `.bat` and `.cmd` go through cmd.exe, whose quoting cannot be made safe (CVE-2024-24576)
        vec![dir.join(format!("{name}.exe"))]
    } else {
        vec![dir.join(name)]
    }
}

fn executable(file: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.metadata().is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        file.is_file()
    }
}

/// A path named by the user, made absolute; on Windows a `.bat` or `.cmd` is refused
fn absolute(p: &Path) -> Result<PathBuf, String> {
    let ext = p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    if cfg!(windows) && matches!(ext.as_deref(), Some("bat" | "cmd")) {
        return Err(format!(
            "{}: a .bat or .cmd file cannot be started safely; give the program itself (.exe)",
            p.display()
        ));
    }
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    std::env::current_dir().map(|d| d.join(p)).map_err(|e| e.to_string())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn named_tools_win_and_the_deck_never_provides_one() {
        let d = std::env::temp_dir().join(format!("glitu-tool-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("deck/bin")).unwrap();
        let fake = d.join("deck/bin/faketool-glitu");
        fs::write(&fake, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        let mut s = Settings::default();
        assert_eq!(find("faketool-glitu", &s, &d.join("deck")).unwrap(), None);
        s.tools.insert("faketool-glitu".into(), fake.clone());
        assert_eq!(find("faketool-glitu", &s, &d.join("deck")).unwrap(), Some(Found::Named(fake.clone())));
        s.tools.insert("faketool-glitu".into(), d.join("missing"));
        assert!(find("faketool-glitu", &s, &d.join("deck")).is_err());
        // `sh` from the PATH, absolute
        let sh = find("sh", &Settings::default(), &d).unwrap().unwrap();
        assert!(sh.path().is_absolute());
        let _ = fs::remove_dir_all(&d);
    }
}
