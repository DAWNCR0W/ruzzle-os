#!/usr/bin/env python3
import argparse
import re
import sys
from pathlib import Path

ALLOWED_KEYS = {
    "slot",
    "summary",
    "provides",
    "requires_caps",
}

CAPABILITIES = {
    "ConsoleWrite",
    "EndpointCreate",
    "ShmCreate",
    "ProcessSpawn",
    "Timer",
    "FsRoot",
    "WindowServer",
    "InputDevice",
    "GpuDevice",
}

SEGMENT_RE = r"[a-z0-9-]+"
SERVICE_RE = re.compile(rf"^ruzzle\.(?:{SEGMENT_RE}\.)*{SEGMENT_RE}$")
SLOT_RE = re.compile(rf"^ruzzle\.slot\.(?:{SEGMENT_RE}\.)*{SEGMENT_RE}@\d+$")


def parse_string(value: str) -> str:
    value = value.strip()
    if len(value) < 2 or not (value.startswith('"') and value.endswith('"')):
        raise ValueError("invalid string literal")
    return value[1:-1]


def parse_list(value: str) -> list[str]:
    value = value.strip()
    if value == "[]":
        return []
    if not (value.startswith("[") and value.endswith("]")):
        raise ValueError("invalid list syntax")
    inner = value[1:-1].strip()
    if not inner:
        return []
    items = []
    for raw in inner.split(","):
        item = raw.strip()
        if not item:
            raise ValueError("empty list item")
        items.append(parse_string(item))
    return items


def parse_contract(text: str) -> dict:
    data: dict[str, object] = {}
    for idx, line in enumerate(text.splitlines(), start=1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if "=" not in stripped:
            raise ValueError(f"line {idx}: missing '='")
        key, raw = stripped.split("=", 1)
        key = key.strip()
        raw = raw.strip()
        if key not in ALLOWED_KEYS:
            raise ValueError(f"line {idx}: unknown key '{key}'")
        if key in data:
            raise ValueError(f"line {idx}: duplicate key '{key}'")
        if key in {"slot", "summary"}:
            data[key] = parse_string(raw)
        else:
            data[key] = parse_list(raw)
    return data


def lint_contract(path: Path) -> list[str]:
    errors: list[str] = []
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        return [f"{path}: cannot read file: {exc}"]

    try:
        data = parse_contract(text)
    except ValueError as exc:
        return [f"{path}: {exc}"]

    slot = str(data.get("slot", ""))
    if not slot:
        errors.append(f"{path}: missing required 'slot'")
    elif not SLOT_RE.match(slot):
        errors.append(f"{path}: invalid slot '{slot}'")

    summary = str(data.get("summary", ""))
    if not summary:
        errors.append(f"{path}: missing required 'summary'")

    provides = data.get("provides", [])
    if isinstance(provides, list):
        for service in provides:
            if not SERVICE_RE.match(service):
                errors.append(f"{path}: invalid service '{service}'")
    else:
        errors.append(f"{path}: provides must be list")

    requires = data.get("requires_caps", [])
    if isinstance(requires, list):
        for cap in requires:
            if cap not in CAPABILITIES:
                errors.append(f"{path}: unknown capability '{cap}'")
    else:
        errors.append(f"{path}: requires_caps must be list")

    return errors


def collect_targets(paths: list[Path]) -> list[Path]:
    targets: list[Path] = []
    for path in paths:
        if path.is_dir():
            targets.extend(sorted(path.glob("*.toml")))
        else:
            targets.append(path)
    return targets


def main() -> int:
    parser = argparse.ArgumentParser(description="Lint slot_contracts/*.toml files")
    parser.add_argument("paths", nargs="+", help="slot contract file or directory")
    args = parser.parse_args()

    targets = collect_targets([Path(p) for p in args.paths])
    if not targets:
        print("No slot contract files found.", file=sys.stderr)
        return 1

    all_errors: list[str] = []
    for target in targets:
        all_errors.extend(lint_contract(target))

    if all_errors:
        for error in all_errors:
            print(error, file=sys.stderr)
        return 1

    for target in targets:
        print(f"ok: {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
