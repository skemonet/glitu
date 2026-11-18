#!/bin/sh
# Builds the glitu website into a folder (default site/dist): the home page of site/, the demo
# presentation as one standalone page (demo.html), install.sh and install-play.sh
# (`curl -fsSL https://glitu.net/install.sh | sh`, the same for glitu-play), and static/, the engine
# and the themes of this version for the light pages (`glitu build --assets https://glitu.net/static`).
# Needs cargo; GLITU,
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
# the installer: it fetches the latest binary from the GitHub releases, so it never goes stale
cp "$root/tools/install.sh" "$out/install.sh"
# and the one of glitu-play: the same script, glitu-play its default program
sed 's/^program="${GLITU_PROGRAM:-glitu}"$/program="${GLITU_PROGRAM:-glitu-play}"/' "$root/tools/install.sh" > "$out/install-play.sh"
grep -q '^program="${GLITU_PROGRAM:-glitu-play}"$' "$out/install-play.sh" || { echo "site.sh: install-play.sh: the program line of install.sh changed" >&2; exit 1; }
"$glitu" build "$root/demo/slides.md" -o "$out/demo.html"
cargo run --quiet --locked -p glitu --example assets -- "$out/static" "$root/themes"
# the documentation (site/docs/*.md), sitemap.xml and robots.txt
cargo run --quiet --locked -p glitu --example docs -- "$root/site/docs" "$out"
# GitHub Pages: serve the files as they are
touch "$out/.nojekyll"
