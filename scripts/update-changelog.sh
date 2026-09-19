#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-}"
LOCAL_BIN="$REPO_ROOT/desktop/node_modules/.bin/git-cliff"
if [[ -x "$LOCAL_BIN" ]]; then
  GIT_CLIFF="$LOCAL_BIN"
elif command -v git-cliff &>/dev/null; then
  GIT_CLIFF="git-cliff"
else
  echo "git-cliff not found. Run: cd desktop && npm install"
  exit 1
fi
echo "Using: $GIT_CLIFF ($($GIT_CLIFF --version))"
if [[ -z "$VERSION" ]]; then
  echo "Preview [Unreleased]:"
  "$GIT_CLIFF" --unreleased --config "$REPO_ROOT/cliff.toml"
  echo "Tip: run 'scripts/update-changelog.sh v0.17.0' to write"
else
  if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Bad version format. Expected vX.Y.Z"
    exit 1
  fi
  "$GIT_CLIFF" --latest --config "$REPO_ROOT/cliff.toml" --tag "$VERSION" --prepend "$REPO_ROOT/CHANGELOG.md"
  echo "Done. CHANGELOG.md updated for $VERSION"
  echo "Next: git add CHANGELOG.md && git commit -m 'chore(release): update CHANGELOG for $VERSION'"
fi
