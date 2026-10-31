#!/bin/sh
# The next glitu version, from the commits since the last release (conventional commits), for the
# maintainer's CI (see .gitlab-ci.yml). The releases are the `v*` tags of github.com/$GITHUB_REPO,
# fetched first: the CI repository has none.
#
#   tools/next-version.sh [file]
#
# Writes to the file (default release.env), as shell variables:
#   LAST_VERSION      the last released version (0.0.0 if none)
#   NEXT_VERSION      the version to release, empty when nothing calls for one
#   GLITU_BUILD_VERSION  the version the binaries and the website say: NEXT_VERSION, or else
#                     LAST_VERSION
#
# The bump, from the commit messages since the last tag:
#   `type!:` or a `BREAKING CHANGE:` line  major (minor while the version is 0.x)
#   `feat:`                                minor
#   `fix:`, `perf:`                        patch
#   anything else (docs, ci, chore…)       no release
# After a pre-release X-rc.N, the bump that X already covers releases X itself.
set -eu

repo=${GITHUB_REPO:-skemonet/glitu}
out=${1:-release.env}

git fetch -q --tags --force "https://github.com/$repo.git" 2> /dev/null \
  || echo "next-version: the tags of github.com/$repo could not be fetched" >&2
if last=$(git describe --tags --abbrev=0 --match 'v[0-9]*' HEAD 2> /dev/null); then
  range="$last..HEAD"
  last=${last#v}
else
  last=0.0.0
  range=HEAD
fi

# 3 major, 2 minor, 1 patch, 0 nothing
bump=$(git log --format='%s%n%b%n%x1e' "$range" | awk '
  BEGIN { RS = "\x1e"; level = 0 }
  {
    subject = $0; sub(/^\n+/, "", subject); sub(/\n.*/, "", subject)
    if (subject ~ /^[a-z]+(\([^)]*\))?!:/ || $0 ~ /\nBREAKING[ -]CHANGE: /) l = 3
    else if (subject ~ /^feat(\([^)]*\))?:/) l = 2
    else if (subject ~ /^(fix|perf)(\([^)]*\))?:/) l = 1
    else l = 0
    if (l > level) level = l
  }
  END { print level }')

next=$(echo "$last $bump" | awk '{
  split($1, v, "-"); pre = (v[2] != ""); split(v[1], n, ".")
  major = n[1]; minor = n[2]; patch = n[3]; b = $2
  if (b == 0) { print ""; exit }
  if (b == 3 && major == 0) b = 2
  if (pre) {
    # X-rc.N: X is already a bump of the release before it; does it cover this one?
    if (b == 1 || (b == 2 && patch == 0) || (b == 3 && minor == 0 && patch == 0)) { print v[1]; exit }
  }
  if (b == 3) print major + 1 ".0.0"
  else if (b == 2) print major "." minor + 1 ".0"
  else print major "." minor "." patch + 1
}')

{
  echo "LAST_VERSION=$last"
  echo "NEXT_VERSION=$next"
  echo "GLITU_BUILD_VERSION=${next:-$last}"
} > "$out"
echo "last release: v$last (commits: $range)"
if [ -n "$next" ]; then echo "next release: v$next"; else echo "nothing to release (no feat, fix, perf or breaking change)"; fi
