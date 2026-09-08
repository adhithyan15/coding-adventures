"""Fail closed when bisect_ppx reports less than the reviewed source floor."""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


def source_percentage(summary: str, source: str) -> float:
    normalized_source = source.replace("\\", "/")
    for line in summary.splitlines():
        normalized_line = line.replace("\\", "/")
        if normalized_source not in normalized_line:
            continue
        parenthesized = re.findall(r"\(([0-9]+(?:\.[0-9]+)?)%\)", line)
        if parenthesized:
            return float(parenthesized[-1])
        per_file = re.match(r"\s*([0-9]+(?:\.[0-9]+)?)\s+%\s+", line)
        if per_file:
            return float(per_file.group(1))
    raise ValueError(f"coverage summary has no percentage for {source}")


def require_minimum(summary: str, source: str, minimum: float) -> float:
    percentage = source_percentage(summary, source)
    if percentage < minimum:
        raise ValueError(f"coverage {percentage:.2f}% is below required {minimum:.2f}%")
    return percentage


def coverage_command(pattern: str, sources: list[str]) -> list[str]:
    coverage_files = sorted(Path.cwd().glob(pattern))
    if not coverage_files:
        raise ValueError(f"coverage glob matched no files: {pattern}")
    command = ["opam", "exec", "--", "bisect-ppx-report", "summary", "--per-file"]
    for source in sources:
        command.extend(("--expect", source))
    command.extend(str(path) for path in coverage_files)
    return command


def generate_summary(path: Path, pattern: str, sources: list[str]) -> None:
    command = coverage_command(pattern, sources)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as output:
        subprocess.run(command, check=True, stdout=output, text=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", required=True, type=Path)
    parser.add_argument("--coverage-glob")
    parser.add_argument("--source", required=True, action="append")
    parser.add_argument("--minimum", required=True, type=float)
    args = parser.parse_args()
    if args.coverage_glob:
        generate_summary(args.summary, args.coverage_glob, args.source)
    summary = args.summary.read_text(encoding="utf-8")
    for source in args.source:
        percentage = require_minimum(summary, source, args.minimum)
        print(f"{source}: {percentage:.2f}% (minimum {args.minimum:.2f}%)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
