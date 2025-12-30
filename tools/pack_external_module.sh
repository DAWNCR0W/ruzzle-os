#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODULE_TOML="${1:-}"
ELF_BIN="${2:-}"
OUT_PATH="${3:-}"

if [ -z "${MODULE_TOML}" ] || [ -z "${ELF_BIN}" ]; then
  echo "Usage: pack_external_module.sh <module.toml> <elf> [output.rpiece]" >&2
  exit 1
fi

"${ROOT_DIR}/tools/module_lint.py" "${MODULE_TOML}"

if [ -z "${OUT_PATH}" ]; then
  name=$(python3 - "${MODULE_TOML}" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
text = path.read_text(encoding='utf-8')
for line in text.splitlines():
    stripped = line.strip()
    if not stripped or stripped.startswith('#'):
        continue
    if stripped.startswith('name'):
        _, raw = stripped.split('=', 1)
        raw = raw.strip()
        if raw.startswith('"') and raw.endswith('"'):
            print(raw[1:-1])
            sys.exit(0)
print('unknown')
PY
  )
  OUT_PATH="${ROOT_DIR}/modules/${name}.rpiece"
fi

mkdir -p "${ROOT_DIR}/modules"
python3 "${ROOT_DIR}/tools/pack_module.py" "${OUT_PATH}" "${MODULE_TOML}" "${ELF_BIN}"

python3 "${ROOT_DIR}/tools/market_scan.py" \
  --input "${ROOT_DIR}/modules" \
  --output "${ROOT_DIR}/modules/index.toml"

echo "Packed module bundle: ${OUT_PATH}"
