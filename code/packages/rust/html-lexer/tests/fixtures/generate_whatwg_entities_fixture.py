#!/usr/bin/env python3

"""Generate Venture's checked-in WHATWG named character reference fixture.

Download the current table from the HTML Standard and point this script at it:

    curl -L https://html.spec.whatwg.org/entities.json -o /tmp/entities.json
    python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_entities_fixture.py \
      /tmp/entities.json

The output is intentionally small and data-only so Rust tests can verify the
static generated lexer without reaching out to the network.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-entities.json")
SOURCE_URL = "https://html.spec.whatwg.org/entities.json"


def main() -> int:
    args = parse_args()
    source = Path(args.entities_json).expanduser().resolve()
    output = Path(args.output).expanduser().resolve()

    entities = json.loads(source.read_text())
    fixture = build_fixture(entities)
    text = json.dumps(fixture, indent=2, ensure_ascii=False, sort_keys=True) + "\n"

    if args.check:
        existing = output.read_text()
        if existing != text:
            raise SystemExit(f"{output} is stale; regenerate it from {source}")
        return 0

    output.write_text(text)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG named character reference fixture JSON."
    )
    parser.add_argument(
        "entities_json",
        help="Path to the WHATWG entities.json source table.",
    )
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="Fixture path to write; defaults beside this script.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Exit non-zero if the checked-in fixture differs from generated output.",
    )
    return parser.parse_args()


def build_fixture(entities: dict[str, Any]) -> dict[str, Any]:
    return {
        "format": "whatwg-html-entities/v1",
        "source": SOURCE_URL,
        "entities": [
            {
                "name": name,
                "characters": data["characters"],
                "codepoints": data["codepoints"],
                "semicolon": name.endswith(";"),
            }
            for name, data in sorted(entities.items())
        ],
    }


if __name__ == "__main__":
    raise SystemExit(main())
