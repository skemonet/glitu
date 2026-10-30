//! Which browser renders the slides for the export (and the oracle): a Chromium-based one,
//! driven through the DevTools protocol.
//!
//! In this order: the `CHROMIUM` variable; the headless shell already downloaded by glitu; a
//! browser installed on the machine (Chrome, Chromium, Edge, Brave, Vivaldi: in the PATH, then in
//! their usual places). Without any, `glitu export --download-browser` downloads Chrome's
//! headless shell ("Chrome for Testing", a Chrome without a window, made for automation): the
//! version below, checked against its SHA-256, unpacked once into `GLITU_BROWSER_DIR`
//! (default `~/.cache/glitu/browser`), after a confirmation (`-y` skips it).

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

/// The headless shell glitu downloads, and the SHA-256 of its archive for each platform
pub const SHELL_VERSION: &str = "156.0.8078.80";
const SHELLS: [(&str, &str); 6] = [
    ("linux64", "547dcd374e02961bf998f6046a267644f8502225c36bb78ca307dab32a0b3eb4"),
    ("linux-arm64", "5bb6a00a4285c13303d3f72c5afdb70f144ea7a93eed462365a52b69631c9607"),
    ("mac-arm64", "ea615baa2e9caf0d02d1898a6562b5f8fa9c35239707dae0f592178df5b986ac"),
    ("mac-x64", "535d417d9034cbd11e7dacd1a71aa3d6e1b525e6d5dcd2671eb2532f01481228"),
    ("win64", "10a77466005e7f64e00f1a75fd1ea81f4a38a60093c65fea6895843b26e60a9a"),
    ("win32", "8d7472c5c60215a24c42e31d0e9567f1fbf41365e46c8d36c59b2a8b8b2b3a65"),
];

/// Names looked for in the PATH
const NAMES: [&str; 11] = [
    "chromium",
    "chromium-browser",
    "google-chrome",
    "google-chrome-stable",
    "chrome",
    "microsoft-edge",
    "microsoft-edge-stable",
    "msedge",
    "brave-browser",
    "brave",
    "vivaldi",
];

/// The browser to use; with `download`, the headless shell is downloaded if there is none
/// (after a confirmation, unless `yes`)
pub fn browser(download: bool, yes: bool) -> Result<PathBuf, String> {
    if let Some(found) = find()? {
        return Ok(found);
    }
    if !download {
        return Err(help());
    }
    fetch_shell(yes)
}

/// `CHROMIUM`, the downloaded shell, or an installed browser; an unusable `CHROMIUM` is an error
pub fn find() -> Result<Option<PathBuf>, String> {
    if let Some(v) = std::env::var_os("CHROMIUM").filter(|v| !v.is_empty()) {
        let p = PathBuf::from(&v);
        let found = if p.components().count() > 1 { Some(p).filter(|p| p.is_file()) } else { in_path(&v.to_string_lossy()) };
        return found
            .map(Some)
            .ok_or_else(|| format!("CHROMIUM={}: no such browser (a path, or a name in the PATH)", v.to_string_lossy()));
    }
    if let Some(shell) = platform().map(|p| shell_exe(&shell_dir(), p)).filter(|p| p.is_file()) {
        return Ok(Some(shell));
    }
    Ok(installed().into_iter().find(|p| works(p)))
}

/// The browsers that may be installed, most likely first
fn installed() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = NAMES.iter().filter_map(|n| in_path(n)).collect();
    if cfg!(target_os = "macos") {
        let apps = [
            "Google Chrome.app/Contents/MacOS/Google Chrome",
            "Chromium.app/Contents/MacOS/Chromium",
            "Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "Brave Browser.app/Contents/MacOS/Brave Browser",
            "Vivaldi.app/Contents/MacOS/Vivaldi",
        ];
        let home = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Applications"));
        for dir in [Some(PathBuf::from("/Applications")), home].into_iter().flatten() {
            found.extend(apps.iter().map(|a| dir.join(a)));
        }
    }
    if cfg!(windows) {
        let exes = [
            r"Google\Chrome\Application\chrome.exe",
            r"Microsoft\Edge\Application\msedge.exe",
            r"BraveSoftware\Brave-Browser\Application\brave.exe",
            r"Chromium\Application\chrome.exe",
            r"Vivaldi\Application\vivaldi.exe",
        ];
        for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(dir) = std::env::var_os(var) {
                found.extend(exes.iter().map(|e| PathBuf::from(&dir).join(e)));
            }
        }
    }
    if cfg!(target_os = "linux") {
        found.push("/snap/bin/chromium".into());
    }
    found.retain(|p| p.is_file());
    found.dedup();
    found
}

/// A name in the PATH (`.exe` added on Windows)
fn in_path(name: &str) -> Option<PathBuf> {
    let file = if cfg!(windows) && !name.ends_with(".exe") { format!("{name}.exe") } else { name.to_string() };
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(&file)).find(|p| p.is_file())
}

/// The browser answers `--version` within 10 s (a stub left by a package manager does not).
/// On Windows, Chrome does not print its version: being there is enough
fn works(bin: &Path) -> bool {
    if cfg!(windows) {
        return true;
    }
    let Ok(mut child) = Command::new(bin).arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
    else {
        return false;
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out = String::new();
                if let Some(mut o) = child.stdout.take() {
                    let _ = std::io::Read::read_to_string(&mut o, &mut out);
                }
                return status.success() && out.chars().any(|c| c.is_ascii_digit());
            }
            Ok(None) if start.elapsed() < Duration::from_secs(10) => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                return false;
            }
        }
    }
}

