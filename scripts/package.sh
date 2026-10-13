#!/usr/bin/env bash
# Builds the version we ship: one standalone binary with its assets inside.
#   ./scripts/package.sh
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> building (fat LTO, this takes a few minutes)"
cargo build --profile dist

out="package"
rm -rf "$out"
mkdir -p "$out"
cp target/dist/stardrop "$out/"
cp README.md "$out/"

size=$(du -h "$out/stardrop" | cut -f1)
echo "==> $out/stardrop ($size), plus the README"
echo "    It needs nothing else: assets are inside the binary."
