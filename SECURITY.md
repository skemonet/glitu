# Security policy

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

Please do not open a public issue. Report it privately with GitHub's
[private vulnerability reporting](https://github.com/skemonet/glitu/security/advisories/new),
with the steps to reproduce it and the version of glitu (`glitu --version`).

You will get an answer within a week. Once the fix is released, the advisory is published, and you
are credited unless you prefer otherwise.

## What glitu does that matters for security

- `glitu server` is a development server: it answers this machine only, unless addresses are
  allowed with `--allow`. Do not expose it to the internet.
- A presentation can include any file you can read (inclusions, `<<<` code imports, images), and
  the page runs the scripts it contains: build only the presentations you trust.
- `glitu build pptx|pdf|png|play` drives a headless Chromium-based browser; `--download-browser` downloads Chrome's
  headless shell from Google and checks its SHA-256 before using it.
