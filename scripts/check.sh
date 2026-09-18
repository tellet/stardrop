#!/usr/bin/env bash
# Quality gate (STARDROP.md §10). Run before every commit: ./scripts/check.sh
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> fmt";     cargo fmt --all -- --check
echo "==> clippy";  cargo clippy --all-targets -- -D warnings
echo "==> test";    cargo test
echo "==> release"; cargo build --release
echo "All checks passed."
