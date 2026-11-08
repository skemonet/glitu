# Diagram engines

Status: implemented (2026-11-06) in `crates/glitu/src/engines/`, `settings.rs`, `confine.rs`;
the hostile presentations of `tests/security/` check §9. Decisions taken with the maintainer are
marked **[decided]**; §10 records how the open questions were settled. Not done yet: diagnostics in
JSON (`glitu-json`, waiting for SK-2), the conversions to skemo (`mermaid.skemo`, `d2.skemo`), and
skemo's `--batch` (SK-10). `graphviz.dot` (2.42), `d2.d2` (0.9.0) and `mermaid.mmdc` (12.2.0)
are checked with the real tools under Landlock (`crates/glitu/tests/engines.rs` for dot and d2).
A manifest may `probe` its tool's features and add arguments `when` it has them (skemo: `--root`,
`--diagnostics json`; with `highlights`, glitu no longer strips `@3:halo`). A tool that runs a
browser (`sandbox: browser`, mmdc) also reads `/proc`, `/sys`, `/dev` and the home folder, and
writes `/proc` (Chromium's own sandbox maps its users there). mmdc installed without its Chrome
(`PUPPETEER_SKIP_DOWNLOAD`) needs `PUPPETEER_EXECUTABLE_PATH`, which its manifest passes.

## 1. Why

glitu renders ` ```skemo ` blocks with the skemo engine linked into its binary. More diagram
languages are wanted (Mermaid, D2, Graphviz, others later), and each comes with its own renderer,
usually an external program (`mmdc`, `d2`, `dot`) or JavaScript (mermaid.js). Linking each one
would make glitu grow with every language; skemo alone is 62 % of the binary today:

| `glitu` binary (release, LTO, stripped) | Size |
|---|---|
| with skemo linked (today) | 21.9 MB |
| without skemo (measured on a stubbed build) | 8.2 MB |

The skemo part is ~7.3 MiB of code (half of it the ELK layout algorithms, `layered` alone
2.8 MiB), 1.9 MB of fonts (Inter, JetBrains Mono, used to measure text) and their data. The wasm
engine (`glitu-web`) does not depend on skemo.

**[decided]** skemo leaves the binary and becomes an external tool like the others. glitu knows a
list of engines and speaks an open protocol that third-party engines can follow.

## 2. Vocabulary

- **Language**: what the fence names: ` ```skemo `, ` ```mermaid `, ` ```d2 `, ` ```dot `.
- **Tool**: an executable on the machine: `skemo`, `skemo-pro`, `mmdc`, `chromium`, `d2`, `dot`.
- **Engine**: a way to render a language with a tool, named `<language>.<implementation>`:
  `skemo.skemo`, `mermaid.mmdc`, `mermaid.chromium`, `mermaid.skemo` (converted to skemo, then
  rendered by skemo), `d2.d2`, `d2.skemo` (through `skemo-pro`), `graphviz.dot`.
- **Manifest**: the description of an engine: which tool, which command line, what it can do.

## 3. Behaviour for the user

### Tools are needed only when used — [decided]

- A presentation without a ` ```d2 ` block never mentions `d2`, even if it is missing.
- `glitu build`, and the exports, fail on the first diagram whose tool is missing, naming the
  block and how to provide the tool:
  `error: slides.md:42: the d2 tool is needed for this diagram; install it (https://d2lang.com) or give its path: --tool d2=PATH`.
- `glitu server` keeps serving: the slide shows the error in place of the diagram (missing tool
  and how to install it, or the faulty line), the terminal repeats it. **[decided]**
- `glitu add diagram` says in its help that the skemo tool is needed. If glitu does not find it,
  the slide it adds starts with a one-line comment:
  `<!-- This diagram needs skemo (https://skemo.net/install): not found on this machine -->`.
  Only `add` writes this comment. **[decided]**

### Finding a tool — [decided]

From the highest priority to the lowest:

1. the command line: `glitu build --tool skemo=/opt/skemo/bin/skemo` (repeatable; also on
   `server` and the formats);
2. an environment variable: `GLITU_TOOL_SKEMO=/opt/skemo/bin/skemo` (`CHROMIUM` stays read, for
   compatibility);
3. the user's configuration: `[tools]` in `~/.config/glitu/config.toml`;
4. the `PATH` (and, for Chromium, today's lookup: the headless shell downloaded by glitu, then the
   installed browsers).

The path of a tool belongs to the machine, never to the document.

### Choosing the engine of a language — [decided: both, the command wins]

1. the command line: `--engine mermaid=mmdc`;
2. the block: ` ```mermaid engine=skemo `;
3. the front matter: `engine.mermaid: chromium`;
4. the user's configuration: `[engines] mermaid = "chromium"`;
5. glitu's default for the language.

An implementation alone (`mmdc`) is enough where the language is known from the context;
`mermaid.mmdc` is the full name.

For Mermaid, the user chooses between `mermaid.mmdc` (mermaid-cli, Node on the user's machine,
not in the project) and `mermaid.chromium` (mermaid.js run in the browser glitu already drives
for the exports). **[decided]**

### Options of a diagram — [decided]

Options are layered, each level overriding the previous one: the user's configuration, then the
front matter, then the block.

- **Common options**, understood by every engine that can: `font-scale`, the theme's stylesheet
  and palette, `lang`. Each manifest maps them to its tool's arguments, or ignores them.
- **Prefixed options**, for one engine: ` ```mermaid mermaid.theme=dark `, front matter
  `mermaid.theme: dark`, configuration `[options.mermaid] theme = "dark"`.
- **Raw arguments**, passed as written to the tool: ` ```d2 args="--layout elk" `, front matter
  `d2.args: --layout elk`, configuration `[options.d2] args = ["--layout", "elk"]`. The levels add
  up in order (configuration, front matter, block).

The front matter stays flat (`key: value` lines, as glitu reads it today): dotted keys fit it,
and `glitu config` can list and check them.

### Cache — [decided: in memory only]

During `glitu server`, a rendered SVG is kept in memory, under a key made of the source, the
resolved options, the engine, the tool's path and version. A reload only re-renders the diagrams
that changed. Each `glitu build` renders everything again.

## 4. Configuration files

**[decided]** The manifests glitu supports live in the binary. `~/.config/glitu/` (XDG; the
platform's configuration folder on macOS and Windows) only holds what the user changes, so a
new version of glitu can improve its defaults without fighting copied files. A command prints the
defaults, commented, for the user to see what can be changed (`glitu config --engines`, name to
settle).

```yaml
# ~/.config/glitu/config.yaml
files:
  allow: [~/talks/shared]   # folders a presentation may read besides its own (SEC-16)
tools:
  skemo: /opt/skemo/bin/skemo
engines:                    # the default engine of a language
  mermaid: chromium
options:
  skemo:                    # options of every diagram of an engine
    font-scale: 1.4
    args: [--contrast, high]
limits:
  timeout: 30
```

```yaml
# ~/.config/glitu/engines/graphviz.dot.yaml: an engine, built-in (overridden) or third-party
id: graphviz.dot
fences: [dot, graphviz]               # the fence names it renders
tool: dot                             # the tool, found as in §3
command: ["{tool}", -Tsvg, "{options}", "{args}"]
version: ["{tool}", -V]               # its version, part of the cache key
input: { file: diagram.dot }          # optional: the source in a temporary file, {input}
output: { file: diagram.svg }         # optional: the SVG from a temporary file, {output}
files: { config.json: '{…}' }         # files of its safe mode, {file:config.json}
stylesheet: { file: x.css, flag: --css }  # the theme's then the deck's x.css, {stylesheets}
set-env: { NO_COLOR: "1" }            # variables set for it; env: [NAME] passes the user's
options:                              # typed options (SEC-5)
  layout: { type: choice, values: [dot, neato], flag: -K, attached: true }
capabilities: []                      # "steps", "ids", "frieze", "highlights" (§6)
sandbox: browser                      # optional: it runs a browser (wider confinement)
safe-mode: "what it may read"         # shown by glitu config --engines
```

`mermaid.chromium` is not a command line: its manifest names a built-in runner (`runner:
chromium`); the mermaid.js it loads is pinned in the binary (version 12.2.0 and its SHA-256).

## 5. The protocol of an engine

- **In**: the source on the standard input (or in a file), the options as arguments; the tool
  runs in the folder of the presentation's main file, so that the relative files it reads
  (images, icons) resolve as in the markdown.
- **Out**: one SVG on the standard output; exit code 0.
- **Diagnostics**: with `diagnostics = "glitu-json"`, one JSON object per line on the standard
  error: `{"severity": "error|warning", "line": 3, "column": 5, "message": "…"}`, lines counted
  in the block. glitu turns them into `slides.md:<line of the block + line>: …`, and a warning
  fails the build, as today. With `diagnostics = "text"`, a failure gives the tool's standard
  error as it is, attached to the block's first line.
- **Version**: the `version` command; a manifest may require a minimum (`min-version`).

## 6. Steps and transitions

**[decided]** The `@n` markers inside a diagram are a skemo speciality: glitu does not convert
them for the other languages. What the other engines get, and what has to be checked before
implementing:

| Feature | Today (skemo, linked) | Any engine | What it relies on |
|---|---|---|---|
| The whole block at a click: ` ```mermaid @2 ` | yes | yes | glitu's fragment on the block, engine-independent |
| Elements appearing click by click (`@n` in the source) | yes | only with capability `steps` | the SVG's `data-step`, `data-until`, `data-focus`; the compiler adds one `.sk-step` fragment per step (`render/steps.rs`, `skemo_steps`) |
| A line replaced by another, morphing (`.sk-diff`) | yes | `steps` | skemo's classes (`sk-off`), `glitu-web/src/skemo.rs` |
| Highlights on a node: `card#pg @3-4:halo` | yes | no | glitu strips them from the skemo source before rendering (`render/visual.rs`, `skemo_marks`) and puts them on `[data-id]` |
| `{.animate}` between two slides | the block slides; nodes with the same `data-id` slide | the block slides; nodes only with capability `ids` | `glitu-web/src/animate.rs` matches `[data-id]` and `.diagram` |
| Frieze that scrolls (`layout="frieze"`) | yes | no (capability `frieze`) | `fr-stem`, `data-for` in `render/visual.rs` |
| PowerPoint and PDF: arrows drawn click by click | yes | the SVG as a static group | `svg.skemo .edge.sk-off[data-fx="draw"]`, `.card[data-id]` in `export.rs` / `export.js` |
| Slide transitions, backdrop, zoom | yes | yes | the slide, not the diagram |

So the "glitu SVG conventions" (`data-step`, `data-until`, `data-focus`, `data-id`, `sk-off`,
`data-fx`, frieze classes) become a documented part of the protocol, declared by capabilities. A
third-party engine that writes them gets the same animations as skemo.

The highlight markers (`@3-4:halo|chase|magnify`) are glitu's syntax written inside skemo's
source: to stay generic, glitu should stop preprocessing the source (see the requests to skemo).

## 7. What skemo's command line has, and what it lacks

Already there (`skemo render`): the source from the standard input (`-`), `-o -`,
`--format svg`, `--css` (repeatable: the theme's `skemo.css`, then the presentation's),
`--font-scale`, `--deny-warnings`, `--no-source`; the messages' language through `SKEMO_LANG`;
images resolved from the current folder when reading the standard input; `--version`.

To ask the skemo side (not to work around in glitu), detailed in
[`skemo-requirements.md`](skemo-requirements.md):

1. **Diagnostics in JSON** on the standard error (`--diagnostics json`): severity, line, column,
   message.
2. **Highlight markers understood by skemo**: `@3-4:halo`, `chase`, `magnify` on a node or an
   arrow, written into the SVG (`data-fx`, with their clicks), so that glitu no longer strips
   them from the source.
3. **The glitu SVG conventions as a stable, documented output** of skemo (§6), versioned.
4. **Several diagrams per process** (optional): a mode reading a stream of sources, for decks with
   many diagrams, where starting a process per diagram costs.
5. **Conversion Mermaid → skemo**, for `mermaid.skemo`: it does not exist. D2 → skemo exists in
   `skemo-pro d2` (paid, D-088).

## 8. Consequences

- **Binary**: 21.9 MB → about 8.2 MB. The minimum Rust version set by skemo's layout engine
  (1.90) no longer applies to glitu. `glitu-core` stays pure (no I/O): it asks its `Assets` for
  the SVG of a block, as today; the engine calls live in `crates/glitu`.
- **First experience**: `glitu init` writes a skemo diagram; without skemo, the preview shows the
  error in that slide, with the way to install it.
- **Reproducibility**: the version of skemo is no longer pinned by glitu's `Cargo.toml`. The
  tool's version goes into the cache key, and a manifest can require a minimum version.
- **Speed**: a process per diagram; the in-memory cache keeps the preview fast.
- **Tests and CI**: the oracles need skemo installed (pinned version in the CI). The unit tests
  that need a tool are skipped when it is missing, unless the CI asks for them all
  (`GLITU_REQUIRE_TOOLS=1`).
- **Security**: the document can choose engines and options; §9 makes sure it cannot choose
  programs, nor arguments that the user did not allow.
- **Viewers of a built page**: no change, the page holds the SVG.

## 9. Security

**Goal: building a markdown file received from anyone runs no program and reads no file that the
user did not allow on their own machine.** A presentation is data; only the user's machine
decides what runs.

### Trust

| Trusted (the user's machine) | Untrusted (the presentation, and what it reaches) |
|---|---|
| the command line | the markdown files, included or imported files |
| the environment variables | the front matter, the attributes of the blocks |
| `~/.config/glitu/` | `style.css`, `skemo.css` next to the markdown |
| the glitu binary and its built-in manifests | the `themes/` folders found next to or above the markdown, their `glitu.yaml`; the themes at an `https://` address |
| | a page or a `.glituplay` given to `glitu unpack` |
| | **what the tools produce**: their SVG, their standard error (they process untrusted input) |

### Requirements

Each one comes with a test in `tests/security/`: a hostile presentation that must fail with a
clear error, or render inert.

- **SEC-1: Only declared programs run.** glitu starts only the tools named by a manifest of the
  binary or of `~/.config/glitu/engines/`, and Chromium for the exports. The document names an
  engine by its identifier only (`mermaid.chromium`); an unknown identifier is an error. A
  document never gives a path nor a command: `engine=/bin/sh`, `engine=../x`,
  `engine.mermaid: /usr/bin/python` are refused.
- **SEC-2: Manifests and tool paths only from trusted places.** Never from the presentation's
  folder, its `themes/`, the parent folders, a theme's `glitu.yaml` or an address. A manifest
  found there is ignored, with a warning that says why.
- **SEC-3: Tools resolved by glitu, to an absolute path, before they start.** The `PATH` lookup
  skips its relative entries (`.`, empty). A tool that would resolve inside the presentation's
  folder (or the folder being built) is refused, unless the user named that absolute path
  (`--tool`, `GLITU_TOOL_*`, `[tools]`). On Windows, only `.exe` files: `.bat` and `.cmd` are
  refused unless configured explicitly (their argument quoting cannot be made safe, CVE-2024-24576).
- **SEC-4: No shell.** A tool is started with a vector of arguments; a placeholder of the
  manifest is replaced by whole arguments, never pasted into a string that a shell or the tool
  splits again.
- **SEC-5: Options from the document are typed.** A manifest declares each option it accepts,
  with its type: a choice among words, a number in a range, a boolean, a text matching a
  pattern. An option it does not declare is an error. A value becomes one single argument, after
  the flag of the manifest; a value starting with `-`, or holding a control character, is refused.
  Example: D2's `--layout` runs an external `d2plugin-<name>` program, so `d2.layout` is a choice
  (`dagre`, `elk`), never a free text.
- **SEC-6: Raw arguments only from the user.** The `args` of the user's configuration and of the
  command line pass as they are. The `args` of the document (front matter, block) are refused,
  unless the user's configuration allows them for this engine, flag by flag
  (`[options.d2] allow-document-args = ["--sketch", "--pad"]`, each value then checked as in
  SEC-5), or the build is run with `--allow-document-args` (one build, the user's decision).
  The error says which of the two to use.
- **SEC-7: Each tool in its safe mode.** A manifest can hold `fixed-args` that glitu always adds
  and that nothing in the document can change or remove: Mermaid's `securityLevel: strict` and
  `htmlLabels: false`, PlantUML's `PLANTUML_SECURITY_PROFILE=SANDBOX`, D2 without remote imports.
  A tool that has no safe mode for a feature that reads files or the network is documented as
  such in its manifest, shown by `glitu config --engines`.
- **SEC-8: Files.** A tool runs in the presentation's root folder. Its input, when it must be a
  file, goes into a private temporary folder (mode 0700), removed afterwards. Its output comes
  only through the standard output: no output path is ever taken from the document. A tool
  confines its own reads to the root folder when it can (skemo: see the requirements to skemo);
  on Linux, glitu should also restrict the tool with Landlock (reading: the root folder, the tool's
  own installation, the system libraries; writing: the temporary folder; later, no network).
- **SEC-9: Limits.** A timeout per diagram (30 s by default, set in the user's configuration),
  after which the tool and its children are killed; at most 20 MB of SVG and 1 MB of standard
  error read; a bounded number of tools at once.
- **SEC-10: A minimal environment.** A tool gets `PATH`, `HOME`, `TMPDIR`, the language
  (`SKEMO_LANG=en`…) and, on Windows, `SYSTEMROOT`; any other variable only if its manifest names
  it (`PUPPETEER_EXECUTABLE_PATH` for `mmdc`). The user's secrets (tokens in the environment) do
  not reach a tool by default.
- **SEC-11: The SVG is sanitized before it enters the page.** The page is published and opened by
  others: a tool fed with a hostile source can produce a hostile SVG. Kept: an allowlist of SVG
  elements (`svg g path rect circle ellipse line polyline polygon text tspan defs marker
  linearGradient radialGradient stop clipPath mask pattern symbol use filter fe* title desc
  metadata`, skemo's `skemo-source` in its namespace) and of attributes (presentation, geometry,
  `class`, `data-*`); `href` only to `#id` in the same SVG, or a `data:image/png|jpeg|webp|gif`
  address; `style` without `url()` other than `#id`, without `@import`, `expression`. Removed:
  `script`, `foreignObject`, `iframe`, `object`, `embed`, `animate`/`set` on `href`, every `on*`
  attribute, `javascript:` addresses, external references. The identifiers of each SVG are made
  unique in the page (two Mermaid diagrams both define `#arrowhead`). The SVG of today's skemo
  passes this filter unchanged (checked: `defs`, `filter`, `fe*`, `g`, `path`, `text`, `metadata`,
  one identifier already prefixed by a hash).
- **SEC-12: What a tool says is text.** Its standard error is stripped of control characters
  (terminal escapes) before it reaches the terminal, escaped before it reaches a slide in
  `glitu server`, cut at a few kilobytes.
- **SEC-13: Downloaded components are pinned.** mermaid.js for `mermaid.chromium`, like the
  headless shell today: a version and a SHA-256 written in the binary, HTTPS only, stored in the
  user's cache; never an address taken from the document. The Chromium runner loads only that
  mermaid.js, blocks every network request, and passes the diagram as data (never as code to
  evaluate).
- **SEC-14: The cache knows what ran.** Its key holds the tool's absolute path and version and
  the manifest's digest: changing the configuration never reuses an SVG made by another tool.
- **SEC-15: Visible.** `glitu config --engines` shows each engine, its manifest's origin (built
  in, user), its tool's resolved path, its version, its safe mode; `glitu build -v` lists the tools
  it started.

### Found while checking the existing code

- **SEC-16: Imported code and included files are not confined.** Today a presentation can read any
  file of the machine and copy it into the built page, which is then published:
  `<<< ../secret.txt`, `<<< /home/me/.ssh/id_ed25519 text` and `<!-- include: ../secret.txt -->`
  all end up in the page (checked: twice, in the slide and in the embedded markdown). Images,
  terminal recordings, the logo and skemo's icons are confined already (skemo refuses `../`).
  Required: every file the presentation reads is inside its root folder (the main file's folder,
  or a root the user sets), checked after resolving the symbolic links; absolute paths refused;
  outside, an error, unless the user allows a folder (`[files] allow = ["~/talks/shared"]`, or
  `--allow-read <dir>`). A fix independent of the engines, to do first.
- **A skemo highlight cancels the appearance effect of its node** (not a security issue): for
  `card#a A @1:up @3:halo`, the node gets `data-fx="halo:3" … data-fx="up"`, two attributes of the
  same name; the browser keeps the first, the `up` effect is lost. See the requirements to skemo
  (SK-4).

## 10. Open questions, settled (2026-11-06)

1. **Confinement** (SEC-8), from the first version: Linux, Landlock, through `glitu __sandbox`
   (glitu confines itself, then becomes the tool), at best (an old kernel leaves the tool
   unconfined); macOS, Seatbelt through `/usr/bin/sandbox-exec`. Windows: the Job Object agreed
   needs unsafe code, which the workspace forbids (`unsafe_code = "forbid"`): the tool and its
   children are killed together at the time limit (`taskkill /T`), files are not confined.
2. **Format of the configuration**: YAML (`config.yaml`, the manifests), and the themes'
   `glitu.json` became `glitu.yaml` (a former `glitu.json` is still read). Crate `serde-saphyr`;
   it reads `yes`/`no`/`on`/`off` as booleans (YAML 1.1).
3. **mermaid.js**: downloaded on first use, pinned (12.2.0, SHA-256), HTTPS only, into the cache.
4. **`d2.skemo`**: not offered by glitu; a user may write its manifest.
5. **Default engines**: `mermaid` → `mermaid.chromium`; the others have one engine.
6. **Command**: `glitu config --engines [ENGINE]` (the list; an engine's manifest).
   `glitu config` lists the front matter's `engine.*` and `<language>.*` keys apart.
7. **Transition**: direct switch; skemo is no longer linked (release binary 9.5 MB). The CI
   installs the skemo command line at `tools/skemo.version`.

## 11. Order of work

0. SEC-16 (confine the files read), on its own, now.
1. Settle the open questions; send skemo its requirements (`skemo-requirements.md`).
2. The engine layer in `crates/glitu`: manifests, tool lookup, options, diagnostics, cache;
   skemo through it, compared with the linked rendering on both oracles.
3. Remove the linked skemo.
4. `graphviz.dot`, the simplest, to test the protocol on a tool that knows nothing of glitu.
5. `mermaid.mmdc`, `mermaid.chromium`, `d2.d2`; then the conversions to skemo.
