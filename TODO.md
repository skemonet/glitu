# TODO before a proper open-source release

Work started on 2026-10-29 and interrupted; this is what remains.

## Blocking
- [x] **skemo dependency**: `skemo-engine` / `skemo-lang` are git dependencies on
      https://github.com/skemonet/skemo, pinned to a revision.
- [x] Pin skemo on a tag: `v0.1.0-rc.8`.
- [x] Re-capture `tests/oracle` (slide 12 differed since skemo changed the default card look).
- [ ] `tests/compat/reference` (rendered by reveal-md, outside the project) differs on slide 0
      by text antialiasing only: 0.68 % with Chromium 1243, 1.67 % since "black" uses the 2D slide
      transition (reveal-md's 3D default changes how the present slide's text is rasterised).
      Re-render it with reveal-md in the browser used by the CI, with `transition: slide`.

## Roadmap: security, then diagram engines

Design: `docs/design/diagram-engines.md` (§9 security, SEC-n) and `docs/design/skemo-requirements.md`
(SK-n, to send to skemo). Done on 2026-11-06 unless noted:

1. Security of what exists:
   - [x] SEC-16: every file a presentation reads stays in its root folder (and the folders of the
         files named on the command line), links resolved; absolute paths refused;
         `--allow-read DIR`, `files: allow:` (`crates/glitu/src/confine.rs`).
   - [x] `card#a A @1:up @3:halo`: glitu writes the highlights in `data-highlight` (SK-4's
         attribute); `data-fx` stays skemo's appearance effect.
2. Security of the engine layer (`crates/glitu/src/engines/`, `tests/security/`):
   - [x] SEC-1 to SEC-6: known engine ids only; manifests from the binary and the user's
         configuration only; tools resolved to absolute paths (no relative `PATH` entry, none in
         the deck, `.exe` only on Windows); no shell; typed options; raw arguments from the
         document only if allowed.
   - [x] SEC-7 to SEC-10: safe mode in the manifests (`files`, fixed arguments); root folder,
         private temporary folder (0700), output on stdout or a temporary file; timeout (kills the
         process group), output limits; minimal environment; Landlock (Linux), Seatbelt (macOS).
   - [ ] Windows: files not confined (a Job Object needs unsafe code, forbidden in the workspace).
   - [x] SEC-11 to SEC-15: SVG sanitized (allowlist), identifiers unique per page; stderr as text;
         mermaid.js pinned (12.2.0, SHA-256); cache key with tool path, version, manifest digest;
         `glitu config --engines`, `glitu build -v`.
3. The rest of `diagram-engines.md`:
   - [x] Open questions settled (§10): YAML, Landlock + Seatbelt, mermaid.js downloaded,
         mermaid → chromium, direct switch.
   - [ ] Send `skemo-requirements.md` to skemo (SK-1 to SK-9: JSON diagnostics, highlights,
         versioned SVG conventions…).
   - [x] The engine layer: manifests, tool lookup, engine choice, layered options, in-memory cache
         for `glitu server`, errors in the slide.
   - [x] skemo through it: both oracles at 100 % with `skemo` 0.1.0-rc.8; linked skemo removed
         (release binary 21.9 → 9.5 MB).
   - [x] `mermaid.chromium` (checked with Chromium); manifests of `graphviz.dot`, `mermaid.mmdc`,
         `d2.d2`.
   - [x] `graphviz.dot`, `mermaid.mmdc`, `d2.d2` checked with the real tools, under Landlock.
   - [x] skemo's working tree checked against SK-n (end of `skemo-requirements.md`); glitu uses
         `--root`, `--diagnostics json` and the native highlights when `skemo capabilities` says so.
   - [ ] Once skemo releases them: a new version number (SK-3), re-capture `tests/oracle` (its
         routing changes slides 6 and 24), bump `tools/skemo.version`; use `--batch` (SK-10).
   - [ ] The conversions to skemo (SK-12).
   - [x] `glitu add diagram` notes when the skemo tool is missing.

