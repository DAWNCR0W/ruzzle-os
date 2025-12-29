#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build"
BUNDLE_DIR="${BUILD_DIR}/aarch64"
KERNEL_BIN="${BUNDLE_DIR}/kernel-aarch64"
INITRAMFS_IMG="${BUNDLE_DIR}/initramfs.img"
NO_REBUILD=0
FORCE_RUN=0

usage() {
  cat <<'USAGE'
Usage: run_qemu_arm.sh [--no-rebuild] [--force]

Options:
  --no-rebuild   Skip rebuild (requires existing build/aarch64 bundle)
  --force        Attempt to run QEMU even though AArch64 boot is stubbed
USAGE
}

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "Missing required tool: ${tool}" >&2
    exit 1
  fi
}

run_qemu() {
  require_tool qemu-system-aarch64
  local qemu_bin="${QEMU_BIN:-qemu-system-aarch64}"
  local timeout_bin=""
  if [ -n "${QEMU_TIMEOUT:-}" ]; then
    timeout_bin="$(command -v gtimeout || command -v timeout || true)"
    if [ -z "${timeout_bin}" ]; then
      echo "QEMU_TIMEOUT set but no timeout command found (install coreutils)." >&2
      exit 1
    fi
  fi

  local cmd=("${qemu_bin}" -machine virt -cpu cortex-a57 -m 512M -nographic \
    -kernel "${KERNEL_BIN}" -initrd "${INITRAMFS_IMG}" -no-reboot -no-shutdown)

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
    --force)
      FORCE_RUN=1
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

if [ "${NO_REBUILD}" -eq 0 ]; then
  "${ROOT_DIR}/tools/build_iso_arm.sh"
else
  if [ ! -f "${KERNEL_BIN}" ] || [ ! -f "${INITRAMFS_IMG}" ]; then
    echo "Missing AArch64 bundle. Run without --no-rebuild first." >&2
    exit 1
  fi
fi

if [ "${FORCE_RUN}" -eq 0 ]; then
  echo "AArch64 boot is experimental on QEMU virt." >&2
  echo "Artifacts ready in: ${BUNDLE_DIR}" >&2
  echo "Run with --force to launch QEMU." >&2
  exit 0
fi

run_qemu
