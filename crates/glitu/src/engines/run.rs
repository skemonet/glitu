//! Running a tool (SEC-4, SEC-8 to SEC-10 of `docs/design/diagram-engines.md`): a vector of
//! arguments, never a shell; in the presentation's root folder; its files in a private temporary
//! folder removed afterwards; a minimal environment; a time limit after which it is killed with its
//! children; bounded outputs; confined by the system when it can (see `sandbox`).

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::sandbox::Rules;

/// At most this much SVG read (SEC-9)
pub const MAX_OUTPUT: usize = 20 * 1024 * 1024;
/// At most this much of the standard error
pub const MAX_ERRORS: usize = 1024 * 1024;

/// A private temporary folder (0700), removed when dropped
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new() -> Result<Scratch, String> {
        static N: AtomicU64 = AtomicU64::new(0);
        // unpredictable: another user of the machine cannot prepare it
        let salt = {
            use std::hash::{BuildHasher, RandomState};
            RandomState::new().hash_one((std::process::id(), N.fetch_add(1, Ordering::Relaxed)))
        };
        let dir = std::env::temp_dir().join(format!("glitu-tool-{}-{salt:016x}", std::process::id()));
        let builder = {
            #[allow(unused_mut)]
            let mut b = fs::DirBuilder::new();
            #[cfg(unix)]
            std::os::unix::fs::DirBuilderExt::mode(&mut b, 0o700);
            b
        };
        builder.create(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Scratch(dir))
    }

    /// Writes `name` (a plain name, checked by the manifest) into it
    pub fn put(&self, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        let p = self.0.join(name);
        fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))?;
        Ok(p)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A tool to start
pub struct Job<'a> {
    pub tool: &'a Path,
    pub args: Vec<String>,
    /// The folder it runs in: the presentation's root
    pub cwd: &'a Path,
    pub stdin: Option<&'a [u8]>,
    pub scratch: &'a Scratch,
    /// Variables of the user's environment it gets, besides the minimal ones
    pub env: &'a [String],
    pub set_env: &'a BTreeMap<String, String>,
    pub timeout: Duration,
    /// It runs a browser (see `sandbox`)
    pub browser: bool,
}

/// What it said
pub struct Ran {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: String,
    /// What confined it: `landlock`, `seatbelt`, `none`
    pub sandbox: &'static str,
}

/// The variables every tool gets (SEC-10)
const KEPT: [&str; 4] = ["PATH", "HOME", "SYSTEMROOT", "USERPROFILE"];

pub fn run(job: &Job) -> Result<Ran, String> {
    let rules = Rules::new(job.cwd, job.tool, &job.scratch.0, job.browser);
    let (program, args, sandbox) = rules.wrap(job.tool, &job.args);
    let mut cmd = Command::new(&program);
    cmd.args(&args)
        .current_dir(job.cwd)
        .env_clear()
        .stdin(if job.stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in KEPT.iter().copied().chain(job.env.iter().map(String::as_str)) {
        if let Some(v) = std::env::var_os(name) {
            cmd.env(name, v);
        }
    }
    for t in ["TMPDIR", "TMP", "TEMP"] {
        cmd.env(t, &job.scratch.0);
    }
    cmd.envs(job.set_env);
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = cmd.spawn().map_err(|e| format!("{}: {e}", job.tool.display()))?;
    if let Some(input) = job.stdin {
        let mut pipe = child.stdin.take().ok_or("no standard input")?;
        let input = input.to_vec();
        // written apart: a tool that writes before reading everything would block us otherwise
        std::thread::spawn(move || {
            let _ = pipe.write_all(&input);
        });
    }
    let out = reader(child.stdout.take(), MAX_OUTPUT);
    let err = reader(child.stderr.take(), MAX_ERRORS);
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if start.elapsed() > job.timeout {
            kill(&mut child);
            return Err(format!(
                "{}: no answer after {} s, stopped (limits.timeout in the user's configuration)",
                job.tool.display(),
                job.timeout.as_secs_f64()
            ));
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let (stdout, over) = out.join().unwrap_or_default();
    let (stderr, _) = err.join().unwrap_or_default();
    if over {
        return Err(format!("{}: more than {} MB of output, refused", job.tool.display(), MAX_OUTPUT >> 20));
    }
    Ok(Ran { success: status.success(), stdout, stderr: String::from_utf8_lossy(&stderr).into_owned(), sandbox })
}

/// Reads a pipe in a thread, keeping `max` bytes at most (the rest is read and dropped, so that
/// the tool does not block): the bytes and whether there were more
fn reader(pipe: Option<impl Read + Send + 'static>, max: usize) -> std::thread::JoinHandle<(Vec<u8>, bool)> {
    std::thread::spawn(move || {
        let Some(mut pipe) = pipe else { return (Vec::new(), false) };
        let (mut kept, mut buf, mut over) = (Vec::new(), [0u8; 65536], false);
        while let Ok(n) = pipe.read(&mut buf) {
            if n == 0 {
                break;
            }
            let room = max.saturating_sub(kept.len());
            kept.extend_from_slice(&buf[..n.min(room)]);
            over |= n > room;
        }
        (kept, over)
    })
}

/// The tool and its children (no unsafe code in glitu: the system's own commands do it)
fn kill(child: &mut std::process::Child) {
    let quiet = |c: &mut Command| {
        let _ = c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    };
    // its process group (`process_group(0)`): the children it started too
    #[cfg(unix)]
    quiet(Command::new("kill").args(["-KILL", "--", &format!("-{}", child.id())]));
    // the tree of processes it started
    #[cfg(windows)]
    quiet(Command::new("taskkill").args(["/T", "/F", "/PID", &child.id().to_string()]));
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn job<'a>(args: &[&str], scratch: &'a Scratch, set: &'a BTreeMap<String, String>, timeout: f64) -> Job<'a> {
        Job {
            tool: Path::new("/bin/sh"),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: &scratch.0,
            stdin: Some(b"input"),
            scratch,
            env: &[],
            set_env: set,
            timeout: Duration::from_secs_f64(timeout),
            browser: false,
        }
    }

    #[test]
    fn a_tool_gets_its_input_a_small_environment_and_a_time_limit() {
        let scratch = Scratch::new().unwrap();
        let set = BTreeMap::from([("SKEMO_LANG".to_string(), "en".to_string())]);
        let r = run(&job(&["-c", "cat; echo \" $SKEMO_LANG ${GLITU_SECRET_TOKEN:-none}\" >&2"], &scratch, &set, 5.0))
            .unwrap();
        assert!(r.success);
        assert_eq!(r.stdout, b"input");
        assert_eq!(r.stderr.trim(), "en none");
        let e = run(&job(&["-c", "sleep 5 & sleep 5"], &scratch, &set, 0.3)).err().unwrap();
        assert!(e.contains("no answer after 0.3 s"), "{e}");
        assert_eq!(
            fs::metadata(&scratch.0)
                .map(|m| std::os::unix::fs::PermissionsExt::mode(&m.permissions()) & 0o777)
                .unwrap(),
            0o700
        );
    }
}
