---
title: Layouts and media
description: Choose a slide layout with an attribute on its heading (columns, quote, key figure, cover, table of contents) and add images, videos, captions, focus zones, web pages and backgrounds.
order: 4
---

By default, a slide has a text panel on the left and a visual on the right. An attribute on the
`##` line chooses another layout.

## Layouts

| Layout | Write | Result |
|---|---|---|
| Large visual | `## Topic {.large}` | The visual takes the full width; the bullets become a caption above it |
| Full | `## Topic {.full}` | The visual takes the whole slide under the header, without margins |
| Two columns | `## Topic {.cols}` | A `::right::` line separates the left column from the right one |
| Quote | `## Topic {.quote}` | The first paragraph in large type, the next one as its author |
| Key figure | `## Topic {.fact}` | The `###` title very large, the text as its caption |
| Statement | `## Topic {.statement}` | The text in large type, centred; `{.center}` only centres it |
| Table of contents | `## Contents {.toc}` | The list of chapters, as links, generated |
| Title slide | `## Title {.cover}` or `{.intro}` | The title in large type, then a subtitle and a byline |
| Interlude | `## Part {.section}` | A section slide, without a number or a change of chapter |
| Closing slide | `## Thank you {.end}` | The final slide |

Two more attributes adjust a slide: `{kicker="Template"}` replaces the label of its header and
`{.compact}` tightens its bullets.

## Columns

````markdown
## Before and after {.cols}

What comes before `::left::` spans both columns.

::left::

### Before
- A monolith

::right::

```yaml
replicas: 3
```
````

Each column can hold text, code, a diagram or an image.

## Images

```markdown
![](img/architecture.png)
```

An image is the visual of its slide. To show it beside the text, covering one half of the slide,
use `{.image-right}` or `{.image-left}`; the first image of the slide is used, or the one given
with `image=`:

```markdown
## Our office {.image-right image="img/office.jpg"}

### A team of twelve
- Remote first
```

Images are embedded in the built page. Files placed in `public/` are copied as they are, which is
useful for pages loaded in an iframe.

Attributes between braces, right after the image, set its click, a class, a caption or focus
zones (see below):

```markdown
![Office](img/office.jpg){@2 .framed}
```

