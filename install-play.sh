#!/bin/sh
# Installs the `glitu` binary from a GitHub release (Linux and macOS), or `glitu-play`, the player
# for slow computers (Linux).
#   curl -fsSL https://glitu.net/install.sh | sh
#   curl -fsSL https://glitu.net/install-play.sh | sh
# (the website serves this file, copied by tools/site.sh at each publication; install-play.sh is
# the same with glitu-play as the default program)
#   GLITU_VERSION=v0.1.1-rc.4 GLITU_BIN=~/bin sh install.sh
# Variables: GLITU_PROGRAM (glitu or glitu-play), GLITU_VERSION (default: latest release,
# pre-releases included), GLITU_BIN (default ~/.local/bin), GITHUB_TOKEN (optional: a higher API
# rate limit).
# The binary is checked against the SHA-256 the release's description gives before it is installed.
set -eu

repo="skemonet/glitu"
program="${GLITU_PROGRAM:-glitu-play}"
bin_dir="${GLITU_BIN:-$HOME/.local/bin}"
api="https://api.github.com/repos/$repo"

die() { echo "install.sh: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required"; }
need curl; need uname

case "$program-$(uname -s)-$(uname -m)" in
  glitu-Linux-x86_64) asset=glitu-linux-x86_64 ;;
  glitu-Linux-aarch64 | glitu-Linux-arm64) asset=glitu-linux-arm64 ;;
  glitu-Darwin-arm64) asset=glitu-macos-arm64 ;;
  glitu-Darwin-x86_64) asset=glitu-macos-x86_64 ;;
  # glibc 2.28 and later
  glitu-play-Linux-x86_64) asset=glitu-play-linux-x86_64 ;;
  glitu-play-Linux-aarch64 | glitu-play-Linux-arm64) asset=glitu-play-linux-arm64 ;;
  glitu-play-Darwin-*) die "no glitu-play binary for macOS: build it with cargo build --release -p glitu-play" ;;
  glitu-* | glitu-play-*) die "no binary for $(uname -s) $(uname -m) (Windows: download $program-windows-x86_64.exe from the releases page)" ;;
  *) die "GLITU_PROGRAM: glitu or glitu-play, not $program" ;;
esac

auth=""
[ -n "${GITHUB_TOKEN:-}" ] && auth="Authorization: Bearer $GITHUB_TOKEN"
get() { # get URL [accept]
  if [ -n "$auth" ]; then curl -fsSL -H "$auth" -H "Accept: ${2:-application/vnd.github+json}" "$1"
  else curl -fsSL -H "Accept: ${2:-application/vnd.github+json}" "$1"; fi
}

tag="${GLITU_VERSION:-}"
if [ -z "$tag" ]; then
  tag="$(get "$api/releases?per_page=1" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)"
  [ -n "$tag" ] || die "no release found"
fi
echo "installing $program $tag ($asset) into $bin_dir"

release="$(get "$api/releases/tags/$tag")"
asset_url() { # asset file name -> API url of the asset
  printf '%s' "$release" | tr ',' '\n' | grep -B8 "\"name\": *\"$1\"" | sed -n 's/.*"url": *"\(https:[^"]*\/assets\/[0-9]*\)".*/\1/p' | tail -n 1
}

tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
name="$asset"; url="$(asset_url "$name")"
[ -n "$url" ] || die "asset $asset not found in release $tag"
get "$url" application/octet-stream > "$tmp/bin"

# the release's description holds one line per binary: "<sha256>  <name>"
expected="$(printf '%s' "$release" | grep -o "[0-9a-f]\{64\}  $name" | head -n 1 | cut -d' ' -f1)"
[ -n "$expected" ] || die "no SHA-256 for $name in the description of release $tag"
if command -v sha256sum >/dev/null 2>&1; then actual="$(sha256sum "$tmp/bin" | cut -d' ' -f1)"
else actual="$(shasum -a 256 "$tmp/bin" | cut -d' ' -f1)"; fi
[ "$expected" = "$actual" ] || die "SHA-256 mismatch (expected $expected, got $actual)"

mkdir -p "$bin_dir"
install -m 755 "$tmp/bin" "$bin_dir/$program"
echo "installed: $bin_dir/$program"
