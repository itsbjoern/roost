#!/usr/bin/env bash
set -euo pipefail

# Release script: bumps version, commits, tags, and pushes.
# Usage: ./scripts/release.sh [major|minor|patch]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$ROOT_DIR/Cargo.toml"

bump="${1:-}"
push_auto=false
[[ "${2:-}" == "-y" || "${2:-}" == "--yes" ]] && push_auto=true

if [[ "$bump" != "major" && "$bump" != "minor" && "$bump" != "patch" ]]; then
  echo "Usage: $0 [major|minor|patch] [-y|--yes]"
  echo ""
  echo "  major  Bump X.0.0 (breaking changes)"
  echo "  minor  Bump 0.X.0 (new features)"
  echo "  patch  Bump 0.0.X (bug fixes)"
  echo ""
  echo "  -y, --yes  Push immediately without prompting"
  exit 1
fi

cd "$ROOT_DIR"

# Ensure we're in a git repo with a clean working tree
if ! git rev-parse --is-inside-work-tree &>/dev/null; then
  echo "Error: not in a git repository"
  exit 1
fi

if [[ -n $(git status --porcelain) ]]; then
  echo "Error: working tree has uncommitted changes"
  exit 1
fi

# Read current version from Cargo.toml
current=$(grep -E '^version\s*=' "$CARGO_TOML" | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
if [[ -z "$current" ]]; then
  echo "Error: could not read version from Cargo.toml"
  exit 1
fi

IFS='.' read -r ma mi pa <<< "$current"

case "$bump" in
  major) ma=$((ma + 1)); mi=0; pa=0 ;;
  minor) mi=$((mi + 1)); pa=0 ;;
  patch) pa=$((pa + 1)) ;;
esac

new_version="$ma.$mi.$pa"
echo "Bumping version: $current -> $new_version"

# Update Cargo.toml
if [[ "$(uname)" == "Darwin" ]]; then
  sed -i '' "s/^version = .*/version = \"$new_version\"/" "$CARGO_TOML"
else
  sed -i "s/^version = .*/version = \"$new_version\"/" "$CARGO_TOML"
fi

# Update Cargo.lock
cargo build --release -q

# Commit and tag
git add Cargo.toml Cargo.lock
git commit -m "Release v$new_version"
git tag "v$new_version"

echo ""
echo "Committed and tagged v$new_version"
echo ""

if $push_auto; then
  git push
  git push origin "v$new_version"
  echo ""
  echo "Pushed. Release workflow will run at: https://github.com/itsbjoern/roost/actions"
else
  echo "Push with:"
  echo "  git push"
  echo "  git push origin v$new_version"
  echo ""
  read -r -p "Push now? [y/N] " resp
  if [[ "$resp" =~ ^[yY]$ ]]; then
    git push
    git push origin "v$new_version"
    echo ""
    echo "Pushed. Release workflow will run at: https://github.com/itsbjoern/roost/actions"
  fi
fi
