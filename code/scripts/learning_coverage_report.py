#!/usr/bin/env python3
"""Inventory learning-material coverage for repository package concepts."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable

import package_parity_report as parity


GENERATED_REPORT_NAMES = {"COVERAGE.md"}
ANNOTATION_RE = re.compile(
    r"<!--\s*learning-concepts:\s*(.*?)\s*-->", re.IGNORECASE | re.DOTALL
)
PRIORITY_ORDER = ("P0", "P1", "P2", "P3")
STATUS_ORDER = ("dedicated", "related", "index-only", "missing")

DOMAIN_RULES: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("cryptography-security", (
        "aes", "argon", "atbash", "blake", "caesar", "chacha", "cipher",
        "crypto", "ed25519", "hkdf", "hmac", "md5", "pbkdf", "scrypt",
        "sha", "scytale", "vigenere", "x25519",
    )),
    ("compression-encoding", (
        "base16", "base32", "base58", "base64", "brotli", "codec",
        "compression", "deflate", "huffman", "lz", "qoi", "rans",
        "range-coder", "reed-solomon", "rle", "zstd",
    )),
    ("sql-storage", (
        "database", "index", "lsm", "postgres", "query", "sqlite", "sql",
        "storage", "wal",
    )),
    ("networking-systems", (
        "dns", "http", "irc", "network", "protocol", "server", "socket",
        "ssh", "tcp", "tls", "udp", "url",
    )),
    ("hardware-architecture", (
        "alu", "arm", "assembler", "cpu", "emulator", "fpga", "gpu",
        "isa", "logic-gate", "risc", "simulator", "x86",
    )),
    ("language-tooling", (
        "ast", "bytecode", "compiler", "grammar", "interpreter", "lexer",
        "parser", "semantic", "token", "transpiler", "virtual-machine",
        "wasm",
    )),
    ("data-structures-algorithms", (
        "algorithm", "array", "b-tree", "binary-tree", "bloom", "deque",
        "fenwick", "graph", "hash", "heap", "hyperloglog", "linked-list",
        "list", "queue", "radix", "red-black", "search", "segment-tree",
        "set", "skip-list", "sort", "stack", "tree", "trie",
    )),
    ("math-ml-logic", (
        "activation", "autograd", "calculus", "gradient", "logic",
        "loss", "matrix", "neural", "probability", "regression",
        "statistics", "tensor",
    )),
    ("documents-media", (
        "audio", "barcode", "csv", "docx", "epub", "image", "jpeg",
        "markdown", "mp3", "pdf", "png", "spreadsheet", "video", "xml",
        "zip",
    )),
    ("graphics-ui", (
        "canvas", "color", "css", "font", "graphics", "html", "layout",
        "paint", "render", "svg", "ui", "window", "xaml",
    )),
    ("applications-products", (
        "browser", "calculator", "editor", "game", "git", "shell",
        "terminal", "visicalc",
    )),
)


def priority_for(language_count: int) -> str:
    """Return a learning-backlog priority from implementation breadth."""

    if language_count >= 10:
        return "P0"
    if language_count >= 5:
        return "P1"
    if language_count >= 2:
        return "P2"
    return "P3"


def domain_for(package: str) -> str:
    """Assign a stable, intentionally coarse learning domain."""

    value = package.lower()
    for domain, markers in DOMAIN_RULES:
        if any(value.startswith(marker) or f"-{marker}" in value for marker in markers):
            return domain
    return "other"


def discover_learning_documents(root: Path) -> list[dict[str, Any]]:
    """Read hand-authored Markdown documents and explicit annotations."""

    learning_root = root / "code" / "learning"
    documents: list[dict[str, Any]] = []
    for path in sorted(learning_root.rglob("*.md")):
        if path.name in GENERATED_REPORT_NAMES:
            continue
        text = path.read_text(encoding="utf-8")
        annotations: set[str] = set()
        for match in ANNOTATION_RE.finditer(text):
            annotations.update(
                parity.package_identity(item.strip())
                for item in match.group(1).split(",")
                if item.strip()
            )
        documents.append({
            "path": path.relative_to(root).as_posix(),
            "is_index": path.name.lower() == "readme.md",
            "stem_identity": parity.package_identity(path.stem),
            "annotations": annotations,
            "search_text": text.lower(),
        })
    return documents


def _aliases(
    identity: str,
    package: str,
    packages: dict[str, dict[str, set[str]]],
) -> set[str]:
    values = {package.lower()}
    for bucket in parity.ALL_BUCKETS:
        values.update(
            name.lower()
            for name in packages.get(bucket, {}).get(identity, set())
        )
    return {value for value in values if value}


def _mentioning_paths(
    documents: Iterable[dict[str, Any]], aliases: set[str]
) -> list[str]:
    patterns = []
    for alias in aliases:
        pieces = [re.escape(piece) for piece in re.split(r"[^a-z0-9]+", alias) if piece]
        if pieces:
            patterns.append(
                re.compile(
                    r"(?<![a-z0-9])" + r"[\s._-]*".join(pieces) + r"(?![a-z0-9])"
                )
            )
    return sorted(
        document["path"]
        for document in documents
        if any(pattern.search(document["search_text"]) for pattern in patterns)
    )


def build_learning_report(
    packages: dict[str, dict[str, set[str]]],
    unknown_buckets: set[str],
    documents: list[dict[str, Any]],
) -> dict[str, Any]:
    """Join package concepts to learning documents."""

    package_report = parity.build_report(packages, unknown_buckets)
    rows: list[dict[str, Any]] = []
    for package_row in package_report["package_frequency"]:
        identity = package_row["identity"]
        aliases = _aliases(identity, package_row["package"], packages)
        explicit = sorted(
            document["path"]
            for document in documents
            if identity == document["stem_identity"]
            or identity in document["annotations"]
        )
        prose = _mentioning_paths(documents, aliases)
        non_index_prose = sorted(
            path
            for path in prose
            if not next(d for d in documents if d["path"] == path)["is_index"]
        )
        index_prose = sorted(set(prose) - set(non_index_prose))

        if explicit:
            status, evidence = "dedicated", explicit
        elif non_index_prose:
            status, evidence = "related", non_index_prose
        elif index_prose:
            status, evidence = "index-only", index_prose
        else:
            status, evidence = "missing", []

        rows.append({
            **package_row,
            "status": status,
            "priority": priority_for(package_row["language_count"]),
            "domain": domain_for(package_row["package"]),
            "evidence": evidence,
        })

    rows.sort(key=lambda row: (
        PRIORITY_ORDER.index(row["priority"]),
        -row["language_count"],
        row["package"],
    ))
    status_counts = Counter(row["status"] for row in rows)
    priority_counts = {
        priority: Counter(
            row["status"] for row in rows if row["priority"] == priority
        )
        for priority in PRIORITY_ORDER
    }
    return {
        "schema_version": 1,
        "methodology": {
            "statuses": {
                "dedicated": "filename or learning-concepts annotation",
                "related": "mentioned in a non-index learning document",
                "index-only": "mentioned only in a learning README",
                "missing": "no evidence in hand-authored learning documents",
            },
            "priorities": {
                "P0": "implemented in 10-15 languages",
                "P1": "implemented in 5-9 languages",
                "P2": "implemented in 2-4 languages",
                "P3": "implemented in 0-1 languages",
            },
        },
        "summary": {
            "concepts": len(rows),
            "documents": len(documents),
            **{status: status_counts[status] for status in STATUS_ORDER},
        },
        "priority_summary": {
            priority: {
                status: priority_counts[priority][status] for status in STATUS_ORDER
            }
            for priority in PRIORITY_ORDER
        },
        "concepts": rows,
    }


def render_markdown(report: dict[str, Any]) -> str:
    """Render a complete, reviewable backlog."""

    tick = chr(96)
    summary = report["summary"]
    lines = [
        "# Learning Coverage Inventory",
        "",
        "<!-- Generated by code/scripts/learning_coverage_report.py. Do not edit by hand. -->",
        "",
        "This inventory compares package concepts with the hand-authored material in",
        f"{tick}code/learning{tick}. It is a planning signal, not a claim that a name",
        "mention provides a complete lesson.",
        "",
        "## Summary",
        "",
        "| Concepts | Documents | Dedicated | Related | Index only | Missing |",
        "| ---: | ---: | ---: | ---: | ---: | ---: |",
        (
            f"| {summary['concepts']} | {summary['documents']} | "
            f"{summary['dedicated']} | {summary['related']} | "
            f"{summary['index-only']} | {summary['missing']} |"
        ),
        "",
        "## Method",
        "",
        "- **Dedicated:** a matching filename or an explicit "
        f"{tick}learning-concepts{tick} annotation.",
        "- **Related:** the concept is named in a non-index lesson.",
        "- **Index only:** the concept appears only in a learning README.",
        "- **Missing:** no evidence was found in hand-authored learning material.",
        "- Priorities derive from implementation breadth: P0 is 10-15 languages, "
        "P1 is 5-9, P2 is 2-4, and P3 is 0-1.",
        "",
        "## Priority Summary",
        "",
        "| Priority | Dedicated | Related | Index only | Missing |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for priority in PRIORITY_ORDER:
        counts = report["priority_summary"][priority]
        lines.append(
            f"| {priority} | {counts['dedicated']} | {counts['related']} | "
            f"{counts['index-only']} | {counts['missing']} |"
        )

    actionable = [
        row for row in report["concepts"]
        if row["status"] in {"missing", "index-only"}
    ]
    for priority in PRIORITY_ORDER:
        priority_rows = [row for row in actionable if row["priority"] == priority]
        if not priority_rows:
            continue
        lines.extend(["", f"## {priority} Backlog", ""])
        by_domain: dict[str, list[dict[str, Any]]] = defaultdict(list)
        for row in priority_rows:
            by_domain[row["domain"]].append(row)
        for domain in sorted(by_domain):
            lines.extend([
                f"### {domain}", "",
                "| Concept | Languages | Status |",
                "| --- | ---: | --- |",
            ])
            for row in by_domain[domain]:
                lines.append(
                    f"| {tick}{row['package']}{tick} | {row['language_count']} | "
                    f"{row['status']} |"
                )
            lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument("--format", choices=("markdown", "json"), default="markdown")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    root = args.root.resolve()
    packages, unknown = parity.discover_packages(root)
    documents = discover_learning_documents(root)
    report = build_learning_report(packages, unknown, documents)
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
