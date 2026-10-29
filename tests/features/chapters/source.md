# Source

This deck, written as files

## The markdown

````md magic-move file="slides.md"
```md
---
lang: en
title: glitu
subtitle: Presentations written in markdown, rendered by a Rust engine
---
```
```md
---
lang: en
title: glitu
subtitle: Presentations written in markdown, rendered by a Rust engine
author: The glitu project
date: 2026
font-title: Poppins
font-mono: Fira Code
global-top: "glitu · demo"
---

<!-- include: chapters/publishing.md -->

<!-- include: chapters/history.md -->

<!-- include: chapters/architecture.md -->
```
<<< ../slides.md md
````

### One markdown file

- The front matter: title, fonts, a mention on every slide
- One chapter per file @1
- The last slide @2


Note: The deck is one markdown file, and its front matter. [click] One include per chapter.
[click] And the closing slide. This window imports the real file: it cannot go stale.

## The diagram
### A skemo block

- The presentation, then the tool @1
- The compiler and skemo @2
- The engine in the browser @3
- What you get @4
- Where the engine is published @5

<<< architecture.md#L10-L42 pug file="chapters/architecture.md" lines="2-11|12-15,21-23|16-19|25-28|30-33" @1

Note: A skemo block: cards, zones and arrows, each with the click at which it appears. These are
the lines of the architecture slide, read from its file.

## The terminal

### An asciicast file

- An asciicast recording: one event per line @1
- A marker ends each segment: the player waits for the click @2

<<< ../casts/build.cast#L1-L8 json file="casts/build.cast" lines="1|2-8" @1

Note: The terminal is an asciicast file, written by `tools/build_cast.py`. Each `"m"` marker is
a pause until the next click.
