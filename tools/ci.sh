#!/usr/bin/env bash
# The glitu checks — the SAME ones as the CI (the `check` job of .gitlab-ci.yml only calls this script).
#
#   tools/ci.sh            format, secrets, lints (with and without the `oracle` feature of
#                          development), tests, documentation, dependencies (cargo-deny),
#                          coverage (cargo-llvm-cov, at least tools/coverage.min % of the lines
#                          outside glitu-web), then a smoke test of the binary (the CI on main)
#   tools/ci.sh --quick    format, secrets, lints with every feature and tests only (the CI on
#                          other branches)
#
# cargo-deny and cargo-llvm-cov are skipped, with a warning, where they are not installed.
set -euo pipefail
quick=false
[ "${1:-}" = --quick ] && quick=true
cd "$(dirname "$0")/.."
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-auto}"

step() { printf '\n\033[1m▶ %s\033[0m\n' "$*"; }

# Everything in the dev profile: the release profile (LTO, one codegen unit) is for the
# distributed binaries, which the `binaries` job builds; here it would only slow the checks down.

# A tool of the checks, or a warning that the step is skipped
have() { command -v "$1" > /dev/null || { printf '\033[33m%s not installed: step skipped\033[0m\n' "$1"; return 1; }; }

step "format (cargo fmt --check)"
cargo fmt --all --check

# Tokens and private keys in the tracked files (no outside tool: the usual shapes of secrets)
step "secrets"
if git grep -nIE 'gh[pousr]_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{60,}|glpat-[A-Za-z0-9_-]{20}|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----|xox[abpr]-[A-Za-z0-9-]{10,}' -- . ':!tools/ci.sh'; then
  echo "a secret seems to be committed (above)" >&2
  exit 1
fi

# The diagram tools the tests and the smoke test need: skemo's command line, at the version of
# tools/skemo.version, installed once into the cached cargo home
step "tools: skemo $(cat tools/skemo.version)"
skemo_version=$(cat tools/skemo.version)
# where cargo install puts it (the CI's cached cargo home is not on its PATH)
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
if ! skemo --version 2> /dev/null | grep -qx "skemo $skemo_version"; then
  cargo install --git https://github.com/skemonet/skemo --tag "v$skemo_version" skemo-cli
fi
export GLITU_TOOL_SKEMO="$(command -v skemo)" GLITU_REQUIRE_TOOLS=skemo

step "lints (clippy, -D warnings): --all-features, then the default build"
cargo clippy --workspace --all-targets --locked --all-features -- -D warnings
$quick || cargo clippy --workspace --all-targets --locked -- -D warnings

# --all-features: the tests of the development commands too (oracle)
step "tests"
cargo test --workspace --locked --all-features
$quick && exit 0

step "documentation (rustdoc, -D warnings)"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --all-features

step "dependencies: advisories, licences, sources (cargo deny)"
if have cargo-deny; then cargo deny --locked check; fi

# glitu-web runs in the browser: its pure logic is tested natively, the rest end to end by the
# visual oracles; it is left out of the measure, like what needs a screen: the windows of
# glitu-play (wayland.rs, player.rs) and the animated backdrops (glitu-backdrop)
step "coverage (cargo llvm-cov): at least $(cat tools/coverage.min) % of the lines"
if have cargo-llvm-cov; then
  cargo llvm-cov --workspace --locked --all-features --summary-only --ignore-filename-regex 'glitu-web/|glitu-backdrop/|glitu-play/src/(wayland|player)\.rs' \
    --fail-under-lines "$(cat tools/coverage.min)"
fi

# The binary as released (default features) starts, builds the demo as one page and exports it
# as a website, and a new presentation, with a theme of its own, outside the repository (the
# embedded themes); it has no oracle
step "smoke test: the binary"
cargo build --locked -p glitu
g=target/debug/glitu
d=$(mktemp -d)
"$g" --version
"$g" version
"$g" build tests/features/slides.md -o "$d/demo.html"
"$g" export tests/features/slides.md --format site -o "$d/site"
test -s "$d/site/index.html" && test -s "$d/demo.html"
"$g" init "$d/talk" --theme mine
"$g" build "$d/talk/slides.md" -o "$d/talk.html"
test -s "$d/talk.html" && test -s "$d/talk/themes/mine/theme.css"
if "$g" oracle check http://localhost/ > /dev/null 2>&1; then echo "oracle must not be in the released binary" >&2; exit 1; fi
rm -rf "$d"
