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
target/release/glitu serve tests/features/slides.md    # http://localhost:8000
```

Diagrams are drawn by [skemo](https://github.com/skemonet/skemo), a git dependency fetched by
Cargo.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/skemonet/glitu/main/tools/install.sh | sh            # latest release into ~/.local/bin, SHA-256 checked
```

Or take a binary from the [releases](https://github.com/skemonet/glitu/releases): `glitu-linux-x86_64`,
`glitu-linux-arm64`, `glitu-macos-arm64`, `glitu-windows-x86_64.exe` (the latest one is always at
`https://github.com/skemonet/glitu/releases/latest/download/<file>`), or build it from source as
above. `glitu export` also needs Chromium.

```bash
glitu init talk                 # a sample presentation: talk/slides.md and talk/skemo.css
glitu serve talk/slides.md      # preview at http://localhost:8000, reloads at every save
glitu --help                    # the commands; glitu <command> --help: their options
```

## Releases

A `vX.Y.Z` tag (`-rc.N` for a pre-release) matching the version of `Cargo.toml` runs the pipeline
(`tools/ci.sh`: lints, tests, smoke test of the binary, the same locally and in the CI); when it
passes, `.github/workflows/release.yml` builds the binary for each platform and publishes a GitHub
release with the SHA-256 of each one.

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
themes are published by the maintainer on **static.glitu.net**, each at its version
(`glitu/0.1.0/`, `themes/crystal/0.1.0/`).

```sh
rustup target add wasm32-unknown-unknown     # once; skemo is fetched by Cargo (git dependency)
cargo build --release                         # builds target/release/glitu (wasm and skemo engines inside)

target/release/glitu init talk                 # a sample presentation in talk/ (overwrites nothing)
target/release/glitu serve                     # http://localhost:8000, reloads at every change
target/release/glitu serve --allow 192.168.0.0/16   # also for these addresses (CIDR, repeatable): phone, tablet
target/release/glitu build -o dist             # static site, to serve with any web server
target/release/glitu build --standalone -o talk.html   # one page, everything inside, no network
target/release/glitu build --web https://static.glitu.net -o talk.html   # light page (see below)
target/release/glitu export -o talk.pptx       # editable PowerPoint, notes included (see "Export")
target/release/glitu export -o talk.pdf        # vector PDF (selectable text)
target/release/glitu export -o captures/       # one PNG image per slide
target/release/glitu unpack talk.html -o talk/ # gets the markdown back from a built page (see "Reopen")
target/release/glitu themes                    # the themes embedded in the binary
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
the view. The mouse wheel with `"mouseWheel": true` in `glitu.json`.
Adding `?print-pdf` to the address puts one slide per page, all steps shown: then print to PDF
from the browser (or `glitu export -o x.pdf`). As in reveal.js: `data-autoplay` starts a video
or a sound on arriving on the slide, `data-src` loads a medium when the slide gets close,
`data-autoslide` (or `"autoSlide"` in ms in `glitu.json`) advances on its own, `"loop": true`
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
| an image | `![](img/photo.jpg)` |
| blocks that replace each other | several `### Title @n` in the slide |
| a full-width diagram | `## Topic {.large}`: the bullets become a caption above it |
| without text | a slide without `###` nor bullets: the visual takes the full width |
| another header | `## Topic {kicker="Template"}`; `{.compact}` tightens the bullets |
| big emojis | an ` ```emoji ` block, one per line: they replace each other on click (`😱 @1`) |
| the file shown | ` ```yaml file="roles/x/tasks/main.yml" `: an explorer lists the files seen so far |
| 2D navigation | `navigation: 2d` in the front matter: one chapter per column (← → and Space follow reading order, Page↓ Page↑ go from one chapter to another, ↑ ↓ within the chapter) |
| a formula | `$\frac{a}{b}$` in the text, `$$…$$` as a block, a ` ```math ` block as a visual (LaTeX, rendered as MathML at build) |
| a word that appears | `[text]{@2}`; `[text]{.class}` sets a class |
| a hand-drawn mark on a word | `[text]{.circle @2}`: `.underline` `.circle` `.box` `.highlight` `.strike`, colour `.red` `.green` `.blue` `.cyan` `.yellow` `.purple`; drawn on click, or at once without `@n` |
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
| a title slide | `## Title {.cover}` or `{.intro}`: the title in large, the 1st paragraph as subtitle, the next ones as byline |
| an interlude | `## Part {.section}` (without a number nor a change of chapter); `## Thank you {.end}` to finish |
| an image next to the text | `{.image-right}` or `{.image-left}`: the first image of the slide (or `image="img/x.jpg"`) covers one half |
| a web page | `## Demo {.iframe url="https://…"}`: full slide, or next to the text (`{.iframe-left}`, `{.iframe-right}`) |
| no margins | `## Topic {.full}`: the visual takes the whole slide under the header |
| two columns and a header | in `{.cols}`, what comes before a `::left::` line spans both columns |
| the logo of the header | `logo: img/logo.png` in the front matter replaces the theme's (an image of `img/` or `public/`); `logo: none` removes it |
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

Code is highlighted at build: YAML, HTML/XML, and the C-like languages (Rust, Python, Bash, Go,
Java, JavaScript, TypeScript, PHP…); the others stay plain text.

## Present

`S` opens the **speaker view**, a separate window: the slide as the audience sees it, the next
step, the notes (the passage of the current click highlighted, see `[click]`), the time and a
timer (a click pauses it; "Reset"). The keys of the presentation work there: you can drive from
this window.

Under `glitu serve`, **the open windows stay together**: changing slide on one moves the others
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
glitu export [slides.md] [-o presentation.pptx | x.pdf | folder/] [--format pptx|pdf|png]
             [--range 1-3,5] [--clicks] [--images]
```

Chromium (headless; the `CHROMIUM` variable for another one) replays the presentation. Each slide
is taken once all its steps have passed, or at each click with `--clicks`; `--range` keeps some
slides (numbers in reading order).

- **PowerPoint**: each element becomes an **editable** PowerPoint object — text boxes (fonts,
  colours, bold, links, bullets, original line breaks), shapes (backdrops, gradients, borders,
  rounded corners, shadows), skemo diagrams as groups of shapes and paths, images. Only the
  animated backdrop of the theme and what does not translate (canvas, video, formulas, element
  rotated or clipped by a frame) stay images. The notes go to the notes page. The theme's fonts
  (Lato, Inter, JetBrains Mono for "crystal") must be installed where the file is opened.
  `--images` falls back to one capture per slide (faithful, not editable).
- **PDF**: the page printed by Chromium, as vectors (selectable text); with `--clicks`, one page
  per click (captures) and an outline of the slide titles.
- **PNG**: `NN.png` per slide (`NN-c.png` per click with `--clicks`) in the given folder.

## Reopen a presentation

Each built page keeps a copy of its markdown, file by file (includes and imported code included),
in a `<script type="application/json" id="glitu-source">` that the browser does not run.
`glitu unpack page.html -o folder/` (or an `https://…` address) gets them back, to change the
presentation and build it again; `--force` replaces existing files. The images and the terminals
are not in the copy: put them back next to it. `source: false` in the front matter builds the
page without a copy.

## One page

`--standalone` writes everything into the page: engine, theme, fonts, images. It opens with a
double click and needs no network (about 2 MB): it is the one to carry around to present.

`--web <url>` only keeps the presentation in the page (text, diagrams, terminals, images); the
engine and the theme come from `<url>`: static.glitu.net, or any address where what
`glitu-site static` produces was published (GitHub Pages works: it lets local pages load them).
The page aims at exact versions, `glitu/<version of the binary>/` and `themes/<name>/<version of
the theme>/`, and checks each file by its SHA-384 hash: a file republished differently makes the
page be refused rather than broken silently. A third-party theme (`theme: ./path`), which is not
published, stays written into the page. Presenting needs the network.

## Versions

The front matter can set the expected versions; without a version, it is the latest available,
that is the one of the binary and of the `themes/` folder used:

```yaml
glitu: 0.1.0               # version of the engine; another one is refused at build
theme: crystal:0.1.0       # official theme at this version (glitu.json, "version"); "crystal" alone: the latest
theme: ./my-themes/x       # third-party theme, path relative to the presentation
```

## Publishing static.glitu.net (maintainer)

`glitu-site static [-o site/static]` (a maintainer tool, not in this repository) writes
`site/static/glitu/<version>/` (the engine, at the version of the repository) and
`site/static/themes/<name>/<version>/` (the `"version"` of each theme's `glitu.json`, required),
to publish as is on static.glitu.net (rsync, scp…). The versions already in `site/static/` stay:
publishing only adds. Only the maintainer publishes.

## Website (GitHub Pages)

`tools/site.sh [site/dist]` writes the website: the home page `site/index.html` (with its
`site/img/`) and the demo, `tests/features/slides.md`, built as one standalone page (`demo.html`).
The `Website` workflow (`.github/workflows/pages.yml`) builds it on every push and publishes it on
GitHub Pages from `main` (Settings → Pages → Source: GitHub Actions), under the custom domain
`glitu.net` (Settings → Pages → Custom domain; `www.glitu.net` redirects to it).

## Diagrams

skemo blocks are rendered on the fly, at every build, by the skemo engine itself: `glitu` is
linked to the `skemo-engine` library of [skemo](https://github.com/skemonet/skemo) (git
dependency, pinned to a revision), with the theme's `skemo.css` sheet, completed by the
presentation's: a `skemo.css` next to the markdown adds its own types
(`.pod { --c: #2563eb; --label: "POD"; }`), kept with the markdown in the built page. No external
binary nor cache: changing a diagram in `slides.md` redraws it at once (`glitu serve` reloads the
page). A skemo warning fails the build, with the faulty line in the block.

The text size for projection comes from the theme: `"skemo": { "fontScale": 1.35 }` in
`glitu.json` (skemo's `--font-scale` option, which its layout takes into account). A block can
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

A theme is a folder `themes/<name>/`: its sheets and fonts, `glitu.json` (sheets to load, size of
the slides, margin, maximum magnification `maxScale` — 2 by default, 8 for the template themes,
which fill a large screen this way —, centring, arrows, progress, number, transition), and
depending on the theme `logo.svg`, `skemo.css`. `glitu.json` also carries its `"version"` and, for
a third-party theme, the `"name"` the engine recognises. It is looked for next to the markdown,
then in the parent folders; the official themes are also embedded in the binary (`glitu themes`),
which uses them when no folder has the theme (written once into `~/.cache/glitu/themes-<hash>/`).

The themes of this repository, except "black", rest on a **shared template**
(`"template": true`): its sheet, `glitu/template.css`, published with the engine, holds the whole
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
transitions, centring. `tests/reveal-md/` checks this rendering against that of reveal-md itself.

## Oracle

`tests/oracle/` keeps a capture of each state (slide × click) of the demo, `tests/features`: the
reference of the engine, which must stay identical to the pixel. `glitu oracle` is a development
command, left out of the released binary:

```sh
cargo build --release --features oracle
target/release/glitu serve tests/features/slides.md -p 8123 &
target/release/glitu oracle check http://localhost:8123/ tests/oracle
```

See `tests/oracle/README.md`.

## Tree

```text
themes/crystal/            the default theme: palette, fonts, logo, skemo.css
themes/constellation/ …    the other template themes (paper, aurora, ascii, blueprint, iso)
themes/black/              reveal-md's default theme (reveal.js, MIT)
crates/glitu/              the tool: site build, single page, server, export (PowerPoint, PDF,
                           PNG), includes, icons and fonts, oracle
crates/glitu-core/         the compiler: markdown → HTML, highlighting, formulas, skemo diagrams
crates/glitu-web/          the engine in the browser (wasm): navigation, steps, theme, diagrams,
                           terminal, code scrolling, animated code, animated slides, scroll view,
                           speaker view, remote, drawing
crates/glitu/assets/       styles of the engine (glitu.css, template.css; reveal.css minified in
                           vendor/), survey of the screen for the PowerPoint export (export.js),
                           formula font
site/                      the website glitu.net: home page and its images (built by tools/site.sh)
tests/features/            the demo: glitu presented with glitu, then every syntax
tests/oracle/              reference captures of tests/features (crystal theme)
tests/reveal-md/           markdown examples of reveal.js and reveal-md, reference rendering
tools/                     ci.sh (the pipeline), install.sh, site.sh, asciicast.py and
                           build_cast.py (terminal recordings)
docs/architecture/         the architecture diagram (skemo source and its image)
```
