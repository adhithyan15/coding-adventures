#!/usr/bin/env python3

"""Audit Venture's checked-in HTML fixtures against upstream html5lib tests.

The script is intentionally dependency-free so it can run from a fresh checkout
with only Python 3 installed. Point it at an html5lib-tests checkout:

    HTML5LIB_TESTS_ROOT=/path/to/html5lib-tests python3 \
      code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py

It compares source fixture signatures rather than test descriptions so local
fixture names can stay stable while still proving upstream coverage.
"""

from __future__ import annotations

import argparse
import json
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


TOKENIZER_SIGNATURE_KEYS = (
    "input",
    "output",
    "initialStates",
    "lastStartTag",
    "doubleEscaped",
    "errors",
)


@dataclass(frozen=True)
class TreeCase:
    source: str
    signature: str


@dataclass(frozen=True)
class TokenizerCase:
    source: str
    signature: str


def main() -> int:
    args = parse_args()
    project_rust_root = Path(__file__).resolve().parents[3]
    upstream_root = resolve_upstream_root(args.upstream_root)

    local_tree_path = (
        project_rust_root
        / "html-parser"
        / "tests"
        / "fixtures"
        / "html5lib-tree-construction-smoke.dat"
    )
    local_tokenizer_raw_path = (
        project_rust_root
        / "html-lexer"
        / "tests"
        / "fixtures"
        / "upstream-html5lib-smoke.test"
    )
    local_tokenizer_normalized_path = (
        project_rust_root
        / "html-lexer"
        / "tests"
        / "fixtures"
        / "html5lib-smoke.json"
    )

    upstream_tree = load_tree_cases(upstream_root / "tree-construction")
    local_tree = parse_tree_construction_dat(local_tree_path)
    missing_tree = missing_cases(upstream_tree, local_tree)

    upstream_tokenizer = load_tokenizer_cases(upstream_root / "tokenizer")
    local_tokenizer = load_tokenizer_cases(local_tokenizer_raw_path)
    missing_tokenizer = missing_cases(upstream_tokenizer, local_tokenizer)

    normalized = json.loads(local_tokenizer_normalized_path.read_text())
    normalized_cases = normalized.get("cases", [])
    normalized_skipped = normalized.get("skipped", [])

    report = {
        "tree_construction": {
            "upstream_cases": len(upstream_tree),
            "local_cases": len(local_tree),
            "missing": len(missing_tree),
            "missing_sources": [case.source for case in missing_tree[: args.max_missing]],
        },
        "tokenizer": {
            "upstream_cases": len(upstream_tokenizer),
            "local_raw_cases": len(local_tokenizer),
            "missing": len(missing_tokenizer),
            "missing_sources": [
                case.source for case in missing_tokenizer[: args.max_missing]
            ],
            "normalized_cases": len(normalized_cases),
            "normalized_skipped": len(normalized_skipped),
        },
    }

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_report(report, upstream_root)

    if missing_tree or missing_tokenizer or normalized_skipped:
        return 1
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Audit checked-in Venture HTML fixtures against html5lib tests."
    )
    parser.add_argument(
        "upstream_root",
        nargs="?",
        default=os.environ.get("HTML5LIB_TESTS_ROOT"),
        help=(
            "Path to html5lib-tests, or a checkout containing that directory. "
            "Defaults to HTML5LIB_TESTS_ROOT."
        ),
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Print a machine-readable JSON report.",
    )
    parser.add_argument(
        "--max-missing",
        type=int,
        default=20,
        help="Maximum missing case sources to include in the report.",
    )
    return parser.parse_args()


def resolve_upstream_root(raw_root: str | None) -> Path:
    if not raw_root:
        raise SystemExit(
            "Provide html5lib-tests via an argument or HTML5LIB_TESTS_ROOT."
        )

    root = Path(raw_root).expanduser().resolve()
    candidates = [root, root / "html5lib-tests"]
    for candidate in candidates:
        if (candidate / "tree-construction").is_dir() and (
            candidate / "tokenizer"
        ).is_dir():
            return candidate

    raise SystemExit(f"{root} does not look like an html5lib-tests checkout")


def load_tree_cases(path: Path) -> list[TreeCase]:
    if path.is_file():
        return parse_tree_construction_dat(path)

    cases: list[TreeCase] = []
    for fixture_path in sorted(path.glob("*.dat")):
        cases.extend(parse_tree_construction_dat(fixture_path))
    return cases


