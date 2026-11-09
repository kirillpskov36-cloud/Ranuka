#!/usr/bin/env bash
# One command that runs everything CI runs. Usage: ./scripts/check.sh
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== fmt"
cargo fmt --all -- --check

echo "== clippy"
cargo clippy --workspace --all-targets "$@" -- -D warnings

echo "== test"
cargo test --workspace "$@"

echo "== ok"
