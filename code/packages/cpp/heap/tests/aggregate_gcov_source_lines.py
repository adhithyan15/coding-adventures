#!/usr/bin/env python3
"""Count executable header source lines covered by any template instance."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: aggregate_gcov_source_lines.py HEADER_GCOV", file=sys.stderr)
        return 2

    report = Path(sys.argv[1])
    source: str | None = None
    executable: set[int] = set()
    covered: set[int] = set()
    row = re.compile(r"^\s*([#=\-]+|\d+(?:\*+|\^*)?)\s*:\s*(\d+)\s*:")

    for line in report.read_text(encoding="utf-8").splitlines():
        if line.lstrip().startswith("-:") and "Source:" in line:
            source = line.split("Source:", 1)[1].strip()
            continue
        match = row.match(line)
        if match is None:
            continue
        count_text, source_line = match.groups()
        number = int(source_line)
        if number == 0 or count_text == "-":
            continue
        executable.add(number)
        numeric = re.match(r"\d+", count_text)
        if numeric is not None and int(numeric.group(0)) > 0:
            covered.add(number)

    if source is None or Path(source.replace("\\", "/")).name != "heap.hpp":
        print(f"expected a gcov report for heap.hpp, found {source!r}", file=sys.stderr)
        return 1
    if not executable:
        print("gcov report contains no executable heap.hpp source lines", file=sys.stderr)
        return 1

    result = {
        "source": source,
        "metric": (
            "unique executable source lines; covered when any template "
            "instantiation executes the physical source line"
        ),
        "executable_source_lines": len(executable),
        "covered_source_lines": len(covered),
        "percentage": round(100 * len(covered) / len(executable), 2),
        "uncovered_source_lines": sorted(executable - covered),
    }
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
