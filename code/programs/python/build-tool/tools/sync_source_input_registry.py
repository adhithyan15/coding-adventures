"""Copy the checked neutral source-input registry into the Python wheel.

The wheel cannot rely on a monorepo checkout at runtime. This deliberately
keeps the exact reviewed JSON bytes as package data, and ``--check`` is a
staleness gate for CI rather than a second authoring surface.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
SOURCE = ROOT / "code/specs/fixtures/build-tool-v1/language-source-input-registry.json"
DESTINATION = (
    ROOT
    / "code/programs/python/build-tool/src/build_tool"
    / "language_source_input_registry.json"
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="fail if projection differs"
    )
    args = parser.parse_args()
    source = SOURCE.read_bytes()
    checked = json.loads(source)
    if checked["schema_version"] != 1:
        raise ValueError("unsupported source-input registry version")
    if args.check:
        if not DESTINATION.is_file() or DESTINATION.read_bytes() != source:
            print("Python source-input registry projection is stale", file=sys.stderr)
            return 1
        return 0
    DESTINATION.write_bytes(source)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
