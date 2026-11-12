---
title: Code, diagrams and terminals
description: Show code windows with line highlighting and annotations, import code from files, draw skemo diagrams that build up click by click, and replay terminal recordings.
order: 5
---

## Code windows

A fenced code block is shown in a window. Its attributes give the window a title and highlight
lines click by click:

````markdown
```yaml title="deployment.yml" lines="2|4-6" @1
apiVersion: apps/v1
kind: Deployment
spec:
  replicas: 3
  template:
    spec: {}
```
````

| Attribute | Effect |
|---|---|
| `title="…"` | The title of the window |
| `lines="2\|4-6"` | Highlighted lines, one group per click; lines are numbered from 1 |
| `file="roles/web/tasks/main.yml"` | The file shown: an explorer lists the files seen so far in the presentation |
| `@n` | The window appears at click *n* |

Code is highlighted when the presentation is built, with the grammars of the bat project: about
200 languages (Rust, Python, Go, TypeScript, Bash, YAML, TOML, Dockerfile, HCL, SQL…), named
after the fence by name or extension (`rust` or `rs`). An unknown language is shown as plain
text. A theme sets the colours with its `--hl-*` variables (`--hl-keyword`, `--hl-string`…). Keep a window under fifteen lines: use `lines=` or
`[!code focus]` to guide the eye through longer code.

## Annotations

An annotation, in a comment at the end of a line, marks that line. It is removed from the code
shown.

```rust
let total = price * quantity;   // [!code --]
let total = price * quantity * (1.0 + tax);   // [!code ++]
println!("{total}");            // [!code focus]
```

| Annotation | Effect |
|---|---|
| `[!code ++]`, `[!code --]` | A line added or removed |
| `[!code focus]` | The other lines fade |
| `[!code highlight]` | A highlighted line |
| `[!code error]`, `[!code warning]` | A line marked as an error or a warning |

## Code from files

A line starting with `<<<` imports code from a file, relative to the Markdown file:

```markdown
<<< src/main.rs
<<< src/main.rs#setup            ← the region between // #region setup and // #endregion
<<< src/main.rs#L3-L8 rust title="main.rs" @2
```

After the path, you can give the language and the attributes of the code block. The imported code
is embedded in the built page, so `glitu unpack` recovers it too.

## Animated code

A ` ````md magic-move ` block (four backticks) holds several versions of the code, one per click:
the tokens that are kept slide to their new place. See
[Steps and animations](/docs/steps/#animated-code). Between two versions, a `<<<` line can import
one of them.

## Diagrams

A diagram block is rendered when the presentation is built, by a tool of your machine: `skemo`
for ` ```skemo `, mermaid.js in the browser of the exports (or `mmdc`) for ` ```mermaid `, `d2`
for ` ```d2 `, Graphviz's `dot` for ` ```dot `. Changing a diagram redraws it immediately under
`glitu server`, which shows a missing tool or a faulty line in the slide. `glitu config --engines`
lists the engines and the tools found; your choices (tool paths, default engines, options) go in
`~/.config/glitu/config.yaml`.

A presentation chooses an engine (` ```mermaid engine=mmdc `, `engine.mermaid: mmdc` in the front
matter) and its declared options (` ```d2 d2.layout=elk `), never a program, a path or raw
arguments: building a presentation received from anyone runs nothing you did not install, reads
only the files of its folder (another folder: `--allow-read DIR`), and the tools run confined
(Landlock on Linux, Seatbelt on macOS). Their SVG is sanitized before it enters the page.

Diagrams in the [skemo](https://skemo.net) language take step markers like the rest of the
slide (the other engines' diagrams appear as a whole: ` ```mermaid @2 `):

```pug
card#api.system Kubernetes API   @2:up        //- appears at click 2, entering from below
  to(k1 k2).dashed orders         @4           //- arrows drawn at click 4
card#note.note Closed ports       @1-2         //- visible at clicks 1 and 2
card#p1.pod front                 @3:halo      //- highlighted at click 3
card#p2.pod(tag=":8080 @1") back               //- its tag appears at click 1
```

A diagram with `diagram(layout="frieze" …)` is a frieze: the strip scrolls to centre the card of
each step. Any skemo error fails the build and names the faulty line of the block.

**Sizes.** Aim for text of 16 px or more on screen: a full-width diagram fits in about
1150 × 590 px. The theme sets the text size of the diagrams (`skemo: { fontScale: 1.35 }` in
its `glitu.yaml`), and a block can override it: ` ```skemo font-scale="2" `.
`diagram(fit="2:1")` fails the build if the proportions are more than 15 % away from the target.

**Types.** The theme's `skemo.css` defines the types of cards (`.system`, `.pod`…). A
`skemo.css` next to the presentation adds your own:

```css
.pod { --c: #2563eb; --label: "POD"; }
```

## Terminals

An [asciicast](https://docs.asciinema.org/manual/asciicast/v2/) recording (version 2 or 3) is
replayed as a terminal, one segment per click:

```markdown
![~/project](casts/build.cast)
```

The text of the image is the title of the window. The player types each command and stops at each
marker until the next click; a click during the typing ends the current segment. To add markers,
record with `asciinema rec` and its marker key (`rec.add_marker_key`), or add
`[time, "m", "label"]` lines to the file.
