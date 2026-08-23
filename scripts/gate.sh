#!/bin/bash
# 이 유닛의 **합격 판정 경로**. 계약이 요구하는 것을 전부, 한 번에, blocking 으로 돌린다 —
# 적합성 7종(선언된 reference state) + lib 유닛 + service_down + 성능 예산(SPEC.md §14.2).
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
SIDECAR="soksak-sidecar-terminal-wezterm"
BENCH_OUT="${1:-}"

echo "== $SIDECAR: conformance and owner tests"
cargo test --release

echo "== $SIDECAR: performance budget (SPEC.md §14.2)"
if [ -n "$BENCH_OUT" ]; then
  SOKSAK_BENCH_OUT="$BENCH_OUT" cargo test --release --test bench -- --ignored --nocapture
else
  cargo test --release --test bench -- --ignored --nocapture
fi

echo "== $SIDECAR: GATE PASS"
