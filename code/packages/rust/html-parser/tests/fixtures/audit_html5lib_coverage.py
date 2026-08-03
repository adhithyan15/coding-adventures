#!/usr/bin/env python3

"""Audit Venture's checked-in HTML fixtures against upstream HTML tests.

The script is intentionally dependency-free so it can run from a fresh checkout
with only Python 3 installed. Tree-construction tests are maintained in WPT,
while tokenizer tests remain in html5lib-tests:

    HTML5LIB_TESTS_ROOT=/path/to/html5lib-tests \
    WPT_ROOT=/path/to/wpt \
    python3 \
      code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py

It compares source fixture signatures rather than test descriptions so local
fixture names can stay stable while still proving upstream coverage.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
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

DEFAULT_REPORT_PATH = Path(__file__).with_name("html5lib-coverage-audit.json")


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
    html5lib_root = resolve_html5lib_root(args.upstream_root)
    upstream_tree_path, upstream_tree_source = resolve_upstream_tree_path(
        args.wpt_root,
        html5lib_root,
    )

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

    upstream_tree = load_tree_cases(upstream_tree_path)
    local_tree = parse_tree_construction_dat(local_tree_path)
    missing_tree = missing_cases(upstream_tree, local_tree)

    upstream_tokenizer = load_tokenizer_cases(html5lib_root / "tokenizer")
    local_tokenizer = load_tokenizer_cases(local_tokenizer_raw_path)
    missing_tokenizer = missing_cases(upstream_tokenizer, local_tokenizer)

    normalized = json.loads(local_tokenizer_normalized_path.read_text())
    normalized_cases = normalized.get("cases", [])
    normalized_skipped = normalized.get("skipped", [])

    report = {
        "tree_construction": {
            "upstream_source": upstream_tree_source,
            "upstream_revision": git_revision(upstream_tree_path),
            "upstream_cases": len(upstream_tree),
            "local_cases": len(local_tree),
            "missing": len(missing_tree),
            "missing_sources": [case.source for case in missing_tree],
        },
        "tokenizer": {
            "upstream_source": "html5lib-tests/tokenizer",
            "upstream_revision": git_revision(html5lib_root),
            "upstream_cases": len(upstream_tokenizer),
            "local_raw_cases": len(local_tokenizer),
            "missing": len(missing_tokenizer),
            "missing_sources": [case.source for case in missing_tokenizer],
            "normalized_cases": len(normalized_cases),
            "normalized_skipped": len(normalized_skipped),
        },
    }

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_report(
            report,
            upstream_tree_path,
            html5lib_root / "tokenizer",
            args.max_missing,
        )

    report_mismatch = None
    if args.write_report:
        write_report(args.report_path, report)
    if args.check_report:
        report_mismatch = check_report(args.report_path, report)
        if report_mismatch:
            sys.stdout.flush()
            print(report_mismatch, file=sys.stderr)

    sys.stdout.flush()
    expectation_mismatches = collect_expectation_mismatches(report, args)
    for mismatch in expectation_mismatches:
        print(f"expectation mismatch: {mismatch}", file=sys.stderr)

    if (
        (missing_tree and args.expect_tree_missing is None)
        or (missing_tokenizer and args.expect_tokenizer_missing is None)
        or normalized_skipped
        or report_mismatch
        or expectation_mismatches
    ):
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
        "--wpt-root",
        default=os.environ.get("WPT_ROOT"),
        help=(
            "Path to a WPT checkout or html/syntax/parsing/resources. "
            "Defaults to WPT_ROOT. When omitted, a legacy html5lib-tests "
            "tree-construction directory is used if present."
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
    parser.add_argument(
        "--report-path",
        type=Path,
        default=DEFAULT_REPORT_PATH,
        help="Path for --write-report or --check-report.",
    )
    parser.add_argument(
        "--write-report",
        action="store_true",
        help="Write the stable machine-readable audit report.",
    )
    parser.add_argument(
        "--check-report",
        action="store_true",
        help="Exit non-zero if the stable audit report differs from --report-path.",
    )
    parser.add_argument(
        "--expect-tree-upstream-cases",
        type=int,
        help="Require the upstream tree-construction case count to match.",
    )
    parser.add_argument(
        "--expect-tree-local-cases",
        type=int,
        help="Require the checked-in tree-construction case count to match.",
    )
    parser.add_argument(
        "--expect-tree-missing",
        type=int,
        help=(
            "Require this many upstream tree-construction cases to be missing. "
            "Supplying the flag accepts that exact checked debt baseline."
        ),
    )
    parser.add_argument(
        "--expect-tokenizer-upstream-cases",
        type=int,
        help="Require the upstream tokenizer case count to match.",
    )
    parser.add_argument(
        "--expect-tokenizer-local-raw-cases",
        type=int,
        help="Require the checked-in raw tokenizer case count to match.",
    )
    parser.add_argument(
        "--expect-tokenizer-missing",
        type=int,
        help=(
            "Require this many upstream tokenizer cases to be missing. "
            "Supplying the flag accepts that exact checked debt baseline."
        ),
    )
    parser.add_argument(
        "--expect-normalized-cases",
        type=int,
        help="Require the normalized tokenizer case count to match.",
    )
    parser.add_argument(
        "--expect-normalized-skipped",
        type=int,
        help="Require the normalized tokenizer skipped-case count to match.",
    )
    return parser.parse_args()


def resolve_html5lib_root(raw_root: str | None) -> Path:
    if not raw_root:
        raise SystemExit(
            "Provide html5lib-tests via an argument or HTML5LIB_TESTS_ROOT."
        )

    root = Path(raw_root).expanduser().resolve()
    candidates = [root, root / "html5lib-tests"]
    for candidate in candidates:
        if (candidate / "tokenizer").is_dir():
            return candidate

    raise SystemExit(f"{root} does not look like an html5lib-tests checkout")


def resolve_upstream_tree_path(
    raw_wpt_root: str | None,
    html5lib_root: Path,
) -> tuple[Path, str]:
    if raw_wpt_root:
        root = Path(raw_wpt_root).expanduser().resolve()
        candidates = [
            root,
            root / "html" / "syntax" / "parsing" / "resources",
            root / "wpt" / "html" / "syntax" / "parsing" / "resources",
        ]
        for candidate in candidates:
            if candidate.is_dir() and any(candidate.glob("*.dat")):
                return candidate, "wpt/html/syntax/parsing/resources"
        raise SystemExit(
            f"{root} does not look like a WPT checkout or parsing resources directory"
        )

    legacy_tree_path = html5lib_root / "tree-construction"
    if legacy_tree_path.is_dir():
        return legacy_tree_path, "html5lib-tests/tree-construction"

    raise SystemExit(
        "Current html5lib-tests no longer contains tree-construction tests. "
        "Provide WPT via --wpt-root or WPT_ROOT."
    )


def git_revision(path: Path) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(path), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (FileNotFoundError, subprocess.CalledProcessError) as error:
        raise SystemExit(
            f"{path} must be inside a Git checkout so the audit can pin its revision"
        ) from error
    return result.stdout.strip()


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


def stable_report_json(report: dict[str, Any]) -> str:
    return json.dumps(report, indent=2, sort_keys=True) + "\n"


def write_report(path: Path, report: dict[str, Any]) -> None:
    path.write_text(stable_report_json(report))


def check_report(path: Path, report: dict[str, Any]) -> str | None:
    actual = stable_report_json(report)
    if not path.exists():
        return f"report mismatch: {path} does not exist"

    expected = path.read_text()
    if actual == expected:
        return None
    return f"report mismatch: regenerate {path} with --write-report"


def collect_expectation_mismatches(
    report: dict[str, Any], args: argparse.Namespace
) -> list[str]:
    tree = report["tree_construction"]
    tokenizer = report["tokenizer"]
    checks = [
        (
            "tree-construction upstream cases",
            tree["upstream_cases"],
            args.expect_tree_upstream_cases,
        ),
        (
            "tree-construction local cases",
            tree["local_cases"],
            args.expect_tree_local_cases,
        ),
        (
            "tree-construction missing cases",
            tree["missing"],
            args.expect_tree_missing,
        ),
        (
            "tokenizer upstream cases",
            tokenizer["upstream_cases"],
            args.expect_tokenizer_upstream_cases,
        ),
        (
            "tokenizer local raw cases",
            tokenizer["local_raw_cases"],
            args.expect_tokenizer_local_raw_cases,
        ),
        (
            "tokenizer missing cases",
            tokenizer["missing"],
            args.expect_tokenizer_missing,
        ),
        (
            "tokenizer normalized cases",
            tokenizer["normalized_cases"],
            args.expect_normalized_cases,
        ),
        (
            "tokenizer normalized skipped",
            tokenizer["normalized_skipped"],
            args.expect_normalized_skipped,
        ),
    ]

    mismatches = []
    for label, actual, expected in checks:
        if expected is not None and actual != expected:
            mismatches.append(f"{label}: expected {expected}, got {actual}")
    return mismatches


def print_report(
    report: dict[str, Any],
    upstream_tree_path: Path,
    upstream_tokenizer_path: Path,
    max_missing: int,
) -> None:
    tree = report["tree_construction"]
    tokenizer = report["tokenizer"]

    print("HTML conformance coverage audit")
    print("")
    print("tree-construction:")
    print(f"  upstream:       {upstream_tree_path}")
    print(f"  revision:       {tree['upstream_revision']}")
    print(f"  upstream cases: {tree['upstream_cases']}")
    print(f"  local cases:    {tree['local_cases']}")
    print(f"  missing:        {tree['missing']}")
    print_missing(tree["missing_sources"], max_missing)
    print("")
    print("tokenizer:")
    print(f"  upstream:           {upstream_tokenizer_path}")
    print(f"  revision:           {tokenizer['upstream_revision']}")
    print(f"  upstream cases:     {tokenizer['upstream_cases']}")
    print(f"  local raw cases:    {tokenizer['local_raw_cases']}")
    print(f"  missing:            {tokenizer['missing']}")
    print(f"  normalized cases:   {tokenizer['normalized_cases']}")
    print(f"  normalized skipped: {tokenizer['normalized_skipped']}")
    print_missing(tokenizer["missing_sources"], max_missing)


def print_missing(sources: list[str], max_missing: int) -> None:
    if not sources:
        return
    print("  first missing sources:")
    for source in sources[:max_missing]:
        print(f"    - {source}")


if __name__ == "__main__":
    raise SystemExit(main())
