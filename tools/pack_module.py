#!/usr/bin/env python3
"""
Pack a module binary + manifest into a Ruzzle piece bundle.

Usage:
  pack_module.py output.rpiece manifest.toml payload.bin
"""

import argparse
import pathlib
import struct

MAGIC = b"RMOD"
VERSION = 1


def build_bundle(manifest: bytes, payload: bytes) -> bytes:
    header = bytearray()
    header.extend(MAGIC)
    header.extend(struct.pack("<H", VERSION))
    header.extend(struct.pack("<I", len(manifest)))
    header.extend(struct.pack("<I", len(payload)))
    return bytes(header) + manifest + payload


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=pathlib.Path)
    parser.add_argument("manifest", type=pathlib.Path)
    parser.add_argument("payload", type=pathlib.Path)
    args = parser.parse_args()

    manifest_bytes = args.manifest.read_bytes()
    payload_bytes = args.payload.read_bytes()

    if not manifest_bytes:
        raise SystemExit("manifest is empty")
    if not payload_bytes:
        raise SystemExit("payload is empty")

    bundle = build_bundle(manifest_bytes, payload_bytes)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(bundle)
    print(f"Wrote module bundle: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
