#!/bin/bash
# Build the wasm-pack output and publish pkg/ to the `dist` branch on origin.
#
# Consumers install via: "hyasynth-engine": "github:<user>/hyasynth-engine#dist"
# The dist branch holds ONLY the pkg/ contents as its root — no source, no history.
# It is force-pushed on every release so the branch tip is always a fresh build.
#
# Uses whatever is on disk (including uncommitted changes). Commit first if you
# want the shipped artifact to match a specific revision.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "→ Building wasm-pack (target=web, profile=release)…"
wasm-pack build --target web --release --out-dir pkg -- --features web

REMOTE_URL=$(git remote get-url origin)
# Prefer SSH for push — HTTPS requires credential setup that may not be present.
PUSH_URL=$(printf '%s' "$REMOTE_URL" | sed -E 's#^https://github.com/#git@github.com:#')
SRC_REV=$(git rev-parse --short HEAD)
SRC_BRANCH=$(git rev-parse --abbrev-ref HEAD)

STAGING=$(mktemp -d)
trap 'rm -rf "$STAGING"' EXIT

cp -R pkg/. "$STAGING/"
# wasm-pack writes pkg/.gitignore ("*") to keep the parent repo from tracking
# build artifacts — but on dist that would ignore everything we want to publish.
rm -f "$STAGING/.gitignore"

cd "$STAGING"
git init -q -b dist
git add -A
GIT_AUTHOR_NAME=$(cd "$SCRIPT_DIR" && git config user.name) \
GIT_AUTHOR_EMAIL=$(cd "$SCRIPT_DIR" && git config user.email) \
GIT_COMMITTER_NAME=$(cd "$SCRIPT_DIR" && git config user.name) \
GIT_COMMITTER_EMAIL=$(cd "$SCRIPT_DIR" && git config user.email) \
  git commit -q -m "build: wasm-pack output from ${SRC_BRANCH}@${SRC_REV}"

git remote add origin "$PUSH_URL"
git push -f origin dist

echo "✓ Published pkg/ to ${REMOTE_URL} (branch: dist, source: ${SRC_BRANCH}@${SRC_REV})"
