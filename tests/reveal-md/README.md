# Compatibility with the writing rules of reveal.js / reveal-md

`demo.md` gathers the official markdown examples of reveal.js (`examples/markdown.html`,
`examples/markdown.md`) and the demo of reveal-md (`demo/a.md`), written with reveal-md's rules:

| Rule | Example |
|---|---|
| slides separated by `---`, vertical slides by `----` | |
| speaker notes | `Note: …` |
| slide attributes | `<!-- .slide: data-background="#000000" -->` |
| element attributes | `- Item <!-- .element: class="fragment" data-fragment-index="2" -->` |
| lines highlighted by steps, shifted numbering | ` ```php [1\|3-5] `, ` ```c [287: 2\|4,6] ` |

`reference/` is the rendering of **reveal-md 6.1.4** (reveal.js 5.1.0) for this file, one capture
per state, going through the presentation with `Page↓`. It was produced outside the project:
`npx reveal-md demo.md --static out`, then `glitu oracle capture`. Its text stays as written for
that rendering (the title slide is in French).

`glitu` reads these rules by default (a `---` separator is enough to recognise them) and applies
reveal-md's "black" theme (`themes/black/`). The Rust engine reproduces the reference:

```sh
target/release/glitu serve tests/reveal-md/demo.md -p 8124 &
target/release/glitu oracle check http://localhost:8124/ tests/reveal-md/reference
```

Not supported: formulas (`$$…$$`, math plugin), mermaid diagrams, the `revealOptions` of the
front matter, the overview (O key) and the PDF export.
