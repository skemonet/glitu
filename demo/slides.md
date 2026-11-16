---
lang: en
title: glitu
subtitle: Presentations in markdown. No mouse was harmed.
author: The glitu project
font-title: Poppins
font-mono: Fira Code
logo: img/glitu-logo.svg
---

## Write text {.animate .statement}

[Write text.]{#write}

## Get slides {.animate .statement}

[Write text.]{#write}

[Get slides.]{#get .highlight}

## That's it {.animate .statement}

[Write text.]{#write}

[Get slides.]{#get}

[That's the whole trick.]{#trick .underline @1}

## You write this {.animate}

`````md magic-move title="slides.md"
````md
---
title: My talk
---
````
````md
---
title: My talk
---

## Hello

- Markdown in @1
- Slides out @2
````
````md
---
title: My talk
---

## Hello

- Markdown in @1
- Slides out @2

## How

```skemo
card#me You  @1
  to(talk) write
card#talk A talk  @2
```
````
<<< slides.md#L1-L9 md
`````

### A text file

- A title
- A slide, its clicks: `@1`, `@2` @1:up
- A diagram, in text too @2:up
- This very deck: the same, with a logo @3:up

Note: A presentation is a text file. [click] A slide, and the clicks. [click] A diagram, written
as text. [click] And this deck's own front matter, read from its file: it shows itself, turtles
all the way down.

## Diagrams, in plain text {.animate .cols}

::left::

````pug magic-move title="slides.md"
<<< slides.md#L92-L94 pug
<<< slides.md#L92-L97 pug
<<< slides.md#L92-L105 pug
````

::right::

```skemo
diagram(dir="down")
  card#md.file(icon="file-text") slides.md
    | markdown
    to(cli)  @1
  card#cli.tool(icon="terminal") glitu  @1:zoom @3-4:halo
    | one command
    to(web)  @2
    to(pdf)  @2
    to(pptx)  @2
  zone#out.output What you get  @2:up
    card#web.output(icon="globe") A web page
    card#pdf.output(icon="file-down") A PDF
    card#pptx.output(icon="presentation") A PowerPoint
      | for your boss
```

Note: The window slides aside: on the left the diagram's text, on the right the diagram. [click]
glitu, and its arrow. [click] What comes out, the PowerPoint included. [click] And the light on
what matters.

## A terminal that waits for you

### `glitu build`

- One page, nothing else needed @1:up
- A PDF @2:up
- A PowerPoint, as promised @3:up

![~/talks/demo](casts/build.cast)

Note: The player types, then waits for the click. [click] The page. [click] The PDF. [click] The
PowerPoint.

## Your videos, on your click {transition="zoom"}

### Look closer

- Nothing to see here…
- …or is there? @1:up

![Big Buck Bunny](img/bunny.webm "*Big Buck Bunny*, © Blender Foundation, [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/)"){muted nocontrols pauses="0:03.8" focus="58 28 28 48 0:04.2-0:08 magnify"}

Note: The film stops by itself, all innocent. [click] Someone was watching: the view zooms in on
the culprit.

## Eight themes, one line {.large transition="zoom"}

![The same slide in each theme](img/themes.webp "`theme: paper` in the front matter")

Note: The same slide in eight themes: one line of the front matter.

## And plenty more {.statement}

We could list it all. Or you could [read]{.underline @1} the [fine]{.circle @2} [manual]{.highlight @3}.

Note: Speaker view, exports and the rest: glitu.net/docs.

## Your turn {.end transition="zoom"}

glitu.net

[Free and open source]{@1:up}
