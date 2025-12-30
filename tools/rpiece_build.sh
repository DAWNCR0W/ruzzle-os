#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: rpiece_build.sh <piece-dir> [target]

Builds a Rust piece on the host and packs it into .rpiece.

Example:
  tools/rpiece_build.sh external/note_piece x86_64-unknown-none
USAGE
}

if [ $# -lt 1 ]; then
  usage >&2
  exit 1
fi

PIECE_DIR="$1"
TARGET="${2:-x86_64-unknown-none}"

MODULE_TOML="${PIECE_DIR}/module.toml"
CARGO_TOML="${PIECE_DIR}/Cargo.toml"

if [ ! -f "${MODULE_TOML}" ] || [ ! -f "${CARGO_TOML}" ]; then
  echo "Missing module.toml or Cargo.toml in ${PIECE_DIR}" >&2
  exit 1
fi

NAME="$(python3 - "${MODULE_TOML}" <<'PY'
import sys
from pathlib import Path

text = Path(sys.argv[1]).read_text(encoding="utf-8")
for line in text.splitlines():
    stripped = line.strip()
    if not stripped or stripped.startswith("#"):
        continue
    if stripped.startswith("name"):
        _, raw = stripped.split("=", 1)
        raw = raw.strip().strip('"')
        print(raw)
        sys.exit(0)
raise SystemExit("module name not found")
PY
)"

cargo build --manifest-path "${CARGO_TOML}" --release --target "${TARGET}"

BIN_PATH="${PIECE_DIR}/target/${TARGET}/release/${NAME}"
if [ ! -f "${BIN_PATH}" ]; then
  echo "Expected binary not found: ${BIN_PATH}" >&2
  exit 1
fi

"${ROOT_DIR}/tools/pack_external_module.sh" "${MODULE_TOML}" "${BIN_PATH}"

echo "Packed ${NAME} for ${TARGET}"
