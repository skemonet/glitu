---
name: glitu-presentation
description: Write a presentation (talk, course, demo, pitch) as a glitu markdown deck, with skemo diagrams, code that unfolds click by click, speaker notes and timings. Use when the user asks for slides, a deck, a talk or a presentation and glitu is available (a `glitu` binary, a `slides.md` with glitu front matter, or the user names glitu), or when editing an existing glitu deck.
---

# Writing a presentation with glitu

glitu turns one markdown file, `slides.md`, into a presentation: a Rust/WebAssembly engine, themes
with an animated backdrop, diagrams drawn by skemo, code windows, terminals, formulas. The deck is
plain text: write it, check it with `glitu build`, preview it with `glitu serve`.

## Workflow

1. **Frame the talk** before writing: audience, goal (what they should remember), duration, language.
   If the user gave none, assume a 20-minute technical talk and say so. Count about one slide per
   minute, less for dense diagrams.
2. **Outline**: 3 to 6 chapters (`#`), each with 2 to 6 slides (`##`). Show the outline to the user
   before writing every slide when the talk is long or the brief is vague.
3. **Start from the sample** when there is no deck yet: `glitu init <dir>` writes `<dir>/slides.md`
   (overwrites nothing); keep its front matter, replace its content. `--theme <name>` picks a theme
   (`glitu themes` lists them).
4. **Write the slides** (syntax below): one idea per slide, a visual on most of them, the clicks of
   the bullets in step with the clicks of the visual, notes for what is said rather than shown.