/// What to do when no browser is found
fn help() -> String {
    format!(
        "no Chromium-based browser found (Chrome, Chromium, Edge, Brave or Vivaldi, in the PATH and their usual places).\n\
         \x20 - one is installed elsewhere: give its path, CHROMIUM=/path/to/chrome glitu export …\n\
         \x20 - otherwise: glitu export --download-browser … downloads Chrome's headless shell {SHELL_VERSION}\n\
         \x20   (about 100 MB, once, after a confirmation; -y skips it) into {} (GLITU_BROWSER_DIR to choose)",
        shell_dir().display()
    )
}

/// Where the headless shell is kept: `GLITU_BROWSER_DIR`, otherwise glitu's cache
pub fn shell_dir() -> PathBuf {
    std::env::var_os("GLITU_BROWSER_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::online::cache_dir().join("browser"))
}

/// The platform's name at Chrome for Testing
fn platform() -> Option<&'static str> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", "aarch64") => "mac-arm64",
        ("macos", "x86_64") => "mac-x64",
        ("windows", "x86_64") => "win64",
        ("windows", "x86") => "win32",
        _ => return None,
    })
}

fn shell_exe(dir: &Path, platform: &str) -> PathBuf {
    let exe = if platform.starts_with("win") { "chrome-headless-shell.exe" } else { "chrome-headless-shell" };
    dir.join(SHELL_VERSION).join(format!("chrome-headless-shell-{platform}")).join(exe)
}

fn shell_url(platform: &str) -> String {
    format!(
        "https://storage.googleapis.com/chrome-for-testing-public/{SHELL_VERSION}/{platform}/chrome-headless-shell-{platform}.zip"
    )
}

/// Says what the download does, asks (unless `yes`), downloads, checks, unpacks
fn fetch_shell(yes: bool) -> Result<PathBuf, String> {
    let platform = platform().ok_or_else(|| {
        format!(
            "no headless shell for this platform ({} {}): install Chromium, or give a browser with CHROMIUM",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let (dir, url) = (shell_dir(), shell_url(platform));
    let dest = dir.join(SHELL_VERSION);
    eprintln!(
        "No Chromium-based browser found. --download-browser will:\n\
         \x20 1. download Chrome's headless shell {SHELL_VERSION} for {platform} (about 100 MB), Google's \"Chrome for\n\
         \x20    Testing\" build: a Chrome without a window, made for automation\n\
         \x20    {url}\n\
         \x20 2. check it against the SHA-256 written in glitu\n\
         \x20 3. unpack it into {} (GLITU_BROWSER_DIR to choose another folder)\n\
         \x20 4. run it, without a window, to render the slides; the next exports reuse it.\n\
         \x20 To remove it: delete that folder.",
        dest.display()
    );
    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err("confirmation needed and no terminal to ask it: add -y to accept the download".into());
        }
        eprint!("Download it? [y/N] ");
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer).map_err(|e| e.to_string())?;
        if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes" | "o" | "oui") {
            return Err("download cancelled".into());
        }
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let archive = dir.join(format!(".chrome-headless-shell-{SHELL_VERSION}.zip"));
    let status = Command::new("curl")
        .args(["-fL", "--progress-bar", "--max-time", "900", "-o"])
        .arg(&archive)
        .arg(&url)
        .status()
        .map_err(|e| format!("curl: {e} (curl is needed to download)"))?;
    let bytes = std::fs::read(&archive).unwrap_or_default();
    let _ = std::fs::remove_file(&archive);
    if !status.success() {
        return Err(format!("{url}: download failed"));
    }
    let expected = SHELLS.iter().find(|(p, _)| *p == platform).map(|(_, h)| *h).unwrap_or_default();
    let sum = format!("{:x}", Sha256::digest(&bytes));
    if sum != expected {
        return Err(format!("{url}: SHA-256 {sum}, expected {expected}: the archive is refused"));
    }
    let partial = dir.join(format!(".{SHELL_VERSION}.partial"));
    let _ = std::fs::remove_dir_all(&partial);
    crate::zip::unpack(&bytes, &partial)?;
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::rename(&partial, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let exe = shell_exe(&dir, platform);
    if !exe.is_file() {
        return Err(format!("{}: missing from the archive", exe.display()));
    }
    eprintln!("headless shell ready: {}", exe.display());
    Ok(exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_platform_has_its_checksum() {
        for (p, sum) in SHELLS {
            assert_eq!(sum.len(), 64, "{p}");
            assert!(shell_url(p).ends_with(&format!("/{p}/chrome-headless-shell-{p}.zip")));
        }
        if let Some(p) = platform() {
            assert!(SHELLS.iter().any(|(q, _)| *q == p));
            assert!(shell_exe(Path::new("x"), p).starts_with(Path::new("x").join(SHELL_VERSION)));
        }
        assert!(help().contains("CHROMIUM=") && help().contains("--download-browser") && help().contains("GLITU_BROWSER_DIR"));
    }
}
