#!/usr/bin/env bash
# The glitu checks — the SAME ones as the CI (the `check` job of .gitlab-ci.yml only calls this script).
#
#   tools/ci.sh            lints (with and without the `oracle` feature of development), tests,
#                          then a smoke test of the binary
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-auto}"

step() { printf '\n\033[1m▶ %s\033[0m\n' "$*"; }

# Everything in the dev profile: the release profile (LTO, one codegen unit) is for the
# distributed binaries, which the `binaries` job builds; here it would only slow the checks down.

step "lints (clippy, -D warnings): default build and --all-features"
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --workspace --all-targets --locked --all-features -- -D warnings

# --all-features: the tests of the development commands too (oracle)
step "tests"
cargo test --workspace --locked --all-features

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
