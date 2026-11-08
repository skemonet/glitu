# Hostile presentations

Each folder is a presentation that tries something the user did not allow
(`docs/design/diagram-engines.md`, §9): its build must fail with the message of its `expect.txt`,
and `GLITU-SECRET-7f3a` (the marker of `secret.txt` and `secret.cast`, outside every presentation)
must never reach a built page. `crates/glitu/tests/security.rs` builds them all with the `glitu`
binary, in an empty user configuration (or the case's `config.yaml`); `tools/` holds fake diagram
tools that misbehave. Its header describes the other files of a case.

| Folder | Requirement | What it tries |
|---|---|---|
| `include-up`, `include-absolute` | SEC-16 | `<!-- include: -->` of a file outside the folder |
| `import-up`, `import-absolute` | SEC-16 | `<<<` of a file outside the folder |
| `import-link`, `link-in-img` | SEC-16 | a symbolic link leading outside (made by the test) |
| `cast-up`, `skemo-icon-up`, `logo-up`, `theme-up` | SEC-16 | a recording, an icon, the logo, a theme outside |
| `engine-path`, `engine-front-matter` | SEC-1 | a program's path as an engine |
| `manifest-in-deck`, `tool-in-deck` | SEC-2, SEC-3 | a manifest next to the deck; a `dot` in the deck put first in the `PATH` |
| `option-untyped`, `option-unknown` | SEC-5 | a path as a choice; an undeclared option |
| `document-args`, `document-args-front-matter` | SEC-6 | raw arguments from the block, the front matter |
| `tool-confined` | SEC-8 | a tool reading outside the folder (Landlock, Linux) |
| `tool-timeout` | SEC-9 | a tool that never ends, with a child |
| `tool-environment` | SEC-10 | a token of the user's environment |
| `svg-sanitized` | SEC-11 | scripts, `javascript:`, external references in the SVG |
| `tool-stderr` | SEC-12 | terminal escapes in the tool's messages |
