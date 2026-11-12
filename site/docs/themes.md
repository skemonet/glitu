---
title: Themes
description: The built-in themes of glitu, customizing one with style.css, creating your own theme, the glitu.yaml settings and the compatibility with reveal.js and reveal-md.
order: 8
---

## Built-in themes

The official themes are built into the binary:

```sh
glitu init --list-themes
```

| Theme | Description |
|---|---|
| `crystal` | The default theme: a header, a text panel, a visual, an animated crystal backdrop |
| `constellation`, `paper`, `aurora`, `ascii`, `blueprint`, `iso`, `graphite` | The same layout with another backdrop and palette: stars, cut paper, aurora, amber ASCII, technical drawing, isometric, charcoal with a purple glow |
| `black`, `white` | The themes of reveal.js; `black` is the default for the reveal-md syntax |

Choose one in the front matter, `theme: paper`, or when creating the presentation:
`glitu init talk --theme paper`.

## Customizing a theme

```sh
glitu init talk --css
```

This writes `talk/style.css`, which lists every variable of the theme (colours, fonts, sizes,
veils), commented out. Uncomment and change the ones you need: the file is loaded after the theme.

```css
:root {
  --accent: #f97316;
  --mark: #f97316;
}
```

## Creating a theme

```sh
glitu init talk --theme mine
```

Any name that is not a built-in theme creates `talk/themes/mine/`, a copy of `crystal` to change
at will: its `theme.css` sets every variable of the template, grouped by role, and it comes with
its fonts, `skemo.css` and `glitu.yaml`. The presentation uses it with `theme: ./themes/mine`. In an
existing presentation, only the theme folder is created.

A theme is looked for next to the presentation, then in the parent folders; the built-in themes
are used when no folder has one with that name.

## glitu.yaml

Each theme has a `glitu.yaml`:

| Setting | Effect |
|---|---|
| `version` | The version of the theme, required for the official themes |
| `name` | The name the engine recognizes, for a third-party theme |
| `styles` | The stylesheets to load, in order |
| `template` | `true` when the theme uses the shared template |
| `backdrop` | The animated backdrop of a template theme: `crystal`, `constellation`, `paper`, `aurora`, `ascii`, `blueprint`, `iso` or `graphite` |
| `maxScale` | The maximum magnification of the slides: 2 by default, 8 for the template themes |
| `skemo.fontScale` | The text size of the diagrams |
| `mouseWheel`, `autoSlide`, `loop` | Change slides with the wheel, advance automatically (milliseconds), loop at the end |

It also sets the size of the slides, their margin, the centring, the arrows, the progress bar, the
slide number and the transition, as reveal.js does.

## The shared template

All the themes except `black` and `white` use the same template, `glitu/template.css`. It holds
the whole layout (header, panel, stage, cover, chapters, progress, layouts) and is tuned only
through CSS variables: palette, fonts, veils, and `--tilt`, the slant of the separation between
the panel and the stage. The `theme.css` of a template theme is limited to these variables.

The backdrop is shared by the whole presentation. It runs under the cover and the chapters
(`data-backdrop="animated"`), stands still elsewhere, and fades on the side of the text.

The template styles the markup that glitu produces:

| Element | Markup |
|---|---|
| Cover | `.cover` with `.eyebrow`, `h1`, `.subtitle`, `.byline` |
| Chapter | `.chapter` with `.num`, `h2`, `p` |
| Slide | `section[data-header="Section\|Topic"]` |
| Text | `.panel` with `h2`, `ul`; `.panel-steps` for blocks that replace each other |
| Visual | `.stage` (`.full` without text) with `.diagram`, `.window`, `.pan-track` |
| Layouts | `section.layout-cols > .cols > .col`, `section.layout-quote > .layout-body` |
| Formula | `.formula`, `math` |

## reveal.js and reveal-md compatibility

A Markdown file written for reveal-md is recognized automatically when it uses `---` as a slide
separator. glitu reads its rules and renders it with the `black` theme:

| Rule | Example |
|---|---|
| Slides and vertical slides | `---` and `----` |
| Speaker notes | `Note: …` |
| Slide attributes | `<!-- .slide: data-background="#000000" -->` |
| Element attributes | `- Item <!-- .element: class="fragment" data-fragment-index="2" -->` |
| Highlighted lines by steps | ` ```php [1\|3-5] ` |

The engine produces the same page structure as reveal.js: vertical slides, backgrounds, arrows,
progress bar, slide number, transitions and centring. In reveal-md files, glitu does not support the math plugin (`$$…$$`), mermaid diagrams, the
`revealOptions` of the front matter, the overview or the PDF export of reveal.js.