## Translation to English (code, comments, messages, docs)
- [x] `crates/glitu-core` (translated and refactored, 41 tests)
- [x] `crates/glitu` Rust sources (translated and refactored, `cli.rs` added, 34 tests)
- [x] `crates/glitu/assets/export.js` comments
- [x] `crates/glitu-web` (all modules: comments, UI strings of the help and speaker view)
- [x] CSS comments: `assets/glitu.css`, `assets/template.css`, `themes/*`
- [x] `tests/oracle/README.md`, Cargo descriptions and comments
- [x] `README.md`, `tests/compat/README.md`,
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
- [x] Apply `cargo fmt` (`rustfmt.toml`: `max_width = 120`, `use_small_heuristics = "Max"`).
- [x] Refactor the remaining complexity hot spots: `glitu-web` `terminal::render`, `engine::show`,
      `notes::update`, `engine::bind_keys`; `export.js` (`pathData`, `walk`).
- [x] Coverage: measured with `cargo llvm-cov` (61 % outside `glitu-web` before the work, 84 % now); the
      `tools/coverage.min` gate. `glitu-web` runs in the browser: extract its pure logic to test it
      natively; it is covered end to end by the visual oracles.
- [x] CI and releases on the maintainer's CI (`.gitlab-ci.yml`, no GitHub Actions): `tools/ci.sh`
      (clippy -D warnings, tests, smoke test), the four binaries with `cargo zigbuild`,
      `tools/github-release.sh` (main, tag, release), `tools/github-pages.sh` (website on gh-pages),
      `tools/install.sh`.
- [x] CI, the rest: fmt, doc, coverage, cargo-deny, no `unsafe`, secret scan; `deny.toml`.
- [x] Install cargo-deny and cargo-llvm-cov in the CI image: `rust:1.99.1-5` (ci-images; its tag
      `rust-v1.99.1-5` still to be pushed).
- [x] Update dependencies (`avt` 0.19, `png` 0.18, `sha2` 0.11, `wasm-bindgen` 0.2.130…).

## Open-source files
- [x] LICENSE-MIT, LICENSE-APACHE, `license` field
- [x] CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, CHANGELOG.md, issue/PR templates
- [x] Cargo metadata: `repository`, `homepage`, `keywords`, `categories`, `readme`, `rust-version`
- [x] Third-party notices (reveal.css MIT, highlight.js BSD-3, fonts OFL) in one NOTICE file
- [x] Replace `desk.jpg` and `cat.jpg` (unknown provenance) by images drawn for the project

## Features
- [ ] External plugins and themes: load third-party themes and plugins (from a path or a URL);
      study compatibility with reveal.js plugins (their API: `Reveal.registerPlugin`, events,
      `Reveal.getConfig`…) — what the Rust engine would have to expose.
- [x] Code highlighting on par with Shiki: syntect with bat's grammars (two-face, ~200 languages),
      scopes mapped to highlight.js classes, colours by `--hl-*` variables of `template.css`; the
      reveal-md compatibility keeps the highlight.js reproduction (`highlight/hljs.rs`).
