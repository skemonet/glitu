# Publishing

One markdown file, four outputs

## Four ways to publish

### From one `slides.md`

- A static site, for any web server @1
- A standalone page: engine, theme and fonts inside, no network @2
- A light page: the engine and the theme come from the web @3
- An export: editable PowerPoint, vector PDF, PNG images @4

```bash title="terminal" lines="1|2|3|4" @1
glitu build -o dist
glitu build --standalone -o talk.html
glitu build --web https://static.glitu.net -o talk.html
glitu export -o talk.pptx
```

Note: Everything starts from the same markdown file. [click] A static site, like Hugo.
[click] One page to carry around, which opens with a double click. [click] A light page that
loads the engine from static.glitu.net. [click] Or an export, for those who need a file.
