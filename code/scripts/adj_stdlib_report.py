#!/usr/bin/env python3
"""Inventory the ADJ fact, formula, and medical-recall standard libraries."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from collections.abc import Iterable
from pathlib import Path
from typing import Any

COLLECTIONS = (
    ("facts", Path("code/specs/data/adj-facts-stdlib")),
    ("formulas", Path("code/specs/data/adj-formula-stdlib")),
    ("medical-recall", Path("code/specs/data/mycin-2026/recall")),
)

CLAUSE_PATTERNS = {
    "tables": re.compile(r"(?m)^\s*table\s+[a-zA-Z_]"),
    "formulas": re.compile(r"(?m)^\s*formula\s+[a-zA-Z_]"),
    "relations": re.compile(r"(?m)^\s*relate\s+[a-zA-Z_]"),
    "rules": re.compile(r"(?m)^\s*rule\s+[a-zA-Z_]"),
    "contributions": re.compile(r"(?m)^\s*contributes\s+"),
}
SOURCE_RE = re.compile(r'(?m)^\s*source\s+"')
LOCATOR_RE = re.compile(r'(?m)^\s*locator\s+"')
TRUST_RE = re.compile(r"(?m)^\s*trust\s+[a-zA-Z_]")
PINNED_QUOTE_RE = re.compile(
    r'(?m)^\s*quote\s+".*?"\s+at\s+\d+\s+snapshot\s+"[0-9a-f]{64}"\s*$'
)
TEST_SUFFIXES = {".py", ".rs", ".js", ".ts"}
TEST_ROOTS = (
    Path("code/packages/rust/adj-lang-cli/tests"),
    Path("code/specs/data/mycin-2026"),
    Path("code/scripts/tests"),
)


def _is_test_path(path: Path) -> bool:
    """Return whether a source file is part of a repository test surface."""

    name = path.name.lower()
    return (
        path.suffix.lower() in TEST_SUFFIXES
        and (
            "tests" in {part.lower() for part in path.parts}
            or name.startswith("test_")
            or name.endswith(("_test.py", ".test.js", ".test.ts"))
        )
    )


def discover_test_texts(root: Path) -> list[str]:
    """Read test sources once for exact shipped-library path references."""

    texts: list[str] = []
    for relative_root in TEST_ROOTS:
        test_root = root / relative_root
        if not test_root.is_dir():
            continue
        texts.extend(
            path.read_text(encoding="utf-8", errors="replace").replace("\\", "/")
            for path in test_root.rglob("*")
            if path.is_file() and _is_test_path(path)
        )
    return texts


def _referenced_by_test(candidates: Iterable[str], test_texts: list[str]) -> bool:
    return any(
        candidate in text
        for candidate in candidates
        for text in test_texts
    )


def inspect_library(
    root: Path,
    collection: str,
    collection_root: Path,
    path: Path,
    test_texts: list[str],
    query_texts: list[str],
) -> dict[str, Any]:
    """Return structural evidence for one shipped ADJ file."""

    text = path.read_text(encoding="utf-8")
    repo_path = path.relative_to(root).as_posix()
    collection_path = path.relative_to(collection_root).as_posix()
    domain = (
        collection_path.split("/", 1)[0]
        if "/" in collection_path
        else "recall"
    )
    clause_counts = {
        name: len(pattern.findall(text))
        for name, pattern in CLAUSE_PATTERNS.items()
    }
    clause_count = sum(clause_counts.values())
    source_count = len(SOURCE_RE.findall(text))
    locator_count = len(LOCATOR_RE.findall(text))
    trust_count = len(TRUST_RE.findall(text))
    pinned_quote_count = len(PINNED_QUOTE_RE.findall(text))

    return {
        "collection": collection,
        "domain": domain,
        "path": repo_path,
        "content_library": clause_count > 0,
        "query_companion": any(
            candidate in query_text
            for candidate in (repo_path, collection_path, path.name)
            for query_text in query_texts
        ),
        "test_reference": _referenced_by_test(
            (repo_path, collection_path, path.name), test_texts
        ),
        "source_envelope": (
            clause_count > 0
            and min(source_count, locator_count, trust_count) >= clause_count
        ),
        "pinned_quote": clause_count > 0 and pinned_quote_count >= clause_count,
        "counts": {
            **clause_counts,
            "clauses": clause_count,
            "sources": source_count,
            "locators": locator_count,
            "trusts": trust_count,
            "pinned_quotes": pinned_quote_count,
        },
    }


def _summary(rows: list[dict[str, Any]]) -> dict[str, int]:
    content = [row for row in rows if row["content_library"]]
    return {
        "adj_files": len(rows),
        "content_libraries": len(content),
        "consumer_programs": len(rows) - len(content),
        "query_companions": sum(row["query_companion"] for row in content),
        "test_references": sum(row["test_reference"] for row in content),
        "source_envelopes": sum(row["source_envelope"] for row in content),
        "byte_pinned_libraries": sum(row["pinned_quote"] for row in content),
        "clauses": sum(row["counts"]["clauses"] for row in content),
        "source_annotations": sum(row["counts"]["sources"] for row in content),
        "pinned_quotes": sum(row["counts"]["pinned_quotes"] for row in content),
    }


def build_report(root: Path) -> dict[str, Any]:
    """Build a deterministic structural report from the checked-out repository."""

    root = root.resolve()
    test_texts = discover_test_texts(root)
    libraries: list[dict[str, Any]] = []
    for collection, relative_root in COLLECTIONS:
        collection_root = root / relative_root
        query_texts = [
            path.read_text(encoding="utf-8").replace("\\", "/")
            for path in collection_root.rglob("*.query.adj")
        ]
        for path in sorted(collection_root.rglob("*.adj")):
            if path.name.endswith(".query.adj"):
                continue
            libraries.append(
                inspect_library(
                    root,
                    collection,
                    collection_root,
                    path,
                    test_texts,
                    query_texts,
                )
            )

    by_collection = {
        collection: _summary(
            [row for row in libraries if row["collection"] == collection]
        )
        for collection, _ in COLLECTIONS
    }
    by_domain: dict[str, Counter[str]] = defaultdict(Counter)
    for row in libraries:
        if row["content_library"]:
            key = f"{row['collection']}/{row['domain']}"
            by_domain[key].update(
                libraries=1,
                clauses=row["counts"]["clauses"],
                queries=int(row["query_companion"]),
                tests=int(row["test_reference"]),
                source_envelopes=int(row["source_envelope"]),
                byte_pins=int(row["pinned_quote"]),
            )

    content = [row for row in libraries if row["content_library"]]
    return {
        "schema_version": 1,
        "scope": {
            "claim": "structural inventory only",
            "limitations": [
                "A file or clause count is not evidence of curriculum mastery.",
                "A source label is not a byte-verified citation without a pinned snapshot.",
                "Test reference means a test names the library, not exhaustive semantic coverage.",
            ],
        },
        "summary": {
            "collections": len(COLLECTIONS),
            **_summary(libraries),
        },
        "collections": by_collection,
        "domains": {key: dict(value) for key, value in sorted(by_domain.items())},
        "gaps": {
            "missing_query_companion": [
                row["path"] for row in content if not row["query_companion"]
            ],
            "missing_test_reference": [
                row["path"] for row in content if not row["test_reference"]
            ],
            "missing_source_envelope": [
                row["path"] for row in content if not row["source_envelope"]
            ],
            "missing_byte_pin": [
                row["path"] for row in content if not row["pinned_quote"]
            ],
        },
        "libraries": libraries,
    }


def _ratio(value: int, total: int) -> str:
    return "n/a" if total == 0 else f"{value / total:.1%}"


def render_markdown(report: dict[str, Any]) -> str:
    """Render a concise, reviewable structural inventory."""

    lines = [
        "# ADJ Standard Library Structural Inventory",
        "",
        "<!-- Generated by code/scripts/adj_stdlib_report.py. Do not edit by hand. -->",
        "",
        "This report measures repository structure. It does not infer that a subject,",
        "grade band, or exam objective is covered merely because a similarly named file",
        "exists. Semantic coverage is tracked in `code/specs/ADJ-STDLIB-COVERAGE.md`.",
        "",
        "## Summary",
        "",
        "| Collection | Content libraries | Clauses | Query companions | Test references | Source envelopes | Byte-pinned |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for collection, values in report["collections"].items():
        total = values["content_libraries"]
        lines.append(
            f"| {collection} | {total} | {values['clauses']} | "
            f"{values['query_companions']} ({_ratio(values['query_companions'], total)}) | "
            f"{values['test_references']} ({_ratio(values['test_references'], total)}) | "
            f"{values['source_envelopes']} ({_ratio(values['source_envelopes'], total)}) | "
            f"{values['byte_pinned_libraries']} ({_ratio(values['byte_pinned_libraries'], total)}) |"
        )

    lines.extend([
        "",
        "A complete source envelope means every grounded clause has `source`,",
        "`locator`, and `trust`. Byte-pinned additionally requires every clause to",
        "have `quote ... at ... snapshot <sha256>`, which",
        "lets `adj-verify --snapshots` check the exact bytes rather than trust a label.",
        "",
        "## Domains",
        "",
        "| Collection/domain | Libraries | Clauses | Queries | Tests | Source envelopes | Byte pins |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ])
    for domain, values in report["domains"].items():
        lines.append(
            f"| `{domain}` | {values['libraries']} | {values['clauses']} | "
            f"{values['queries']} | {values['tests']} | "
            f"{values['source_envelopes']} | {values['byte_pins']} |"
        )

    lines.extend([
        "",
        "## Structural Gaps",
        "",
    ])
    labels = (
        ("missing_query_companion", "Missing worked-query import"),
        ("missing_test_reference", "Not named by a repository test"),
        ("missing_source_envelope", "Missing source envelope"),
        ("missing_byte_pin", "Missing pinned source bytes"),
    )
    for key, label in labels:
        paths = report["gaps"][key]
        lines.append(f"### {label} ({len(paths)})")
        lines.append("")
        if not paths:
            lines.append("None.")
        elif len(paths) <= 20:
            lines.extend(f"- `{path}`" for path in paths)
        else:
            lines.append(
                "See the JSON form of this report for the complete machine-readable list."
            )
        lines.append("")

    lines.extend([
        "## What This Cannot Claim",
        "",
        "- It cannot measure curriculum coverage because shipped libraries have no",
        "  objective IDs, grade bands, prerequisites, or standards crosswalk.",
        "- It cannot prove quoted text came from a locator because the stdlib does not",
        "  currently pin source snapshots and byte offsets.",
        "- It cannot prove a domain is complete from library count or green examples.",
        "- It cannot measure retrieval quality, decomposition quality, multi-hop",
        "  composition, conflict handling, calibration, or held-out exam performance.",
        "",
    ])
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument("--format", choices=("markdown", "json"), default="markdown")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--fail-on-unreferenced-tests", action="store_true")
    parser.add_argument("--fail-on-missing-source-envelope", action="store_true")
    parser.add_argument("--require-byte-pins", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    report = build_report(root)
    output = (
        json.dumps(report, indent=2, sort_keys=True) + "\n"
        if args.format == "json"
        else render_markdown(report)
    )
    if args.output:
        output_path = args.output
        if not output_path.is_absolute():
            output_path = root / output_path
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(output, encoding="utf-8")
    else:
        print(output, end="")

    if args.fail_on_unreferenced_tests and report["gaps"]["missing_test_reference"]:
        return 1
    if (
        args.fail_on_missing_source_envelope
        and report["gaps"]["missing_source_envelope"]
    ):
        return 1
    if args.require_byte_pins and report["gaps"]["missing_byte_pin"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
