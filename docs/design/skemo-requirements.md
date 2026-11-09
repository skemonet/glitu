# Requirements to skemo, for glitu

Status: request from glitu to skemo (2026-11-06), to review on the skemo side. glitu runs the
`skemo` command line since 2026-11-06 (SK-0 checked on both oracles with `v0.1.0-rc.8`); meanwhile
glitu writes the highlights itself in `data-highlight` (SK-4's attribute), no longer in `data-fx`.
Context: [`diagram-engines.md`](diagram-engines.md). glitu stops linking `skemo-engine` and
`skemo-lang` (tag `v0.1.0-rc.8`) and runs the `skemo` command line instead, like any diagram
engine. These requirements are what the command line must offer for glitu to switch **without
losing anything** of today's rendering, and safely.

Wording: **MUST** is needed before glitu switches; **SHOULD** is wanted soon after; **COULD** is
for later. Each requirement has an acceptance test; the global one is SK-0.

## SK-0: Same rendering as the linked library — MUST

The switch must not change a pixel of glitu's presentations.

- Given the same source, stylesheets, text scale and images, `skemo render` produces **the same
  bytes** of SVG as glitu's call to the library today
  (`skemo_engine::from_source` + `svg::render` + `source::embed_text` with glitu's `Bundle`, see
  `crates/glitu-core/src/skemo.rs`).
- Acceptance: glitu's two oracles at 100 % (`tests/features/slides.md`, `tests/compat/demo.md`)
  with the command line; a test in skemo comparing, for the diagrams of
  `glitu/tests/features/`, the output of the library call and of the command line.

## SK-1: A stable non-interactive contract — MUST

`skemo render - -o - --format svg [options]`, as today, with these guarantees, documented as a
contract and covered by tests:

1. Reads the source on the standard input only; writes the SVG on the standard output only; writes
   **no file** and opens no window, no browser, no network connection.
2. Never asks a question; never waits on a terminal.
3. No colour nor terminal escape on the standard error when it is not a terminal (or always with
   `--diagnostics json`, SK-2).
4. Exit codes: `0` rendered (warnings allowed, unless `--deny-warnings`); `1` the diagram is
   refused (errors, or warnings with `--deny-warnings`): nothing on the standard output; `2`
   misuse of the command line; anything else is a crash, to report.
5. Language of the messages: `SKEMO_LANG=en` wins over the system's (already the case: keep it in
   the contract). glitu always sets it.

Acceptance: tests that run the binary with a closed standard input, no terminal, a read-only
current folder (it must still work), and check the exit codes of each case.

## SK-2: Diagnostics glitu can read — MUST

`--diagnostics json`: on the standard error, **one JSON object per line, nothing else**:

```json
{"severity":"error","code":"unknown-target","message":"unknown target `db`","help":"declare a card with #db","file":"-","line":4,"column":9,"end_line":4,"end_column":11}
```

- `severity`: `error` or `warning`. `code`: the stable code of `skemo_lang::Diagnostic`.
  `message`, `help` (optional): in the language of SK-1.5.
- `file`: `-` for the source; the stylesheet's name as given to `--css` when the problem is in
  a stylesheet; an image's name for an image problem.
- `line`, `column`, `end_line`, `end_column`: from 1; columns counted in Unicode characters (not
  bytes), so that glitu can point at the line of the markdown block (glitu adds the line of the
  block). Absent only for a problem without a place (a missing file).
- A crash still prints a last line `{"severity":"error","code":"internal",…}` when it can.

Acceptance: golden tests of the JSON for an error, a warning, an error in a stylesheet, a missing
image; every line parses as JSON.

## SK-3: Version and capabilities — MUST

`skemo capabilities --format json` (or `skemo --version --format json`):

```json
{"version":"0.1.0","svg_conventions":1,"features":["diagnostics-json","highlights","root","batch"]}
```

- `version`: semantic version. `svg_conventions`: the version of SK-5, raised on any change of it
  that is not an addition. `features`: what this binary supports among the requirements here.
- Plain `skemo --version` keeps printing `skemo X.Y.Z` on its first line.

glitu checks a minimum version and the features it needs, and names what is missing.

## SK-4: Highlights written in the diagram — MUST

