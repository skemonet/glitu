---
title: My presentation
subtitle: Written in markdown, presented by glitu
author: Your name
date: 2026
theme: crystal
---

<!-- Preview: `glitu serve`, then open http://localhost:8000 (it reloads at every save).
     Present: → or Space to advance, S for the speaker view, ? for every key.
     Publish: `glitu build -o talk.html` (one page, no network needed). -->

# Introduction

A chapter: numbered, with this line as its subtitle

## Steps

### Each click shows the next point

- Visible from the start
- Appears at the first click @1
- Then this one, coming up @2:up
- Appears and [word]{.circle @3} circled at click 3 @3

Note: The speaker notes, shown by the S key. [click] The first click. [click] The second one.

## Code

### Lines highlighted click by click

- The data @1
- The loop @2
- The result @3

```rust title="main.rs" lines="2-3|4|5" @1
fn main() {
    let words = ["markdown", "slides", "glitu"];
    let mut total = 0;
    for w in words { total += w.len(); }
    println!("{total} letters");
}
```

# Diagrams

Drawn by skemo, step by step

## From markdown to the screen

### A skemo block

- The source @1
- The tool @2
- What you get @3

```skemo
diagram(dir="right")
  card#md.file(icon="file-text") slides.md          @1
    | text, code, diagrams
    to(tool)
  card#tool.tool(icon="terminal") glitu             @2
    | build · serve · export
    to(page)
  card#page.output(icon="presentation") Your slides @3
    | a page, a PowerPoint, a PDF
```

## Statement {.statement}

One idea per slide, [one click]{.highlight @1} at a time.

## Thank you {.end}

Questions?
