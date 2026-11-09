#!/usr/bin/env bash
# One command that runs everything CI runs. Usage: ./scripts/check.sh
#
# The program's tests run the compiled program, which needs the Solana CLI and
# Anchor. Without them the rest is still checked and those tests are skipped
# with a notice. CI always has the toolchain and never skips.
set -euo pipefail
cd "$(dirname "$0")/.."

PROGRAM_SO=target/deploy/ranuka_program.so
test_scope=(--workspace)

if command -v anchor >/dev/null 2>&1; then
    echo "== build program"
    anchor build --ignore-keys
elif [ -n "${CI:-}" ]; then
    echo "error: Anchor is not installed in CI; refusing to skip the program tests" >&2
    exit 1
elif [ ! -f "$PROGRAM_SO" ]; then
    skipped=1
    test_scope=(--workspace --exclude ranuka-program)
fi

echo "== fmt"
cargo fmt --all -- --check

echo "== clippy"
cargo clippy --workspace --all-targets "$@" -- -D warnings

echo "== test"
cargo test "${test_scope[@]}" "$@"

if [ -n "${skipped:-}" ]; then
    echo
    echo "NOTE: program tests were SKIPPED: Anchor is not installed and the program"
    echo "      is not built. Everything else passed. Install the Solana CLI and"
    echo "      Anchor (see README) to run them locally; CI runs them on every push."
fi

echo "== ok"
