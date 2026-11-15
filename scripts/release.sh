#!/usr/bin/env bash
#
# Cuts a release of Querydown: sets the version of every package, checks that the release is ready,
# publishes the crates to crates.io and the packages to npm, and tags the release. RELEASING.md
# describes the whole process, of which this script is the mechanical part.
#
#   scripts/release.sh prepare <version>   Set every package's version, and start the release notes
#   scripts/release.sh check               Build the packages and dry-run publishing them (quick)
#   scripts/release.sh verify              `check`, plus every test. Required before `publish`.
#   scripts/release.sh publish             Publish to crates.io and npm, and tag the release
#   scripts/release.sh github-release      Create the GitHub release, once the tag is pushed
#
# Run it in the dev container (see DEVELOPMENT.md), which has the tools that the tests need.
# `github-release` is the exception: run it wherever you can push to GitHub.

set -euo pipefail

cd "$(dirname "$0")/.."

# The crates, in the order they're published (each depends on the one before it).
CRATES=(querydown-parser querydown querydown-cli)
# The npm packages, by directory. `bindings/js/pkg` is built by `wasm-pack` from `bindings/js`.
NPM_PACKAGES=(bindings/js/pkg bindings/codemirror)
REPO=seancolsen/querydown
SITE=https://querydown.org
NOTES_DIR=site/content/releases
UNRELEASED_NOTE="Not yet released"
VERIFIED_MARKER=target/release-verified

# Output
# ------------------------------------------------------------------------------------------------

step() { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }
ok() { printf '\033[32m✓\033[0m %s\n' "$*"; }
warn() { printf '\033[33mwarning:\033[0m %s\n' "$*" >&2; }
fail() {
  printf '\033[31merror:\033[0m %s\n' "$*" >&2
  exit 1
}

# Versions
# ------------------------------------------------------------------------------------------------

# The version in `[workspace.package]` of Cargo.toml, which every package shares.
version() {
  sed -n '/^\[workspace\.package\]/,/^\[/ s/^version = "\(.*\)"$/\1/p' Cargo.toml
}

is_prerelease() { [[ $1 == *-* ]]; }

notes_file() { echo "$NOTES_DIR/$1.md"; }

