#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build"
ISO_PATH="${BUILD_DIR}/ruzzle-x86_64.iso"
NO_REBUILD=0
ENABLE_GDB=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--no-rebuild] [--gdb]

Options:
  --no-rebuild   Skip ISO rebuild (requires existing ${ISO_PATH})
  --gdb          Wait for GDB on tcp::1234 (-s -S)
EOF
}

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "Missing required tool: ${tool}" >&2
    exit 1
  fi
}

run_qemu() {
  local qemu_bin="${QEMU_BIN:-qemu-system-x86_64}"
  local timeout_bin=""
  if [ -n "${QEMU_TIMEOUT:-}" ]; then
    timeout_bin="$(command -v gtimeout || command -v timeout || true)"
    if [ -z "${timeout_bin}" ]; then
      echo "QEMU_TIMEOUT set but no timeout command found (install coreutils)." >&2
      exit 1
    fi
  fi

  local cmd=("${qemu_bin}" -m 512M -cdrom "${ISO_PATH}" -serial stdio -no-reboot -no-shutdown)
  cmd+=(-device virtio-keyboard-pci,disable-modern=on)
  if [ "${ENABLE_GDB}" -eq 1 ]; then
    cmd+=(-s -S)
  fi
  if [ -n "${timeout_bin}" ]; then
    if "${timeout_bin}" "${QEMU_TIMEOUT}" "${cmd[@]}"; then
      return 0
    fi
    local status=$?
    if [ "${status}" -eq 124 ]; then
      echo "QEMU timed out after ${QEMU_TIMEOUT} (expected timeout)." >&2
      return 0
    fi
    return "${status}"
  else
    "${cmd[@]}"
  fi
}

while [ $# -gt 0 ]; do
  case "$1" in
    --no-rebuild|--no-build)
      NO_REBUILD=1
      ;;
    --gdb)
      ENABLE_GDB=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
  shift
done

require_tool qemu-system-x86_64
if [ "${NO_REBUILD}" -eq 0 ]; then
  "${ROOT_DIR}/tools/build_iso_x86.sh"
else
  if [ ! -f "${ISO_PATH}" ]; then
    echo "Missing ISO: ${ISO_PATH}. Run without --no-rebuild first." >&2
    exit 1
  fi
fi
run_qemu
