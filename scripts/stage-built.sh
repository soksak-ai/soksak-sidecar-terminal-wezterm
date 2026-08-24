#!/bin/sh
set -eu
[ "$#" -eq 2 ] && [ -n "$1" ] && [ -n "$2" ] || { echo 'usage: stage-built.sh <out> <target>' >&2; exit 2; }
out=$1
target=$2
repository=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# An absolute candidate output is allowed only outside the source repository.
case "$out" in ''|/|.|*..*|"$repository"|"$repository"/*) echo 'stage output is unsafe or inside the source repository' >&2; exit 2 ;; esac
name=soksak-sidecar-terminal-wezterm
case "$target" in *windows*) ext=.exe ;; *) ext= ;; esac
source=target/$target/release/$name$ext
[ -f "$source" ] || { echo "release binary is missing: $source" >&2; exit 1; }
mkdir -p "$out"
[ ! -L "$out" ] || { echo 'stage output must not be a symbolic link' >&2; exit 2; }
staged=$name$ext
if [ -e "$out/$staged" ]; then
  cmp -s "$source" "$out/$staged" || { echo "staged binary conflicts with current build" >&2; exit 1; }
else
  cp "$source" "$out/.$staged.next.$$"
  chmod +x "$out/.$staged.next.$$"
  mv "$out/.$staged.next.$$" "$out/$staged"
fi
generated=$out/.sidecar.json.next.$$
sed "s#\"process\": \"dist/$name\"#\"process\": \"dist/$staged\"#" sidecar.json > "$generated"
if [ -e "$out/sidecar.json" ]; then
  cmp -s "$generated" "$out/sidecar.json" || { echo "staged manifest conflicts with source" >&2; exit 1; }
  find "$generated" -delete
else
  mv "$generated" "$out/sidecar.json"
fi
echo "SIDECAR_STAGED target=$target output=$out/$staged"
