#!/bin/sh
# Builds the glitu website into a folder (default site/dist): the home page of site/ and the
# demo presentation as one standalone page (demo.html). Needs target/release/glitu.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$root/site/dist}
glitu=${GLITU:-$root/target/release/glitu}

rm -rf "$out"
mkdir -p "$out"
cp "$root/site/index.html" "$out/"
cp -r "$root/site/img" "$out/"
"$glitu" build "$root/tests/features/slides.md" --standalone -o "$out/demo.html"
# the page shown by the demo's {.iframe} slide, next to demo.html
cp -r "$root/tests/features/public/." "$out/"
# GitHub Pages: serve the files as they are
touch "$out/.nojekyll"