# Checks that every manifest and lockfile agrees on the version.
check_versions() {
  local v=$1 problems=()
  local cargo_versions
  cargo_versions=$(cargo metadata --format-version 1 --no-deps --locked 2>/dev/null |
    node -e 'const m = JSON.parse(require("fs").readFileSync(0, "utf8"));
      for (const p of m.packages) console.log(`${p.name} ${p.version}`);') ||
    fail "Cargo.lock is out of date. Run \`scripts/release.sh prepare $v\` again."
  while read -r name crate_version; do
    [[ $crate_version == "$v" ]] || problems+=("crate $name is $crate_version")
  done <<<"$cargo_versions"
  for dep in querydown querydown-parser; do
    grep -q "^$dep = { version = \"=$v\"" Cargo.toml ||
      problems+=("the pin of $dep in [workspace.dependencies] isn't =$v")
  done
  local cm
  cm=$(node -p 'const p = require("./bindings/codemirror/package.json");
    const l = require("./bindings/codemirror/package-lock.json");
    [p.version, p.peerDependencies["querydown-js"], l.version].join(" ")')
  read -r cm_version cm_peer cm_lock <<<"$cm"
  [[ $cm_version == "$v" ]] || problems+=("querydown-codemirror is $cm_version")
  [[ $cm_peer == "^$v" ]] || problems+=("querydown-codemirror's peer querydown-js is $cm_peer")
  [[ $cm_lock == "$v" ]] || problems+=("bindings/codemirror/package-lock.json says $cm_lock")
  if ((${#problems[@]})); then
    printf '  %s\n' "${problems[@]}" >&2
    fail "Not every package is at version $v. Run \`scripts/release.sh prepare $v\`."
  fi
  ok "Every package is at version $v"
}

# Registries
# ------------------------------------------------------------------------------------------------

# Whether a version of a crate is on crates.io, according to its sparse index.
crate_published() {
  local name=$1 v=$2
  curl -fsS "https://index.crates.io/${name:0:2}/${name:2:2}/$name" 2>/dev/null |
    grep -q "\"vers\":\"$v\""
}

# Whether a version of an npm package is on npm.
npm_published() {
  local name=$1 v=$2
  [[ $(npm view "$name@$v" version 2>/dev/null) == "$v" ]]
}

npm_name() { node -p "require('./$1/package.json').name"; }

# Preflight
# ------------------------------------------------------------------------------------------------

require_tools() {
  local tool
  for tool in "$@"; do
    command -v "$tool" >/dev/null || fail "\`$tool\` isn't installed. Run this in the dev container."
  done
}

# Checks the state of the repository: on `main`, with nothing uncommitted, and nothing on GitHub
# that's missing here.
check_git() {
  local v=$1
  [[ $(git branch --show-current) == main ]] || fail "Release from the main branch."
  [[ -z $(git status --porcelain) ]] ||
    fail "The working tree has changes. Commit them first (the packages are built from HEAD)."
  if git fetch --quiet origin main 2>/dev/null; then
    git merge-base --is-ancestor origin/main HEAD ||
      fail "origin/main has commits that HEAD doesn't. Pull them first."
    if git ls-remote --exit-code --tags origin "v$v" >/dev/null 2>&1; then
      fail "The tag v$v is already on GitHub."
    fi
  else
    warn "Couldn't fetch from GitHub, so I can't check that HEAD is up to date with it."
  fi
  if git rev-parse -q --verify "refs/tags/v$v" >/dev/null; then
    fail "The tag v$v already exists here."
  fi
  ok "On main at $(git rev-parse --short HEAD), with no uncommitted changes"
}

# Checks that the release notes exist and are final.
check_notes() {
  local v=$1 file
  file=$(notes_file "$v")
  [[ -f $file ]] || fail "There are no release notes for $v. Write $file."
  if grep -q "$UNRELEASED_NOTE" "$file"; then
    fail "$file still says \"$UNRELEASED_NOTE\". Replace that note with the date of the release."
  fi
  ok "The release notes, $file, are final"
}

# Checks that none of the packages has been published at this version.
check_unpublished() {
  local v=$1 name dir published=()
  for name in "${CRATES[@]}"; do
    crate_published "$name" "$v" && published+=("$name (crates.io)")
  done
  for dir in "${NPM_PACKAGES[@]}"; do
    [[ -f $dir/package.json ]] || continue # `bindings/js/pkg` before its first build
    name=$(npm_name "$dir")
    npm_published "$name" "$v" && published+=("$name (npm)")
  done
  if ((${#published[@]})); then
    printf '  %s\n' "${published[@]}" >&2
    fail "Version $v of these is already published. If an earlier \`publish\` stopped partway, run it again; it skips what's done."
  fi
  ok "Version $v isn't published anywhere yet"
}

# Building and packaging
# ------------------------------------------------------------------------------------------------

build_js_package() {
  local v=$1
  wasm-pack build bindings/js --release --target web
  local built
  built=$(node -p 'const p = require("./bindings/js/pkg/package.json"); `${p.name} ${p.version}`')
  [[ $built == "querydown-js $v" ]] || fail "wasm-pack built $built, not querydown-js $v."
}

# Builds every package, and does everything short of publishing them.
check_packages() {
  local v=$1
  step "Building the querydown-js package"
  build_js_package "$v"
  (cd bindings/js/pkg && npm pack --dry-run)

  step "Building the querydown-codemirror package"
  (cd bindings/codemirror && npm ci --no-audit --no-fund && npm pack --dry-run)

  step "Packaging the crates"
  # With `--workspace`, Cargo verifies each crate against the others as they'll be published, even
  # though the earlier ones aren't on crates.io yet.
  cargo publish --workspace --dry-run
  ok "Every package builds, and is ready to publish"
}

run_tests() {
  step "Checking formatting"
  cargo fmt --all --check

  step "Running Clippy"
  cargo clippy --all-targets -- -D warnings

  step "Running the Rust tests, against real PostgreSQL and DuckDB databases"
  cargo test --features db-tests

  step "Running the JavaScript bindings' tests"
  wasm-pack test --node bindings/js

  step "Running the CodeMirror package's tests"
  (cd bindings/codemirror &&
    npm ci --no-audit --no-fund &&
    npm run typecheck &&
    npm run build:wasm &&
    npm run test:visual)

  step "Building the site, which compiles every example and checks every link"
  (cd site && npm ci --no-audit --no-fund && npm run build)
}

# Commands
# ------------------------------------------------------------------------------------------------

cmd_prepare() {
  local v=${1:-}
  [[ $v =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]] ||
    fail "Give a version, such as 0.2.0 or 0.2.0-rc.1."
  require_tools cargo node npm
  local old
  old=$(version)

  step "Setting every package's version to $v (it was $old)"
  sed -i "/^\[workspace\.package\]/,/^\[/ s/^version = \".*\"$/version = \"$v\"/" Cargo.toml
  sed -i -E "s/^(querydown(-parser)? = \{ version = \")=[^\"]*\"/\1=$v\"/" Cargo.toml
  cargo metadata --format-version 1 >/dev/null # Updates Cargo.lock
  (cd bindings/codemirror &&
    npm version "$v" --no-git-tag-version --allow-same-version >/dev/null &&
    npm pkg set "peerDependencies.querydown-js=^$v" &&
    npm install --package-lock-only --ignore-scripts --no-audit --no-fund --silent)
  check_versions "$v"

  local file
  file=$(notes_file "$v")
  if [[ ! -f $file ]]; then
    cat >"$file" <<EOF
# $v

_In development. $UNRELEASED_NOTE._

One or two sentences that sum up the release, for the page's description.

## How to get it

See [installation](../../../docs/setup/installation.md).

## What's new

-

## Changes that break things

These change the behavior of queries or code written for the previous release:

-
EOF
    ok "Started the release notes, $file"
  fi

  cat <<EOF

Next:
  1. Write the release notes, $file. When they're done, replace the
     "$UNRELEASED_NOTE" note with the date of the release, as in "_Released on $(date +%Y-%m-%d)._".
  2. Commit, with a message such as "Release $v".
  3. Run \`scripts/release.sh verify\`.
EOF
}

cmd_check() {
  require_tools cargo wasm-pack node npm git curl
  local v
  v=$(version)
  step "Checking that $v is ready to release"
  check_versions "$v"
  check_notes "$v"
  check_git "$v"
  check_unpublished "$v"
  check_packages "$v"
}

cmd_verify() {
  cmd_check
  run_tests
  local v
  v=$(version)
  mkdir -p "$(dirname "$VERIFIED_MARKER")"
  echo "$(git rev-parse HEAD) $v" >"$VERIFIED_MARKER"
  step "$v is verified, at $(git rev-parse --short HEAD)"
  echo "Next: \`scripts/release.sh publish\`."
}

cmd_publish() {
  require_tools cargo wasm-pack node npm git curl
  local v head
  v=$(version)
  head=$(git rev-parse HEAD)
  step "Publishing $v"
  check_versions "$v"
  check_notes "$v"
  [[ -z $(git status --porcelain) ]] || fail "The working tree has changes."
  [[ -f $VERIFIED_MARKER && $(cat "$VERIFIED_MARKER") == "$head $v" ]] ||
    fail "Run \`scripts/release.sh verify\` first. (It hasn't passed at HEAD.)"
  ok "Verified at $(git rev-parse --short HEAD)"

  local npm_user
  npm_user=$(npm whoami 2>/dev/null) || fail "Log in to npm first, with \`npm login\`."
  ok "Logged in to npm as $npm_user"
  if [[ -z ${CARGO_REGISTRY_TOKEN:-} && ! -f ${CARGO_HOME:-$HOME/.cargo}/credentials.toml ]]; then
    fail "Log in to crates.io first, with \`cargo login\`."
  fi

  local npm_tag=latest
  is_prerelease "$v" && npm_tag=next

  echo
  echo "This publishes version $v of ${CRATES[*]} to crates.io, and of"
  echo "querydown-js and querydown-codemirror to npm (with the dist-tag \"$npm_tag\")."
  echo "A published version can't be replaced, only yanked or deprecated."
  read -r -p "Publish? [y/N] " answer
  [[ $answer == [yY] ]] || fail "Stopped."

  local name
  for name in "${CRATES[@]}"; do
    step "Publishing $name to crates.io"
    if crate_published "$name" "$v"; then
      ok "Already published"
    else
      cargo publish -p "$name"
    fi
  done

  step "Publishing querydown-js to npm"
  if npm_published querydown-js "$v"; then
    ok "Already published"
  else
    build_js_package "$v"
    (cd bindings/js/pkg && npm publish --tag "$npm_tag")
  fi

  step "Publishing querydown-codemirror to npm"
  if npm_published querydown-codemirror "$v"; then
    ok "Already published"
  else
    (cd bindings/codemirror && npm ci --no-audit --no-fund && npm publish --tag "$npm_tag")
  fi

  step "Tagging the release"
  git tag -a "v$v" -m "Querydown $v"
  ok "Tagged v$v"

  cat <<EOF

$v is published. Next, from wherever you can push to GitHub:
  1. git push origin main v$v
  2. scripts/release.sh github-release
  3. Deploy the site, and run the checks after a release (RELEASING.md).
EOF
}

# The release notes as Markdown for GitHub, whose relative links lead to the site.
github_release_notes() {
  local v=$1
  sed -e '1{/^# /d;}' \
    -e "s|](\.\./\.\./\.\./docs/\([^)#]*\)\.md|]($SITE/docs/\1|g" \
    -e "s|](\./\([^)#]*\)\.md|]($SITE/releases/\1|g" \
    "$(notes_file "$v")"
  printf '\n---\n\nThese notes are also at %s/releases/%s.\n' "$SITE" "$v"
}

cmd_github_release() {
  local v
  v=$(version)
  git rev-parse -q --verify "refs/tags/v$v" >/dev/null || fail "There's no tag v$v. Run \`publish\` first."
  local notes
  notes=$(github_release_notes "$v")
  local prerelease=()
  is_prerelease "$v" && prerelease=(--prerelease)
  if command -v gh >/dev/null; then
    gh release create "v$v" --repo "$REPO" --verify-tag --title "Querydown $v" \
      --notes "$notes" ${prerelease[@]+"${prerelease[@]}"}
  else
    local out=target/github-release-$v.md
    mkdir -p target
    echo "$notes" >"$out"
    cat <<EOF
The GitHub CLI (\`gh\`) isn't installed, so create the release in your browser:

  1. Open https://github.com/$REPO/releases/new?tag=v$v
  2. Set the title to "Querydown $v".
  3. Paste the notes from $out.
EOF
    is_prerelease "$v" && echo "  4. Tick \"Set as a pre-release\"."
    echo "  Then publish the release."
  fi
}

case ${1:-} in
prepare) cmd_prepare "${2:-}" ;;
check) cmd_check ;;
verify) cmd_verify ;;
publish) cmd_publish ;;
github-release) cmd_github_release ;;
*)
  sed -n '3,15p' "$0" | sed 's/^# \{0,1\}//'
  exit 1
  ;;
esac
