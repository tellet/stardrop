#!/usr/bin/env bash
# Quality gate (STARDROP.md §10). Run before every commit: ./scripts/check.sh
#   --smoke   also start the real game for a few seconds and fail if it panics
#             (opens a window; catches startup problems unit tests can't see).
#   --visual  also check that the shapes still look like themselves
#             (opens a window, needs a GPU; not run in CI for that reason).
set -euo pipefail
cd "$(dirname "$0")/.."

# The incremental cache is worth a gigabyte in `target/` and buys nothing
# here: this script compiles everything anyway.
export CARGO_INCREMENTAL=0

echo "==> fmt";     cargo fmt --all -- --check
echo "==> clippy";  cargo clippy --all-targets -- -D warnings
# Debug-only code (F2 shapes) is behind the `dev` feature; lint it too. CI
# skips this one: it is a second full compile of Bevy for very little code.
echo "==> clippy (dev)"; cargo clippy --all-targets --features dev -- -D warnings
echo "==> test";    cargo test
echo "==> release"; cargo build --release

if [[ "${1:-}" == "--visual" ]]; then
    echo "==> shapes"
    cargo run --release --example golden
fi

if [[ "${1:-}" == "--smoke" ]]; then
    echo "==> smoke (6 s)"
    log="$(mktemp)"
    # `timeout` stopping the game is the expected outcome (exit code 124).
    BEVY_ASSET_ROOT="$PWD" timeout 6 ./target/release/stardrop >"$log" 2>&1 || true
    if grep -qE "panicked|ERROR" "$log"; then
        grep -E "panicked|ERROR" -A3 "$log"
        rm -f "$log"
        echo "Smoke test FAILED."
        exit 1
    fi
    rm -f "$log"
fi
echo "All checks passed."
