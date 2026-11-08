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
per state, going through the presentation with `Page↓`. It is produced outside the project, with
the 2D slide transition of the "black" theme (reveal-md's 3D default rasterises the text of the
present slide differently), in the same browser as the checks:

```sh
cd "$(mktemp -d)" && cp -r <glitu>/tests/compat/{demo.md,img} . && echo '{"transition": "slide"}' > reveal.json
npx reveal-md@6.1.4 demo.md --static out && (cd out && python3 -m http.server 8125 &)
<glitu>/target/release/glitu oracle capture http://localhost:8125/ <glitu>/tests/compat/reference
```

`glitu` reads these rules by default (a `---` separator is enough to recognise them) and applies
reveal-md's "black" theme (`themes/black/`). The Rust engine reproduces the reference:

```sh
target/release/glitu server tests/compat/demo.md -p 8124 &
target/release/glitu oracle check http://localhost:8124/ tests/compat/reference
```

Not supported: formulas (`$$…$$`, math plugin), mermaid diagrams, the `revealOptions` of the
front matter, the overview (O key) and the PDF export.