def parse_tree_construction_dat(path: Path) -> list[TreeCase]:
    cases: list[TreeCase] = []
    lines = path.read_text().splitlines()
    index = 0
    source_hint: str | None = None
    case_number = 0

    while index < len(lines):
        line = lines[index]
        if not line:
            index += 1
            continue
        if line.startswith("#source "):
            source_hint = line.removeprefix("#source ").strip()
            index += 1
            continue
        if line != "#data":
            index += 1
            continue

        case_number += 1
        index += 1
        data_lines: list[str] = []
        while index < len(lines) and lines[index] != "#errors":
            data_lines.append(lines[index])
            index += 1

        if index >= len(lines):
            raise ValueError(f"{path}: case {case_number} is missing #errors")
        index += 1

        fragment_context: str | None = None
        script_mode: str | None = None
        while index < len(lines):
            marker = lines[index]
            if marker == "#document":
                index += 1
                break
            if marker == "#document-fragment":
                index += 1
                if index >= len(lines):
                    raise ValueError(
                        f"{path}: case {case_number} is missing fragment context"
                    )
                fragment_context = lines[index]
                index += 1
                continue
            if marker == "#script-off":
                script_mode = "off"
                index += 1
                continue
            if marker == "#script-on":
                script_mode = "on"
                index += 1
                continue
            index += 1
        else:
            raise ValueError(f"{path}: case {case_number} is missing #document")

        document_lines: list[str] = []
        while index < len(lines):
            if lines[index] == "#data" or lines[index].startswith("#source "):
                break
            document_lines.append(lines[index])
            index += 1
        while document_lines and document_lines[-1] == "":
            document_lines.pop()

        source = source_hint or f"{path.name}:{case_number}"
        cases.append(
            TreeCase(
                source=source,
                signature=stable_signature(
                    {
                        "data": "\n".join(data_lines),
                        "document": document_lines,
                        "fragment_context": fragment_context,
                        "script_mode": script_mode,
                    }
                ),
            )
        )
        source_hint = None

    return cases


def load_tokenizer_cases(path: Path) -> list[TokenizerCase]:
    if path.is_file():
        return parse_tokenizer_file(path)

    cases: list[TokenizerCase] = []
    for fixture_path in sorted(path.glob("*.test")):
        cases.extend(parse_tokenizer_file(fixture_path))
    return cases


def parse_tokenizer_file(path: Path) -> list[TokenizerCase]:
    raw = json.loads(path.read_text())
    cases: list[TokenizerCase] = []
    for index, test in enumerate(raw.get("tests", []), start=1):
        signature = {
            key: test[key] for key in TOKENIZER_SIGNATURE_KEYS if key in test
        }
        description = test.get("description", f"case {index}")
        cases.append(
            TokenizerCase(
                source=f"{path.name}:{index} {description}",
                signature=stable_signature(signature),
            )
        )
    return cases


def missing_cases(
    upstream: Iterable[TreeCase] | Iterable[TokenizerCase],
    local: Iterable[TreeCase] | Iterable[TokenizerCase],
) -> list[TreeCase] | list[TokenizerCase]:
    local_signatures = {case.signature for case in local}
    return [case for case in upstream if case.signature not in local_signatures]


def stable_signature(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def print_report(report: dict[str, Any], upstream_root: Path) -> None:
    tree = report["tree_construction"]
    tokenizer = report["tokenizer"]

    print("html5lib coverage audit")
    print(f"upstream: {upstream_root}")
    print("")
    print("tree-construction:")
    print(f"  upstream cases: {tree['upstream_cases']}")
    print(f"  local cases:    {tree['local_cases']}")
    print(f"  missing:        {tree['missing']}")
    print_missing(tree["missing_sources"])
    print("")
    print("tokenizer:")
    print(f"  upstream cases:     {tokenizer['upstream_cases']}")
    print(f"  local raw cases:    {tokenizer['local_raw_cases']}")
    print(f"  missing:            {tokenizer['missing']}")
    print(f"  normalized cases:   {tokenizer['normalized_cases']}")
    print(f"  normalized skipped: {tokenizer['normalized_skipped']}")
    print_missing(tokenizer["missing_sources"])


def print_missing(sources: list[str]) -> None:
    if not sources:
        return
    print("  first missing sources:")
    for source in sources:
        print(f"    - {source}")


if __name__ == "__main__":
    raise SystemExit(main())
