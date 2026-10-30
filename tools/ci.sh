#!/usr/bin/env bash
# The glitu pipeline — the SAME one as the CI (the workflow only calls this script).
#
#   tools/ci.sh            lints, tests (with and without the `oracle` feature of development),
#                          then a smoke test of the release binary
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-auto}"

step() { printf '\n\033[1m▶ %s\033[0m\n' "$*"; }

step "lints (clippy, -D warnings)"
cargo clippy --workspace --release --all-targets --locked -- -D warnings
cargo clippy --workspace --release --all-targets --locked --all-features -- -D warnings

step "tests"
cargo test --workspace --release --locked
cargo test --workspace --release --locked --all-features

# The release binary starts and builds the demo, as a site and as one page
step "smoke test: release binary"
cargo build --release --locked -p glitu
d=$(mktemp -d)
target/release/glitu --version
target/release/glitu build tests/features/slides.md -o "$d/site"
target/release/glitu build tests/features/slides.md --standalone -o "$d/demo.html"
test -s "$d/site/index.html" && test -s "$d/demo.html"
# a new presentation, outside the repository: the embedded themes
target/release/glitu init "$d/talk"
target/release/glitu build "$d/talk/slides.md" --standalone -o "$d/talk.html"
test -s "$d/talk.html"
rm -rf "$d"
