# glitu — notes for working in this repository

Markdown presentations → HTML page; presentation engine in Rust/wasm (replaces reveal.js).
Everything is written in English (code, comments, messages, docs). No npm, no Node.

## Map

- `crates/glitu-core` — pure compiler (no I/O): `markdown.rs`, `render/` (sections, layouts,
  steps, visuals; diagrams asked of `Assets::diagram`), `html.rs`, `highlight/` (syntect, and the highlight.js reproduction for reveal-md), `math.rs`, `reveal_md.rs`.
- `crates/glitu` — the tool: `cli.rs`, `config.rs` (`glitu config`: the front matter), `add.rs`
  (`glitu add`: slides from examples), `build.rs`, `source.rs`, `online.rs`, `standalone.rs`,
  `serve.rs`, `engines/` (diagram engines: manifests in `crates/glitu/engines/`, tool lookup,
  confined runs, SVG sanitizer, mermaid.js), `settings.rs` (the user's `config.yaml`), `confine.rs`
  (files a presentation may read), `export.rs` + `assets/export.js` + `pptx/` + `pdf.rs`, `browser.rs` (driving) +
  `chromium.rs` (finding or downloading the browser), `oracle.rs`.
- `crates/glitu-web` — wasm engine (navigation, steps, speaker view, themes' backdrops).
- `crates/glitu-play` — native player for slow computers (winit + softbuffer + tiny-skia): plays
  `glitu build play` (`deck.rs` the format, `scene.rs` the animation, `show.rs` what the window
  shows, `player.rs` the windows on winit, `wayland.rs` on Wayland with `keymap.rs`).
- `themes/` — `crystal` (default), `constellation`, `paper`, `aurora`, `ascii`, `blueprint`, `iso`
  share `assets/template.css`; `black` is the reveal-md theme.
- `tests/features/slides.md` — the demo of every syntax; `tests/oracle` its reference screenshots.
- `tests/security/` — hostile presentations (`crates/glitu/tests/security.rs`); see
  `docs/design/diagram-engines.md` §9.

## Rules

- The oracle is a development command, left out of the released binary: build it with
  `cargo build --release --features oracle`.
- After any change to the engine or the themes: both oracles at 100 %, the skemo command line at
  `tools/skemo.version` installed (or `GLITU_TOOL_SKEMO=…`)
  (`glitu server tests/features/slides.md -p <port> &` then
  `glitu oracle check http://localhost:<port>/ tests/oracle`, and the same for
  `tests/compat/demo.md` against `tests/compat/reference`). No script of the page may call
  `Math.random` before the engine (the animated backdrop replays it).
- `cargo test --workspace`; `cargo clippy --workspace --all-targets` without warnings.
- A presentation is untrusted data: what it reads stays in its folder (`confine.rs`), what it runs
  is chosen by the machine (`engines/`). No unsafe code (`unsafe_code = "forbid"`).
