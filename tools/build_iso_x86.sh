#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build"
ISO_DIR="${BUILD_DIR}/iso"
LIMINE_DIR="${BUILD_DIR}/limine"
KERNEL_BIN="${BUILD_DIR}/kernel-x86_64"
INITRAMFS_DIR="${BUILD_DIR}/initramfs"
INITRAMFS_IMG="${BUILD_DIR}/initramfs.img"
ISO_PATH="${BUILD_DIR}/ruzzle-x86_64.iso"

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "Missing required tool: ${tool}" >&2
    exit 1
  fi
}

copy_manifest() {
  local source="$1"
  local dest="$2"
  if [ -f "${source}" ]; then
    cp "${source}" "${dest}"
  fi
}

require_tool cargo
require_tool python3
require_tool xorriso

"${ROOT_DIR}/tools/build_limine.sh"

mkdir -p "${BUILD_DIR}"

cargo build -p kernel --features x86_64,qemu_x86_64,kernel_bin --target x86_64-unknown-none
cp "${ROOT_DIR}/target/x86_64-unknown-none/debug/kernel" "${KERNEL_BIN}"

rm -rf "${INITRAMFS_DIR}"
mkdir -p "${INITRAMFS_DIR}"

cargo build -p user_init --target x86_64-unknown-none --release
cargo build -p user_console_service --target x86_64-unknown-none --release
cargo build -p user_tui_shell --target x86_64-unknown-none --release
cargo build -p user_fs_service --target x86_64-unknown-none --release
cargo build -p user_net_service --target x86_64-unknown-none --release
cargo build -p user_user_service --target x86_64-unknown-none --release
cargo build -p user_text_editor --target x86_64-unknown-none --release
cargo build -p user_file_manager --target x86_64-unknown-none --release
cargo build -p user_settings_service --target x86_64-unknown-none --release
cargo build -p user_session_service --target x86_64-unknown-none --release
cargo build -p user_setup_wizard --target x86_64-unknown-none --release
cargo build -p user_sysinfo_service --target x86_64-unknown-none --release
cargo build -p user_rust_toolchain --target x86_64-unknown-none --release
cargo build -p user_container_service --target x86_64-unknown-none --release
cargo build -p user_server_stack --target x86_64-unknown-none --release
cargo build -p user_net_manager --target x86_64-unknown-none --release
cargo build -p user_device_manager --target x86_64-unknown-none --release
cargo build -p user_input_service --target x86_64-unknown-none --release
cargo build -p user_gpu_service --target x86_64-unknown-none --release
cargo build -p user_ml_runtime --target x86_64-unknown-none --release

cp "${ROOT_DIR}/target/x86_64-unknown-none/release/init" "${INITRAMFS_DIR}/init"
cp "${ROOT_DIR}/target/x86_64-unknown-none/release/console-service" "${INITRAMFS_DIR}/console-service"
cp "${ROOT_DIR}/target/x86_64-unknown-none/release/tui-shell" "${INITRAMFS_DIR}/tui-shell"

copy_manifest "${ROOT_DIR}/crates/user_init/module.toml" "${INITRAMFS_DIR}/init.module.toml"
copy_manifest "${ROOT_DIR}/crates/user_console_service/module.toml" "${INITRAMFS_DIR}/console-service.module.toml"
copy_manifest "${ROOT_DIR}/crates/user_tui_shell/module.toml" "${INITRAMFS_DIR}/tui-shell.module.toml"

