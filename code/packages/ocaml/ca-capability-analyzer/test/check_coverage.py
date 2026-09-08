"""Fail closed when bisect_ppx reports less than the reviewed source floor."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


def source_percentage(summary: str, source: str) -> float:
    normalized_source = source.replace("\\", "/")
    for line in summary.splitlines():
        normalized_line = line.replace("\\", "/")
        if normalized_source not in normalized_line:
            continue
        percentages = re.findall(r"\(([0-9]+(?:\.[0-9]+)?)%\)", line)
        if percentages:
            return float(percentages[-1])
    raise ValueError(f"coverage summary has no percentage for {source}")


def require_minimum(summary: str, source: str, minimum: float) -> float:
    percentage = source_percentage(summary, source)
    if percentage < minimum:
        raise ValueError(f"coverage {percentage:.2f}% is below required {minimum:.2f}%")
    return percentage


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", required=True, type=Path)
    parser.add_argument("--source", required=True)
    parser.add_argument("--minimum", required=True, type=float)
    args = parser.parse_args()
    percentage = require_minimum(
        args.summary.read_text(encoding="utf-8"), args.source, args.minimum
    )
    print(f"{args.source}: {percentage:.2f}% (minimum {args.minimum:.2f}%)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