Today glitu strips `@3-4:halo`, `@5:chase`, `@6:magnify` from the source before skemo sees it,
then writes them on the node (`crates/glitu-core/src/render/visual.rs`, `skemo_marks`). It is
glitu's syntax inside skemo's language, and it has a bug: the node gets two attributes of the
same name. For `card#a A @1:up @3:halo`, the SVG holds today:

```html
<g class="card" data-id="a" data-fx="halo:3" data-step="1" data-fx="up">
```

The browser keeps the first `data-fx`: the appearance effect `up` is lost.

Required:

1. skemo's language accepts the highlight markers after the other markers of a line:
   `@<range>:halo`, `@<range>:chase`, `@<range>:magnify`, where `<range>` is `n` or `n-m`, the
   same steps as the appearance markers. Several on one line: `@1:up @3-4:halo @6:magnify`.
2. On a card, a zone, a line (`|`) and an arrow (`to(…)`); on an element without `#id` too
   (today glitu refuses it: "a highlight needs a card or a zone with an #id").
3. Written in the SVG in **its own attribute**, on the element's `<g>`:
   `data-highlight="halo:3-4 magnify:6"` (effects in the order written, separated by a space).
   `data-fx` stays the appearance effect only.
4. Its steps count in the diagram's steps (`data-states` of the root), like an appearance.
5. An unknown effect after `@n:` keeps its current treatment. A diagram's steps are **clicks of
   the whole slide**: between two steps of a diagram, the clicks may belong to the text
   (`card A @1` then `card B @8`, the text taking clicks 2 to 7). Only glitu knows them, so
   skemo's `step-gap` warning (D-094, a gap inside a diagram on its own) does not apply: glitu
   passes `--allow step-gap` when the probe lists the `allow` feature (D-098, `0.1.0-rc.9`; an
   older skemo does not emit `step-gap`). glitu has no upper bound to check either: the engine
   groups a slide's fragments by index and numbers them again, as reveal.js does, so every step
   written is a click and a gap in the numbers never gives a click that changes nothing.
6. In the formats without clicks (PNG, GIF, APNG, WebP), skemo may draw them or ignore them;
   documented.

Acceptance: the line above gives
`data-step="1" data-fx="up" data-highlight="halo:3"`; glitu removes `skemo_marks` and its
oracle stays at 100 % once glitu-web reads `data-highlight`.

## SK-5: The SVG conventions, documented and versioned — MUST

glitu's engine and exports rely on these parts of skemo's SVG
(`crates/glitu-web/src/skemo.rs`, `fx.rs`, `animate.rs`, `crates/glitu/src/export.rs`,
`assets/export.js`, `crates/glitu-core/src/render/steps.rs`, `visual.rs`). They become a
documented contract, version 1 (SK-3), kept within a version; a removal or a change of meaning
raises the version.

