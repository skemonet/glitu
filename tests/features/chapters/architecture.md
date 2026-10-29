# Architecture

Three crates, one diagram engine

## The open-source pieces {.large}

- The markdown goes through glitu; the engine runs it in the browser

```skemo
diagram(dir="right")
  card#deck.project(icon="folder-open") Your presentation
    li slides.md, chapters
    li terminals, images
    to(cli)

  zone#glitu Repository glitu                                       @1
    card#cli.crate(icon="terminal") glitu                           @1
      li build · serve · export
      to(core)
      to(files).dashed exports                                      @4
    card#core.crate(icon="cpu") glitu-core                          @2
      li markdown → HTML slides
      li code, formulas
      to(engine)
    card#web.wasm(icon="monitor") glitu-web                         @3
      li navigation, steps
      li speaker view, themes
      to(site page).dashed runs                                     @4

  card#engine.crate(icon="share-2") skemo                           @2
    li github.com/skemonet/skemo
    li diagrams, rendered at build

  zone#outputs.output What you get                                  @4
    card#site.output(icon="globe") Static site
    card#page.output(icon="file-down") Standalone page
    card#files.output(icon="presentation") PPTX · PDF · PNG

  card#static.static(icon="cloud") static.glitu.net                 @5
    li engine and themes
    li one version per folder
    to(web).dashed hosts                                            @5
```

Note: Your presentation is plain files. [click] The glitu tool reads them. [click] Its compiler,
glitu-core, draws the diagrams with the skemo engine, from its own repository. [click] glitu-web,
compiled to WebAssembly, presents them. [click] What you get. [click] And static.glitu.net hosts
each version of the engine and of the themes.
