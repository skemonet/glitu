//! Which browser renders the slides for the export (and the oracle): a Chromium-based one,
//! driven through the DevTools protocol.
//!
//! In this order: the `CHROMIUM` variable; the headless shell already downloaded by glitu; a
//! browser installed on the machine (Chrome, Chromium, Edge, Brave, Vivaldi: in the PATH, then in
//! their usual places). Without any, `glitu build <format> --download-browser` downloads Chrome's
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
        let found =
            if p.components().count() > 1 { Some(p).filter(|p| p.is_file()) } else { in_path(&v.to_string_lossy()) };
        return found.map(Some).ok_or_else(|| {
            format!("CHROMIUM={}: no such browser (a path, or a name in the PATH)", v.to_string_lossy())
        });
    }
    if let Some(shell) = platform().map(|p| shell_exe(&shell_dir(), p)).filter(|p| p.is_file()) {
        let missing = missing_for(&shell);
        if missing.is_empty() {
            return Ok(Some(shell));
        }
        // the shell cannot run: an installed browser will do, otherwise say what to install
        return match installed().into_iter().find(|p| works(p)) {
            Some(found) => Ok(Some(found)),
            None => Err(format!(
                "Chrome's headless shell, downloaded into {}, cannot run on this system:\n\
                 {}\n\
                 Then run your command again (the shell is already downloaded):\n\
                 \x20   {}",
                shell_dir().display(),
                libraries_help(&missing),
                rerun(None)
            )),
        };
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
    in_dirs(name, std::env::split_paths(&std::env::var_os("PATH")?).collect())
}

/// A name in the absolute folders among `dirs`: a relative one (`.`, an empty entry) would mean
/// the current folder, the presentation's
fn in_dirs(name: &str, dirs: Vec<PathBuf>) -> Option<PathBuf> {
    let file = if cfg!(windows) && !name.ends_with(".exe") { format!("{name}.exe") } else { name.to_string() };
    dirs.into_iter().filter(|d| d.is_absolute()).map(|d| d.join(&file)).find(|p| p.is_file())
}

/// The browser answers `--version` within 10 s (a stub left by a package manager does not).
/// On Windows, Chrome does not print its version: being there is enough
fn works(bin: &Path) -> bool {
    if cfg!(windows) {
        return true;
    }
    let Ok(mut child) =
        Command::new(bin).arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
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
         \x20 - one is installed elsewhere: give its path,\n\
         \x20       CHROMIUM=/path/to/chrome {}\n\
         \x20 - otherwise, download Chrome's headless shell {SHELL_VERSION} (about 100 MB, once, after a\n\
         \x20   confirmation; -y skips it) into {} (GLITU_BROWSER_DIR to choose):\n\
         \x20       {}",
        rerun(None),
        shell_dir().display(),
        rerun(Some("--download-browser"))
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
            "no headless shell is available for this platform ({} {}); install Chromium, or set CHROMIUM to a browser",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let (dir, url) = (shell_dir(), shell_url(platform));
    let dest = dir.join(SHELL_VERSION);
    // before 100 MB: the libraries the shell needs from the system
    let missing = missing_for_shell();
    if !missing.is_empty() {
        return Err(format!(
            "no Chromium-based browser was found, and Chrome's headless shell could not run on this system:\n\
             {}\n\
             Nothing was downloaded. Once they are installed, run your command again:\n\
             \x20   {}",
            libraries_help(&missing),
            rerun(None)
        ));
    }
    eprintln!(
        "No Chromium-based browser was found. glitu will:\n\
         \x20 1. Download Chrome's headless shell {SHELL_VERSION} for {platform} (about 100 MB), from Google's\n\
         \x20    \"Chrome for Testing\" builds:\n\
         \x20    {url}\n\
         \x20 2. Verify its SHA-256 checksum\n\
         \x20 3. Install it into {} (set GLITU_BROWSER_DIR to change this)\n\
         \x20 4. Reuse it for later exports. To uninstall it, delete that directory.",
        dest.display()
    );
    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err("the download requires confirmation, but no terminal is available; use -y to accept it".into());
        }
        eprint!("Proceed? [y/N] ");
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
        .map_err(|e| format!("cannot run curl, which the download requires: {e}"))?;
    let bytes = std::fs::read(&archive).unwrap_or_default();
    let _ = std::fs::remove_file(&archive);
    if !status.success() {
        return Err(format!("download failed: {url}"));
    }
    let expected = SHELLS.iter().find(|(p, _)| *p == platform).map(|(_, h)| *h).unwrap_or_default();
    let sum: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    if sum != expected {
        return Err(format!(
            "checksum mismatch for {url} (SHA-256 {sum}, expected {expected}); the archive was discarded"
        ));
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
    eprintln!("Installed {}", exe.display());
    Ok(exe)
}

