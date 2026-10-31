# TODO before a proper open-source release

Work started on 2026-10-29 and interrupted; this is what remains.

## Blocking
- [x] **skemo dependency**: `skemo-engine` / `skemo-lang` are git dependencies on
      https://github.com/skemonet/skemo, pinned to a revision.
- [ ] Pin skemo on a tag once skemo publishes one.
- [x] Re-capture `tests/oracle` (slide 12 differed since skemo changed the default card look).
- [ ] `tests/reveal-md/reference` (rendered by reveal-md, outside the project) differs on slide 0
      by text antialiasing only: 0.68 % with Chromium 1243, 1.67 % since "black" uses the 2D slide
      transition (reveal-md's 3D default changes how the present slide's text is rasterised).
      Re-render it with reveal-md in the browser used by the CI, with `transition: slide`.

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
- [x] CI and releases on the maintainer's CI (`.gitlab-ci.yml`, no GitHub Actions): `tools/ci.sh`
      (clippy -D warnings, tests, smoke test), the four binaries with `cargo zigbuild`,
      `tools/github-release.sh` (main, tag, release), `tools/github-pages.sh` (website on gh-pages),
      `tools/install.sh`.
- [ ] CI, the rest: fmt, doc, coverage, cargo-deny, no `unsafe`, secret scan; `deny.toml`.
- [ ] Update dependencies (`png` 0.17, `tungstenite`, `wasm-bindgen` pinned at 0.2.129…).

## Open-source files
- [x] LICENSE-MIT, LICENSE-APACHE, `license` field
- [ ] CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, CHANGELOG.md, issue/PR templates
- [ ] Cargo metadata: `repository`, `homepage`, `keywords`, `categories`, `readme`, `rust-version`
- [ ] Third-party notices (reveal.css MIT, highlight.js BSD-3, fonts OFL) in one NOTICE file
- [ ] Check the provenance and licence of `tests/features/img/desk.jpg` and `tests/reveal-md/img/cat.jpg`
