# History

From the desktop to the browser

## Slides, a short history

```skemo
diagram(layout="frieze" start="1985" end="2027" scale="year")
  card#desktop.era(start="1987" end="2026") Desktop applications
  card#browser.era(start="2011" end="2026") In the browser                 @3
  card#powerpoint.event(date="1987-04") PowerPoint
    | slides on a computer
  card#beamer.event(date="2003") LaTeX Beamer                              @1
    | slides as source code
  card#keynote.event(date="2003-01") Keynote                               @2
  card#reveal.event(date="2011") reveal.js                                 @3
    | HTML and JavaScript
  card#slidev.event(date="2021") Slidev                                    @4
    | markdown, Vue, npm
  card#glitu.now(date="2026") glitu                                        @5
    | markdown, Rust, no npm
```

Note: PowerPoint opens the story. [click] Beamer turns slides into source code. [click] Keynote.
[click] reveal.js brings them to the browser. [click] Slidev writes them in markdown.
[click] glitu keeps the markdown and drops npm.
