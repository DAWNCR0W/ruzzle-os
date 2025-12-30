#!/usr/bin/env python3
import argparse
from pathlib import Path

ALLOWED_KEYS = {
    "slot",
    "summary",
    "provides",
    "requires_caps",
}


def format_list(values: list[str]) -> str:
    if not values:
        return "-"
    return ", ".join(values)


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


def load_contract(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    return parse_contract(text)


def generate_docs(contracts: list[dict]) -> str:
    lines: list[str] = []
    lines.append("# Slot Contracts")
    lines.append("")
    lines.append("Auto-generated from `slot_contracts/*.toml`.")
    lines.append("")
    lines.append("| Slot | Summary | Provides | Requires Caps |")
    lines.append("| --- | --- | --- | --- |")
    for contract in contracts:
        slot = contract.get("slot", "")
        summary = contract.get("summary", "")
        provides = format_list(list(contract.get("provides", [])))
        caps = format_list(list(contract.get("requires_caps", [])))
        lines.append(f"| `{slot}` | {summary} | {provides} | {caps} |")
    lines.append("")
    lines.append("## Maintenance")
    lines.append("")
    lines.append("Regenerate:")
    lines.append("")
    lines.append("```bash")
    lines.append("tools/slot_lint.py slot_contracts")
    lines.append("tools/generate_slot_docs.py")
    lines.append("```")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description="Generate slot contract docs")
    parser.add_argument(
        "--input",
        type=Path,
        default=root / "slot_contracts",
        help="slot_contracts directory",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root / "docs" / "slot_contracts.md",
        help="output markdown path",
    )
    args = parser.parse_args()

    contracts = []
    for path in sorted(args.input.glob("*.toml")):
        contracts.append(load_contract(path))
    contracts.sort(key=lambda entry: str(entry.get("slot", "")))

    content = generate_docs(contracts)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(content, encoding="utf-8")
    print(f"Wrote slot docs: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