// ---- What the browser needs from the system (Linux) ----

/// The libraries the Linux headless shell loads (its `NEEDED` entries, glibc's left out), and the
/// package that provides each on Debian/Ubuntu and on Arch. Fedora and openSUSE install a library
/// by its own name.
const SHELL_LIBS: &[(&str, &str, &str)] = &[
    ("libglib-2.0.so.0", "libglib2.0-0", "glib2"),
    ("libgobject-2.0.so.0", "libglib2.0-0", "glib2"),
    ("libgio-2.0.so.0", "libglib2.0-0", "glib2"),
    ("libnspr4.so", "libnspr4", "nspr"),
    ("libnss3.so", "libnss3", "nss"),
    ("libnssutil3.so", "libnss3", "nss"),
    ("libatk-1.0.so.0", "libatk1.0-0", "at-spi2-core"),
    ("libatk-bridge-2.0.so.0", "libatk-bridge2.0-0", "at-spi2-core"),
    ("libatspi.so.0", "libatspi2.0-0", "at-spi2-core"),
    ("libdbus-1.so.3", "libdbus-1-3", "dbus"),
    ("libX11.so.6", "libx11-6", "libx11"),
    ("libXcomposite.so.1", "libxcomposite1", "libxcomposite"),
    ("libXdamage.so.1", "libxdamage1", "libxdamage"),
    ("libXext.so.6", "libxext6", "libxext"),
    ("libXfixes.so.3", "libxfixes3", "libxfixes"),
    ("libXrandr.so.2", "libxrandr2", "libxrandr"),
    ("libgbm.so.1", "libgbm1", "mesa"),
    ("libexpat.so.1", "libexpat1", "expat"),
    ("libxcb.so.1", "libxcb1", "libxcb"),
    ("libxkbcommon.so.0", "libxkbcommon0", "libxkbcommon"),
    ("libudev.so.1", "libudev1", "systemd-libs"),
    ("libasound.so.2", "libasound2", "alsa-lib"),
];

/// The Debian packages above renamed `…t64` by the 64-bit time transition (Debian 13, Ubuntu 24.04)
const T64: [&str; 5] = ["libglib2.0-0", "libatk1.0-0", "libatk-bridge2.0-0", "libatspi2.0-0", "libasound2"];

/// The libraries of `SHELL_LIBS` this system lacks (none when it cannot be told, or off Linux)
fn missing_for_shell() -> Vec<String> {
    if !cfg!(target_os = "linux") {
        return Vec::new();
    }
    let Some(present) = system_libs() else { return Vec::new() };
    SHELL_LIBS.iter().filter(|(lib, ..)| !present.contains(*lib)).map(|(lib, ..)| lib.to_string()).collect()
}

/// The libraries `bin` needs that the system lacks, according to `ldd` (none without it, or off Linux)
pub fn missing_for(bin: &Path) -> Vec<String> {
    if !cfg!(target_os = "linux") {
        return Vec::new();
    }
    let Ok(out) = Command::new("ldd").arg(bin).stderr(Stdio::null()).output() else { return Vec::new() };
    parse_ldd(&String::from_utf8_lossy(&out.stdout))
}

/// `\tlibnss3.so => not found` → `libnss3.so`
fn parse_ldd(out: &str) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    for lib in out.lines().filter_map(|l| l.trim().strip_suffix("=> not found")).map(str::trim) {
        if !missing.iter().any(|m| m == lib) {
            missing.push(lib.to_string());
        }
    }
    missing
}

