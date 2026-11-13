//! The operating system's confinement of a tool (SEC-8), at best: what a tool may read and write
//! when a hostile diagram makes it misbehave.
//!
//! - Linux: Landlock (kernel 5.13 or later; the network from 6.7). glitu starts itself as
//!   `glitu __sandbox …`, which restricts itself then becomes the tool (`exec`): the rules hold for
//!   the tool and its children. Reading: the presentation's root, the tool's installation, the
//!   system's folders; writing: the private temporary folder; no TCP connection (unless the tool
//!   runs a browser).
//! - macOS: Seatbelt, through `/usr/bin/sandbox-exec`: the home folder unreadable but the root and
//!   the tool, writing only into the temporary folder, no network (unless browser).
//! - Windows: no confinement of the files (a Job Object or an AppContainer needs unsafe code, which
//!   glitu forbids); the tool and its children are killed together at the time limit (`taskkill
//!   /T`, see `run`).
//!
//! Without the feature (old kernel, no `sandbox-exec`), the tool runs unconfined; `-v` says so.

use std::path::{Path, PathBuf};

/// What a tool may reach
#[derive(Clone, Debug)]
pub struct Rules {
    pub read: Vec<PathBuf>,
    pub write: Vec<PathBuf>,
    /// The tool runs a browser: TCP to itself, its own sandbox
    pub network: bool,
}

/// The hidden command of glitu that confines itself then becomes the tool
pub const COMMAND: &str = "__sandbox";

/// The program that understands `COMMAND`: the glitu binary, once it said so (`enable`); a
/// library or a test that did not starts its tools unconfined on Linux
static LAUNCHER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Called by the glitu binary: its tools are confined through it
pub fn enable() {
    if let Ok(me) = std::env::current_exe() {
        let _ = LAUNCHER.set(me);
    }
}

/// The folders every program needs to read
#[cfg(target_os = "linux")]
const SYSTEM: [&str; 10] = ["/usr", "/lib", "/lib64", "/lib32", "/bin", "/sbin", "/etc", "/opt", "/nix", "/proc/self"];

/// What a tool reads of its installation: `prefix/bin/tool` → `prefix`, a tool elsewhere → its
/// folder; but never the whole disk nor a folder holding the home folder (`/bin/dot` → `/bin`,
/// `~/bin/tool` → `~/bin`, `~/tool` → the tool alone)
fn installation(tool: &Path, homes: &[PathBuf]) -> PathBuf {
    let wide = |p: &Path| p.parent().is_none() || homes.iter().any(|h| h.starts_with(p));
    let bin = tool.parent();
    let prefix = bin.filter(|b| b.ends_with("bin")).and_then(Path::parent);
    [prefix, bin].into_iter().flatten().find(|p| !wide(p)).unwrap_or(tool).to_path_buf()
}

impl Rules {
    /// `root`: the presentation's; `tool`: the program; `tmp`: the private temporary folder
    pub fn new(root: &Path, tool: &Path, tmp: &Path, browser: bool) -> Rules {
        let mut read = vec![root.to_path_buf(), tmp.to_path_buf()];
        // the tool's installation: `prefix/bin/tool` → `prefix`, links resolved (a tool linked from
        // ~/.local/bin into its own folder)
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let homes: Vec<PathBuf> =
            home.iter().flat_map(|h| [h.canonicalize().ok(), Some(h.clone())]).flatten().collect();
        for t in [tool.to_path_buf(), tool.canonicalize().unwrap_or_else(|_| tool.to_path_buf())] {
            read.push(installation(&t, &homes));
        }
        // a browser reads its own installation and profile, and fonts: the home folder (not
        // `~/.ssh`-proof, the price of running a browser)
        if browser && let Some(home) = std::env::var_os("HOME") {
            read.push(PathBuf::from(home));
        }
        // and the system's description of the machine (processors, memory, devices)
        if browser {
            read.extend(["/proc", "/sys", "/dev"].map(PathBuf::from));
        }
        let read = read.iter().filter_map(|p| p.canonicalize().ok()).collect();
        // a browser's own sandbox maps its users (`/proc/<pid>/uid_map`): the kernel's permissions
        // still apply there
        let mut write: Vec<PathBuf> = [tmp].iter().filter_map(|p| p.canonicalize().ok()).collect();
        if browser {
            write.push(PathBuf::from("/proc"));
        }
        Rules { read, write, network: browser }
    }

    /// The program and arguments that start `tool args` confined, and what confines it
    pub fn wrap(&self, tool: &Path, args: &[String]) -> (PathBuf, Vec<String>, &'static str) {
        let plain = (tool.to_path_buf(), args.to_vec(), "none");
        if std::env::var_os("GLITU_NO_SANDBOX").is_some_and(|v| !v.is_empty()) {
            return plain;
        }
        if cfg!(target_os = "linux") {
            let Some(me) = LAUNCHER.get().cloned() else { return plain };
            let mut a = vec![COMMAND.to_string()];
            for r in &self.read {
                a.extend(["--read".into(), r.display().to_string()]);
            }
            for w in &self.write {
                a.extend(["--write".into(), w.display().to_string()]);
            }
            if self.network {
                a.push("--network".into());
            }
            a.push("--".into());
            a.push(tool.display().to_string());
            a.extend(args.iter().cloned());
            return (me, a, "landlock");
        }
        if cfg!(target_os = "macos") && Path::new(SANDBOX_EXEC).is_file() {
            let mut a = vec!["-p".to_string(), self.seatbelt()];
            a.push(tool.display().to_string());
            a.extend(args.iter().cloned());
            return (PathBuf::from(SANDBOX_EXEC), a, "seatbelt");
        }
        plain
    }

