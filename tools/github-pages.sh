#!/bin/sh
# Publishes the website built by tools/site.sh on GitHub Pages, from the maintainer's CI (see
# .gitlab-ci.yml): the folder becomes the only commit of the `gh-pages` branch of
# github.com/$GITHUB_REPO, replaced at each publication (the branch holds nothing else). GitHub
# Pages serves that branch under the custom domain of the CNAME file.
#
#   tools/github-pages.sh [site/dist]
#
# Needs: git; GITHUB_TOKEN (allowed to push to the repository), never written in a command line or
# in the log; CI_COMMIT_SHA (optional, named in the commit message).
set -eu

repo=${GITHUB_REPO:-skemonet/glitu}
domain=${PAGES_DOMAIN:-glitu.net}
dir=${1:-site/dist}
: "${GITHUB_TOKEN:?GITHUB_TOKEN is missing (CI variable)}"
[ -s "$dir/index.html" ] || { echo "github-pages: $dir/index.html is missing (tools/site.sh first)" >&2; exit 1; }

echo "$domain" > "$dir/CNAME"
touch "$dir/.nojekyll"
cd "$dir"
rm -rf .git
git init -q -b gh-pages
git add -A
git -c user.name="glitu CI" -c user.email="ci@glitu.net" commit -q -m "Website of ${CI_COMMIT_SHA:-a local build}"
GIT_CONFIG_COUNT=1 \
GIT_CONFIG_KEY_0="http.https://github.com/.extraheader" \
GIT_CONFIG_VALUE_0="AUTHORIZATION: basic $(printf 'x-access-token:%s' "$GITHUB_TOKEN" | base64 | tr -d '\n')" \
  git push -q --force "https://github.com/$repo.git" gh-pages
echo "gh-pages: $(git ls-files | wc -l) files published ($domain)"
rm -rf .git
