#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build"
BUNDLE_DIR="${BUILD_DIR}/aarch64"
KERNEL_BIN="${BUNDLE_DIR}/kernel-aarch64"
INITRAMFS_IMG="${BUNDLE_DIR}/initramfs.img"

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "Missing required tool: ${tool}" >&2
    exit 1
  fi
}

require_tool cargo
require_tool python3
if command -v rustup >/dev/null 2>&1; then
  if ! rustup target list --installed | grep -q "^aarch64-unknown-none$"; then
    echo "Missing target: aarch64-unknown-none" >&2
    echo "Install with: rustup target add aarch64-unknown-none" >&2
    exit 1
  fi
else
  echo "rustup not found; ensure the aarch64-unknown-none target is installed." >&2
  exit 1
fi

mkdir -p "${BUNDLE_DIR}"
INITRAMFS_DIR="${BUNDLE_DIR}/initramfs"

cargo build -p kernel --features aarch64,qemu_virt,kernel_bin --target aarch64-unknown-none
cp "${ROOT_DIR}/target/aarch64-unknown-none/debug/kernel" "${KERNEL_BIN}"

rm -rf "${INITRAMFS_DIR}"
mkdir -p "${INITRAMFS_DIR}"

cargo build -p user_init --target aarch64-unknown-none --release
cargo build -p user_console_service --target aarch64-unknown-none --release
cargo build -p user_tui_shell --target aarch64-unknown-none --release
cargo build -p user_fs_service --target aarch64-unknown-none --release
cargo build -p user_net_service --target aarch64-unknown-none --release
cargo build -p user_user_service --target aarch64-unknown-none --release
cargo build -p user_text_editor --target aarch64-unknown-none --release
cargo build -p user_file_manager --target aarch64-unknown-none --release
cargo build -p user_settings_service --target aarch64-unknown-none --release
cargo build -p user_session_service --target aarch64-unknown-none --release
cargo build -p user_setup_wizard --target aarch64-unknown-none --release
cargo build -p user_sysinfo_service --target aarch64-unknown-none --release

cp "${ROOT_DIR}/target/aarch64-unknown-none/release/init" "${INITRAMFS_DIR}/init"
cp "${ROOT_DIR}/target/aarch64-unknown-none/release/console-service" "${INITRAMFS_DIR}/console-service"
cp "${ROOT_DIR}/target/aarch64-unknown-none/release/tui-shell" "${INITRAMFS_DIR}/tui-shell"

cp "${ROOT_DIR}/crates/user_init/module.toml" "${INITRAMFS_DIR}/init.module.toml"
cp "${ROOT_DIR}/crates/user_console_service/module.toml" "${INITRAMFS_DIR}/console-service.module.toml"
cp "${ROOT_DIR}/crates/user_tui_shell/module.toml" "${INITRAMFS_DIR}/tui-shell.module.toml"

STORE_DIR="${INITRAMFS_DIR}/store"
mkdir -p "${STORE_DIR}"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/fs-service.rpiece" \
  "${ROOT_DIR}/crates/user_fs_service/module.toml" \
  "${ROOT_DIR}/target/aarch64-unknown-none/release/fs-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/net-service.rpiece" \
  "${ROOT_DIR}/crates/user_net_service/module.toml" \
  "${ROOT_DIR}/target/aarch64-unknown-none/release/net-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/user-service.rpiece" \
  "${ROOT_DIR}/crates/user_user_service/module.toml" \
  "${ROOT_DIR}/target/aarch64-unknown-none/release/user-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/text-editor.rpiece" \
  "${ROOT_DIR}/crates/user_text_editor/module.toml" \
  "${ROOT_DIR}/target/aarch64-unknown-none/release/text-editor"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/file-manager.rpiece" \
  "${ROOT_DIR}/crates/user_file_manager/module.toml" \
  "${ROOT_DIR}/target/aarch64-unknown-none/release/file-manager"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/settings-service.rpiece" \
  "${ROOT_DIR}/crates/user_settings_service/module.toml" \
  "${ROOT_DIR}/target/aarch64-unknown-none/release/settings-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/session-service.rpiece" \
  "${ROOT_DIR}/crates/user_session_service/module.toml" \
  "${ROOT_DIR}/target/aarch64-unknown-none/release/session-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/setup-wizard.rpiece" \
  "${ROOT_DIR}/crates/user_setup_wizard/module.toml" \
  "${ROOT_DIR}/target/aarch64-unknown-none/release/setup-wizard"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/sysinfo-service.rpiece" \
  "${ROOT_DIR}/crates/user_sysinfo_service/module.toml" \
  "${ROOT_DIR}/target/aarch64-unknown-none/release/sysinfo-service"

EXTERNAL_DIR="${ROOT_DIR}/modules"
if compgen -G "${EXTERNAL_DIR}/*.rpiece" > /dev/null; then
  cp "${EXTERNAL_DIR}"/*.rpiece "${STORE_DIR}/"
fi

"${ROOT_DIR}/tools/mk_initramfs.py" "${INITRAMFS_IMG}" "${INITRAMFS_DIR}"

echo "AArch64 bundle ready (ISO not supported yet):"
echo "  ${KERNEL_BIN}"
echo "  ${INITRAMFS_IMG}"
