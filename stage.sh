#!/bin/bash
# dist 스테이징의 단일 진실 — dev 와 CI 가 같은 스크립트를 쓴다. release 바이너리를 빌드해
# dist/soksak-sidecar-terminal-wezterm 로 원자 배치(resolve_sidecar_cmd 경로 규격).
# 사용: stage.sh [<dist-dir>]   (기본 dist/)
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

dist="${1:-dist}"
name="soksak-sidecar-terminal-wezterm"

cargo build --release --bin "$name"

# 독립 워크스페이스라도 target dir 은 설정에 따라 다를 수 있다 — 가정하지 말고 해석.
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
src="$TARGET_DIR/release/$name"
if [ ! -x "$src" ]; then
  echo "release binary not found at $src" >&2
  exit 1
fi

mkdir -p "$dist"
# 원자적 교체(temp + mv). in-place cp 로 실행 중 바이너리의 inode 를 덮으면 이미 mmap 한
# 프로세스가 서명/페이지 불일치로 죽을 수 있다 — rename 은 새 inode 를 주어 옛 매핑과
# 분리한다(reach fetch 의 원자 install 과 동형 — 실행 중 바이너리를 지키는 rename 규율).
tmp="$dist/.$name.tmp.$$"
cp "$src" "$tmp"
chmod +x "$tmp"
mv -f "$tmp" "$dist/$name"
echo "staged: $dist/$name"
