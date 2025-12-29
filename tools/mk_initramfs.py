#!/usr/bin/env python3
"""
Builds a Ruzzle initramfs image.

Usage:
  mk_initramfs.py output.img input_dir

All files under input_dir are packed with their relative paths.
"""

import argparse
import pathlib
import struct


MAGIC = b"RUZZLEFS"
VERSION = 1


def align_up(value: int, align: int) -> int:
    return value if value % align == 0 else value + (align - (value % align))


def build_image(entries: list[tuple[str, bytes]]) -> bytes:
    image = bytearray()
    image.extend(MAGIC)
    image.extend(struct.pack("<H", VERSION))
    image.extend(struct.pack("<H", len(entries)))

    for name, data in entries:
        name_bytes = name.encode("utf-8")
        image.extend(struct.pack("<H", len(name_bytes)))
        image.extend(struct.pack("<Q", len(data)))
        image.extend(name_bytes)
        image.extend(data)
        padded_len = align_up(len(image), 8)
        image.extend(b"\x00" * (padded_len - len(image)))

    return bytes(image)


def collect_entries(input_dir: pathlib.Path) -> list[tuple[str, bytes]]:
    entries: list[tuple[str, bytes]] = []
    for path in sorted(input_dir.rglob("*")):
        if path.is_file():
            rel = path.relative_to(input_dir).as_posix()
            entries.append((rel, path.read_bytes()))
    return entries


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=pathlib.Path)
    parser.add_argument("input_dir", type=pathlib.Path)
    args = parser.parse_args()

    if not args.input_dir.is_dir():
        raise SystemExit("input_dir must be a directory")

    entries = collect_entries(args.input_dir)
    image = build_image(entries)
    args.output.write_bytes(image)
    print(f"Wrote initramfs with {len(entries)} files to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
