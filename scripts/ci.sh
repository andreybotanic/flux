#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

if command -v python3 >/dev/null 2>&1; then
  python3 scripts/check_plan_index.py
elif command -v python >/dev/null 2>&1; then
  python scripts/check_plan_index.py
else
  echo "Neither python3 nor python was found; cannot validate docs/plan_index.json" >&2
  exit 1
fi