| Where | What | Used by glitu for |
|---|---|---|
| root `<svg>` | `class="skemo …"` | finding skemo's diagrams |
| root `<svg>` | `data-states` (JSON list of the steps as written) | the number of clicks |
| root `<svg>` | `viewBox`, `width`, `height` | size and zoom |
| node, line, arrow `<g>` | `data-id` (never `id`) | `{.animate}` between slides, highlights, exports |
| node, line, arrow | `data-step` (text as written), `data-until`, `data-focus` (`"4 6-7"`) | appearances, disappearances, focus |
| node, line, arrow | `data-fx` (appearance effect; `draw` on arrows) | the effect; arrows drawn on click in the exports |
| node, line, arrow | `data-highlight` (SK-4) | halo, chase, magnify |
| arrow `<g>` | `data-from`, `data-to`; class `edge` | arrows drawn click by click, the card they wait for |
| node `<g>` | class `card` (and the type's class) | exports to PowerPoint |
| frieze, timeline | `fr-stem`, `fr-dot`, `tl-mark` with `data-for`; `data-col`, `data-tag` | the strip that scrolls (`layout="frieze"`) |
| `<metadata>` | `<skemo-source xmlns="https://skemo…">` | reopening a diagram (`skemo extract`, `glitu unpack`) |
| `<defs>` | identifiers prefixed by a digest of the diagram (`skfaa3a531-ombre0`) | several diagrams in one page without collisions |

Also part of the contract, for glitu's sanitizer (`diagram-engines.md`, SEC-11), which keeps
only an allowlist:

- no `<script>`, no `<foreignObject>`, no `on*` attribute, no `javascript:`;
- no external reference: images embedded as `data:image/png|jpeg|webp|gif` (or inline SVG
  elements), `href` only to `#id` inside the same SVG;
- only these elements: `svg g path rect circle ellipse line polyline polygon text tspan defs
  marker linearGradient radialGradient stop clipPath mask pattern symbol use filter fe* title
  desc metadata` and `skemo-source`. A new element is announced in the conventions first.

Acceptance: a document in skemo's repository (`FORMAT` §4 already holds part of it); a test that
renders skemo's examples and checks the SVG against the allowlist.

## SK-6: Files read only inside a root — MUST

Today glitu reads the images of a diagram itself and checks them
(`skemo_engine::images::check_name`, `MAX_BYTES`); with the command line, skemo reads them. glitu
runs skemo in the presentation's folder; skemo must then guarantee:

1. `--root <dir>` (default: the folder of the source file; the current folder for `-`): every
   file read because of the diagram (images, icons, a stylesheet it imports) is inside the root.
2. Refused: absolute paths, `..` that leaves the root, a symbolic link whose target is outside the
   root (checked after resolving it), devices and other special files.
3. At most `MAX_BYTES` (1 MiB) per image; the type checked from the content, not only the
   extension.
4. No network, ever (already "only local files are accepted").
5. The stylesheets given with `--css` are the user's files: read wherever they are; what *they*
   reference (`url()`) follows 1–4.

Acceptance: tests with `../x.svg`, `/etc/passwd`, a link to a file outside the root, a 2 MiB
image, a `.svg` that is a PNG: each refused, with a diagnostic (SK-2).

## SK-7: Determinism — MUST

The same source, options, files and version give the same bytes: no date, no random number, no
order that depends on a hash map or on the machine (fonts are embedded already).

Needed by glitu's cache (key: source, options, tool version) and its oracles.
Acceptance: rendering skemo's examples twice, in two processes and on two platforms of the CI,
gives identical files.

## SK-8: Text scale and stylesheets as glitu passes them — MUST

glitu passes, in this order: the theme's `skemo.css`, the presentation's `skemo.css`, then a text
scale (`--font-scale`, from the theme's `glitu.yaml` or ` ```skemo font-scale="2" `). Today glitu
adds the rule `.stage { --font-scale: k; }` and sets `bundle.font_scale`.

Required: `--css <theme> --css <presentation> --font-scale k` produces exactly that (SK-0), the
stylesheets embedded in the source bundle under the names given, so that a diagram saved from the
page reopens with its styles.

## SK-9: Distribution — MUST

glitu does not install skemo: it tells the user to (only when the presentation has a skemo
diagram). So:

1. Prebuilt binaries for the targets of glitu: Linux x86_64 and ARM64 (static), macOS ARM64 and
   x86_64, Windows x86_64 (`.exe`).
2. Stable addresses: `https://github.com/skemonet/skemo/releases/latest/download/skemo-<target>`,
   a `SHA256SUMS` file next to them, and an install script like glitu's (`tools/install.sh`).
3. An install page whose address glitu can print: `https://skemo.net/install`.

## SK-10: Several diagrams in one process — SHOULD

A presentation can hold dozens of diagrams; one process each costs, especially at every reload of
`glitu server`.

`skemo render --batch`: requests on the standard input, one JSON object per line; one answer per
request, in the same order, on the standard output:

```json
{"id":"slide4-1","source":"diagram\n  card#a A\n","css":["themes/crystal/skemo.css"],"font_scale":1.35}
{"id":"slide4-1","svg":"<svg …>","diagnostics":[]}
```

- `diagnostics`: the objects of SK-2. A refused diagram gives `"svg":null`, not the end of the
  process.
- The process ends at the end of the input; the root (SK-6) is the one of the command line.

Acceptance: the same SVG as SK-1 for each request; a benchmark of a deck with 30 diagrams, one
process against thirty.

## SK-11: Bounded work — SHOULD

glitu kills a tool after a timeout (30 s by default). skemo should not need it: a limit on the
size of a diagram (nodes, arrows, text) with a clear error (SK-2), rather than a layout that
never ends; documented limits.

## SK-12: Conversions for glitu — COULD

For the engines `mermaid.skemo` and `d2.skemo` (a diagram in another language, rendered with the
theme's skemo look):

1. `skemo convert --from mermaid` (flowchart first) on the standard input → skemo source on the
   standard output, with SK-2 diagnostics for what has no counterpart. Does not exist today.
2. D2 → skemo exists in `skemo-pro d2` (paid, D-088): to decide whether glitu may call it, and
   then with the same contract (standard input and output, SK-2).

## Summary

| Id | Requirement | Priority | Exists today |
|---|---|---|---|
| SK-0 | Same bytes as the linked library | MUST | to check |
| SK-1 | Non-interactive contract, exit codes, `SKEMO_LANG` | MUST | mostly |
| SK-2 | Diagnostics in JSON lines | MUST | no |
| SK-3 | Version and capabilities in JSON | MUST | no (plain `--version` only) |
| SK-4 | Highlights in the language, `data-highlight` | MUST | no (and a bug in glitu's workaround) |
| SK-5 | SVG conventions v1, allowlist | MUST | partly documented (`FORMAT` §4) |
| SK-6 | Files confined to a root | MUST | partly (`../` refused, no `--root`, links unchecked) |
| SK-7 | Determinism | MUST | to check |
| SK-8 | Stylesheets and text scale as glitu passes them | MUST | to check against SK-0 |
| SK-9 | Binaries, addresses, install page | MUST | to check |
| SK-10 | Batch mode | SHOULD | no |
| SK-11 | Bounded work | SHOULD | no |
| SK-12 | Conversions Mermaid, D2 | COULD | D2 in skemo-pro |

## Check of skemo's working tree (2026-11-06, `e840338` + uncommitted routing work)

Built from `../skemo/repos/skemo` and run by glitu. glitu now probes `skemo capabilities` and, when
told, passes `--root` and `--diagnostics json` and leaves the highlights to skemo; the release
`v0.1.0-rc.8` keeps working without them (glitu strips and writes the highlights itself).

| Id | Result |
|---|---|
| SK-0 | Oracle: 75 of 82 states identical; slides 6 and 24 differ (0.74 %, 0.82 %): the work in progress on routing (the `runs` arrows enter elsewhere) and on stretched cards (a card's text recentred). To re-capture `tests/oracle` once skemo releases it. |
| SK-1 | Exit codes 0, 1, 2; nothing on stdout when refused; works in a read-only folder; no escape when not a terminal. An empty standard input renders an empty diagram (exit 0): acceptable? |
| SK-2 | One JSON object per line, as specified (code, line, columns, help). In English, the quotes are still `« »` (`unknown target « zz »`). |
| SK-3 | `capabilities --format json` gives the features; **the version is still `0.1.0-rc.8`**, the same as the release without them: glitu cannot require a minimum version, it relies on the features. A new version number is needed before release. |
| SK-4 | `data-highlight` next to `data-fx`, on cards without `#id` and on arrows; the clicks count in `data-states`; same rendering as glitu's (oracle). A highlight on a click that does not exist cannot happen in glitu (SK-4.5): skemo's `step-gap` (rc.9) is passed with `--allow step-gap`. |
| SK-6 | Refused with a diagnostic: `../x`, an absolute path, a link leading out, a link to `/dev/zero`, an image over 1 MiB, a PNG named `.svg`. `sub/../ok.png` (inside the root) is refused too: stricter than asked, fine. An absolute path without an image extension (`icon="/etc/passwd"`) is read as an icon name ("unknown icon"): nothing read, fine. |
| SK-7 | Two runs, the same bytes. |
| SK-8 | `--font-scale` applied. **Two `--css` with the same file name (the theme's and the presentation's `skemo.css`) are both embedded as `skemo.css`**: a diagram reopened from the page cannot tell them apart (glitu passes one merged file meanwhile). |
| SK-10 | `--batch`: one answer per line, in order; a refused diagram or a malformed line does not end the process; the same SVG as one call. Not used by glitu yet. |
