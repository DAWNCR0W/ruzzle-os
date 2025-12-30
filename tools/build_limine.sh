#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build"
LIMINE_SRC="${BUILD_DIR}/limine-src"
LIMINE_DIR="${BUILD_DIR}/limine"

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "Missing required tool: ${tool}" >&2
    exit 1
  fi
}

fetch_latest_limine_tag() {
  python3 - <<'PY'
import json
import urllib.request

with urllib.request.urlopen("https://api.github.com/repos/limine-bootloader/limine/releases/latest") as response:
    data = json.load(response)
tag = data.get("tag_name")
if not tag:
    raise SystemExit("Unable to resolve latest limine tag")
print(tag)
PY
}

ensure_limine() {
  if [ -x "${LIMINE_DIR}/limine-deploy" ] && [ -f "${LIMINE_DIR}/limine-bios.sys" ] && [ -f "${LIMINE_DIR}/limine-bios-cd.bin" ]; then
    return 0
  fi

  require_tool curl
  require_tool tar
  require_tool make
  require_tool python3
  require_tool nasm
  require_tool mtools

  mkdir -p "${BUILD_DIR}"
  rm -rf "${LIMINE_SRC}" "${LIMINE_DIR}"

  local tag="${LIMINE_TAG:-}"
  if [ -z "${tag}" ]; then
    tag="$(fetch_latest_limine_tag)"
  fi
  local version="${tag#v}"
  local tarball="limine-${version}.tar.xz"
  local url="https://github.com/limine-bootloader/limine/releases/download/${tag}/${tarball}"
  local archive_path="${BUILD_DIR}/${tarball}"

  echo "Downloading limine ${tag}..."
  curl -L -o "${archive_path}" "${url}"
  tar -xf "${archive_path}" -C "${BUILD_DIR}"
  rm -f "${archive_path}"

  if [ ! -d "${BUILD_DIR}/limine-${version}" ]; then
    echo "Limine archive did not extract as expected." >&2
    exit 1
  fi

  mv "${BUILD_DIR}/limine-${version}" "${LIMINE_SRC}"

  local configure_env=()
  if [ "$(uname)" = "Darwin" ]; then
    local sysroot host llvm_bin
    sysroot="$(rustc --print sysroot)"
    host="$(rustc -vV | awk '/host:/ {print $2}')"
    llvm_bin="${sysroot}/lib/rustlib/${host}/bin"

    local objcopy="${llvm_bin}/llvm-objcopy"
    local objdump="${llvm_bin}/llvm-objdump"
    local readelf="/opt/homebrew/opt/binutils/bin/readelf"
    if [ ! -x "${readelf}" ]; then
      readelf="$(command -v readelf || true)"
    fi
    if [ -z "${readelf}" ]; then
      echo "Missing required tool: readelf (install binutils)" >&2
      exit 1
    fi

    local ld_wrapper="${LIMINE_SRC}/ld-limine.sh"
    cat > "${ld_wrapper}" <<'SH'
#!/usr/bin/env bash
exec "$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | awk '/host:/ {print $2}')/bin/rust-lld" -flavor ld "$@"
SH
    chmod +x "${ld_wrapper}"

    configure_env+=(
      "LD_FOR_TARGET=${ld_wrapper}"
      "OBJCOPY_FOR_TARGET=${objcopy}"
      "OBJDUMP_FOR_TARGET=${objdump}"
      "READELF_FOR_TARGET=${readelf}"
    )
  fi

  pushd "${LIMINE_SRC}" >/dev/null
  if [ "${#configure_env[@]}" -eq 0 ]; then
    ./configure --enable-bios --enable-bios-cd --enable-uefi-x86-64 --enable-uefi-cd
  else
    env "${configure_env[@]}" ./configure --enable-bios --enable-bios-cd --enable-uefi-x86-64 --enable-uefi-cd
  fi
  make
  popd >/dev/null

  mkdir -p "${LIMINE_DIR}"
  cp "${LIMINE_SRC}/bin/limine-bios.sys" "${LIMINE_DIR}/limine-bios.sys"
  cp "${LIMINE_SRC}/bin/limine-bios-cd.bin" "${LIMINE_DIR}/limine-bios-cd.bin"
  cp "${LIMINE_SRC}/bin/limine-uefi-cd.bin" "${LIMINE_DIR}/limine-uefi-cd.bin"
  cp "${LIMINE_SRC}/bin/BOOTX64.EFI" "${LIMINE_DIR}/BOOTX64.EFI"
  cat > "${LIMINE_DIR}/limine-deploy" <<SH
#!/usr/bin/env bash
exec "${LIMINE_SRC}/bin/limine" bios-install "\$@"
SH
  chmod +x "${LIMINE_DIR}/limine-deploy"
}

ensure_limine
