#!/bin/sh
# Publishes glitu on GitHub, from the maintainer's CI (see .gitlab-ci.yml), on main once the tests
# passed:
#
# 1. pushes main to github.com/$GITHUB_REPO (fast-forward only: a refused push means GitHub's main
#    has commits the CI repository lacks, to bring back first);
# 2. if tools/next-version.sh found a version to release (NEXT_VERSION of release.env): pushes the
#    tag v<version> on this commit and creates the release with the binaries of dist/ and their
#    SHA-256 in its description. A version with a hyphen (1.0.0-rc.1…) gives a pre-release.
#
# Needs: git, curl, jq, sha256sum; GITHUB_TOKEN (allowed to push to the repository and to create
# its releases), never written in a command line or in the log; CI_COMMIT_SHA; release.env.
set -eu

repo=${GITHUB_REPO:-skemonet/glitu}
api="https://api.github.com/repos/$repo"
: "${GITHUB_TOKEN:?GITHUB_TOKEN is missing (CI variable)}"
: "${CI_COMMIT_SHA:?}"

die() { echo "github-release: $*" >&2; exit 1; }

# curl with the token read from stdin (`-K -`), never as an argument
gh_curl() {
  printf 'header = "Authorization: Bearer %s"\nheader = "Accept: application/vnd.github+json"\n' "$GITHUB_TOKEN" \
    | curl -sS -K - "$@"
}

# git with the token as an HTTP header given through the environment, never as an argument
gh_git() {
  GIT_CONFIG_COUNT=1 \
  GIT_CONFIG_KEY_0="http.https://github.com/.extraheader" \
  GIT_CONFIG_VALUE_0="AUTHORIZATION: basic $(printf 'x-access-token:%s' "$GITHUB_TOKEN" | base64 | tr -d '\n')" \
    git "$@"
}

[ -f release.env ] || die "release.env is missing (tools/next-version.sh first)"
. ./release.env
version=$NEXT_VERSION
tag="v$version"

remote="https://github.com/$repo.git"
echo "main → $remote ($CI_COMMIT_SHA)"
gh_git push -q "$remote" "$CI_COMMIT_SHA:refs/heads/main"
[ -n "$version" ] || { echo "nothing to release"; exit 0; }

code=$(gh_curl -o /dev/null -w '%{http_code}' "$api/releases/tags/$tag")
case "$code" in
  200) echo "$tag is already released: nothing to publish"; exit 0 ;;
  404) ;;
  *) die "GitHub API: HTTP $code while looking for the release $tag" ;;
esac

ls dist/glitu-* > /dev/null 2>&1 || die "no binary in dist/"
echo "tag $tag → $CI_COMMIT_SHA"
gh_git push -q "$remote" "$CI_COMMIT_SHA:refs/tags/$tag"

# the description: what to download, and the SHA-256 of each binary, one line each
notes=$(cat <<EOF
One binary per platform: download it, make it executable, run it.

| Your machine | File |
|---|---|
| Linux, Intel or AMD | \`glitu-linux-x86_64\` |
| Linux, ARM (Raspberry Pi 4/5, Graviton…) | \`glitu-linux-arm64\` |
| Mac with Apple silicon (M1 and later) | \`glitu-macos-arm64\` |
| Windows, Intel or AMD | \`glitu-windows-x86_64.exe\` |

Linux and macOS in one line: \`curl -fsSL https://raw.githubusercontent.com/$repo/main/tools/install.sh | sh\`

Check it against its line below (\`sha256sum -c\`, or \`shasum -a 256 -c\` on macOS).
Start with \`glitu init talk && glitu serve talk/slides.md\`.

\`\`\`
$(cd dist && sha256sum glitu-*)
\`\`\`
EOF
)
pre=false
case "$version" in *-*) pre=true ;; esac
# the request goes through a file: curl's stdin carries the token (`-K -`)
body=$(mktemp)
trap 'rm -f "$body"' EXIT
jq -n --arg tag "$tag" --arg sha "$CI_COMMIT_SHA" --arg notes "$notes" --argjson pre "$pre" \
  '{tag_name: $tag, target_commitish: $sha, name: ("glitu " + $tag), body: $notes, prerelease: $pre}' > "$body"
release=$(gh_curl -f -X POST --data-binary "@$body" "$api/releases") \
  || die "the release $tag could not be created"
id=$(printf '%s' "$release" | jq -r .id)
echo "release $tag created (id $id)"

for f in dist/glitu-*; do
  name=$(basename "$f")
  gh_curl -f -o /dev/null -X POST -H "Content-Type: application/octet-stream" --data-binary "@$f" \
    "https://uploads.github.com/repos/$repo/releases/$id/assets?name=$name" \
    || die "upload of $name failed"
  echo "  $name: $(wc -c < "$f") bytes"
done
printf '%s' "$release" | jq -r .html_url
