//! The engines with their real tools (Graphviz's `dot`, D2's `d2`): skipped when a tool is not
//! installed, unless `GLITU_REQUIRE_TOOLS` names it (`dot,d2`; `1`: every tool).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn installed(tool: &str) -> bool {
    let found = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).any(|d| d.join(tool).is_file());
    let required =
        std::env::var("GLITU_REQUIRE_TOOLS").is_ok_and(|r| r == "1" || r.split(',').any(|t| t.trim() == tool));
    assert!(found || !required, "{tool} is required (GLITU_REQUIRE_TOOLS)");
    found
}

/// The page of a one-slide presentation holding `block`
fn build(name: &str, block: &str) -> String {
    let dir = std::env::temp_dir().join(format!("glitu-engines-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("config")).unwrap();
    fs::write(dir.join("slides.md"), format!("# Engines\n\n## {name}\n\n{block}\n")).unwrap();
    let out: PathBuf = dir.join("out.html");
    let run = Command::new(env!("CARGO_BIN_EXE_glitu"))
        .arg("build")
        .arg(dir.join("slides.md"))
        .arg("-o")
        .arg(&out)
        .env("GLITU_CONFIG_DIR", dir.join("config"))
        .output()
        .unwrap();
    let page = fs::read_to_string(&out).unwrap_or_default();
    let _ = fs::remove_dir_all(&dir);
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    // the page without the copy of its markdown
    page.split("id=\"glitu-source\"").next().unwrap_or("").to_string()
}

#[test]
fn graphviz_draws_with_its_options() {
    if !installed("dot") {
        return;
    }
    let page = build("dot", "```dot graphviz.layout=neato\ndigraph { alpha -> beta }\n```");
    assert!(page.contains("<div class=\"diagram\"><svg") && page.contains(">alpha</text>"), "{page}");
    // without the white background of dot
    assert!(!page.contains("fill=\"white\" stroke=\"transparent\""));
}

#[test]
fn d2_draws_with_its_options() {
    if !installed("d2") {
        return;
    }
    let page = build("d2", "```d2 d2.layout=elk d2.sketch=true\nweb -> api: calls\n```");
    assert!(page.contains("<div class=\"diagram\"><svg") && page.contains("calls"), "{page}");
}
