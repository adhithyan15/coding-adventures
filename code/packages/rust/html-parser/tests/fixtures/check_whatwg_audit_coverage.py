#!/usr/bin/env python3

"""Check that focused WHATWG parser audits cover the local smoke corpus."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
DEFAULT_SMOKE = FIXTURE_DIR / "html5lib-tree-construction-smoke.dat"
AUDIT_GLOB = "whatwg-*-audit.json"


def main() -> int:
    args = parse_args()
    smoke_path = Path(args.smoke).expanduser().resolve()
    audit_paths = sorted(smoke_path.parent.glob(AUDIT_GLOB))

    if not audit_paths:
        raise SystemExit(f"no audit fixtures matched {smoke_path.parent / AUDIT_GLOB}")

    smoke_sources = parse_smoke_sources(smoke_path)
    result = check_audits(smoke_path, smoke_sources, audit_paths)

    print("WHATWG parser audit coverage")
    print(f"smoke fixture: {smoke_path}")
    print(f"smoke cases:   {len(smoke_sources)}")
    print(f"audit files:   {len(audit_paths)}")
    print(f"indexed cases: {len(result.indexed_sources)}")
    print(f"missing:       {len(result.missing_sources)}")
    print(f"stale:         {len(result.stale_sources)}")

    errors = []
    if result.missing_sources:
        errors.append(
            "smoke cases missing from focused audits:\n"
            + "\n".join(f"  {source}" for source in result.missing_sources)
        )
    if result.stale_sources:
        errors.append(
            "audit cases missing from smoke fixture:\n"
            + "\n".join(
                f"  {source} ({', '.join(files)})"
                for source, files in result.stale_sources.items()
            )
        )
    if result.duplicate_sources:
        errors.append(
            "duplicate sources inside audit fixture:\n"
            + "\n".join(
                f"  {source} ({', '.join(files)})"
                for source, files in result.duplicate_sources.items()
            )
        )
    if result.bad_source_fixtures:
        errors.append(
            "audit fixtures pointing at unexpected source fixture:\n"
            + "\n".join(
                f"  {file}: {source_fixture}"
                for file, source_fixture in result.bad_source_fixtures.items()
            )
        )

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check that every html5lib tree-construction smoke case is indexed "
            "by at least one focused WHATWG parser audit fixture."
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


def parse_smoke_sources(path: Path) -> list[str]:
    sources = [
        line.removeprefix("#source ").strip()
        for line in path.read_text(errors="replace").splitlines()
        if line.startswith("#source ")
    ]
    if not sources:
        raise SystemExit(f"{path} does not contain any #source markers")
    duplicate_sources = sorted(
        source for source in set(sources) if sources.count(source) > 1
    )
    if duplicate_sources:
        raise SystemExit(
            "duplicate #source markers in smoke fixture:\n"
            + "\n".join(f"  {source}" for source in duplicate_sources)
        )
    return sources


class AuditCoverageResult:
    def __init__(self) -> None:
        self.indexed_sources: set[str] = set()
        self.missing_sources: list[str] = []
        self.stale_sources: dict[str, list[str]] = {}
        self.duplicate_sources: dict[str, list[str]] = {}
        self.bad_source_fixtures: dict[str, str] = {}


def check_audits(
    smoke_path: Path, smoke_sources: list[str], audit_paths: list[Path]
) -> AuditCoverageResult:
    smoke_source_set = set(smoke_sources)
    result = AuditCoverageResult()

    for audit_path in audit_paths:
        audit = json.loads(audit_path.read_text())
        source_fixture = audit.get("source_fixture")
        if source_fixture != smoke_path.name:
            result.bad_source_fixtures[audit_path.name] = str(source_fixture)

        seen_in_file: set[str] = set()
        for case in audit.get("cases", []):
            source = case.get("source")
            if not isinstance(source, str):
                raise SystemExit(f"{audit_path} contains a case without a string source")
            if source in seen_in_file:
                result.duplicate_sources.setdefault(source, []).append(audit_path.name)
            seen_in_file.add(source)
            result.indexed_sources.add(source)
            if source not in smoke_source_set:
                result.stale_sources.setdefault(source, []).append(audit_path.name)

    result.missing_sources = [
        source for source in smoke_sources if source not in result.indexed_sources
    ]
    return result


if __name__ == "__main__":
    raise SystemExit(main())
