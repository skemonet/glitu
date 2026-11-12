# Changelog

The notable changes of glitu. The format follows [Keep a Changelog](https://keepachangelog.com/),
and the versions [Semantic Versioning](https://semver.org/).

## Unreleased

### Security
- The SVG sanitizer reads the entities of a diagram once, before its checks: an address written
  with entities (`&#106;avascript:`) no longer passes them, and an `&` before accented text no
  longer stops the build.
- A quote in a class or in the language of a code block stays inside its attribute, and a terminal
  recording holding `<!--` or `<script` can no longer swallow the rest of the page.
- A folder given to glitu no longer reads a markdown file that is a symbolic link out of the
  folder, unless the file is named on the command line.
- The preview server bounds the length of a request's lines, their number and the time to send
  them; a damaged `%` sequence in an address no longer stops it.
- The browser is not looked for in the relative entries of `PATH`.
- Files including each other many times (`<!-- include: -->`) are refused past 4096 files read.
- The preview server refuses the positions and the rehearsal durations sent by the page of another
  site (a page open in the same browser could write `⏳` durations into the markdown).
- The name of a font (`font:`) or of an icon (`:mdi:github:`) is checked before it goes into an
  address and a stylesheet; a quote in a slide's `background` address stays inside its quotes.
- The browser's profile is a private folder (0700) with a name nobody can foresee, no longer
  `glitu-browser-<pid>-<port>` in the shared temporary folder.
- A diagram tool installed in `/bin` or in `~/bin` no longer reads the whole disk or the whole home
  folder: its `bin` folder only.
- `glitu-play` refuses a damaged or hostile `.glituplay` file with a message instead of crashing or
  asking for gigabytes: sizes larger than the file, slides, films, arrows or a backdrop of an
  impossible size, images missing from the archive or of another size than the slides.

### Fixed
- A d2 diagram is centred in its place, like the others (d2 pinned it to the top left corner).
- `glitu oracle check` matches the slides of a vertical stack one by one (they share their state
  names).
- `glitu-play`: an image that cannot be prepared is reported instead of freezing the player; the
  speaker view in a window smaller than its bar no longer crashes; `--start` past the last slide
  opens the last click instead of the first; on Wayland, keys whose keymap names their type
  before their symbols are no longer dropped.
- `html` and `xml` code of a reveal-md presentation with accented text or an unfinished attribute
  no longer stops the build.
- A slide's title holding a `|` is whole in the header and in the speaker view.
- A front matter whose lines end the Windows way (`\r\n`) is read: it was taken for text.
- Two long versions of an animated code no longer take gigabytes of memory to compare (12 000
  words each: 620 MB, 45 MB now); beyond about 4000 words each, only what they start and end with
  moves from one version to the next.
- A reveal-md presentation whose slides are all hidden no longer stops the engine in the browser.
- A terminal recording announcing an endless screen is shown on a bounded one (1000 × 500); click
  numbers near the largest one and a `data-autoslide` that is not a number are read safely.
- `tools/ci.sh` passes on macOS.

### Changed
- The demo shows the eight template themes and a diagram of each engine (Mermaid, D2, Graphviz);
  its architecture diagram shows the diagram tools run by glitu.

## 0.2.0 — 2026-11-12

### Security
- A presentation reads only the files of its folder: `<!-- include: -->`, `<<<` code, terminal
  recordings, skemo icons, the logo, a theme by path and the copied `img/`, `public/` folders,
  symbolic links resolved; absolute paths are refused. Another folder: `--allow-read DIR` or
  `files: allow:` in the user's configuration. Before, `<<< /home/me/.ssh/id_ed25519 text` copied
  the key into the page.
- Diagram tools run confined: chosen by the machine, never the presentation (engine names only,
  typed options, raw arguments refused unless allowed), resolved to absolute paths outside the
  presentation's folder, without a shell, with a minimal environment, a private temporary folder,
  a time limit, Landlock on Linux and Seatbelt on macOS; their SVG is sanitized and their messages
  are shown as text. The hostile presentations of `tests/security/` check it.

### Changed
- A skemo diagram whose steps leave room for the text's clicks (`card A @1`, then `@8:halo` after
  the text's clicks 2 to 7) is no longer refused by skemo 0.1.0-rc.9: glitu passes
  `--allow step-gap` when skemo has the `allow` feature.
- `glitu oracle check` matches the captures by state (slide and click), not by position: a missing
  click fails alone instead of shifting every later slide.
- **skemo is no longer linked into glitu**: ` ```skemo ` blocks are rendered by the `skemo` command
  line, to install (https://skemo.net/install); the binary goes from 21.9 to 9.5 MB.
- A theme's settings are `glitu.yaml` instead of `glitu.json` (still read).
- A skemo highlight (`@3:halo`) no longer cancels the node's appearance effect (`@1:up`): it is
  written in `data-highlight`.
- `glitu serve` becomes `glitu server`.
- `glitu export` joins `glitu build`, the format as a subcommand with its own options and help:
  `glitu build` (an HTML page, the default), `glitu build site|pptx|pdf|png|play`.
- Without a file, `glitu build` and `glitu server` take the current folder: its `slides.md`,
  otherwise all its `.md` files merged in natural order (README, CHANGELOG, LICENSE and the names
  starting with `_` or a dot left out); files given on the command line are merged in that order.
- `glitu build play` writes one `.glituplay` file (an uncompressed zip archive, the markdown
  inside: `glitu unpack` gets it back) instead of a folder.
- `glitu themes` becomes `glitu init --list-themes`.
- Clearer help (`--help`) and messages; errors are printed as `error: …`, and `glitu server` stops
  when the presentation does not exist.
- The compatibility test of the reveal-md syntax moves from `tests/reveal-md` to `tests/compat`.
- The demonstration images are drawn for the project.
- Dependencies updated (`avt` 0.19, `png` 0.18, `sha2` 0.11, `base64` 0.23, `miniz_oxide` 0.9,
  `wasm-bindgen` 0.2.130); the oldest supported compiler is Rust 1.90.

- Code highlighting by syntect and the grammars of bat: about 200 languages instead of fifteen,
  finer colours (functions, types, operators…), set by the theme's `--hl-*` variables. A
  presentation written with reveal-md's rules keeps highlight.js's rendering.

### Added
- Captions under the visuals, to credit a source: the title of an image
  (`![](img/x.png "Source: …")`) or `caption="…"` on an image or a block (code, diagram, formula),
  in Markdown; in every layout, laid on the image in `{.image-left}` and `{.image-right}`.
- Videos written as images (`![](img/demo.mp4)`, `webm`, `ogv`), in the stage, the columns or
  beside the text: `autoplay`, `muted`, `loop`, `nocontrols`, `poster=`; `pauses="0:12 1:05"`
  stops the video at these times, each next click resuming it.
- Focus zones on an image or a video, `focus="x y w h …; …"` (in % of the picture): a hand-drawn
  mark (circle by default), `.spot` (the rest darkens) or a zoom (`@3:magnify`), at a click, or on
  a video during a time range (`0:12-0:18`). Images and videos can appear at a click
  (`{@2}`).
- `glitu add video`: a video slide with its pauses, caption and focus zone (`--src`, `--pauses`);
  without `--src`, a 6 s placeholder with a clock, `img/placeholder.webm`.
- `glitu server` serves the videos with their type and in parts (`Range` requests), so the
  browser can seek in them.
- glitu-play's speaker view (`s`, `--speaker`): a second window, full screen on the other screen,
  with the current and next clicks, the notes, the slide and a timer (`t` pause, `r` reset, `+`
  `-` size of the notes).
- Diagram engines: ` ```mermaid ` (mermaid.js in the browser of the exports, pinned and checked, or
  `mmdc`), ` ```d2 `, ` ```dot `; the engine and its options from the block, the front matter or
  the user's configuration (`~/.config/glitu/config.yaml`, manifests in `engines/`); `--tool`,
  `--engine`, `GLITU_TOOL_*`; `glitu config --engines`; `glitu build -v`. `glitu server` shows a
  diagram's error in its slide and renders again only the diagrams that changed.
- Built pages carry a description, an author and Open Graph tags, from the front matter
  (`description:`, otherwise `subtitle:`, and `author:`).
- The documentation of the website, https://glitu.net/docs/.
- `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `NOTICE`, issue and pull request
  templates.
- CI: format, documentation, dependency (cargo-deny), coverage and secret checks.
- A binary for Intel Macs, `glitu-macos-x86_64`.
- The `graphite` theme: charcoal with a purple glow and a line along the top, monospace labels.
- `transition="dive:api"` between two slides: the previous slide's diagram grows around its
  element `api` (or a zone `x y w h`) and the new slide's diagram comes out of it;
  `surface:api` the other way round; going back reverses either.
- `box="x y w h"` on a visual: its place and size forced, in % of the visual's area.
- glitu-play, the native player for slow computers, plays `glitu build play`; binaries for
  Linux x86_64 and ARM and Windows x86_64.

### Fixed
- The cover title and the key figure keep their descenders (no longer cut below).

## 0.1.1-rc.4 — 2026-10-31

### Added
- Binaries for Linux x86_64 and ARM, macOS ARM and Windows x86_64, built by the maintainer's CI.

## 0.1.1-rc.3 — 2026-10-30

### Added
- `glitu export --download-browser`: finds the installed browser, or downloads Chrome's headless
  shell.
- `glitu build --assets local|<url>` and `glitu export --format site`; `glitu init --theme <name>`
  creates a theme of your own.
- Speaker view: timer under the slide, timing table, rehearsals.
- A skill to write glitu presentations.

## 0.1.1-rc.2 — 2026-10-30

### Added
- `glitu init --css`, the "white" theme, the theme's and the presentation's logo and marks.

## 0.1.1-rc.1 — 2026-10-30

### Added
- A command line on clap, `glitu init`, themes embedded in the binary; the oracle is left out of
  the released binary.

## 0.1.0 — 2026-10-30

First release: presentations written in markdown, rendered by a Rust/WebAssembly engine; static
site, standalone page, PPTX, PDF and PNG exports; skemo diagrams; the website glitu.net.
