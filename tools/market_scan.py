#!/usr/bin/env python3
"""
Generate a local marketplace index from .rpiece bundles.

Usage:
  market_scan.py [--input DIR] [--input PATH] [--output PATH]

Defaults:
  input  -> <repo>/modules
  output -> <repo>/modules/index.toml

Environment:
  RUZZLE_MARKETPLACE_KEY  overrides the signing key.
"""

from __future__ import annotations

import argparse
import hashlib
import hmac
import os
import struct
import sys
from pathlib import Path

MAGIC = b"RMOD"
VERSION_V1 = 1
VERSION_V2 = 2
HEADER_LEN = 4 + 2 + 4 + 4
SIGNATURE_LEN = 32
DEFAULT_KEY = b"ruzzle-dev-key"

ALLOWED_KEYS = {
    "name",
    "version",
    "provides",
    "slots",
    "requires_caps",
    "depends",
}


def load_key() -> bytes:
    raw = os.environ.get("RUZZLE_MARKETPLACE_KEY")
    if raw is None:
        return DEFAULT_KEY
    return raw.encode("utf-8")


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
    items: list[str] = []
    for raw in inner.split(","):
        item = raw.strip()
        if not item:
            raise ValueError("empty list item")
        items.append(parse_string(item))
    return items


def parse_manifest(text: str) -> dict[str, object]:
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


def parse_bundle(path: Path, key: bytes) -> dict[str, object]:
    data = path.read_bytes()
    if len(data) < HEADER_LEN:
        raise ValueError("bundle too small")
    if data[:4] != MAGIC:
        raise ValueError("invalid magic")
    version = struct.unpack_from("<H", data, 4)[0]
    if version not in (VERSION_V1, VERSION_V2):
        raise ValueError("unsupported version")
    manifest_len = struct.unpack_from("<I", data, 6)[0]
    payload_len = struct.unpack_from("<I", data, 10)[0]
    manifest_start = HEADER_LEN
    manifest_end = manifest_start + manifest_len
    payload_end = manifest_end + payload_len
    if payload_end > len(data):
        raise ValueError("truncated bundle")

    manifest_bytes = data[manifest_start:manifest_end]
    payload_bytes = data[manifest_end:payload_end]

    try:
        manifest_text = manifest_bytes.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ValueError("manifest not utf-8") from exc

    verified = False
    if version == VERSION_V1:
        if payload_end != len(data):
            raise ValueError("trailing bytes in v1 bundle")
    else:
        sig_end = payload_end + SIGNATURE_LEN
        if sig_end != len(data):
            raise ValueError("invalid signature length")
        signature = data[payload_end:sig_end]
        expected = hmac.new(key, manifest_bytes + payload_bytes, hashlib.sha256).digest()
        if signature != expected:
            raise ValueError("signature mismatch")
        verified = True

    manifest = parse_manifest(manifest_text)
    name = str(manifest.get("name", ""))
    version_str = str(manifest.get("version", ""))
    provides = list(manifest.get("provides", []))
    slots = list(manifest.get("slots", []))
    caps = list(manifest.get("requires_caps", []))
    depends = list(manifest.get("depends", []))
    if not name or not version_str:
        raise ValueError("missing name/version")

    return {
        "name": name,
        "version": version_str,
        "provides": provides,
        "slots": slots,
        "caps": caps,
        "depends": depends,
        "verified": verified,
        "file": path.name,
    }


def discover_inputs(paths: list[Path]) -> list[Path]:
    bundles: list[Path] = []
    for path in paths:
        if path.is_dir():
            bundles.extend(sorted(path.glob("*.rpiece")))
        elif path.suffix == ".rpiece":
            bundles.append(path)
        else:
            raise ValueError(f"unsupported input: {path}")
    return bundles


def format_list(items: list[str]) -> str:
    if not items:
        return "[]"
    quoted = ", ".join(f'"{item}"' for item in items)
    return f"[{quoted}]"


def write_index(output: Path, entries: list[dict[str, object]]) -> None:
    lines: list[str] = []
    lines.append("# Auto-generated by tools/market_scan.py")
    for entry in entries:
        lines.append("")
        lines.append("[[piece]]")
        lines.append(f'name = "{entry["name"]}"')
        lines.append(f'version = "{entry["version"]}"')
        lines.append(f"slots = {format_list(entry['slots'])}")
        lines.append(f"caps = {format_list(entry['caps'])}")
        lines.append(f"verified = {str(entry['verified']).lower()}")
        lines.append(f"provides = {format_list(entry['provides'])}")
        lines.append(f"depends = {format_list(entry['depends'])}")
        lines.append(f'file = "{entry["file"]}"')
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


def resolve_default_paths() -> tuple[Path, Path]:
    root = Path(__file__).resolve().parents[1]
    return root / "modules", root / "modules" / "index.toml"


def main() -> int:
    default_input, default_output = resolve_default_paths()
    parser = argparse.ArgumentParser(description="Scan local .rpiece bundles into index.toml")
    parser.add_argument(
        "--input",
        dest="inputs",
        action="append",
        help="directory or .rpiece file to scan (repeatable)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=default_output,
        help="output index.toml path",
    )
    args = parser.parse_args()

    inputs = [Path(p) for p in (args.inputs or [default_input])]
    bundles = discover_inputs(inputs)
    entries: list[dict[str, object]] = []
    names: set[str] = set()
    key = load_key()

    for bundle in bundles:
        try:
            entry = parse_bundle(bundle, key)
        except ValueError as exc:
            print(f"{bundle}: {exc}", file=sys.stderr)
            return 1
        if entry["name"] in names:
            print(f"duplicate module name: {entry['name']}", file=sys.stderr)
            return 1
        names.add(entry["name"])
        entries.append(entry)

    entries.sort(key=lambda item: str(item["name"]))
    write_index(args.output, entries)
    print(f"Wrote market index: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
