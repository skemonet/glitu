#!/bin/sh
# Builds the glitu website into a folder (default site/dist): the home page of site/, the demo
# presentation as one standalone page (demo.html), and static/, the engine and the themes of this
# version for the light pages (`glitu build --assets https://glitu.net/static`). Needs cargo; GLITU,
# a glitu binary to use (default: the debug build, compiled here). The dev profile is enough: the
# engine (wasm) is always compiled in release, the pages come out the same.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$root/site/dist}
if [ -z "${GLITU:-}" ]; then
  cargo build --quiet --locked -p glitu
  GLITU=$root/target/debug/glitu
fi
glitu=$GLITU

rm -rf "$out"
mkdir -p "$out"
cp "$root/site/index.html" "$out/"
cp -r "$root/site/img" "$out/"
"$glitu" build "$root/tests/features/slides.md" -o "$out/demo.html"
cargo run --quiet --locked -p glitu --example assets -- "$out/static" "$root/themes"
# the page shown by the demo's {.iframe} slide, next to demo.html
cp -r "$root/tests/features/public/." "$out/"
# GitHub Pages: serve the files as they are
touch "$out/.nojekyll"