/// The libraries the dynamic loader finds: its cache (`ldconfig -p`), otherwise the usual folders;
/// `None` if neither shows the C library
fn system_libs() -> Option<std::collections::HashSet<String>> {
    let tag = match std::env::consts::ARCH {
        "x86_64" => "x86-64",
        "aarch64" => "AArch64",
        _ => return None,
    };
    let extra: Vec<PathBuf> =
        std::env::var_os("LD_LIBRARY_PATH").map(|v| std::env::split_paths(&v).collect()).unwrap_or_default();
    let scan = |dirs: &[PathBuf], libs: &mut std::collections::HashSet<String>| {
        for dir in dirs {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                libs.insert(e.file_name().to_string_lossy().into_owned());
            }
        }
    };
    for ldconfig in ["ldconfig", "/sbin/ldconfig", "/usr/sbin/ldconfig"] {
        let Ok(out) = Command::new(ldconfig).arg("-p").stderr(Stdio::null()).output() else { continue };
        let mut libs = parse_ldconfig(&String::from_utf8_lossy(&out.stdout), tag);
        if libs.contains("libc.so.6") {
            scan(&extra, &mut libs);
            return Some(libs);
        }
    }
    let triplet = format!("{}-linux-gnu", std::env::consts::ARCH);
    let mut dirs = extra;
    for d in ["/lib64", "/usr/lib64", "/lib", "/usr/lib"] {
        dirs.push(PathBuf::from(d));
        dirs.push(Path::new(d).join(&triplet));
    }
    let mut libs = std::collections::HashSet::new();
    scan(&dirs, &mut libs);
    libs.contains("libc.so.6").then_some(libs)
}

/// The libraries of `ldconfig -p` for this architecture (`tag`: `x86-64`, `AArch64`):
/// `\tlibnss3.so (libc6,x86-64) => /lib/x86_64-linux-gnu/libnss3.so`
fn parse_ldconfig(out: &str, tag: &str) -> std::collections::HashSet<String> {
    out.lines()
        .filter_map(|l| {
            let (name, rest) = l.trim().split_once(" (")?;
            let flags = rest.split_once(')')?.0;
            flags.split(',').any(|f| f.trim() == tag).then(|| name.to_string())
        })
        .collect()
}

/// The package managers glitu knows how to speak to
#[derive(Clone, Copy, Debug, PartialEq)]
enum Manager {
    /// Debian, Ubuntu and their derivatives; `t64`: the packages' names since the 64-bit time transition
    Apt {
        t64: bool,
    },
    Dnf,
    Zypper,
    Pacman,
}

fn manager() -> Option<Manager> {
    if in_path("apt-get").is_some() {
        let os = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let known = |p: &str| {
            Command::new("apt-cache")
                .args(["show", "--no-all-versions", p])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
        };
        return Some(Manager::Apt { t64: t64_release(&os) || known("libglib2.0-0t64") });
    }
    [("dnf", Manager::Dnf), ("zypper", Manager::Zypper), ("pacman", Manager::Pacman)]
        .into_iter()
        .find(|(bin, _)| in_path(bin).is_some())
        .map(|(_, m)| m)
}

/// Debian 13 and later, Ubuntu 24.04 and later (from `/etc/os-release`)
fn t64_release(os_release: &str) -> bool {
    let field =
        |k: &str| os_release.lines().find_map(|l| l.strip_prefix(k)?.strip_prefix('=')).map(|v| v.trim_matches('"'));
    let major = field("VERSION_ID").and_then(|v| v.split('.').next()?.parse::<u32>().ok());
    match field("ID") {
        Some("debian") => major.is_some_and(|v| v >= 13),
        Some("ubuntu") => major.is_some_and(|v| v >= 24),
        _ => false,
    }
}

