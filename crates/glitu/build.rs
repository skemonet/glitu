//! Compiles the web engine (crates/glitu-web) to wasm and generates its JavaScript glue with
//! wasm-bindgen; the glitu binary embeds the result. Requires the target:
//! `rustup target add wasm32-unknown-unknown`.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
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
