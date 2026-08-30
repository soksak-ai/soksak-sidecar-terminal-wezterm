#!/bin/sh
# Compare complete sidecar releases produced from the same commit in two clean roots.
set -eu

[ "$#" -eq 2 ] && [ -n "$1" ] && [ -n "$2" ] || {
  echo 'usage: check-cross-root-release.sh <left-release> <right-release>' >&2
  exit 2
}

left=$1
right=$2
for release in "$left" "$right"; do
  case "$release" in /*) ;; *) echo "cross-root release input must be absolute: $release" >&2; exit 2 ;; esac
  [ -d "$release" ] && [ ! -L "$release" ] || {
    echo "cross-root release input is not a regular directory: $release" >&2
    exit 2
  }
  if find "$release" -type l -print -quit | grep -q .; then
    echo "cross-root release input contains a symbolic link: $release" >&2
    exit 2
  fi
done

temp_root=$(CDPATH= cd -- "${TMPDIR:-/tmp}" && pwd -P)
work=$(mktemp -d "$temp_root/soksak-cross-root-release.XXXXXX")
trap 'find "$work" -depth -delete' EXIT HUP INT TERM
(cd "$left" && find . -type f -print | LC_ALL=C sort) > "$work/left.files"
(cd "$right" && find . -type f -print | LC_ALL=C sort) > "$work/right.files"
if ! cmp -s "$work/left.files" "$work/right.files"; then
  echo 'CROSS_ROOT_RELEASE_FILE_SET_MISMATCH' >&2
  diff -u "$work/left.files" "$work/right.files" >&2 || true
  exit 1
fi

while IFS= read -r relative; do
  if ! cmp -s "$left/$relative" "$right/$relative"; then
    left_sha=$(shasum -a 256 "$left/$relative" | awk '{print $1}')
    right_sha=$(shasum -a 256 "$right/$relative" | awk '{print $1}')
    printf 'CROSS_ROOT_RELEASE_BYTE_MISMATCH file=%s left_sha256=%s right_sha256=%s\n' \
      "$relative" "$left_sha" "$right_sha" >&2
    exit 1
  fi
done < "$work/left.files"

printf 'CROSS_ROOT_RELEASE_REPRODUCIBLE files=%s\n' "$(wc -l < "$work/left.files" | tr -d ' ')"