/// The command that installs the `missing` libraries, and those it does not know the package of
fn install_command(m: Manager, missing: &[String]) -> (String, Vec<String>) {
    let (mut packages, mut unknown): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for lib in missing {
        let known = SHELL_LIBS.iter().find(|(l, ..)| l == lib);
        let package = match (m, known) {
            (Manager::Dnf | Manager::Zypper, _) => format!("'{lib}()(64bit)'"),
            (Manager::Apt { t64 }, Some((_, deb, _))) if t64 && T64.contains(deb) => format!("{deb}t64"),
            (Manager::Apt { .. }, Some((_, deb, _))) => deb.to_string(),
            (Manager::Pacman, Some((.., arch))) => arch.to_string(),
            (_, None) => {
                unknown.push(lib.clone());
                continue;
            }
        };
        if !packages.contains(&package) {
            packages.push(package);
        }
    }
    let head = match m {
        Manager::Apt { .. } => "sudo apt install",
        Manager::Dnf => "sudo dnf install",
        Manager::Zypper => "sudo zypper install",
        Manager::Pacman => "sudo pacman -S --needed",
    };
    (if packages.is_empty() { String::new() } else { format!("{head} {}", packages.join(" ")) }, unknown)
}

/// What to do about `missing` libraries: which ones, the command that installs them, the other way out
pub fn libraries_help(missing: &[String]) -> String {
    let mut s = format!(
        "  It needs {} missing from the system: {}.\n\n\
         What to do:\n",
        if missing.len() == 1 { "a library".to_string() } else { format!("{} libraries", missing.len()) },
        missing.join(", ")
    );
    match manager() {
        Some(m) => {
            let (command, unknown) = install_command(m, missing);
            if !command.is_empty() {
                s += &format!("  1. Install them (administrator rights needed):\n         {command}\n");
            }
            if !unknown.is_empty() {
                let search = match m {
                    Manager::Apt { .. } => "apt-file search",
                    Manager::Dnf => "dnf provides",
                    Manager::Zypper => "zypper search --provides",
                    Manager::Pacman => "pacman -F",
                };
                s += &format!(
                    "  {}. Find the package that provides {}, then install it:\n         {search} {}\n",
                    if command.is_empty() { 1 } else { 2 },
                    if unknown.len() == 1 { "it" } else { "each of these" },
                    unknown.join(" ")
                );
            }
        }
        None => s += "  1. Install the packages that provide them, with your distribution's package manager\n",
    }
    s += &format!(
        "  Or, without installing anything: point glitu to a Chromium-based browser that already\n\
         \x20 works on this machine (Chrome, Chromium, Edge, Brave, Vivaldi):\n\
         \x20      CHROMIUM=/path/to/chrome {}\n",
        rerun(None)
    );
    s
}

