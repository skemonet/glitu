# glitu — presentations written in markdown, rendered by a Rust engine

> [!WARNING]
> **This code is 100% AI-generated.** It is provided "as is", without warranty of any kind; the
> repository owner accepts no responsibility for its use. Read it and test it before relying on it.
>
> **Status:** work in progress (see [TODO.md](TODO.md)).

## ▶ Demo: glitu, presented with glitu

**Website:** https://glitu.net — the demo as one page: https://glitu.net/demo.html

[**`tests/features/slides.md`**](tests/features/slides.md) presents glitu itself: its publishing
modes, a frieze, the architecture diagram built click by click, its own source with the project
tree, and a terminal recording of `glitu build`; then a tour of every syntax.

```sh
rustup target add wasm32-unknown-unknown
cargo build --release
target/release/glitu server tests/features/slides.md    # http://localhost:8000
```

Diagrams are drawn by [skemo](https://github.com/skemonet/skemo), a git dependency fetched by
Cargo.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/skemonet/glitu/main/tools/install.sh | sh            # latest release into ~/.local/bin, SHA-256 checked
```

Or take a binary from the [releases](https://github.com/skemonet/glitu/releases): `glitu-linux-x86_64`,
`glitu-linux-arm64` (both static), `glitu-macos-arm64`, `glitu-macos-x86_64`, `glitu-windows-x86_64.exe` (the latest one is
always at `https://github.com/skemonet/glitu/releases/latest/download/<file>`), or build it from
source as above. `glitu build pptx|pdf|png|play` also needs a Chromium-based browser: the one
installed is found, otherwise `--download-browser` fetches one (see "Export"). The diagrams need
their tool: [skemo](https://skemo.net/install) for ` ```skemo ` blocks (see "Diagrams").

```bash
glitu init talk                 # a sample presentation: talk/slides.md
glitu init talk --css           # and its talk/style.css: the theme's variables, to adjust it
glitu init talk --theme mine    # and your own theme, talk/themes/mine/, to change at will
glitu config -f talk theme paper  # a setting of the front matter (glitu config: all of them)
glitu add diagram -f talk       # a slide with an example to adapt (glitu add --help: the kinds)
glitu server talk                # preview at http://localhost:8000, reloads at every save
glitu build talk                # presentation.html: one page, opens anywhere without a network
glitu build pptx talk           # presentation.pptx (or site, pdf, png, play: see "Export")
glitu --help                    # the commands; glitu <command> --help: their options
```

## Writing with Claude

`skills/glitu-presentation/` is a skill for Claude Code (and other agents that read skills): it
teaches the deck syntax, skemo diagrams and the workflow (outline, write, `glitu build` to check,
`glitu server` to preview). Copy the folder into `~/.claude/skills/` (or `.claude/skills/` of a
project), then ask for a presentation.

## Releases

Releases follow the commit messages ([conventional commits](https://www.conventionalcommits.org)):
once the tests pass on `main`, a `fix:` or `perf:` since the last release gives a patch release, a
`feat:` a minor one, a `!` after the type (`feat!:`) or a `BREAKING CHANGE:` line a major one
(minor while the version is 0.x); `docs:`, `ci:`, `chore:`… release nothing. The CI then tags
`v<version>` and publishes the GitHub release with the five binaries and their SHA-256. The
version the binary says comes from that tag; `Cargo.toml`'s only serves local builds.

The pipeline is `.gitlab-ci.yml`, run by the maintainer's own CI (`gitlab-ci-local`), not by GitHub
Actions: on other branches, `tools/ci.sh --quick` (lints and tests); on `main`, `tools/ci.sh` (the
same plus a smoke test of the binary, also to run locally), `tools/next-version.sh` (the version
to release, if any), the five binaries (cross-compiled with `cargo zigbuild`) when there is one,
`tools/github-release.sh` (pushes `main` to GitHub, then the tag and the release), and the website
(below). `target/` and the cargo registry are cached between pipelines.

**License:** dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

## Overview

The whole presentation fits in **`slides.md`**: the text, the diagrams (skemo code), the code
shown on screen. Only the terminal recordings (`casts/*.cast`) and the images, if any, are
separate files. The rendering comes from a **theme** (`themes/crystal/`): a header, a text panel
on the left, a visual on the right, the "crystal" backdrop, progress by chapter. Six other themes
share this template with another animated backdrop: `constellation`, `paper` (cut paper),
`aurora`, `ascii` (amber ASCII), `blueprint` (technical drawing), `iso` (isometric).

The `glitu` tool (Rust) compiles the markdown into a static page; the presentation engine, also
written in Rust and compiled to WebAssembly, replaces reveal.js in the browser. No npm
dependency. The writing rules of reveal.js / reveal-md are accepted too. The engine and the
themes are also published on **glitu.net/static**, each at its version (`glitu/0.1.0/`,
`themes/crystal/0.1.0/`), for the light pages.

```sh
rustup target add wasm32-unknown-unknown     # once; skemo is fetched by Cargo (git dependency)
cargo build --release                         # builds target/release/glitu (wasm and skemo engines inside)

target/release/glitu init talk [--css]         # a sample presentation in talk/, [and its style.css] (overwrites nothing)
target/release/glitu init talk --theme mine    # with a theme of your own, talk/themes/mine/ (see "Theme")
target/release/glitu config                     # the settings of the front matter, with their values
target/release/glitu config theme paper         # sets one, after checking its value (--unset to remove it)
target/release/glitu add code --lang python     # adds a slide filled with an example (see "Adding slides")
target/release/glitu server                     # http://localhost:8000, reloads at every change
target/release/glitu server --allow 192.168.0.0/16   # also for these addresses (CIDR, repeatable): phone, tablet
target/release/glitu build -o talk.html        # one page, everything inside, no network (see "One page")
target/release/glitu build --assets https://glitu.net/static -o talk.html   # light page
target/release/glitu build site -o dist       # static website, to serve with any web server
target/release/glitu build pptx -o talk.pptx   # editable PowerPoint, notes and animations (see "Export")
target/release/glitu build pdf -o talk.pdf     # vector PDF (selectable text)
target/release/glitu build png -o captures     # one PNG image per slide
target/release/glitu unpack talk.html -o talk/ # gets the markdown back from a built page (see "Reopen")
target/release/glitu init --list-themes        # the themes embedded in the binary
```

While presenting (`?` shows this list on screen):

| Key | Action |
|---|---|
| `→` Space `N` | next step or slide, in reading order |
| `←` Shift+Space `P` | previous step or slide |
| `Page↓` / `Page↑` | next / previous chapter in 2D navigation (otherwise next / previous slide) |
| `↓` / `↑` | within the chapter (`H` `J` `K` `L`: left, down, up, right) |
| Home / End | first / last slide |
| `G` | go to a slide: its number, `5.2`, or its identifier |
| Esc / `O` | overview: arrows to choose, Enter or click to go there |
| `F` | full screen |
| `S` | speaker view (see "Present") |
| `D` | draw on the slide: 1 to 5 the colour, `Z` undoes, `X` clears, `D` or Esc leaves |
| `B` `V` `.` | black screen, then resume |
| `A` | pause or resume the automatic advance |
| Alt + click (Ctrl + click on Linux) | zoom on an element, then back |

On a touch screen, swiping with one finger changes slide; with two fingers, you zoom and move
the view. The mouse wheel with `mouseWheel: true` in `glitu.yaml`.
Adding `?print-pdf` to the address puts one slide per page, all steps shown: then print to PDF
from the browser (or `glitu build pdf -o x.pdf`). As in reveal.js: `data-autoplay` (`{autoplay}` on a video written as an image) starts a video
or a sound on arriving on the slide, `data-src` loads a medium when the slide gets close,
`data-autoslide` (or `autoSlide` in ms in `glitu.yaml`) advances on its own, `loop: true`
loops, `data-visibility="hidden"` hides a slide, `data-state="x"` sets the class `x` on the page.

## Writing the slides

~~~markdown
---
title: Kubernetes                       ← the cover
subtitle: Deploy and expose an application
author: Ada Lovelace
date: October 2026
theme: crystal                        ← or crystal:0.1.0; without a version: the one on disk
---

# Resources                             ← numbered chapter (01, 02…); `# End {-}`: without a number

Deploy, then expose                     ← subtitle of the chapter

## Deploying an application            ← a slide: header "RESOURCES / Deploying…"

### Deployment                          ← title of the left panel

- Number of replicas @1                 ← appears at the 1st click
- ReplicaSet creates and deletes the pods @+1   ← one click after the previous one (@2)
- Visible from click 2 to 3 @2-3:up     ← then disappears; comes in from below

```yaml title="deployment.yml" lines="8|9-11" @1     ← code window, lines highlighted on click
…
```

An energy $E = mc^2$, a [key]{.circle @3} word circled at click 3.

Note: what the speaker should say (S key). [click] At the first click, this is highlighted.
~~~

| For… | Write |
|---|---|
| a diagram | a ` ```skemo ` block; its `@n` markers move forward with the clicks of the slide |
| a frieze that scrolls | a skemo block `diagram(layout="frieze" …)`: the strip centres on the card of each step |
| a terminal | `![window title](casts/demo.cast)`: one command per click |
| an image | `![](img/photo.jpg)`; `![](img/x.png){@2}` appears at click 2 |
| a caption, a source | `![](img/x.png "Source: [INSEE](https://insee.fr)")` or `{caption="…"}` after an image; ` ```skemo caption="…" ` on a block: a line under the visual, in every layout |
| a video | `![](img/demo.mp4){muted autoplay loop}` (`mp4`, `webm`, `ogv`; `nocontrols`, `poster="img/x.jpg"`); `pauses="0:12 1:05"`: plays when shown, stops at each time, the next click resumes it (one click per pause) |
| a focus on part of an image or a video | `{focus="x y w h …; …"}` after it, in % of the picture: a hand-drawn circle by default (`.box` `.underline` `.highlight` `.strike`, colours), `.spot` (the rest darkens), `@2` `@2-3` (clicks), `@3:magnify` (zoom); on a video, a time range instead: `0:12-0:18`, `magnify` to zoom |
| blocks that replace each other | several `### Title @n` in the slide |
| a full-width diagram | `## Topic {.large}`: the bullets become a caption above it |
| without text | a slide without `###` nor bullets: the visual takes the full width |
| another header | `## Topic {kicker="Template"}`; `{.compact}` tightens the bullets |
| big emojis | an ` ```emoji ` block, one per line: they replace each other on click (`😱 @1`) |
| the file shown | ` ```yaml file="roles/x/tasks/main.yml" `: an explorer lists the files seen so far |
| 2D navigation | `navigation: 2d` in the front matter: one chapter per column (← → and Space follow reading order, Page↓ Page↑ go from one chapter to another, ↑ ↓ within the chapter) |
| a formula | `$\frac{a}{b}$` in the text, `$$…$$` as a block, a ` ```math ` block as a visual (LaTeX, rendered as MathML at build) |
| a word that appears | `[text]{@2}`; `[text]{.class}` sets a class |
| a hand-drawn mark on a word | `[text]{.circle @2}`: `.underline` `.circle` `.box` `.highlight` `.strike`, colour `.red` `.green` `.blue` `.cyan` `.yellow` `.purple`, otherwise the theme's (`--mark`); drawn on click, or at once without `@n` |
| to adjust the theme | a `style.css` next to the markdown, loaded after the theme: `glitu init --css` writes one with every variable of the theme (colours, fonts, sizes), commented out |
| an icon | `:mdi:github:`: any [Iconify](https://icon-sets.iconify.design) icon (`collection:name`), downloaded at build |
| another font | `font: Inter`, `font-title: Poppins`, `font-mono: Fira Code` in the front matter (Google Fonts, downloaded at build) |
| two columns | `## Topic {.cols}`; a `::right::` line separates the left column from the right one (texts, code, diagram, image) |
| a quote | `## Topic {.quote}`: the first paragraph in large, the next one for the author |
| a key figure | `## Topic {.fact}`: the `###` title very large, the text as a caption |
| a statement | `## Topic {.statement}`: the text in large, centred; `{.center}`: just centred |
| a table of contents | `## Contents {.toc}`: the chapters, as links, generated |
| a transition, a backdrop | `## Topic {transition="zoom" background="#123"}` (colour, or image: `background="img/x.jpg"`), `class="x"`, `zoom="0.8"` |
| notes along the clicks | in `Note:`, `[click]` moves to the next click, `[click:3]` to click 3: the speaker view highlights the current passage |
| animated code | a ` ````md magic-move ` block (four backticks) holding several ` ``` ` blocks: one version per click, the words kept slide to their new place, the others fade out or appear |
| annotated code | at the end of the line, in a comment: `// [!code ++]` (added), `[!code --]` (removed), `[!code focus]` (the rest fades), `[!code highlight]`, `[!code error]`, `[!code warning]`; the annotation disappears from the code shown |
| code taken from a file | a line `<<< src/main.rs` (path relative to the markdown file), `#name` for a region (`// #region name` … `// #endregion`), `#L3-L8` for lines; then, if needed, the language and the attributes of the block (`title="…"`, `@2`); between two versions of an animated code, such a line imports one |
| animated slides | `{.animate}` on two slides that follow each other: the shared elements (same text, same image, same window title, or same `[text]{#name}`) slide from one to the other, the new ones fade in |
| zooming into a diagram | `## Detail {transition="dive:api"}`: the diagram of the previous slide grows around its element `api` (a skemo card) until it fills the stage, and the diagram of this slide comes out of it; the text of the panel fades out, then the new one in. A zone instead of an element: `dive:40 30 20 15` (x y width height, in % of the picture). `surface:api` zooms back out to the general diagram; ← plays either the other way |
| a visual at a forced place | `box="x y w h"`, in % of the area of the visual: ` ```skemo box="5 10 90 80" `, or `{box="…"}` after an image |
| a title slide | `## Title {.cover}` or `{.intro}`: the title in large, the 1st paragraph as subtitle, the next ones as byline |
| an interlude | `## Part {.section}` (without a number nor a change of chapter); `## Thank you {.end}` to finish |
| an image next to the text | `{.image-right}` or `{.image-left}`: the first image of the slide (or `image="img/x.jpg"`) covers one half |
| a web page | `## Demo {.iframe url="https://…"}`: full slide, or next to the text (`{.iframe-left}`, `{.iframe-right}`) |
| no margins | `## Topic {.full}`: the visual takes the whole slide under the header |
| two columns and a header | in `{.cols}`, what comes before a `::left::` line spans both columns |
| a logo | `logo: img/logo.png` in the front matter: an image of `img/` or `public/`, or an `https://` address; in the header of the template themes, in a corner for black and white; no `logo:`, no logo |
| a logo, a mention on every slide | `global-top: "© ACME"` (in front of the content, bottom right) or `global-bottom:` (behind, bottom left) in the front matter, in markdown: `![](img/logo.svg)` |
| several files | a line `<!-- include: chapters/deployment.md -->` inserts this file (path relative to the including file; its front matter is ignored, its images stay relative to the main file) |

Steps are integers; `@n` on a bullet, a `###` title, a paragraph, a code window, a formula or in
a skemo diagram means the same click. `@+n` counts from the previous click of the slide, `@2-4`
bounds the presence (visible from click 2 to 4), `@2:effect` chooses the appearance: `up` `down`
`left` `right` `grow` `shrink` `zoom` `red` `green` `blue` `dim`, or any fragment class of
reveal.js. A **highlight** adds to the appearance, for the time of its clicks: `@3-4:halo` (a
glow, once), `@3-4:chase` (a light running around), both out after 3 s at most, and
`@3-4:magnify` (the view grows around the element, then comes back); on a bullet
(`- Point @1 @3:halo`), a paragraph, a word (`[key]{@2-3:magnify}`), a code window
(` ```yaml @1 @4:chase `) or a skemo diagram node (`card#pg Base @1:up @3-4:halo`).
Colour: `--fx-color`, otherwise the theme's accent. The spaces before `: ; ! ? »` become
non-breaking (French typography). A line made of a single `=` (or `-`) underlines the text above
as a title, a markdown rule: in a formula over several lines, keep the `=` on the same line as a
term.

Code is highlighted at build with the Sublime Text grammars of [bat](https://github.com/sharkdp/bat)
(syntect, about 200 languages: Rust, Python, Go, TypeScript, Bash, YAML, TOML, Dockerfile, HCL,
SQL…; the fence names the language by name or extension); an unknown language stays plain text.
The colours are the theme's `--hl-*` variables (`--hl-keyword`, `--hl-string`, `--hl-title`…, in
`template.css`). A presentation written with reveal-md's rules keeps highlight.js's rendering.

## Settings and new slides

`glitu config` manages the front matter without editing it by hand: alone, it lists every setting
glitu reads (title, theme, navigation, logo, fonts, `global-top`, `source`, `glitu`…) with its
value in the presentation and what it does; `glitu config <key>` prints a value,
`glitu config <key> <value>` sets it after checking it (a built-in theme or a theme folder, `2d`,
a language tag, a version…; a missing logo file is only a warning), `glitu config --unset <key>`
removes it. The other lines of the file are kept as written. `-f` names the presentation (a file
or a folder; for several files, the first one holds the front matter).

### Adding slides

`glitu add <kind>` adds a slide filled with an example of its syntax, at the end of the
presentation (in its last file), before its closing `{.end}` slide if it has one:

| Kind | The example |
|---|---|
| `bullets` | points appearing click by click, a word circled |
| `code` | a code window, its lines highlighted click by click; `--annotated`: `[!code ++]`, `[!code --]`… |
| `magic-move` | animated code, three versions |
| `diagram` (`skemo`) | a skemo diagram, one card per click |
| `terminal` (`asciicast`, `cast`) | a terminal recording, one command per click: writes a sample `casts/<title>.cast` |
| `formula` (`math`) | LaTeX formulas, in the text and as the visual |
| `image` | an image beside the text; without `--src`, writes `img/placeholder.svg` |
| `video` (`movie`) | a video that stops at given times, one click per pause, a caption and a focus zone; without `--src`, writes `img/placeholder.webm` |
| `emoji`, `cols`, `quote`, `fact`, `statement`, `toc`, `title`, `interlude`, `end`, `iframe`, `animate`, `chapter` | the layout of that name |

Every kind takes `-t` (title), `--chapter <name>` (opens a chapter before the slide),
`--transition`, `--background`, `--no-notes`, `--dry-run` (prints the slide instead) and `-f`;
then its own options (`glitu add <kind> --help`): `--steps <n>` (number of clicks, 0: all at
once) and `--effect up|down|grow|…` (how each step comes in), `--layout panel|large|full` for the
slides with a visual, `--lang rust|python|javascript|yaml|bash` for code, `--dir right|down` for a
diagram, `-c <command>` (repeatable) and `--speed <chars per second>` for the typing of a terminal
recording, `--src` and `--layout right|left|panel|full` for an image, `--src` and `--pauses "0:12 1:05"` for a video, `--url` for a web page. The
recording written is a sample (`echo` prints its text, `ls` the folder): record the real one with
`asciinema rec`. No existing file is overwritten.

## Present

`S` opens the **speaker view**, a separate window: the slide as the audience sees it, the next
step, the notes (the passage of the current click highlighted, see `[click]`), the time and a
timer. The keys of the presentation work there: you can drive from this window. Under the
current slide, the timing of every slide; at the bottom: play/pause, the elapsed time, and, if slides have a
planned duration, the lead or delay on the schedule; `⋯` resets the timer or changes the layout.
The columns resize (drag the separators, `A−`/`A+` for the notes) and the browser keeps the
layout; the timer survives a reload or a closed speaker window.

**Planned durations** are written at the end of a slide's title: `## Demo ⏳95🏃3` — 95 seconds,
averaged over 3 rehearsals (`🏃` optional). The table under the current slide lists them with
the time spent on each slide.

**Rehearsals** (under `glitu server`): `⋯` → "Rehearse from the start" (or "from here") times each
slide; `↺` restarts the current slide, `⏸` pauses. "Finish" shows a table, slide by slide: the
time measured, the planned one, and what to do with it — 📊 average it with the previous
rehearsals (default), ✏️ replace the planned duration, 🗑️ skip; "All" sets every row at once.
glitu then writes the durations into the titles of the markdown (save your edits first: the
file is rewritten).

Under `glitu server`, **the open windows stay together**: changing slide on one moves the others
(projector, laptop, tablet). With `--allow 192.168.0.0/16`, opening on a phone the address shown
followed by `?remote` gives a **remote**: the slide, two big buttons and the notes; it joins the
presentation where it is (`?follow` does the same without the buttons).

`D` lets you **draw** on the slide (mouse, finger, stylus). The strokes stay with their slide
and are kept by the browser from one session to the next.

`?view=scroll` at the end of the address gives the **scroll view**: the slides one under the
other, at the width of the window, all steps shown, to read like a web page. A phone (window
narrower than 435 px) gets it by default, except as a remote; turned to landscape, it goes back
to the presentation (the page reloads, at the same slide). The finger, the wheel and the arrows
scroll the page.

## Export

```sh
glitu build [html] [files…|folder] [-o presentation.html] [--assets local|<url>]
glitu build site   [files…|folder] [-o dist]
glitu build pptx   [files…|folder] [-o presentation.pptx] [--images]   [--range 1-3,5] [--width 1920]
glitu build pdf    [files…|folder] [-o presentation.pdf]  [--clicks]   [--range …] [--width …]
glitu build png    [files…|folder] [-o presentation]      [--clicks]   [--range …] [--width …]
glitu build play   [files…|folder] [-o presentation.glituplay]          [--range …] [--width …]
```

Each format has its options and its help (`glitu build pptx --help`); an output without an
extension gets the format's. **The input**: without files, the current folder (a folder given
alone, the same): its `slides.md`, otherwise all its `.md` files merged in natural order
(`01-intro.md`, `02-deploy.md`… `10-end.md`), leaving out `README`, `CHANGELOG`, `LICENSE` and
the names starting with `_` or a dot; files named on the command line are merged in that order,
a `README.md` included. Only the first file's front matter counts (glitu warns about the others);
the files merged are listed. `glitu server` reads its input the same way, and takes a file added
to the folder.

`site` writes a **static website** (default `dist/`): `index.html`, the engine, the theme and the
resources, to serve with any web server; no browser needed.

For the other formats, a Chromium-based browser, without a window, replays the presentation. Each
slide is taken once all its steps have passed, or at each click with `--clicks` (PDF, PNG; always
for `play`); `--range` keeps some slides (numbers in reading order); `--width` sets the width of
the pictures in pixels (1920 by default).

**The browser** is, in this order: the one given by the `CHROMIUM` variable (a path, or a name in
the PATH); Chrome's headless shell, if glitu downloaded it; a browser installed on the machine —
Chrome, Chromium, Edge, Brave or Vivaldi, in the PATH then in their usual places (`/Applications`
on macOS, `Program Files` on Windows). Without any, the export stops and says what to do.
`--download-browser` then downloads Chrome's headless shell (Google's "Chrome for Testing" build,
a Chrome without a window, made for automation; about 100 MB, once): glitu first says what it
will download and where, and asks for a confirmation (`-y` accepts it without a question, for
scripts and CI). The archive is checked against the SHA-256 written in glitu, then unpacked into
`~/.cache/glitu/browser/` (`GLITU_BROWSER_DIR` chooses another folder); the next exports reuse
it. Deleting the folder removes it. Firefox is not supported: it no longer speaks the DevTools
protocol.

- **PowerPoint**: each element becomes an **editable** PowerPoint object — text boxes (fonts,
  colours, bold, links, bullets, original line breaks), shapes (backdrops, gradients, borders,
  rounded corners, shadows), skemo diagrams as groups of shapes and paths, images. Only the
  animated backdrop of the theme and what does not translate (canvas, video, formulas, element
  rotated or clipped by a frame) stay images. The steps become PowerPoint animations, one click
  per click of the slide, with the browser's durations: fade, coming from a side (`up` `down`
  `left` `right`), `zoom`, `grow` `shrink`, `dim`, disappearances (`@2-3`, `fade-out`), and in
  skemo diagrams the cards, their `up` and `zoom`, the arrows drawn (a wipe in their direction,
  the card at the end appearing after it). Colour highlights, a word inside a paragraph and the
  versions of animated code stay as they are at the last click. The notes go to the notes page.
  The theme's fonts (Lato, Inter, JetBrains Mono for "crystal") must be installed where the file
  is opened.
  `--images` falls back to one capture per slide (faithful, not editable).
- **PDF**: the page printed by the browser, as vectors (selectable text); with `--clicks`, one page
  per click (captures), an outline of the slide titles, page labels (the pages of one slide carry
  its number) and a `.pdfpc` file beside it with the speaker notes, for pdfpc.
- **PNG** (`--format png`): `NN.png` per slide (`NN-c.png` per click with `--clicks`) in the
  given folder (default `presentation/`).
- **play** (`--format play`): one `.glituplay` file for `glitu-play`, the native player (see
  below): an uncompressed zip archive of the pictures, their animations and the markdown.

## Slow computers

On a computer too slow for a browser (an old netbook), present pictures prepared on another
computer, exported at the width of the slow one's screen:

```sh
glitu build pdf --clicks --width 1024 -o talk.pdf   # then: pdfpc talk.pdf
glitu build play --width 1024 -o talk               # then: glitu-play talk.glituplay
```

- **PDF + [pdfpc](https://pdfpc.github.io/)**: one picture per click, the speaker view with the
  notes and a timer; no animation.
- **glitu-play** (`cargo build --release -p glitu-play`, 4 MB, no browser): shows the pictures and
  animates by software only what moves: the skemo arrows drawn step by step, the terminal
  recordings typing (recorded at export), the fades. The last picture of each animation is the
  browser's, pixel for pixel. A speaker view (`s`, `--speaker`) on the other screen: the current
  and next clicks, the notes, a timer. Keys: → Space Enter next, ← previous, Home End, `f` full
  screen, `t` timer, `q` quit; `--window`, `--start <slide>`, `--speaker`.
  `GLITU_PLAY_TRACE=1` prints, per click, the time from the key to the first frame, the frames
  and their spacing. On Wayland, the animations are drawn at the slide's own size and enlarged by
  the compositor (`wp_viewporter`), 3.5 times fewer pixels to move for 1024 px slides on a
  1080p screen; the last picture comes back at full size. `GLITU_PLAY_SMALL=0` draws them at the
  screen's size, `GLITU_PLAY_WINIT=1` goes through winit as on X11.

## Reopen a presentation

Each built page keeps a copy of its markdown, file by file (includes and imported code included),
in a `<script type="application/json" id="glitu-source">` that the browser does not run.
`glitu unpack page.html -o folder/` (or an `https://…` address) gets them back, to change the
presentation and build it again; `--force` replaces existing files. A `.glituplay` file keeps the
same copy: `glitu unpack talk.glituplay -o folder/`. The images and the terminals are not in the
copy: put them back next to it. `source: false` in the front matter builds the page (or the
file) without a copy.

## One page

`glitu build [files…|folder] [-o presentation.html] [--assets local|<url>]` builds one page.

`--assets local` (the default) writes everything into the page: engine, theme, fonts, images. It
opens with a double click and needs no network (about 2 MB): it is the one to carry around to
present.

`--assets <url>` only keeps the presentation in the page (text, diagrams, terminals, images); the
engine and the theme come from `<url>`: https://glitu.net/static, or any address where the
`assets` example has been published (`cargo run --release -p glitu --example assets -- folder`;
GitHub Pages works: it lets local pages load them).
The page aims at exact versions, `glitu/<version of the binary>/` and `themes/<name>/<version of
the theme>/`, and checks each file by its SHA-384 hash: a file republished differently makes the
page be refused rather than broken silently. A third-party theme (`theme: ./path`), which is not
published, stays written into the page. Presenting needs the network.

## Versions

The front matter can set the expected versions; without a version, it is the latest available,
that is the one of the binary and of the `themes/` folder used:

```yaml
glitu: 0.1.0               # version of the engine; another one is refused at build
theme: crystal:0.1.0       # official theme at this version (glitu.yaml, version); "crystal" alone: the latest
theme: ./my-themes/x       # third-party theme, path relative to the presentation
```

## Website (GitHub Pages)

`tools/site.sh [site/dist]` writes the website: the home page `site/index.html` (with its
`site/img/`), the documentation (`site/docs/*.md`, rendered by the `docs` example of
`crates/glitu`, with `sitemap.xml` and `robots.txt`), the demo, `tests/features/slides.md`, built as one standalone page (`demo.html`),
and `static/`, the assets of the light pages: `static/glitu/<version>/` (the engine, at the
version last released) and `static/themes/<name>/<version>/` (the `"version"` of each theme's
`glitu.yaml`, required), written by the `assets` example of `crates/glitu`. A deployment replaces
the whole site: only the versions of `main` stay published.
The CI builds it on every push on `main` and `tools/github-pages.sh` publishes it as the only commit
of the `gh-pages` branch, which GitHub Pages serves (Settings → Pages → Source: deploy from the
`gh-pages` branch) under the custom domain `glitu.net` (`www.glitu.net` redirects to it).

## Diagrams

A diagram block is rendered at every build by a program of your machine, its **engine**:

| Block | Engines (the first one by default) | The tool to install |
|---|---|---|
| ` ```skemo ` | `skemo` | [skemo](https://skemo.net/install) |
| ` ```mermaid ` | `chromium` (mermaid.js in the browser of the exports), `mmdc` | a Chromium-based browser; or [mermaid-cli](https://github.com/mermaid-js/mermaid-cli) |
| ` ```d2 ` | `d2` | [D2](https://d2lang.com/tour/install) |
| ` ```dot `, ` ```graphviz ` | `dot` | [Graphviz](https://graphviz.org/download/) |

A presentation without a ` ```d2 ` block never needs `d2`. When a tool is missing, `glitu build`
names the block and how to provide it; `glitu server` shows the error in the slide, in place of
the diagram. `glitu config --engines` lists the engines, the tool each one found and what
confines it; `glitu build -v` says which tools ran. For `mermaid.chromium`, glitu downloads
mermaid.js once (a pinned version, checked by its SHA-256) into its cache.

- **The tool**: `--tool skemo=/opt/skemo/bin/skemo`, otherwise `GLITU_TOOL_SKEMO`, otherwise
  `tools:` of your configuration, otherwise the `PATH`.
- **The engine**: `--engine mermaid=mmdc`, over the block (` ```mermaid engine=mmdc `), over the
  front matter (`engine.mermaid: mmdc`), over `engines:` of your configuration.
- **Options**, each level over the previous one: the theme, your configuration, the front matter
  (`d2.layout: elk`), the block (` ```d2 d2.layout=elk `, and ` ```skemo font-scale="2" `). Each
  engine declares its options and their values (`glitu config --engines d2.d2`): another one is
  an error.
- **Raw arguments** (`args`) are yours: `options: { d2: { args: [--pad, "20"] } }` in your
  configuration. Those of a presentation (` ```d2 args="--sketch" `, `d2.args:`) are refused,
  unless you allow them: flag by flag (`options: { d2: { allow-document-args: [--sketch] } }`), or
  for one build (`--allow-document-args`).

Your configuration is `~/.config/glitu/config.yaml` (`%APPDATA%\glitu` on Windows,
`~/Library/Application Support/glitu` on macOS, `$GLITU_CONFIG_DIR` to choose); an engine of your
own, or a built-in one changed, is a manifest in `engines/<language>.<engine>.yaml` next to it
(`glitu config --engines graphviz.dot` prints one to start from):

```yaml
files:
  allow: [~/talks/shared]        # folders a presentation may read besides its own
tools:
  skemo: /opt/skemo/bin/skemo
engines:
  mermaid: mmdc
options:
  d2:
    layout: elk
    allow-document-args: [--sketch]
limits:
  timeout: 30                    # seconds per diagram
```

**Security.** Building a presentation received from anyone runs no program and reads no file you
did not allow. A presentation reads only the files of its folder (inclusions, `<<<` code,
recordings, images, logo, theme), symbolic links resolved; another folder only with
`--allow-read DIR` or `files.allow`. It names engines, never programs nor paths; manifests and
tools come from your machine only (a tool inside the presentation's folder is refused). A tool
gets its source on its standard input, runs in the presentation's folder with a minimal
environment (no tokens of yours), a private temporary folder and a time limit; on Linux it is
confined by Landlock (it reads the presentation's folder, its own installation and the system,
writes only its temporary folder, opens no connection), on macOS by Seatbelt. Its SVG is
sanitized before it enters the page (no script, no external reference, identifiers made unique);
what it says is shown as text. The hostile presentations of `tests/security/` check it.

skemo blocks get the theme's `skemo.css`, completed by the presentation's: a `skemo.css` next to
the markdown adds its own types (`.pod { --c: #2563eb; --label: "POD"; }`), kept with the
markdown in the built page. A skemo warning fails the build, with the faulty line in the block.
Their clicks (`@n`), highlights and friezes below are skemo's; the other engines' diagrams appear
as a whole (` ```mermaid @2 `).

The text size for projection comes from the theme: `skemo: { fontScale: 1.35 }` in
`glitu.yaml` (skemo's `--font-scale` option, which its layout takes into account). A block can
change it: ` ```skemo font-scale="2" `. `skemo.css` only sets the colours and the labels of the
types. Aim at 16 px at least on screen: a full-width diagram fits in ~1150 × 590 px.
`diagram(fit="2:1")` fails the build if the proportions are more than 15 % away from the target
(`skemo fit` suggests settings; check that they do not lose size).

```pug
card#api.system Kubernetes API   @2:up        //- appears at click 2, coming up
  to(k1 k2).dashed orders         @4           //- arrows drawn at click 4
card#note.note Closed ports       @1-2         //- visible at clicks 1 and 2, then fades
card#p1.pod front                 @3:focus     //- emphasised at click 3
card#p2.pod(tag=":8080 @1") back               //- the tag appears at click 1
```

## Terminal

An asciicast recording (v2 or v3) is replayed segment by segment: the player types the commands,
stops on each marker and waits for the next click; a click during the typing ends the current
segment. The terminal emulation is done by `avt`, asciinema's. For a real recording:
`asciinema rec` with `rec.add_marker_key`, or add `[time, "m", "label"]` lines to the file.
`tools/build_cast.py` writes the one of the demo, with `tools/asciicast.py`.

## Theme

A theme is a folder `themes/<name>/`: its sheets and fonts, `glitu.yaml` (sheets to load, size of
the slides, margin, maximum magnification `maxScale` — 2 by default, 8 for the template themes,
which fill a large screen this way —, centring, arrows, progress, number, transition), and
depending on the theme `skemo.css` (the types of the diagrams). `glitu.yaml` also carries its `version` and, for
a third-party theme, the `"name"` the engine recognises. It is looked for next to the markdown,
then in the parent folders; the official themes are also embedded in the binary (`glitu init --list-themes`),
which uses them when no folder has the theme (written once into `~/.cache/glitu/themes-<hash>/`).

**A theme of your own**: `glitu init talk --theme mine` writes `talk/themes/mine/`, a copy of
crystal (fonts, `skemo.css`, `glitu.yaml` without a version) whose `theme.css` sets every variable
of the template, commented by role; the presentation uses it with `theme: ./themes/mine`. On an
existing presentation, it only writes the theme folder.

The themes of this repository, except "black" and "white" (reveal.js's), rest on a **shared template**
(`template: true`): its sheet, `glitu/template.css`, published with the engine, holds the whole
layout (header, panel, stage, cover, chapters, progress, layouts) and is only tuned through CSS
variables (palette, fonts, veils, `--tilt`: slant of the panel/stage separation, 2° clockwise);
the `theme.css` of a theme is limited to these variables. Its dynamic part (header, progress,
backdrop) is `crates/glitu-web/src/themes/template.rs`, and the backdrop is chosen by
`"backdrop"`: `crystal`, or a scene of `crates/glitu-web/src/themes/backdrops/` (`constellation`,
`paper`, `aurora`, `ascii`, `blueprint`, `iso`). It dresses the markup that the compiler
produces:

| Layout | Markup |
|---|---|
| Cover | `.cover`: `.eyebrow`, `h1`, `.subtitle`, `.byline` |
| Chapter | `.chapter`: `.num`, `h2`, `p` |
| Slide | `section[data-header="Section\|Topic"]` |
| Text | `.panel`: `h2`, `ul`; `.panel-steps` for blocks that replace each other |
| Visual | `.stage` (`.full` without text): `.diagram`, `.window` (code or `.cast-stage`), `.pan-track` |
| Caption | `.stage.full > .caption`: `h2` + `.caption-steps` |
| Layouts | `section.layout-cols > .cols > .col`; `section.layout-quote` (`fact`, `statement`, `center`, `toc`) `> .layout-body` |
| Formula | `.formula` (` ```math ` block), `math` (MathML) |

The rules shared by all themes (`.until` steps, `.mark` marks, `svg.drawing` drawing,
`.remote-pad` remote, `glitu/math.css` formulas) are in the engine's sheet; a theme can override
them.

The backdrop is unique for the whole presentation (`.tpl-backdrop`, under the slides): a slide
only asks it to run (`data-backdrop="animated"`, cover and chapters) or to stand still
(`data-backdrop="still"`); it slows down or starts again without changing image. The new
backdrops also fade on the text side, progressively: whole under the cover and the chapters, half
next to a panel, erased under a full-width text slide. The compiler marks each slide
`data-layout="full"` or `"panel"`, and the engine sets on the incoming slide the layout of the
previous one (`data-from-layout`): full screen → panel, the content fades out then the panel
slides in from the left; panel → full screen, one fades out then the other appears.

## Writing rules of reveal.js / reveal-md

A markdown written for reveal-md (slides separated by `---`, vertical ones by `----`, `Note:`,
`<!-- .slide: … -->`, `<!-- .element: … -->`, code ` ```php [1|3-5] `) is recognised by default
and rendered with reveal-md's "black" theme. The engine produces the same DOM as reveal.js:
vertical slides, backdrops (`data-background…`), arrows, progress bar, slide number,
transitions, centring. `tests/compat/` checks this rendering against that of reveal-md itself.

## Oracle

`tests/oracle/` keeps a capture of each state (slide × click) of the demo, `tests/features`: the
reference of the engine, which must stay identical to the pixel. `glitu oracle` is a development
command, left out of the released binary:

```sh
cargo build --release --features oracle
target/release/glitu server tests/features/slides.md -p 8123 &
target/release/glitu oracle check http://localhost:8123/ tests/oracle
```

See `tests/oracle/README.md`.

## Tree

```text
themes/crystal/            the default theme: palette, fonts, skemo.css
themes/constellation/ …    the other template themes (paper, aurora, ascii, blueprint, iso)
themes/black/, white/      reveal-md's default theme and its light counterpart (reveal.js, MIT)
crates/glitu/              the tool: site build, single page, server, export (PowerPoint, PDF,
                           PNG, play), includes, icons and fonts, oracle
crates/glitu-core/         the compiler: markdown → HTML, highlighting, formulas, skemo diagrams
crates/glitu-web/          the engine in the browser (wasm): navigation, steps, theme, diagrams,
                           terminal, code scrolling, animated code, animated slides, scroll view,
                           speaker view, remote, drawing
crates/glitu-play/         the native player for slow computers: the pictures of `build play`,
                           arrows, terminals and fades animated by software
crates/glitu/assets/       styles of the engine (glitu.css, template.css; reveal.css minified in
                           vendor/), survey of the screen for the PowerPoint export (export.js),
                           formula font
site/                      the website glitu.net: home page and its images (built by tools/site.sh)
tests/features/            the demo: glitu presented with glitu, then every syntax
tests/oracle/              reference captures of tests/features (crystal theme)
tests/compat/           markdown examples of reveal.js and reveal-md, reference rendering
tools/                     ci.sh (the pipeline), install.sh, site.sh, asciicast.py and
                           build_cast.py (terminal recordings)
docs/architecture/         the architecture diagram (skemo source and its image)
```