    /// The Seatbelt profile: everything allowed but reading the home folder (the root and the tool
    /// excepted), writing (the temporary folder excepted) and the network
    fn seatbelt(&self) -> String {
        let q = |p: &Path| format!("\"{}\"", p.display().to_string().replace('\\', "\\\\").replace('"', "\\\""));
        let mut s = String::from("(version 1)\n(allow default)\n");
        if let Some(home) = std::env::var_os("HOME").and_then(|h| PathBuf::from(h).canonicalize().ok())
            && !self.network
        {
            s += &format!("(deny file-read* (subpath {}))\n", q(&home));
        }
        for r in &self.read {
            s += &format!("(allow file-read* (subpath {}))\n", q(r));
        }
        s += "(deny file-write*)\n(allow file-write* (literal \"/dev/null\") (literal \"/dev/stdout\") (literal \"/dev/stderr\"))\n";
        for w in &self.write {
            s += &format!("(allow file-write* (subpath {}))\n", q(w));
        }
        if !self.network {
            s += "(deny network*)\n";
        }
        s
    }
}

const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// `glitu __sandbox --read DIR… --write DIR… [--network] -- TOOL ARGS…`: confines this process,
/// then becomes the tool. Returns only on failure
pub fn exec(args: &[String]) -> String {
    let mut rules = Rules { read: Vec::new(), write: Vec::new(), network: false };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--read" if i + 1 < args.len() => {
                rules.read.push(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--write" if i + 1 < args.len() => {
                rules.write.push(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--network" => rules.network = true,
            "--" => break,
            other => return format!("{COMMAND}: unexpected `{other}`"),
        }
        i += 1;
    }
    let Some(tool) = args.get(i + 1) else { return format!("{COMMAND}: no program") };
    #[cfg(target_os = "linux")]
    if let Err(e) = landlock(&rules) {
        return format!("{COMMAND}: {e}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let e = std::process::Command::new(tool).args(&args[i + 2..]).exec();
        format!("{tool}: {e}")
    }
    #[cfg(not(unix))]
    {
        let _ = rules;
        format!("{COMMAND}: {tool}: not available on this system")
    }
}

/// Landlock, at best: a kernel without it leaves the tool unconfined
#[cfg(target_os = "linux")]
fn landlock(rules: &Rules) -> Result<(), String> {
    use landlock::{
        ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
        RulesetCreatedAttr,
    };
    let abi = ABI::V5;
    let err = |e: landlock::RulesetError| e.to_string();
    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(AccessFs::from_all(abi))
        .map_err(err)?;
    if !rules.network {
        ruleset = ruleset.handle_access(AccessNet::ConnectTcp | AccessNet::BindTcp).map_err(err)?;
    }
    let mut created = ruleset.create().map_err(err)?;
    let read: Vec<PathBuf> =
        rules.read.iter().cloned().chain(SYSTEM.iter().map(PathBuf::from)).filter(|p| p.exists()).collect();
    for p in &read {
        let fd = PathFd::new(p).map_err(|e| e.to_string())?;
        created = created.add_rule(PathBeneath::new(fd, AccessFs::from_read(abi))).map_err(err)?;
    }
    let devices = ["/dev/null", "/dev/zero", "/dev/urandom", "/dev/random", "/dev/tty", "/dev/shm"];
    for p in rules.write.iter().map(PathBuf::as_path).chain(devices.iter().map(Path::new)).filter(|p| p.exists()) {
        let fd = PathFd::new(p).map_err(|e| e.to_string())?;
        let access = if p.is_dir() { AccessFs::from_all(abi) } else { AccessFs::from_file(abi) };
        created = created.add_rule(PathBeneath::new(fd, access)).map_err(err)?;
    }
    created.restrict_self().map_err(err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules_name_the_root_the_tool_and_the_temporary_folder() {
        let tmp = std::env::temp_dir();
        let r = Rules::new(&tmp, Path::new("/usr/bin/sh"), &tmp, false);
        assert!(r.read.iter().any(|p| p.ends_with("usr") || p == Path::new("/")) || cfg!(not(unix)));
        assert!(!r.network);
        let home = [PathBuf::from("/home/me")];
        let at = |tool: &str| installation(Path::new(tool), &home);
        assert_eq!(at("/opt/graphviz/bin/dot"), Path::new("/opt/graphviz"));
        assert_eq!(at("/opt/tools/dot"), Path::new("/opt/tools"));
        assert_eq!(at("/home/me/.local/bin/d2"), Path::new("/home/me/.local"));
        // never the whole disk nor the whole home folder
        assert_eq!(at("/bin/dot"), Path::new("/bin"));
        assert_eq!(at("/home/me/bin/d2"), Path::new("/home/me/bin"));
        assert_eq!(at("/home/me/d2"), Path::new("/home/me/d2"));
        assert_eq!(at("/home/bin/d2"), Path::new("/home/bin"));
        let profile = Rules { read: vec!["/a \"b\"".into()], write: vec!["/t".into()], network: false }.seatbelt();
        assert!(
            profile.contains("(allow file-read* (subpath \"/a \\\"b\\\"\"))") && profile.contains("(deny network*)")
        );
    }
}