/// The command being run, to show again (`add`: an option to give it, at the end)
pub fn rerun(add: Option<&str>) -> String {
    let mut args: Vec<String> = std::env::args().collect();
    if let Some(a) = add.filter(|a| !args.iter().any(|x| x == a)) {
        args.push(a.to_string());
    }
    if let Some(first) = args.first_mut() {
        *first = Path::new(first.as_str()).file_name().map_or(first.clone(), |n| n.to_string_lossy().into_owned());
    }
    args.iter()
        .map(|a| {
            let safe = !a.is_empty() && a.chars().all(|c| c.is_ascii_alphanumeric() || "-_./=:,@%+".contains(c));
            if safe { a.clone() } else { format!("'{}'", a.replace('\'', r"'\''")) }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Why the browser `bin` did not start, and what to do: `exit`, its status if it stopped (otherwise
/// it did not answer in time), `stderr` what it wrote
pub fn launch_failure(bin: &Path, exit: Option<String>, stderr: &str) -> String {
    let mut s = format!("the browser did not start: {}\n", bin.display());
    s += &match &exit {
        Some(status) => format!("  It stopped at once ({status}).\n"),
        None => "  It did not answer on its DevTools port within 10 seconds.\n".into(),
    };
    let lines: Vec<&str> = stderr.lines().map(str::trim_end).filter(|l| !l.trim().is_empty()).collect();
    if !lines.is_empty() {
        s += "  Its last messages:\n";
        for l in &lines[lines.len().saturating_sub(8)..] {
            s += &format!("    | {l}\n");
        }
    }
    let downloaded = bin.starts_with(shell_dir());
    let missing = missing_for(bin);
    if !missing.is_empty() {
        s += &libraries_help(&missing);
        s += &format!("Then run your command again:\n    {}", rerun(None));
        return s;
    }
    s += "\nWhat to do:\n";
    if exit.is_none() {
        s += "  - On a busy or slow machine, close other programs and run your command again.\n";
    }
    s += &format!(
        "  - See what the browser says when run alone:\n\
         \x20      '{}' --headless --no-sandbox --dump-dom about:blank\n",
        bin.display()
    );
    if downloaded {
        s += &format!(
            "  - Download the headless shell again: delete {}, then\n\
             \x20      {}\n",
            shell_dir().join(SHELL_VERSION).display(),
            rerun(Some("--download-browser"))
        );
    }
    s += &format!(
        "  - Use another Chromium-based browser (Chrome, Chromium, Edge, Brave, Vivaldi):\n\
         \x20      CHROMIUM=/path/to/chrome {}",
        rerun(None)
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As for the diagram tools (SEC-3): a relative entry of the PATH (`.`, an empty one) would
    /// run a `chromium` lying in the presentation's folder
    #[test]
    fn only_the_absolute_folders_of_the_path_are_searched() {
        let dir = std::env::temp_dir().join(format!("glitu-path-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tool"), "").unwrap();
        let dirs = |d: &[&Path]| d.iter().map(|p| p.to_path_buf()).collect::<Vec<_>>();
        assert_eq!(in_dirs("tool", dirs(&[&dir])), Some(dir.join("tool")));
        let relative = Path::new("src");
        assert!(relative.join("lib.rs").is_file());
        assert_eq!(in_dirs("lib.rs", dirs(&[relative, Path::new(""), Path::new(".")])), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

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
        assert!(
            help().contains("CHROMIUM=")
                && help().contains("--download-browser")
                && help().contains("GLITU_BROWSER_DIR")
        );
    }

    #[test]
    fn missing_libraries_are_read_and_named_by_package() {
        let ldd = "\tlinux-vdso.so.1 (0x00007ffd)\n\tlibnspr4.so => not found\n\tlibnss3.so => not found\n\
                   \tlibnspr4.so => not found\n\tlibc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x7f)\n";
        assert_eq!(parse_ldd(ldd), ["libnspr4.so", "libnss3.so"]);
        let cache = "1234 libs found in cache `/etc/ld.so.cache'\n\
                     \tlibnss3.so (libc6,x86-64) => /lib/x86_64-linux-gnu/libnss3.so\n\
                     \tlibc.so.6 (libc6,x86-64, OS ABI: Linux 3.2.0) => /lib/x86_64-linux-gnu/libc.so.6\n\
                     \tlibnspr4.so (libc6) => /lib/i386-linux-gnu/libnspr4.so\n";
        let libs = parse_ldconfig(cache, "x86-64");
        assert!(libs.contains("libnss3.so") && libs.contains("libc.so.6") && !libs.contains("libnspr4.so"));

        let missing: Vec<String> =
            ["libnspr4.so", "libnss3.so", "libnssutil3.so", "libasound.so.2", "libfoo.so.1"].map(String::from).into();
        let (apt, unknown) = install_command(Manager::Apt { t64: true }, &missing);
        assert_eq!(apt, "sudo apt install libnspr4 libnss3 libasound2t64");
        assert_eq!(unknown, ["libfoo.so.1"]);
        assert_eq!(install_command(Manager::Apt { t64: false }, &missing[3..4]).0, "sudo apt install libasound2");
        assert_eq!(install_command(Manager::Pacman, &missing[..3]).0, "sudo pacman -S --needed nspr nss");
        assert!(install_command(Manager::Dnf, &missing).0.ends_with("'libfoo.so.1()(64bit)'"));

        assert!(t64_release("PRETTY_NAME=\"Debian GNU/Linux 13 (trixie)\"\nID=debian\nVERSION_ID=\"13\"\n"));
        assert!(t64_release("ID=ubuntu\nID_LIKE=debian\nVERSION_ID=\"24.04\"\n"));
        assert!(!t64_release("ID=ubuntu\nVERSION_ID=\"22.04\"\n") && !t64_release("ID=debian\nVERSION_ID=\"12\"\n"));
    }

    #[test]
    fn every_library_of_the_shell_has_its_packages() {
        assert!(T64.iter().all(|p| SHELL_LIBS.iter().any(|(_, deb, _)| deb == p)));
        assert!(
            SHELL_LIBS.iter().all(|(lib, deb, arch)| lib.starts_with("lib") && !deb.is_empty() && !arch.is_empty())
        );
    }
}