STORE_DIR="${INITRAMFS_DIR}/store"
mkdir -p "${STORE_DIR}"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/fs-service.rpiece" \
  "${ROOT_DIR}/crates/user_fs_service/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/fs-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/net-service.rpiece" \
  "${ROOT_DIR}/crates/user_net_service/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/net-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/user-service.rpiece" \
  "${ROOT_DIR}/crates/user_user_service/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/user-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/text-editor.rpiece" \
  "${ROOT_DIR}/crates/user_text_editor/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/text-editor"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/file-manager.rpiece" \
  "${ROOT_DIR}/crates/user_file_manager/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/file-manager"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/settings-service.rpiece" \
  "${ROOT_DIR}/crates/user_settings_service/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/settings-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/session-service.rpiece" \
  "${ROOT_DIR}/crates/user_session_service/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/session-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/setup-wizard.rpiece" \
  "${ROOT_DIR}/crates/user_setup_wizard/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/setup-wizard"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/sysinfo-service.rpiece" \
  "${ROOT_DIR}/crates/user_sysinfo_service/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/sysinfo-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/rust-toolchain.rpiece" \
  "${ROOT_DIR}/crates/user_rust_toolchain/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/rust-toolchain"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/docker-service.rpiece" \
  "${ROOT_DIR}/crates/user_container_service/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/docker-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/server-stack.rpiece" \
  "${ROOT_DIR}/crates/user_server_stack/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/server-stack"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/net-manager.rpiece" \
  "${ROOT_DIR}/crates/user_net_manager/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/net-manager"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/device-manager.rpiece" \
  "${ROOT_DIR}/crates/user_device_manager/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/device-manager"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/input-service.rpiece" \
  "${ROOT_DIR}/crates/user_input_service/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/input-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/gpu-service.rpiece" \
  "${ROOT_DIR}/crates/user_gpu_service/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/gpu-service"

python3 "${ROOT_DIR}/tools/pack_module.py" \
  "${STORE_DIR}/ml-runtime.rpiece" \
  "${ROOT_DIR}/crates/user_ml_runtime/module.toml" \
  "${ROOT_DIR}/target/x86_64-unknown-none/release/ml-runtime"

EXTERNAL_DIR="${ROOT_DIR}/modules"
if compgen -G "${EXTERNAL_DIR}/*.rpiece" > /dev/null; then
  cp "${EXTERNAL_DIR}"/*.rpiece "${STORE_DIR}/"
fi

python3 "${ROOT_DIR}/tools/market_scan.py" \
  --input "${STORE_DIR}" \
  --output "${STORE_DIR}/index.toml"

"${ROOT_DIR}/tools/mk_initramfs.py" "${INITRAMFS_IMG}" "${INITRAMFS_DIR}"

rm -rf "${ISO_DIR}"
mkdir -p "${ISO_DIR}/boot/limine"

cp "${KERNEL_BIN}" "${ISO_DIR}/boot/kernel"
cp "${INITRAMFS_IMG}" "${ISO_DIR}/boot/initramfs.img"
cp "${ROOT_DIR}/tools/limine.conf" "${ISO_DIR}/boot/limine/limine.conf"
cp "${LIMINE_DIR}/limine-bios.sys" "${ISO_DIR}/boot/limine/"
cp "${LIMINE_DIR}/limine-bios-cd.bin" "${ISO_DIR}/boot/limine/"
cp "${LIMINE_DIR}/limine-uefi-cd.bin" "${ISO_DIR}/boot/limine/"

mkdir -p "${ISO_DIR}/EFI/BOOT"
cp "${LIMINE_DIR}/BOOTX64.EFI" "${ISO_DIR}/EFI/BOOT/BOOTX64.EFI"

xorriso -as mkisofs \
  -R -r -J \
  -b boot/limine/limine-bios-cd.bin \
  -no-emul-boot \
  -boot-load-size 4 \
  -boot-info-table \
  -hfsplus \
  -apm-block-size 2048 \
  --efi-boot boot/limine/limine-uefi-cd.bin \
  -efi-boot-part \
  -efi-boot-image \
  --protective-msdos-label \
  "${ISO_DIR}" \
  -o "${ISO_PATH}"

"${LIMINE_DIR}/limine-deploy" "${ISO_PATH}"

echo "ISO ready: ${ISO_PATH}"
