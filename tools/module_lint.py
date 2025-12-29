#!/usr/bin/env python3
import argparse
import re
import sys
from pathlib import Path

ALLOWED_KEYS = {
    "name",
    "version",
    "provides",
    "slots",
    "requires_caps",
    "depends",
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
SLOT_RE = re.compile(rf"^ruzzle\.slot\.(?:{SEGMENT_RE}\.)*{SEGMENT_RE}$")
MODULE_RE = re.compile(rf"^{SEGMENT_RE}(?:-{SEGMENT_RE})*$")
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:[-+][A-Za-z0-9._-]+)?$")


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


def parse_manifest(text: str) -> dict:
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
        if key in {"name", "version"}:
            data[key] = parse_string(raw)
        else:
            data[key] = parse_list(raw)
    return data


def lint_manifest(path: Path) -> list[str]:
    errors: list[str] = []
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        return [f"{path}: cannot read file: {exc}"]

    try:
        data = parse_manifest(text)
    except ValueError as exc:
        return [f"{path}: {exc}"]

    for key in ("name", "version"):
        if key not in data:
            errors.append(f"{path}: missing required '{key}'")

    name = str(data.get("name", ""))
    if name and not MODULE_RE.match(name):
        errors.append(f"{path}: invalid module name '{name}'")

    version = str(data.get("version", ""))
    if version and not VERSION_RE.match(version):
        errors.append(f"{path}: invalid version '{version}'")

    provides = data.get("provides", [])
    slots = data.get("slots", [])
    requires = data.get("requires_caps", [])
    depends = data.get("depends", [])

    if not isinstance(provides, list) or not isinstance(slots, list):
        errors.append(f"{path}: provides/slots must be list")
    else:
        for service in provides:
            if not SERVICE_RE.match(service):
                errors.append(f"{path}: invalid service '{service}'")
        for slot in slots:
            if not SLOT_RE.match(slot):
                errors.append(f"{path}: invalid slot '{slot}'")

    if isinstance(requires, list):
        for cap in requires:
            if cap not in CAPABILITIES:
                errors.append(f"{path}: unknown capability '{cap}'")
    else:
        errors.append(f"{path}: requires_caps must be list")

    if isinstance(depends, list):
        for dep in depends:
            if not MODULE_RE.match(dep):
                errors.append(f"{path}: invalid dependency '{dep}'")
    else:
        errors.append(f"{path}: depends must be list")

    return errors


def collect_targets(paths: list[Path]) -> list[Path]:
    targets: list[Path] = []
    for path in paths:
        if path.is_dir():
            targets.extend(sorted(path.glob("**/module.toml")))
        else:
            targets.append(path)
    return targets


def main() -> int:
    parser = argparse.ArgumentParser(description="Lint Ruzzle module.toml manifests")
    parser.add_argument("paths", nargs="+", help="module.toml file or directory")
    args = parser.parse_args()

    targets = collect_targets([Path(p) for p in args.paths])
    if not targets:
        print("No module.toml files found.", file=sys.stderr)
        return 1

    all_errors: list[str] = []
    for target in targets:
        all_errors.extend(lint_manifest(target))

    if all_errors:
        for error in all_errors:
            print(error, file=sys.stderr)
        return 1

    for target in targets:
        print(f"ok: {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
