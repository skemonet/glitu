# TODO before a proper open-source release

Work started on 2026-10-29 and interrupted; this is what remains.

## Blocking
- [x] **skemo dependency**: `skemo-engine` / `skemo-lang` are git dependencies on
      https://github.com/skemonet/skemo, pinned to a revision.
- [ ] Pin skemo on a tag once skemo publishes one.
- [x] Re-capture `tests/oracle` (slide 12 differed since skemo changed the default card look).
- [ ] `tests/reveal-md/reference` (rendered by reveal-md, outside the project) differs by 0.68 % on
      slide 0 with Chromium 1243 (text antialiasing only); re-render it with reveal-md in the
      browser used by the CI.

## Translation to English (code, comments, messages, docs)
- [x] `crates/glitu-core` (translated and refactored, 41 tests)
- [x] `crates/glitu` Rust sources (translated and refactored, `cli.rs` added, 34 tests)
- [x] `crates/glitu/assets/export.js` comments
- [x] `crates/glitu-web` (all modules: comments, UI strings of the help and speaker view)
- [x] CSS comments: `assets/glitu.css`, `assets/template.css`, `themes/*`
- [x] `tests/oracle/README.md`, Cargo descriptions and comments
- [x] `README.md`, `tests/reveal-md/README.md`,
      `themes/black/README.md`, `docs/architecture/`
- [x] `tools/asciicast.py`

## Demo presentation (`tests/features/slides.md`)
- [x] Rewrite in English as a presentation of glitu itself, following the structure of the
      maintainer's Kubernetes deck: title → text appearing step by step (the publishing modes:
      static site, standalone page, `--web` page, PPTX/PDF/PNG export) → a frieze appearing step by
      step → a component diagram of the open-source architecture built click by click → code
      appearing step by step (the presentation's own markdown and skemo source, with the project
      tree in the explorer: markdown, asciicast) → an asciicast of `glitu build` with pauses at
      each click.
- [x] Include it in the website (`site/`, `tools/site.sh`, GitHub Pages); linked from the README.
- [x] Re-capture `tests/oracle` afterwards.
- [x] Theme logo placeholder reads "YOUR BRAND" (`themes/crystal/logo.svg`).

## Quality
- [ ] Apply `cargo fmt` (decide on `rustfmt.toml`: default or `max_width = 120`).
- [ ] Refactor the remaining complexity hot spots: `glitu-web` `terminal::render`, `engine::show`,
      `notes::update`, `engine::bind_keys`; `export.js` (`pathData`, `walk`).
- [ ] Coverage: measured with `cargo llvm-cov` (61 % outside `glitu-web` before the work); add a
      `tools/coverage.min` gate. `glitu-web` runs in the browser: extract its pure logic to test it
      natively; it is covered end to end by the visual oracles.
- [ ] CI: `tools/ci.sh` (fmt, clippy -D warnings, tests, doc, coverage, cargo-deny, no `unsafe`,
      secret scan) and a hardened GitHub workflow calling it; `deny.toml`; dependabot.
- [ ] Update dependencies (`png` 0.17, `tungstenite`, `wasm-bindgen` pinned at 0.2.129…).

## Open-source files
- [x] LICENSE-MIT, LICENSE-APACHE, `license` field
- [ ] CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, CHANGELOG.md, issue/PR templates
- [ ] Cargo metadata: `repository`, `homepage`, `keywords`, `categories`, `readme`, `rust-version`
- [ ] Third-party notices (reveal.css MIT, highlight.js BSD-3, fonts OFL) in one NOTICE file
- [ ] Check the provenance and licence of `tests/features/img/desk.jpg` and `tests/reveal-md/img/cat.jpg`
