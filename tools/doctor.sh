#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

check_cmd() {
  local label="$1"
  local cmd="$2"
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "[ok]  ${label}: $(command -v "$cmd")"
    return 0
  fi
  echo "[err] ${label}: missing" >&2
  return 1
}

check_optional() {
  local label="$1"
  local cmd="$2"
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "[ok]  ${label}: $(command -v "$cmd")"
  else
    echo "[warn] ${label}: missing" >&2
  fi
}

status=0

echo "Ruzzle OS toolchain doctor"
check_cmd "cargo" cargo || status=1
check_cmd "rustc" rustc || status=1
check_cmd "python3" python3 || status=1
check_cmd "xorriso" xorriso || status=1
check_cmd "curl" curl || status=1
check_cmd "tar" tar || status=1
check_cmd "make" make || status=1
check_cmd "nasm" nasm || status=1
check_optional "qemu-system-x86_64" qemu-system-x86_64
check_optional "qemu-system-aarch64" qemu-system-aarch64
if [ "$(uname)" = "Darwin" ]; then
  check_cmd "readelf" readelf || status=1
else
  check_optional "readelf" readelf
fi

if [ "$status" -ne 0 ]; then
  echo "\nMissing required tools. See docs/boot.md for install hints." >&2
  exit 1
fi

echo "\nAll required tools detected."
