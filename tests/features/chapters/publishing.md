# Publishing

One markdown file, four outputs

## Four ways to publish

### From one `slides.md`

- A standalone page: engine, theme and fonts inside, no network @1
- A light page: the engine and the theme come from the web @2
- A static site, for any web server @3
- An export: editable PowerPoint, vector PDF, PNG images @4

```bash title="terminal" lines="1|2|3|4" @1
glitu build -o talk.html
glitu build --assets https://glitu.net/static
glitu export --format site -o dist
glitu export -o talk.pptx
```

Note: Everything starts from the same markdown file. [click] One page to carry around, which
opens with a double click. [click] A light page that loads the engine from glitu.net. [click] A
static site, like Hugo. [click] Or an export, for those who need a file.
