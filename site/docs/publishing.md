---
title: Building and exporting
description: Build a self-contained HTML page or a lightweight one, export to PowerPoint, PDF, PNG or a static website, pin versions and recover the Markdown source of a page.
order: 7
---

## A self-contained page

```sh
glitu build                          # slides.md → presentation.html
glitu build talk/slides.md -o talk.html
```

The page contains everything: the engine, the theme, the fonts, the images, the diagrams and the
terminal recordings (about 2 MB). It opens with a double click and needs no network, which makes
it the safest file to carry to a conference.

## A lightweight page

```sh
glitu build --assets https://glitu.net/static -o talk.html
```

With `--assets`, the page keeps only the presentation (text, diagrams, terminals, images) and
loads the engine and the theme from the given address. It requires network access when you
present.

The page requests exact versions, `glitu/<version>/` and `themes/<name>/<version>/`, and checks
every file against its SHA-384 hash: a file published again with different content is refused
rather than silently breaking the page. A local theme (`theme: ./path`) is not published, so it
stays embedded in the page.

https://glitu.net/static hosts the engine and the official themes. To host them yourself, write
them into a folder and publish it, for instance with GitHub Pages:

```sh
cargo run --release -p glitu --example assets -- static/
```

## Export

```sh
glitu build pptx -o talk.pptx             # editable PowerPoint
glitu build pdf -o talk.pdf               # vector PDF
glitu build png -o images                 # one PNG image per slide
glitu build site -o dist                  # static website
glitu build play -o talk.glituplay        # for glitu-play, on a slow computer
```

The format comes after `build`, with its own options (`glitu build pdf --help`). These options
apply to `pptx`, `pdf`, `png` and `play` (`--clicks` and `--images` to some of them):

| Option | Effect |
|---|---|
| `--clicks` | One page, slide or image per click instead of one per slide |
| `--range 1-3,5` | Only these slides, numbered in reading order |
| `--images` | PowerPoint only: one picture per slide, exact but not editable |
| `--width 1024` | Width of the pictures in pixels (1920 by default): the screen's own, for a slow computer |

### PowerPoint

Each element becomes an editable PowerPoint object: text boxes with their fonts, colours, links
and bullets; shapes with their fills, gradients, borders, rounded corners and shadows; skemo
diagrams as groups of shapes and paths; images. The speaker notes go to the notes pages. Only
what PowerPoint cannot represent stays a picture: the animated backdrop, canvases, videos,
formulas, and rotated or clipped elements.

The fonts of the theme (Lato, Inter and JetBrains Mono for `crystal`) must be installed on the
computer that opens the file.

### PDF

The PDF is printed by the browser as vectors, so its text can be selected and searched. With
`--clicks`, it has one page per click, an outline of the slide titles, and page labels: the pages
of one slide carry its number. A `.pdfpc` file is written beside it, with the speaker notes, for
the [pdfpc](https://pdfpc.github.io/) presenter (see [Slow computers](/docs/slow-computers/)).

### PNG

`glitu build png` writes `NN.png` for each slide, or `NN-c.png` for each click with `--clicks`, into
the output directory (`presentation/` by default).

### Native player

`glitu build play` writes one `.glituplay` file for `glitu-play`, which presents without a
browser: one picture per click, the description of what moves, and the Markdown (`glitu unpack`
gets it back). See [Slow computers](/docs/slow-computers/).

### Static website

`glitu build site` writes `index.html`, the engine, the theme and the resources into a directory
(`dist/` by default), ready for any web server. It needs no browser.

## The browser

The exports to PowerPoint, PDF and PNG replay the presentation in a Chromium-based browser,
without a window. glitu looks for one in this order:

1. the `CHROMIUM` environment variable, a path or a command name;
2. Chrome's headless shell, if glitu downloaded it;
3. Chrome, Chromium, Edge, Brave or Vivaldi, in the `PATH` and then in their usual locations
   (`/Applications` on macOS, `Program Files` on Windows).

If none is found, `--download-browser` downloads Chrome's headless shell, Google's "Chrome for
Testing" build made for automation (about 100 MB, once). glitu first says what it will download
and where, and asks for confirmation; `-y` accepts without asking, for scripts and CI. The archive
is verified against the SHA-256 checksum built into glitu, then installed into
`~/.cache/glitu/browser/` (`GLITU_BROWSER_DIR` changes this directory). To uninstall it, delete
the directory. Firefox is not supported, because it no longer implements the DevTools protocol.

## Versions

The front matter can require versions. Without a version, glitu uses the version of the binary
and of the theme it finds.

```yaml
glitu: 0.1.0               # the engine version; any other version is refused at build time
theme: crystal:0.1.0       # an official theme at this version
theme: ./my-themes/x       # a local theme, relative to the presentation
```

## Recovering the source

Every page built by glitu embeds a copy of its Markdown, file by file, including included files
and imported code. `glitu unpack` writes it back:

```sh
glitu unpack talk.html -o talk/
glitu unpack https://glitu.net/demo.html -o demo/
```

`--force` overwrites existing files. Images and terminal recordings are not embedded: copy them
back next to the Markdown. `source: false` in the front matter builds a page without the copy.
