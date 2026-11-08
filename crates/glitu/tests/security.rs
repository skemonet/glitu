//! The hostile presentations of `tests/security/` (see its README): each build must fail with the
//! message of its `expect.txt`, and the secret never reaches a page.
//!
//! `expect.txt`: its first line, `success` or a part of the error; the next ones, `!TEXT`: a text
//! that must not reach the page (`!ESC`: no terminal escape in the error).
//!
//! A case may hold `args.txt`: more arguments for `glitu build`, one per line; `env.txt`: variables
//! of its environment, `NAME=VALUE` (`{PATH}`: the current `PATH`); `config.yaml`: the user's
//! configuration of that build (`{tmp}` in all three: the copy's folder). A file `unix-only`,
//! `landlock-only` or `needs-skemo` limits where it runs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SECRET: &str = "GLITU-SECRET-7f3a";

struct Temp(PathBuf);

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// The cases, copied into a temporary folder (with their symbolic links)
fn cases(name: &str) -> Temp {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/security");
    let tmp = Temp(std::env::temp_dir().join(format!("glitu-security-{name}-{}", std::process::id())));
    let _ = fs::remove_dir_all(&tmp.0);
    copy(&src, &tmp.0);
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink("../secret.txt", tmp.0.join("import-link/innocent.txt")).unwrap();
        fs::create_dir_all(tmp.0.join("link-in-img/img")).unwrap();
        symlink("../../secret.txt", tmp.0.join("link-in-img/img/innocent.png")).unwrap();
    }
    tmp
}

/// `glitu build` of a case: (success, standard error, the page if written)
fn build(root: &Path, case: &str, extra: &[String]) -> (bool, String, Option<String>) {
    let dir = root.join(case);
    let tmp = root.display().to_string();
    let config = root.join(format!("{case}.config"));
    fs::create_dir_all(&config).unwrap();
    if let Ok(text) = fs::read_to_string(dir.join("config.yaml")) {
        fs::write(config.join("config.yaml"), text.replace("{tmp}", &tmp)).unwrap();
    }
    let out = root.join(format!("{case}.html"));
    let lines = |name: &str| -> Vec<String> {
        let path = std::env::var("PATH").unwrap_or_default();
        fs::read_to_string(dir.join(name))
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.replace("{tmp}", &tmp).replace("{PATH}", &path))
            .collect()
    };
    let mut args = lines("args.txt");
    args.extend(extra.iter().cloned());
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_glitu"));
    cmd.arg("build").arg(dir.join("slides.md")).arg("-o").arg(&out).args(&args).env("GLITU_CONFIG_DIR", &config);
    let keep_skemo = dir.join("needs-skemo").exists();
    for (k, _) in std::env::vars() {
        if k.starts_with("GLITU_TOOL_") && !(keep_skemo && k == "GLITU_TOOL_SKEMO") {
            cmd.env_remove(k);
        }
    }
    for line in lines("env.txt") {
        let (k, v) = line.split_once('=').unwrap();
        cmd.env(k, v);
    }
    let run = cmd.output().unwrap();
    (run.status.success(), String::from_utf8_lossy(&run.stderr).into_owned(), fs::read_to_string(&out).ok())
}

/// Can this case run here?
fn runs_here(dir: &Path) -> bool {
    let landlock = Path::new("/sys/kernel/security/landlock").exists()
        || fs::read_to_string("/sys/kernel/security/lsm").is_ok_and(|l| l.contains("landlock"));
    let skemo = std::env::var_os("GLITU_TOOL_SKEMO").is_some()
        || std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).any(|d| d.join("skemo").is_file());
    let required =
        std::env::var("GLITU_REQUIRE_TOOLS").is_ok_and(|r| r == "1" || r.split(',').any(|t| t.trim() == "skemo"));
    !(dir.join("unix-only").exists() && cfg!(not(unix))
        || dir.join("landlock-only").exists() && !(cfg!(target_os = "linux") && landlock)
        || dir.join("needs-skemo").exists() && !skemo && !required)
}

#[test]
fn hostile_presentations_are_refused() {
    let tmp = cases("all");
    let mut names: Vec<String> = fs::read_dir(&tmp.0)
        .unwrap()
        .flatten()
        .filter(|e| e.path().join("expect.txt").is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(names.len() >= 10, "{names:?}");
    let mut failures = Vec::new();
    for case in &names {
        if cfg!(not(unix)) && case.contains("link") || !runs_here(&tmp.0.join(case)) {
            continue;
        }
        let expect = fs::read_to_string(tmp.0.join(case).join("expect.txt")).unwrap();
        let mut lines = expect.lines();
        let first = lines.next().unwrap_or("").trim();
        let (ok, stderr, page) = build(&tmp.0, case, &[]);
        if first == "success" {
            if !ok {
                failures.push(format!("{case}: expected a success, got:\n{stderr}"));
            }
        } else if ok || !stderr.contains(first) {
            failures.push(format!("{case}: success {ok}, expected `{first}`, got:\n{stderr}"));
        }
        let page = page.unwrap_or_default();
        // the page without the copy of its markdown (which holds the source as written)
        let shown = page.split("id=\"glitu-source\"").next().unwrap_or("");
        for bad in lines.filter_map(|l| l.strip_prefix('!')) {
            if bad == "ESC" && stderr.contains('\u{1b}') {
                failures.push(format!("{case}: a terminal escape in the error: {stderr:?}"));
            } else if bad != "ESC" && shown.contains(bad) {
                failures.push(format!("{case}: `{bad}` reached the page"));
            }
        }
        if page.contains(SECRET) {
            failures.push(format!("{case}: the secret reached the page"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn the_user_can_allow_a_folder() {
    let tmp = cases("allow");
    let (ok, stderr, page) = build(&tmp.0, "include-up", &["--allow-read".into(), tmp.0.display().to_string()]);
    assert!(ok, "{stderr}");
    assert!(page.unwrap().contains(SECRET));
    // the same, from the user's configuration
    fs::write(tmp.0.join("include-up/config.yaml"), "files:\n  allow: ['{tmp}']\n").unwrap();
    let (ok, stderr, _) = build(&tmp.0, "include-up", &[]);
    assert!(ok, "{stderr}");
}
