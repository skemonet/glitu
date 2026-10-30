//! For whoever hosts the light pages (`glitu build --assets <url>`): writes the engine and the
//! themes, each one at its version (`glitu/<version>/`, `themes/<name>/<version>/`), into a
//! folder to publish at that address. The versions already in the folder stay.
//!
//! `cargo run --release -p glitu --example assets -- <folder> [themes folder]`

use std::path::{Path, PathBuf};

use glitu::build;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = PathBuf::from(args.first().map_or("static", String::as_str));
    let themes = match args.get(1).map(PathBuf::from).or_else(|| build::themes_root(Path::new("."))) {
        Some(dir) => Ok(dir),
        None => glitu::themes::root(),
    };
    let run = || -> Result<usize, String> {
        let mut n = 0;
        for published in build::runtime(&themes?)? {
            build::write(&published.files, &out.join(&published.dir))?;
            n += published.files.len();
        }
        Ok(n)
    };
    match run() {
        Ok(n) => println!("{}: {n} files", out.display()),
        Err(e) => {
            eprintln!("assets: {e}");
            std::process::exit(1);
        }
    }
}
