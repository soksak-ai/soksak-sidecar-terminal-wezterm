#!/bin/sh
set -eu
[ "$#" -eq 1 ] && [ -n "$1" ] || { echo 'usage: check-build-environment.sh <target>' >&2; exit 78; }
target=$1
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) host_target=aarch64-apple-darwin; node_platform=darwin; node_arch=arm64 ;;
  Darwin-x86_64)
    if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" = 1 ]; then host_target=aarch64-apple-darwin; node_platform=darwin; node_arch=arm64;
    else host_target=x86_64-apple-darwin; node_platform=darwin; node_arch=x64; fi ;;
  Linux-aarch64|Linux-arm64) host_target=aarch64-unknown-linux-gnu; node_platform=linux; node_arch=arm64 ;;
  Linux-x86_64) host_target=x86_64-unknown-linux-gnu; node_platform=linux; node_arch=x64 ;;
  MINGW*-x86_64|MSYS*-x86_64|CYGWIN*-x86_64) host_target=x86_64-pc-windows-msvc; node_platform=win32; node_arch=x64 ;;
  *) echo "TOOLCHAIN_MISMATCH: unsupported native host $(uname -s)-$(uname -m)" >&2; exit 78 ;;
esac
rust_expected=$(sed -n 's/^channel = "\([^"]*\)"$/\1/p' rust-toolchain.toml)
rust_actual=$(rustc --version 2>/dev/null | awk '{print $2}' || true)
rust_host=$(rustc -vV 2>/dev/null | sed -n 's/^host: //p' || true)
node_actual_platform=$(node -p process.platform 2>/dev/null || true)
node_actual_arch=$(node -p process.arch 2>/dev/null || true)
if [ "$target" != "$host_target" ] || [ -z "$rust_expected" ] || [ "$rust_actual" != "$rust_expected" ] || \
   [ "$rust_host" != "$target" ] || [ "$node_actual_platform" != "$node_platform" ] || [ "$node_actual_arch" != "$node_arch" ]; then
  printf 'TOOLCHAIN_MISMATCH: target=%s hostTarget=%s rust=%s/%s nodeRuntime=%s/%s; expected rust=%s/%s nodeRuntime=%s/%s\n' \
    "$target" "$host_target" "${rust_actual:-missing}" "${rust_host:-unknown}" "${node_actual_platform:-unknown}" "${node_actual_arch:-unknown}" \
    "$rust_expected" "$target" "$node_platform" "$node_arch" >&2
  exit 78
fi
printf 'BUILD_ENVIRONMENT_READY target=%s rust=%s/%s nodeRuntime=%s/%s\n' "$target" "$rust_actual" "$rust_host" "$node_actual_platform" "$node_actual_arch"
