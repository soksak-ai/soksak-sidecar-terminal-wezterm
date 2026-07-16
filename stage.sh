#!/bin/bash
# dist 스테이징의 단일 진실 — dev 와 CI 가 같은 스크립트를 쓴다. release 바이너리를 빌드해
# dist/soksak-sidecar-terminal-wezterm 로 원자 배치(resolve_sidecar_cmd 경로 규격).
# 사용: stage.sh [<dist-dir>] [<target-triple>]
#   target 를 주면 그 타깃으로 크로스 빌드해 target/<triple>/release 에서 집는다(멀티플랫폼 CI).
#   생략하면 호스트 네이티브(dev). Windows 타깃은 .exe 확장자를 붙인다.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

dist="${1:-dist}"
target="${2:-}"
name="soksak-sidecar-terminal-wezterm"

# 실행 파일 확장자 — Windows 타깃만 .exe.
ext=""
case "$target" in *windows*) ext=".exe" ;; esac

if [ -n "$target" ]; then
  cargo build --release --target "$target" --bin "$name"
  reldir="$target/release"
else
  cargo build --release --bin "$name"
  reldir="release"
fi

# target dir 은 CARGO_TARGET_DIR 로만 재정의된다(설정 override 는 그 env 로 CI 에 전달) —
# 없으면 워크스페이스 기본 ./target. python/jq 같은 런타임 의존 없이 전 러너에서 동작한다.
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
src="$TARGET_DIR/$reldir/$name$ext"
if [ ! -f "$src" ]; then
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
mv -f "$tmp" "$dist/$name$ext"
echo "staged: $dist/$name$ext"
