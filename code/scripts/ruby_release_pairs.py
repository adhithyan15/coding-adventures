#!/usr/bin/env python3
"""Materialize the Ruby LANG84 release pairs from `pair-evidence.csv`.

WHY THIS EXISTS
===============

LANG84 gives every tagged Ruby release its own `ruby<release>.tokens` and
`ruby<release>.grammar` (no aliases, no symlinks). Most releases do not change
the syntax files at all, so writing each pair by hand would be busywork and,
worse, invites copy errors. LANG84 therefore allows a pair to be produced
MECHANICALLY when the release's syntax files are byte-identical to the release
it is copied from, provided the header cites both releases' blob IDs.

This script is that mechanism, and nothing more:

    pair-evidence.csv            releases.csv
    (blob IDs + basis)           (date, source_ref, source_sha)
            \\                         /
             +----> header (citations) +----> ruby<release>.tokens
                                       |      ruby<release>.grammar
    basis = reviewed  -> body kept from the hand-written pair on disk
    basis = copy:<R>  -> body copied from ruby<R>.* (blobs must match R's)

A "body" is everything after the header: the header is the leading block of
`#` lines up to the first blank line. Reviewed pairs are written by hand
(their body is the review); the script only refreshes their header.

USAGE
=====

    python3 code/scripts/ruby_release_pairs.py write   # rewrite headers/copies
    python3 code/scripts/ruby_release_pairs.py check   # fail if anything differs

`check` is what a reviewer (or CI) runs: it proves every copy really is a copy
of a release with identical syntax blobs, and that every header cites the
inventory's source_ref and source_sha.
"""

from __future__ import annotations

import argparse
import csv
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RUBY = REPO / "code" / "grammars" / "ruby"
BLOB_COLUMNS = [
    ("parse_y", "parse.y"),
    ("keywords", "keywords"),
    ("defs_keywords", "defs/keywords"),
    ("prism_config", "prism/config.yml"),
]
KINDS = {
    ".tokens": "tokens: LANG84 R0 lexical slice",
    ".grammar": "grammar: rb-int-puts subset over the R0 tokens",
}
PARTIAL = {
    ".tokens": "# PARTIAL: lexes only the R0 constructs listed below; see code/grammars/ruby/README.md.",
    ".grammar": "# PARTIAL: parses only the rb-int-puts subset below; everything else is a parse error.",
}


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as handle:
        body = [line for line in handle if not line.startswith("#")]
    return list(csv.DictReader(body))


def split_header(text: str) -> tuple[str, str]:
    """Return (header, body). The header is the leading `#` block."""
    lines = text.split("\n")
    index = 0
    while index < len(lines) and lines[index].startswith("#"):
        index += 1
    if index == len(lines) or lines[index] != "":
        raise ValueError("pair file must start with a `#` header and a blank line")
    return "\n".join(lines[:index]), "\n".join(lines[index + 1 :])


def blob_line(row: dict[str, str]) -> str:
    return "; ".join(f"{name} blob {row[column]}" for column, name in BLOB_COLUMNS)


def header(suffix: str, inv: dict[str, str], ev: dict[str, str]) -> str:
    release = inv["release"]
    lines = [
        f"# Ruby {release} ({inv['date']}) {KINDS[suffix]}.",
        "# @version 1",
        PARTIAL[suffix],
        f"# source_ref: {inv['source_ref']}",
        f"# source_sha: {inv['source_sha']}",
        f"# evidence: {blob_line(ev)}",
    ]
    basis = ev["basis"]
    if basis == "reviewed":
        lines.append(
            f"# basis: reviewed against the tag's own syntax files. {ev['note']}"
        )
    else:
        source = basis.removeprefix("copy:")
        lines.append(
            f"# basis: mechanical copy of ruby{source}{suffix}; every syntax blob above is"
        )
        lines.append(
            f'#   identical to release {source}\'s (LANG84 "Materialized pair per release").'
        )
    return "\n".join(lines)


def plan() -> dict[Path, str]:
    """Every pair file's expected text, keyed by path."""
    inventory = {row["release"]: row for row in read_rows(RUBY / "releases.csv")}
    evidence = read_rows(RUBY / "pair-evidence.csv")
    by_release = {row["release"]: row for row in evidence}
    expected: dict[Path, str] = {}
    for ev in evidence:
        release = ev["release"]
        if release not in inventory:
            raise SystemExit(f"pair-evidence.csv: {release} is not in releases.csv")
        if inventory[release]["source_sha"] != ev["commit"]:
            raise SystemExit(
                f"pair-evidence.csv: {release} commit differs from releases.csv"
            )
        basis = ev["basis"]
        if basis.startswith("copy:"):
            source = basis.removeprefix("copy:")
            if source not in by_release:
                raise SystemExit(f"{release}: copy source {source} has no evidence row")
            for column, name in BLOB_COLUMNS:
                if ev[column] != by_release[source][column]:
                    raise SystemExit(
                        f"{release}: {name} blob differs from {source}'s, so it cannot be a copy"
                    )
        elif basis != "reviewed":
            raise SystemExit(f"{release}: basis must be `reviewed` or `copy:<release>`")
        for suffix in KINDS:
            if basis == "reviewed":
                current = (RUBY / f"ruby{release}{suffix}").read_text(encoding="utf-8")
            else:
                current = expected[RUBY / f"ruby{source}{suffix}"]
            _, body = split_header(current)
            expected[RUBY / f"ruby{release}{suffix}"] = (
                header(suffix, inventory[release], ev) + "\n\n" + body
            )
    return expected


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", choices=["write", "check"])
    args = parser.parse_args(argv)
    stale = []
    for path, text in plan().items():
        current = path.read_text(encoding="utf-8") if path.exists() else None
        if current != text:
            stale.append(path)
            if args.command == "write":
                path.write_text(text, encoding="utf-8")
    for path in stale:
        verb = "wrote" if args.command == "write" else "stale"
        print(f"{verb}: {path.relative_to(REPO)}")
    return 1 if stale and args.command == "check" else 0


if __name__ == "__main__":
    sys.exit(main())
