# glitu — notes for working in this repository

Markdown presentations → HTML page; presentation engine in Rust/wasm (replaces reveal.js).
Everything is written in English (code, comments, messages, docs). No npm, no Node.

## Map

- `crates/glitu-core` — pure compiler (no I/O): `markdown.rs`, `render/` (sections, layouts,
  steps, visuals), `html.rs`, `highlight.rs`, `math.rs`, `reveal_md.rs`, `skemo.rs`.
- `crates/glitu` — the tool: `cli.rs`, `build.rs`, `source.rs`, `online.rs`, `standalone.rs`,
  `serve.rs`, `export.rs` + `assets/export.js` + `pptx/` + `pdf.rs`, `browser.rs`, `oracle.rs`.
- `crates/glitu-web` — wasm engine (navigation, steps, speaker view, themes' backdrops).
- `themes/` — `crystal` (default), `constellation`, `paper`, `aurora`, `ascii`, `blueprint`, `iso`
  share `assets/template.css`; `black` is the reveal-md theme.
- `tests/features/slides.md` — the demo of every syntax; `tests/oracle` its reference screenshots.

## Rules

- The oracle is a development command, left out of the released binary: build it with
  `cargo build --release --features oracle`.
- After any change to the engine or the themes: both oracles at 100 %
  (`glitu serve tests/features/slides.md -p <port> &` then
  `glitu oracle check http://localhost:<port>/ tests/oracle`, and the same for
  `tests/reveal-md/demo.md` against `tests/reveal-md/reference`). No script of the page may call
  `Math.random` before the engine (the animated backdrop replays it).
- `cargo test --workspace`; `cargo clippy --workspace --all-targets` without warnings.
