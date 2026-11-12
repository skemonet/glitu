---
title: Getting started
description: Install glitu, create your first Markdown presentation, preview it with live reload and build it into a single HTML page.
order: 1
---

glitu turns a Markdown file into a presentation. One `slides.md` holds the text, the diagrams,
the code shown on screen and the terminal recordings; glitu previews it, builds it into a web page
and exports it to PowerPoint, PDF or images. The presentation engine is written in Rust and runs
in the browser as WebAssembly: there is no npm and no Node.

## Install

On Linux and macOS, the install script downloads the latest release into `~/.local/bin` and
verifies its SHA-256 checksum:

```sh
curl -fsSL https://raw.githubusercontent.com/skemonet/glitu/main/tools/install.sh | sh
```

You can also download a binary from the
[releases](https://github.com/skemonet/glitu/releases/latest): `glitu-linux-x86_64`,
`glitu-linux-arm64`, `glitu-macos-arm64`, `glitu-macos-x86_64` or `glitu-windows-x86_64.exe`.

To build from source, you need Rust 1.90 or later and its WebAssembly target:

```sh
rustup target add wasm32-unknown-unknown
git clone https://github.com/skemonet/glitu && cd glitu
cargo build --release          # target/release/glitu
```

Diagrams are drawn by tools of your machine: install the [skemo](https://skemo.net/install)
command line for ` ```skemo ` blocks, and `d2` or Graphviz's `dot` if you use them.

The exports to PowerPoint, PDF and PNG also need a Chromium-based browser (Chrome, Chromium, Edge,
Brave or Vivaldi). If none is installed, glitu can download one: see
[Building and exporting](/docs/publishing/#the-browser).

## Your first presentation

```sh
glitu init talk                # creates talk/slides.md
glitu server talk/slides.md     # preview at http://localhost:8000
```

Open <http://localhost:8000>: the page reloads every time you save `slides.md`. Press
<kbd>→</kbd> or <kbd>Space</kbd> to advance, <kbd>S</kbd> for the speaker view and <kbd>?</kbd>
for every shortcut.

A presentation is a front matter followed by chapters (`#`) and slides (`##`):

````markdown
---
title: Kubernetes in practice
subtitle: Deploy and expose an application
author: Ada Lovelace
theme: crystal
---

# Resources

## Deploying an application

### Deployment

- Number of replicas @1
- The ReplicaSet creates the pods @2

```yaml title="deployment.yml" @1
apiVersion: apps/v1
kind: Deployment
spec:
  replicas: 3
```

Note: What to say while this slide is shown.
````

Each slide has a text panel on the left and one visual on the right: here a code window. The
`@1` and `@2` markers make the elements appear click by click. [Writing slides](/docs/writing/)
explains the structure, [Steps and animations](/docs/steps/) the clicks.

## Share it

```sh
glitu build talk/slides.md -o talk.html    # one page, works offline
glitu build pptx talk -o talk.pptx         # an editable PowerPoint
glitu build pdf talk -o talk.pdf           # a vector PDF
```

The page built by `glitu build` contains everything: engine, theme, fonts and images. Send it by
email or put it on a USB key; it opens with a double click. See
[Building and exporting](/docs/publishing/) for the other formats.

## The commands

| Command | Purpose |
|---|---|
| `glitu init` | Create a presentation, a `style.css` to customize its theme, or a theme of your own |
| `glitu server` | Preview a presentation with live reload, and keep every open window in sync |
| `glitu build` | Build a self-contained HTML page, or with a format (`site`, `pptx`, `pdf`, `png`, `play`) a static website, PowerPoint, PDF, PNG images or a file for glitu-play |
| `glitu unpack` | Recover the Markdown source of a built page |

`glitu --help` lists the commands and `glitu <command> --help` describes their options. The exit
code is `0` on success, `1` when the command fails and `2` when the command line is invalid.

## Writing with an AI assistant

The repository includes a skill for Claude Code and other agents that read skills,
[`skills/glitu-presentation`](https://github.com/skemonet/glitu/tree/main/skills/glitu-presentation).
It teaches the syntax, the diagrams and the workflow. Copy the folder into `~/.claude/skills/`,
then ask for a presentation.
