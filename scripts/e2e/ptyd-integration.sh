#!/bin/bash
# 실 ptyd 통합의 단일 진실 하니스 — 코어 워크트리에서 soksak-ptyd 를 빌드해 그 바이너리를
# SOKSAK_PTYD_BIN 으로 주입하고 tests/ptyd_integration.rs 를 돌린다. 멱등·재사용(임시
# 스크립트 아님). 이 유닛(soksak-sidecar-terminal-wezterm)은 코어를 링크하지 않으므로 코어
# 경로는 주입한다. 데몬 wire 는 엔진-불가지라 wezterm 미러도 같은 왕복을 탄다.
#
# 사용: SOKSAK_CORE_WORKTREE=<코어 워크트리> scripts/e2e/ptyd-integration.sh
#   기본 코어 경로가 맞으면 인자 없이 실행 가능.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

SIDECAR_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
CORE="${SOKSAK_CORE_WORKTREE:-/Users/max/ai/cli/vsterm-tauri/.claude/worktrees/p0-contracts}"

if [ ! -d "$CORE/src-tauri" ]; then
  echo "core worktree not found at $CORE (set SOKSAK_CORE_WORKTREE)" >&2
  exit 1
fi

echo "building soksak-ptyd in $CORE/src-tauri ..."
( cd "$CORE/src-tauri" && cargo build -p soksak-ptyd )

# target dir 은 워크스페이스 설정에 따라 공유될 수 있다(worktree → 본체 target). 경로를
# 가정하지 말고 cargo metadata 로 해석한다.
TARGET_DIR="$(cd "$CORE/src-tauri" && cargo metadata --format-version 1 --no-deps \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
BIN="$TARGET_DIR/debug/soksak-ptyd"
if [ ! -x "$BIN" ]; then
  echo "ptyd binary not found at $BIN after build" >&2
  exit 1
fi

export SOKSAK_PTYD_BIN="$BIN"
echo "running real-daemon integration with SOKSAK_PTYD_BIN=$BIN"
cd "$SIDECAR_DIR"
cargo test --test ptyd_integration -- --nocapture --test-threads=1
