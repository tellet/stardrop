#!/usr/bin/env bash
# Rewrites docs/THIRD-PARTY.md from the dependency tree: every crate that
# ends up in the shipped binary, and the licence it is offered under.
#   ./scripts/third-party.sh
set -euo pipefail
cd "$(dirname "$0")/.."

cargo metadata --format-version 1 --filter-platform x86_64-unknown-linux-gnu \
    | python3 -I scripts/third-party.py