- [x] Diagram engines: see "Roadmap: security, then diagram engines" above.
- [ ] Presenting on slow hardware (e.g. an Eee PC 1025C: Atom N2600, GMA 3600), where a browser
      is too heavy. Two ways, both prepared by `glitu export` on another machine:
      - [x] PDF (`export --clicks -o deck.pdf`) presented with pdfpc or mupdf: page labels (the
            pages of one slide are overlays), a `deck.pdfpc` file with the speaker notes,
            `--width` to export at the screen's resolution;
      - [ ] native player (`crates/glitu-play`): the browser lays out once, at export
            (`--format play`: one screenshot per click and the animations described); the player
            composites and animates only what moves, by software (tiny-skia + softbuffer):
            - [x] progressive skemo arrows: Chromium's pixels revealed along the arrow's path,
                  between two extra screenshots (without the arrows; without heads and waiting
                  cards), exact final frame;
            - [x] other changes of a click: cross-fade of the changed area;
            - [x] asciicasts: the typing recorded by Chromium's screencast at export, settled
                  pixels taken from the exact screenshots;
            - [x] speaker view (notes, timer, next slide) on a second window (`s`, `--speaker`);
                  placement on two screens not yet checked on real hardware;
            - [x] animated code and animated slides: recorded as films at export (`ANIMATED_AREA`
                  in `export.rs`, `prefers-reduced-motion` lifted during the recording only);
            - [ ] the animated backdrop (2026-11-06, work in progress, NOT committed):
                  - [x] `crates/glitu-backdrop`: the 7 scenes behind a `Canvas` trait, shared by
                        glitu-web (`themes/canvas.rs`, both oracles still at 100 %) and glitu-play
                        (`skia.rs` on tiny-skia, `backdrop.rs` the driver);
                  - [x] export (`play`): `deck.backdrop` (scene, seed, rect, size) and per click
                        `motion`, `dim`, `glass`, `NNN-alpha.png` (opacity of what covers it);
                  - [x] `glitu-play --check`: native backdrop against Chromium's screenshots;
                        all pass but ascii's cover (8 %: Plex Mono TTF 2.3 ≠ the theme's woff2);
                  - [ ] the plan was: GPU (glium 0.36, safe API; `catch_unwind` around
                        `SimpleWindowBuilder::build`, which unwraps) shows
                        `screenshot + (1 − α)·(B_live − B_ref)`, blurred under glass. BUT the
                        native backdrop costs 7–79 ms per frame on a fast machine (crystal 79 ms at
                        1024×576): far too slow for the Atom on the CPU; the scene itself would
                        have to be drawn by the GPU, and the GMA 3600 has no driver yet (llvmpipe
                        would be slower). On hold: first make the software path faster on the
                        Eee PC (PNG decoding, films, slide cross-fades); the maintainer finds
                        glitu-play still slow there (what exactly: not yet known);
                  - [x] the software path on the Eee PC (`glitu-play-perf-spec.md`, measured there:
                        1.1 s of bicubic per slide on the event loop, 3 frames per cross-fade):
                        images read and scaled by a worker thread (`loader.rs`, two clicks ahead),
                        own separable Catmull-Rom in 16-bit SIMD (`resample.rs`, `wide`; ×17
                        faster than tiny-skia's bicubic, same look), cross-fade by row spans on
                        two threads (`scene.rs`), damaged rows only (`buffer.age()`), frames paced
                        by Wayland's frame callbacks, `GLITU_PLAY_TRACE`. Measured on the Eee PC
                        (`glitu-play-perf-next.md`): 20–29 ms to the first frame, ~33 frames/s at
                        1080p, bound by memory bandwidth. Second round (2026-11-06, to measure
                        there): the animation's frames drawn straight into the softbuffer buffer
                        (`screen` keeps the previous click; `stale` copies what a buffer lacks,
                        whatever its age), the transitions of the next two clicks built ahead on a
                        worker thread while nothing moves (`Worker` in `loader.rs`; the trace says
                        "transition prepared ahead"), two warm-up frames at start-up (first writes
                        into a new shm pool). Then `wp_viewporter` (2026-11-06, to measure on the
                        Eee PC): on Wayland, `wayland.rs` (smithay-client-toolkit, our own shm
                        buffers, frame callbacks) draws the animations without films at the slide's
                        size, enlarged by the compositor, the last image at full size (identical to
                        winit's, checked under a headless labwc); the keys through the compositor's
                        keymap read by `keymap.rs` (no xkbcommon: AZERTY, QWERTZ tested); the speaker
                        view in the same loop (`SpeakerView`); `show.rs` shared by both front-ends.
                        `GLITU_PLAY_SMALL=0` / `GLITU_PLAY_WINIT=1` to compare. Not checked: a real
                        keyboard and two real screens on Wayland (the speaker view goes to the
                        `eDP`/`LVDS`/`DSI` screen), fractional scales (integer scale only). Not
                        done: the preload delayed during a fade (1–2 ms per frame), the films still
                        read on the event loop (`Show::render`);
                  - [ ] clippy warnings left in glitu-play/glitu-backdrop (unused `fps`, `moving`,
                        `advance` until the GPU path uses them; `let _ =` on unit values in the
                        moved scenes; `Result<_, ()>`);
            - [ ] GPU rendering (OpenGL ES 2), optional and switchable (`--no-gpu`): the
                  maintainer is writing a GMA 3600 driver;
            - [x] `glitu-play` in the CI's `binaries` job and the releases (Linux x86_64/ARM
                  glibc 2.28, Windows; no macOS: winit needs the Apple SDK);
      - [x] documentation: `site/docs/slow-computers.md`, README "Slow computers".

## Releases
- [x] A macOS Intel binary (`x86_64-apple-darwin`) in the CI's `binaries` job, `tools/install.sh`
      and the documentation.
