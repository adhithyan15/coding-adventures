#!/usr/bin/env python3

"""Check the local html5lib tree-construction smoke fixture metadata."""

from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
import re


FIXTURE_DIR = Path(__file__).resolve().parent
DEFAULT_SMOKE = FIXTURE_DIR / "html5lib-tree-construction-smoke.dat"
SOURCE_RE = re.compile(
    r"^(?:[A-Za-z0-9_.-]+/)*[A-Za-z0-9_.-]+\.dat:[1-9][0-9]*$"
)
SCRIPTING_MARKERS = {"#script-on", "#script-off"}


@dataclass(frozen=True)
class TreeConstructionCase:
    source: str
    data: list[str]
    errors: list[str]
    scripting_markers: list[str]
    fragment_context: str | None
    document: list[str]


def main() -> int:
    args = parse_args()
    smoke_path = Path(args.smoke).expanduser().resolve()
    cases = parse_tree_construction_cases(smoke_path)
    errors = validate_cases(smoke_path, cases)

    sources = Counter(case.source.split(":", 1)[0] for case in cases)
    fragments = sum(1 for case in cases if case.fragment_context is not None)
    script_off = sum(1 for case in cases if "#script-off" in case.scripting_markers)

    print("html5lib tree-construction smoke metadata")
    print(f"fixture:          {smoke_path}")
    print(f"cases:            {len(cases)}")
    print(f"source files:     {len(sources)}")
    print(f"fragment cases:   {fragments}")
    print(f"script-off cases: {script_off}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check case boundaries and metadata in the checked-in html5lib "
            "tree-construction smoke fixture."
        )
    )
    parser.add_argument(
        "--smoke",
        default=str(DEFAULT_SMOKE),
        help="Local html5lib tree-construction smoke fixture to check.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for the generated fixture manifest.",
    )
    return parser.parse_args()


def parse_tree_construction_cases(path: Path) -> list[TreeConstructionCase]:
    lines = path.read_text(errors="replace").splitlines()
    cases: list[TreeConstructionCase] = []
    index = 0

    while index < len(lines):
        if lines[index] == "":
            index += 1
            continue
        if not lines[index].startswith("#source "):
            raise SystemExit(
                f"{path}: line {index + 1}: expected #source marker, got {lines[index]!r}"
            )
        source = lines[index].removeprefix("#source ").strip()
        index += 1

        index = expect_marker(path, lines, index, "#data")
        data: list[str] = []
        while index < len(lines) and lines[index] != "#errors":
            data.append(lines[index])
            index += 1

        index = expect_marker(path, lines, index, "#errors")
        errors: list[str] = []
        while index < len(lines):
            line = lines[index]
            if (
                line == "#document"
                or line == "#document-fragment"
                or line in SCRIPTING_MARKERS
            ):
                break
            if line.startswith("#source ") or line == "#data":
                raise SystemExit(
                    f"{path}: line {index + 1}: case {source} is missing #document"
                )
            errors.append(line)
            index += 1

        scripting_markers: list[str] = []
        fragment_context = None
        while index < len(lines):
            line = lines[index]
            if line == "#document":
                index += 1
                break
            if line in SCRIPTING_MARKERS:
                scripting_markers.append(line)
                index += 1
                continue
            if line == "#document-fragment":
                if fragment_context is not None:
                    raise SystemExit(
                        f"{path}: line {index + 1}: duplicate #document-fragment "
                        f"marker in case {source}"
                    )
                index += 1
                if index >= len(lines) or is_section_marker(lines[index]):
                    raise SystemExit(
                        f"{path}: line {index + 1}: #document-fragment in case "
                        f"{source} must name a context element"
                    )
                fragment_context = lines[index]
                index += 1
                continue
            raise SystemExit(
                f"{path}: line {index + 1}: unexpected metadata marker {line!r} "
                f"in case {source}"
            )
        else:
            raise SystemExit(f"{path}: case {source} is missing #document")

        document: list[str] = []
        while index < len(lines):
            line = lines[index]
            if line == "#data" or line.startswith("#source "):
                break
            document.append(line)
            index += 1
        while document and document[-1] == "":
            document.pop()

        cases.append(
            TreeConstructionCase(
                source=source,
                data=data,
                errors=errors,
                scripting_markers=scripting_markers,
                fragment_context=fragment_context,
                document=document,
            )
        )

    if not cases:
        raise SystemExit(f"{path}: no tree-construction cases found")
    return cases


def expect_marker(path: Path, lines: list[str], index: int, marker: str) -> int:
    if index >= len(lines):
        raise SystemExit(f"{path}: expected {marker}, reached end of file")
    if lines[index] != marker:
        raise SystemExit(
            f"{path}: line {index + 1}: expected {marker}, got {lines[index]!r}"
        )
    return index + 1


def validate_cases(path: Path, cases: list[TreeConstructionCase]) -> list[str]:
    errors: list[str] = []
    seen_sources: set[str] = set()
    duplicate_sources: list[str] = []

    for case_number, case in enumerate(cases, start=1):
        prefix = f"{path.name} case {case_number} ({case.source})"

        if not SOURCE_RE.match(case.source):
            errors.append(f"{prefix} has invalid source marker")
        if case.source in seen_sources:
            duplicate_sources.append(case.source)
        seen_sources.add(case.source)

        if not case.document:
            errors.append(f"{prefix} has empty #document")
        if case.fragment_context is not None and not is_valid_context(case.fragment_context):
            errors.append(
                f"{prefix} has invalid fragment context {case.fragment_context!r}"
            )
        if len(case.scripting_markers) > 1:
            errors.append(
                f"{prefix} has multiple scripting markers: "
                + ", ".join(case.scripting_markers)
            )
        for line in case.document:
            if line.startswith("#source ") or line in {
                "#data",
                "#errors",
                "#document",
            }:
                errors.append(f"{prefix} has section marker inside #document: {line!r}")
                break

    if duplicate_sources:
        errors.append(
            "duplicate #source markers:\n"
            + "\n".join(f"  {source}" for source in sorted(set(duplicate_sources)))
        )
    return errors


def is_section_marker(line: str) -> bool:
    return line.startswith("#")


def is_valid_context(context: str) -> bool:
    return bool(
        re.fullmatch(
            r"(?:[A-Za-z][A-Za-z0-9:-]*)(?: [A-Za-z][A-Za-z0-9:-]*)?",
            context,
        )
    )


if __name__ == "__main__":
    raise SystemExit(main())
