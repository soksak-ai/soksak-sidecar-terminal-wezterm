#!/bin/bash
# 이 유닛의 **합격 판정 경로**. 계약이 요구하는 것을 전부, 한 번에, blocking 으로 돌린다 —
# 적합성 7종(선언된 골든) + lib 유닛 + service_down + 실 데몬 통합 + 성능 예산(SPEC.md §14.2).
#
# 예산 시험은 평시 `cargo test` 에서 #[ignore] 다(벤치가 개발 루프를 느리게 하고 노이즈를 낸다).
# 그래서 여기서 명시적으로 부른다 — 게이트 밖에서 예산은 검사되지 않고, 게이트 안에서 예산은
# 비켜갈 수 없다. 어느 한 단계라도 실패하면 이 스크립트가 실패한다.
#
# 사용: scripts/gate.sh [<bench-out-dir>]
#   bench-out-dir 를 주면 측정 결과를 거기에 남긴다(계약의 함대 게이트가 상대 가드를 볼 때 쓴다).
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

cd "$(dirname "$0")/.."
UNIT="soksak-sidecar-terminal-wezterm"
BENCH_OUT="${1:-}"

echo "== $UNIT: 적합성 + 유닛 + 통합"
cargo test --release

# 예산의 상대는 **수요**이고, 수요는 실 데몬이 tee 로 배달하는 속도다(SPEC.md §14.1). 그러니
# 예산을 판정하려면 데몬이 있어야 한다 — 없으면 판정을 건너뛰는 것이 아니라 판정이 불가능하다.
# 실 데몬 통합 시험이 이미 쓰는 하니스가 그것을 코어에서 빌드한다.
if [ -z "${SOKSAK_PTYD_BIN:-}" ]; then
  CORE="${SOKSAK_CORE_WORKTREE:-/Users/max/ai/cli/vsterm-tauri}"
  echo "== 수요 측정용 데몬을 빌드한다($CORE)"
  ( cd "$CORE/src-tauri" && cargo build --release -p soksak-ptyd )
  TARGET_DIR="$(cd "$CORE/src-tauri" && cargo metadata --format-version 1 --no-deps \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
  export SOKSAK_PTYD_BIN="$TARGET_DIR/release/soksak-ptyd"
fi

echo "== $UNIT: 성능 예산(SPEC.md §14.2) — 수요는 실 데몬이 배달하는 속도다"
if [ -n "$BENCH_OUT" ]; then
  SOKSAK_BENCH_OUT="$BENCH_OUT" cargo test --release --test bench -- --ignored --nocapture
else
  cargo test --release --test bench -- --ignored --nocapture
fi

echo "== $UNIT: GATE PASS"
