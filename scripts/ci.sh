#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

if command -v python3 >/dev/null 2>&1; then
  python3 scripts/check_plan_index.py
else
  echo "python3 not found; skipping docs/plan_index.json validation"
fi