5. **Check**: `glitu build slides.md -o /tmp/check.html` must succeed. It compiles every skemo
   diagram and reports errors with their line; fix them all. Then suggest
   `glitu serve slides.md` (http://localhost:8000, reloads at every save; `S` opens the speaker view).
6. **Deliver**: `glitu build` (one page, works offline), `glitu export -o talk.pdf` / `talk.pptx`
   (needs a Chromium-based browser; `--download-browser` fetches one), `glitu export --format site`.

Always write in the user's language; the slides, notes and diagram labels too.

## The deck

````markdown
---
title: Kubernetes in practice        # the cover slide
subtitle: Deploy and expose an application
author: Ada Lovelace
date: October 2026
theme: crystal                       # glitu themes: crystal (default), constellation, paper, aurora, ascii, blueprint, iso, black, white
logo: img/logo.svg                   # optional: an image of img/ or public/, or an https:// URL
---

# Resources                          ← chapter, numbered 01, 02…  (`# End {-}`: no number)

Deploy, then expose                  ← the chapter's subtitle

## Deploying an application ⏳90     ← a slide (header "RESOURCES / Deploying…"); ⏳90 = planned 90 s

### Deployment                       ← title of the text panel, on the left

- Number of replicas @1              ← appears at click 1
- The ReplicaSet creates the pods @+1   ← one click after the previous one
- Rolling update @3:up               ← click 3, comes in from below

```yaml title="deployment.yml" lines="2|4-6" @1
apiVersion: apps/v1
kind: Deployment
spec:
  replicas: 3
```

Note: What to say. [click] Said at click 1, highlighted in the speaker view. [click] Click 2.
````

A slide is a text panel on the left (`###` title, bullets, short paragraphs) and **one visual** on
the right: a code block, a skemo diagram, an image, a terminal recording, a formula. No visual: the
text takes the whole width. No text: the visual does.

### Steps (clicks)

- `@n` on a bullet, a `###` title, a paragraph, a code block, a word or a diagram element: shown at
  click n. The same number means the same click everywhere in the slide.
- `@+n`: n clicks after the previous step. `@2-4`: visible from click 2 to 4. `@0-2`: from the start.
- Effects: `@2:up` (`down` `left` `right` `grow` `shrink` `zoom` `red` `green` `blue` `dim`).
- Highlights, added to a visible element: `@3-4:halo`, `@3-4:chase`, `@3-4:magnify`.
- Words: `[text]{@2}`; hand-drawn marks `[word]{.circle @2}`, `.underline`, `.box`, `.highlight`,
  `.strike`, colours `.red` `.green` `.blue` `.cyan` `.yellow` `.purple`.
- Several `### Title @n` in one slide: blocks that replace each other.

Keep the clicks of the bullets and of the visual aligned: bullet `@1` explains what appears at `@1`
in the diagram or the code.

### Code

- ` ```lang title="file.rs" lines="2-3|4|5" @1 `: a window; `lines` highlights a range per click
  (line numbers of the block, from 1; `|` separates the clicks).
- ` ```yaml file="roles/x/tasks/main.yml" `: an explorer shows the files seen so far.
- Annotations at the end of a line: `// [!code ++]`, `[!code --]`, `[!code focus]`,
  `[!code highlight]`, `[!code error]`, `[!code warning]`.
- `<<< src/main.rs#region` or `<<< src/main.rs#L3-L8`: code taken from a file.
- Code that evolves: a ` ````md magic-move ` block (four backticks) holding several ` ``` `
  blocks, one version per click.
- Keep a window under about 15 lines; split or use `lines=` / `[!code focus]` otherwise.

### Layouts (attributes on the `##` line)

| Need | Write |
|---|---|
| a wide diagram, bullets as a caption | `## Topic {.large}` |
| two columns | `## Topic {.cols}`, a `::right::` line between the columns |
| a quote | `## Topic {.quote}` (first paragraph large, next one the author) |
| a key figure | `## Topic {.fact}` (the `###` title very large) |
| one sentence, large | `## Topic {.statement}` |
| table of contents | `## Contents {.toc}` |
| an interlude, the end | `## Part {.section}`, `## Thank you {.end}` |
| image beside the text | `{.image-right}` / `{.image-left}` |
| a web page | `## Demo {.iframe url="https://…"}` |
| elements that move between two slides | `{.animate}` on both slides |
| another header label | `{kicker="Template"}`; tighter bullets `{.compact}` |

### Other content

- Terminal: `![window title](casts/demo.cast)` (asciicast; one command per click).
- Formula: `$E = mc^2$` inline, `$$…$$` as a block, a ` ```math ` block as the visual.
- Big emojis: an ` ```emoji ` block, one per line, `😱 @1`.
- Icons: `:mdi:github:` (any Iconify icon).
- Fonts: `font: Inter`, `font-title: …`, `font-mono: …` in the front matter (Google Fonts).
- Several files: `<!-- include: chapters/intro.md -->`.
- Timing: `⏳95🏃3` at the end of a title = 95 s planned, averaged over 3 rehearsals (`🏃` optional;
  glitu's rehearsal mode writes them).
- Adjusting the look: `glitu init <dir> --css` writes `style.css` with every variable of the theme;
  `glitu init <dir> --theme my-theme` creates a theme of one's own.

## Diagrams: skemo

A ` ```skemo ` block is the visual of the slide. Read [reference/skemo.md](reference/skemo.md)
before writing one: syntax, layouts (graph, timeline, frieze, mind map), types, steps, pitfalls.
The short form:

```skemo
diagram(dir="right")
  card#md.file(icon="file-text") slides.md        @1
    | text, code, diagrams
    to(tool)
  card#tool.tool(icon="terminal") glitu           @2
    | build · serve · export
    to(page) renders                              @3
  card#page.output(icon="presentation") Slides    @3
```

## Style rules

- One idea per slide; a title that says it (a claim, not a topic: "Pods are cattle" beats "Pods").
- 3 to 5 bullets of a few words; the details go into the notes, not on the slide.
- A visual on most slides; a diagram of 3 to 8 cards, revealed in the order the talk tells it.
- Every slide with clicks has notes with matching `[click]` markers.
- Open with the stakes, close with what to remember (`{.statement}` or `{.end}`).
- Do not paste long code: show the 5 lines that matter.

## Before handing over

- [ ] `glitu build slides.md -o /tmp/check.html` succeeds, without warnings left unexplained.
- [ ] Every `@n` in a slide has a counterpart (bullet ↔ visual) and the numbers have no gaps.
- [ ] Images and casts exist at their paths (relative to `slides.md`).
- [ ] The total of the `⏳` durations, if any, fits the slot.
