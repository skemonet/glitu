//! Compiles the web engine (crates/glitu-web) to wasm and generates its JavaScript glue with
//! wasm-bindgen; the glitu binary embeds the result. Requires the target:
//! `rustup target add wasm32-unknown-unknown`.
//!
//! Also lists the official themes (`themes/` of the repository), which the binary embeds: a
//! presentation builds without a `themes/` folder of its own.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    embed_themes(&manifest.parent().unwrap().parent().unwrap().join("themes"), &out.join("themes.rs"));
    let web = manifest.parent().unwrap().join("glitu-web");
    println!("cargo:rerun-if-changed={}", web.join("src").display());
    println!("cargo:rerun-if-changed={}", web.join("Cargo.toml").display());

    // A separate target folder: the parent cargo holds the lock on its own
    let target = out.join("web-target");
    let mut cargo = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    // Under cargo-llvm-cov, the wrapper would instrument the wasm too, which has no profiler
    if env::var_os("CARGO_LLVM_COV").is_some() {
        cargo.env_remove("RUSTC_WRAPPER");
    }
    let status = cargo
        .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "glitu-web"])
        .arg("--target-dir")
        .arg(&target)
        .current_dir(manifest.parent().unwrap().parent().unwrap())
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_TARGET_DIR")
        .status()
        .expect("cargo not found");
    assert!(status.success(), "failed to compile glitu-web (is the wasm32-unknown-unknown target installed?)");

    wasm_bindgen_cli_support::Bindgen::new()
        .input_path(target.join("wasm32-unknown-unknown/release/glitu_web.wasm"))
        .web(true)
        .unwrap()
        .typescript(false)
        // Function names only help debugging: the page loads faster without them
        .remove_name_section(true)
        .remove_producers_section(true)
        .generate(out.join("pkg"))
        .expect("wasm-bindgen");
}

/// `themes.rs`: `THEMES`, every file of the official themes as (path under themes/, bytes)
fn embed_themes(themes: &Path, dest: &Path) {
    println!("cargo:rerun-if-changed={}", themes.display());
    let mut files = Vec::new();
    collect(themes, &mut files);
    files.sort();
    let mut rs = String::from("pub static THEMES: &[(&str, &[u8])] = &[\n");
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let rel = path.strip_prefix(themes).unwrap().to_string_lossy().replace('\\', "/");
        rs += &format!("    ({rel:?}, include_bytes!({:?})),\n", path.display().to_string());
    }
    rs += "];\n";
    fs::write(dest, rs).unwrap();
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            println!("cargo:rerun-if-changed={}", path.display());
            collect(&path, files);
        } else {
            files.push(path);
        }
    }
}
