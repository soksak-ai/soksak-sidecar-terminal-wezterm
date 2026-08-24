#!/bin/bash
# Sidecar owner verification uses its own implementation and contract fixtures only.
# The product-owned PTY demand benchmark is the separate Make benchmark target.
set -euo pipefail

cd "$(dirname "$0")/.."
SIDECAR="soksak-sidecar-terminal-wezterm"
TARGET="${1:?target triple is required}"

echo "== $SIDECAR: conformance and owner tests"
cargo test --locked --release --target "$TARGET"

echo "== $SIDECAR: GATE PASS"
