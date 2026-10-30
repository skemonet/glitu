# skemo diagrams in glitu

skemo (https://skemo.net) describes a diagram as text: an indented, Pug-like file with a few tags.
glitu renders a ` ```skemo ` block at build time; `glitu build` reports its errors with the line.

## Tags

| Tag | Role |
|---|---|
| `diagram(...)` | optional root: options for the whole diagram, everything else indented under it |
| `title` | a title above the drawing, written right under `diagram`; `\|` lines = subtitles, `\|.pill x` = pills |
| `card#id.type` | a card: text after the tag = its name |
| `cards#id.type(count="4")` | a pile of cards (several instances), `×4` badge |
| `zone#id.type` | a group framing cards and zones indented under it |
| `stack` | an invisible group laying its children one after the other |
| `to(target).type label` | an arrow, written **inside its source card** |
| `\| text` | a line under a card's name (or a zone's description, a title's subtitle) |

Indent with 2 spaces, never tabs. `//-` starts a comment.

```skemo
diagram(dir="down")
  title Online shop
    | Production, eu-west-3
  card#web.app(icon="globe" label="Frontend" tag="team A") Storefront
    | React, served by the CDN
    to(api) HTTPS
  zone#back Backend
    | private subnet
    card#api.service(icon="server") API
      | **8080** · `shop/api:2.3`
      to(db)
    card#db.data(icon="database") Orders
```

## Cards and zones

- `#id`: required when an arrow targets the card; unique.
- `.type`: one or more classes (colour, badge). Types styled by glitu's themes: `.app` `.service`
  `.data` `.user` `.system` `.cluster` `.node` `.pod` `.svc-network` `.agent` `.tool` `.file`
  `.output` `.workstation` `.milestone`. Other names work and get the default colour; a deck can
  style its own in a `skemo.css` next to `slides.md` (`.pod { --c: #2563eb; --label: "POD"; }`).
- Built-in classes: `.note` (dotted annotation), `.chip` (one-line pill), `.info` (small box),
  `.wide` (spans the width, like a network bar), `.bare` zone (no frame), `.ok` `.warn` `.error`
  (state mark and colour, on a card or a line), `.legend` card (each `|.class text` line shows a
  sample of that class), `.text` card (plain text, e.g. a footnote).
- Attributes, one group per tag: `icon="database"` (Lucide icon names, kebab-case: `server`,
  `user`, `cloud`, `cpu`, `terminal`, `file-text`, `folder-open`, `globe`, `lock`…, or an image
  path `img/logo.svg`), `label="RDS PostgreSQL"` (category above the name), `tag="8080"` (chip),
  `href="https://…"`, `at="s"` (dock outside the drawing: `n` `s` `e` `w` `ne` `nw` `se` `sw`),
  `wrap="3"` (a zone: at most 3 children per row).
- Lines: `| text` with Markdown emphasis (`**bold**`, `*italic*`, `` `mono` ``, `==mark==`);
  `|.warn text` for a state; a line ending with `|` is a table row (Markdown table, `|---|`
  separator for a header).

## Arrows

- `to(b)`, `to(a b c)` (several targets), with a label after it: `to(db) SQL`.
- Classes: `.dashed`, `.link` (dotted, for notes), `.both` (two heads), or any type styled in CSS.
- `side="right-left"` (exit-entry sides), `via="x,y x,y"` (waypoints), `join="trunk"` (one trunk
  that branches to the targets), `constraint="false"` (drawn but ignored by the layout: cross links).
- The target must exist: an unknown id is a blocking error.

## Layouts

`diagram(dir="down")` (default) / `right` / `up` / `left` lays out a graph from the arrows.
Prefer `dir="right"` on slides (wide screen) for chains of 3 to 5 cards, `down` for hierarchies.

**Timeline** (planning; top-level zones are lanes):

```skemo
diagram(layout="timeline" start="2026-10" end="2027-03" scale="month")
  zone#dev.lane Engine
    card#e0(start="2026-10-01" end="2026-11-15") Prototype
      to(v1)
  zone#rel.lane Releases
    card#v1.milestone(date="2026-12-01") v1.0
```

`scale`: `day` `week` `month` `quarter` `year`; a card has `date` (milestone) or `start`/`end`
(period). Dates `YYYY`, `YYYY-MM`, `YYYY-MM-DD`; `end` is included.

**Frieze** (history, long periods; negative years = BC, no year 0):

```skemo
diagram(layout="frieze" start="1985" end="2027" scale="year")
  card#desktop.era(start="1987" end="2026") Desktop applications
  card#ppt.event(date="1987-04") PowerPoint                 @1
    | slides on a computer
  card#reveal.event(date="2011") reveal.js                  @2
```

`scale`: `year` `decade` `century` `millennium`. `start`/`end` = an era (band on the axis),
`date` = an event (card above or below the axis).

**Mind map** (the tree comes from the arrows, from the root):

```skemo
diagram(layout="mindmap")
  card#root skemo
    to(lang engine)
  card#lang Language
  card#engine Engine
```

## Steps

Markers at the **end** of a line, after a space, on a card, zone, line, table row or arrow:

| Marker | Effect |
|---|---|
| `@2` | appears at click 2 (fade; an arrow is drawn) |
| `@2:up` | appears with an effect: `fade` `up` `zoom` `draw` |
| `@2-4` | visible from click 2 to 4 |
| `@0-2` then `@3` on two lines | the line changes at click 3, in the same place |
| `@4:focus`, `@4-6:focus` | highlighted (already visible) |
| `@3-4:halo` | glow, as in the slides |

At most one appearance marker per element, any number of `:focus`. An element without a marker is
there from the start. The clicks of the diagram are the clicks of the slide: match them with the
bullets. An arrow should not appear before both of its cards.

## Pitfalls

- `to(...)` must be indented under its source card; a `to` at the top level is an error.
- One attribute group per tag: `to(core side="right-top").dashed`, not `to(core).dashed(side=…)`.
- Classes stick to their symbol: `|.warn text`; `| .env` (with a space) is the text ".env".
- Escape a literal `@` followed by a digit as `\@2`; `user@example.com` needs nothing.
- Too many cards make text tiny on a slide: keep 3 to 8, split into two slides or use steps.
- Long names widen cards: put details in `|` lines, not in the name.