`box="x y w h"` forces the place and size of a visual, in % of the area of the visual: after an
image (`{box="5 10 90 80"}`) or on a block (` ```skemo box="5 10 90 80" `).

## Captions and sources

A caption is a short line under the visual, typically to credit a source. On an image (or a
video, or a terminal recording), write it as the title of the image, or with `caption=`:

```markdown
![Population](img/population.png "Source: [INSEE](https://insee.fr), 2024")
![Population](img/population.png){caption="Source: INSEE, 2024"}
```

On a code window, a diagram or a formula, add `caption="…"` to the block:

````markdown
```skemo caption="After the Kubernetes documentation"
diagram(dir="right")
  card#pod Pod
```
````

The caption is Markdown (links, emphasis), appears with its visual, and fits every layout: under
the visual by default and in `{.large}`, `{.cols}` and `{.full}`, laid on the bottom of the image
in `{.image-left}` and `{.image-right}`. Its colour and size follow the theme
(`.visual-caption` in your `style.css` to change them).

## Videos

A video is written like an image; the files go into `img/` too:

```markdown
![Demo](img/demo.mp4)
![Demo](img/demo.mp4 "Recorded on glitu 0.2"){autoplay muted loop}
```

| Attribute | Effect |
|---|---|
| `pauses="0:12 1:05"` | Plays when shown, stops at each time; the next click resumes it |
| `autoplay` | Starts when the slide is shown (without `pauses`) |
| `muted`, `loop` | Without sound, in a loop |
| `nocontrols` | Hides the play bar |
| `poster="img/x.jpg"` | The picture shown before the video starts |
| `@2` | The video appears at click 2 (and starts then, with `pauses`) |

`mp4`, `webm` and `ogv` files are shown as videos. A video loads only when its slide gets close,
and stops when you leave the slide. Browsers may refuse to start a video with sound before the
first key press or click on the page: add `muted` to a video that must start on the first slide.

### Pausing at precise times

`pauses` turns a video into a series of clicks. The video plays as soon as it is shown, stops
exactly at the first time, and waits; the next click plays it to the next time, and so on until
the end:

```markdown
## The rollout

### Watch the pods
- The new version starts @1
- The old pods stop @2

![](img/rollout.mp4){muted pauses="0:08 0:21"}
```

Here the video stops at 0:08 and 0:21; clicks 1 and 2 resume it, and the bullets marked `@1` and
`@2` appear at the same time. Each pause takes one click of the slide, counted from the click
where the video appears. Going back a click replays the previous part. Times are written in
seconds (`12`, `12.5`) or as `m:ss` and `h:mm:ss`.

## Focus on part of an image or a video

`focus="…"` draws attention to zones of an image or a video: circle them, darken the rest, or
zoom on them. A zone is written `x y width height`, as percentages of the picture measured from
its top-left corner, followed by what happens there; zones are separated by `;`:

```markdown
![](img/dashboard.png){focus="62 8 30 20 @1; 5 55 40 35 .box .red @2; 70 70 20 20 @3:magnify"}
```

| In a zone | Effect |
|---|---|
| nothing | A hand-drawn circle |
| `.circle`, `.box`, `.underline`, `.highlight`, `.strike` | The same hand-drawn marks as on words |
| `.red`, `.green`, `.blue`, `.cyan`, `.yellow`, `.purple` | The colour of the mark (otherwise the theme's) |
| `.spot` | The rest of the picture darkens |
| `@2`, `@2-3` | Shown at click 2, until click 3 (without a click: from the start) |
| `@3:magnify`, `@3-4:magnify` | The view zooms on the zone, then returns |
| `@3:halo`, `@3:chase` | The highlights of [Steps](/docs/steps/#highlights) |

A zone that only zooms (`@3:magnify` alone) draws nothing. The zones stay on the picture whatever
its size, including in `{.image-left}` and `{.image-right}`, where the image is cropped to fill
its half: measure them on the whole image.

To find the coordinates, open the image in any editor that shows the cursor position, and divide
by the width and the height of the image: a point at 620 × 80 px in a 1000 × 400 px image is at
`62 20`.

### On a video

On a video, a zone can follow the clicks as on an image, or be tied to a time range of the video:
`0:12-0:18` shows it from 0:12 to 0:18, `0:12` from 0:12 to the end, and `magnify` zooms on it
during that time:

```markdown
![](img/demo.mp4){muted pauses="0:12 0:30" focus="10 60 25 20 0:12-0:16; 55 10 40 40 .spot 0:30; 0 0 30 30 0:20-0:24 magnify"}
```

With `pauses`, a zone whose range starts at a pause is shown while the video waits there: here
the circle appears when the video stops at 0:12 and leaves four seconds after the click.

## Web pages

```markdown
## Live demo {.iframe url="https://example.com"}
```

The page fills the slide; `{.iframe-left}` and `{.iframe-right}` place it beside the text.

## Backgrounds

```markdown
## Results {background="#0b1020"}
## The team {background="img/team.jpg"}
```

A background is a colour or an image, which covers the slide. The template themes also have an
animated backdrop, shared by the whole presentation: see [Themes](/docs/themes/).

## Media and visibility

These attributes behave as in reveal.js:

| Attribute | Effect |
|---|---|
| `data-autoplay` | Starts a video or a sound when the slide is shown (on a video written as an image: `{autoplay}`) |
| `data-src` | Loads a medium only when its slide gets close |
| `data-visibility="hidden"` | Hides the slide |
| `data-state="x"` | Adds the class `x` to the page while the slide is shown |
