---
title: Writing slides
description: The structure of a glitu presentation in Markdown: front matter, chapters, slides, text panels, speaker notes, formulas, icons, fonts and multi-file decks.
order: 2
---

A presentation is a single Markdown file, usually `slides.md`. Images go into `img/`, terminal
recordings into `casts/`, and larger decks can be split into several files.

## Front matter

The front matter, between two `---` lines at the top of the file, describes the presentation. It
also produces the cover slide.

```yaml
---
title: Kubernetes in practice      # cover title and page title
subtitle: Deploy and expose an application
author: Ada Lovelace
date: October 2026
description: A hands-on introduction to Kubernetes deployments   # for search engines
lang: en                           # language of the page
theme: crystal                     # see Themes
logo: img/logo.svg                 # in img/ or public/, or an https:// URL
navigation: 2d                     # one column per chapter (optional)
---
```

| Key | Effect |
|---|---|
| `title`, `subtitle`, `author`, `date` | The cover slide; `title` is also the title of the page |
| `description` | The summary shown by search engines and link previews (defaults to `subtitle`) |
| `lang` | The language of the page |
| `theme` | The theme, optionally at a version: `crystal:0.1.0`, or a local theme: `./themes/mine` |
| `logo` | A logo in the header (template themes) or in a corner (`black`, `white`) |
| `global-top`, `global-bottom` | A mention on every slide, in Markdown: `global-top: "© ACME"` |
| `font`, `font-title`, `font-mono` | Fonts from Google Fonts, downloaded at build time |
| `navigation: 2d` | Two-dimensional navigation: one column per chapter |
| `source: false` | Do not embed the Markdown source in the built page |
| `glitu` | The engine version the presentation requires, such as `0.1.0` |

## Chapters and slides

````markdown
# Resources                    ← chapter, numbered 01, 02…

Deploy, then expose            ← subtitle of the chapter slide

## Deploying an application    ← a slide; its header reads "RESOURCES / Deploying…"

### Deployment                 ← title of the text panel

- Number of replicas
- Rolling updates

```yaml title="deployment.yml"
replicas: 3
```
````

- A level-1 heading starts a **chapter** and produces a chapter slide. `# End {-}` creates a
  chapter without a number.
- A level-2 heading starts a **slide**. Its header shows the chapter and the slide title.
- A level-3 heading is the title of the **text panel**, on the left.
- The first visual of the slide (code, diagram, image, terminal, formula) goes on the right. A
  slide without a visual gives the full width to its text; a slide without text gives it to its
  visual.

Several `### Title @n` headings in the same slide are blocks that replace each other, click after
click.

## Speaker notes

A paragraph starting with `Note:` holds the speaker notes. They appear in the speaker view
(<kbd>S</kbd>), not on the slide.

```markdown
Note: Start with the problem. [click] Then show the manifest. [click:3] Finally, the result.
```

`[click]` moves to the next click and `[click:3]` to click 3: the speaker view highlights the
passage that matches the current click.

## Text

| To get | Write |
|---|---|
| a word that appears on click | `[text]{@2}` |
| a hand-drawn mark | `[word]{.circle @2}`: `.underline`, `.circle`, `.box`, `.highlight`, `.strike` |
| a coloured mark | add `.red`, `.green`, `.blue`, `.cyan`, `.yellow` or `.purple` |
| a CSS class | `[text]{.class}` |
| an icon | `:mdi:github:`: any [Iconify](https://icon-sets.iconify.design) icon, as `collection:name` |
| big emojis | an ` ```emoji ` block, one per line; `😱 @1` replaces the previous one at click 1 |

Marks are drawn when their click comes, or immediately without `@n`. Their colour defaults to the
theme's (`--mark`). The spaces before `: ; ! ? »` become non-breaking, as French typography
requires.

## Formulas

Formulas are written in LaTeX and rendered as MathML when the presentation is built, so the page
needs no script to display them.

```markdown
The energy $E = mc^2$ inline, and as a block:

$$\int_0^1 x^2 \, dx = \frac{1}{3}$$
```

A ` ```math ` block is a formula used as the visual of the slide. In a formula written over
several lines, keep `=` on the same line as a term: a line made of a single `=` is a Markdown
heading underline.

## Fonts and logos

```yaml
font: Inter
font-title: Poppins
font-mono: Fira Code
logo: img/logo.svg
global-top: "© ACME 2026"
```

Google Fonts are downloaded at build time and embedded in the page. The logo must be in `img/` or
`public/` (both are copied with the presentation) or be an `https://` URL. `global-top` is shown
in front of the content, bottom right; `global-bottom` behind it, bottom left.

## Splitting a presentation into files

```markdown
<!-- include: chapters/deployment.md -->
```

The line is replaced by the content of the file, relative to the file that includes it. The front
matter of an included file is ignored, and its images stay relative to the main file.

## The reveal-md syntax

A Markdown file written for reveal.js or reveal-md is recognized automatically and rendered with
the `black` theme: slides separated by `---`, vertical slides by `----`, `Note:`, and the
`<!-- .slide: … -->` and `<!-- .element: … -->` comments. See [Themes](/docs/themes/#reveal-js-and-reveal-md-compatibility).
