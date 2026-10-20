#!/usr/bin/env python3
"""Validate the LANG84 release inventories, `code/grammars/<dir>/releases.csv`.

WHY THIS EXISTS
===============

LANG84 (`code/specs/LANG84-grammar-driven-versioned-frontends.md`) makes every
upstream tagged release of a language a first-class, selectable thing:
`lang-aot --lang ruby@2.7.8`. The list of releases lives in one CSV per
grammar directory. Those files are large (Ruby has about 1,200 rows) and are
edited by hand and by scripts, so their invariants are checked here rather
than trusted:

    header        the twelve required columns, in order, before any extras
    release       unique, non-empty, charset [A-Za-z0-9._-]
    date          ISO date, optionally reduced to YYYY-MM or YYYY, optional `?`
    order         rows are in canonical inventory order: dates never decrease
    channel       one of the LANG84 channels
    source_ref    present
    source_sha    a 40- or 64-hex digest, or `-` (pending evidence)
    predecessor   names an EARLIER row; empty only on the first row, so the
                  lineage graph is a forest rooted at row one and is acyclic
    statuses      from the allowed sets, and the monotonic claims hold:
                    backend(S) => iir(S) => grammar_stage P
                    grammar_status partial|complete => grammar_stage P
    pair files    a row past stage `none` has both pair files, and every
                  versioned pair file in the directory has a row

Columns are always looked up by header NAME. A missing column is an error
rather than a silent shift of every later field (lessons.d: "read the
inventory header before grouping release statuses").

RELEASES ARE NOT SEMANTIC VERSIONS
==================================

Ordering is by date, never by comparing release IDs. Ruby 1.8.7-p374 (June
2013) was released after 2.0.0-p0 (February 2013), so a version sort would
misplace it. A date that is only known to the month (`1997-06`) or year
(`1999`) sorts as the start of that period:

    1999        -> 1999-00-00
    1999-12     -> 1999-12-00
    1999-12-01? -> 1999-12-01      (the `?` marks an uncertain date)

LEGACY SCHEMA
=============

Perl's inventory predates LANG84 and still uses the LANG82 header
`release,date,perlhist_line,grammar_status`. LANG84 phase 2b migrates it. Until
then it is reported as skipped, and ONLY the directories listed in
`LEGACY_SCHEMA` may be skipped: any other inventory with a missing column
fails.

USAGE
=====

    python3 code/scripts/release_inventory.py check [GRAMMARS_DIR]

`--matrix` (LANG84 "CI bounding") is not implemented yet; no row claims a
backend stage, so there is nothing for it to check.
"""

from __future__ import annotations

import argparse
import csv
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
GRAMMARS = REPO / "code" / "grammars"

REQUIRED = [
    "release",
    "date",
    "channel",
    "source_ref",
    "source_sha",
    "predecessor",
    "grammar_status",
    "grammar_stage",
    "grammar_subset",
    "iir_status",
    "backend_status",
    "profile",
]

CHANNELS = {
    "final",
    "patch",
    "preview",
    "alpha",
    "beta",
    "rc",
    "dev",
    "trial",
    "edition",
}
STATUSES = {"pending", "partial", "complete"}
STAGES = {"none", "G", "P"}

# Directory -> why its inventory may keep a pre-LANG84 header for now.
LEGACY_SCHEMA = {
    "perl": "LANG82 header; LANG84 phase 2b migrates it to the common schema",
}

# Pair-file stem prefix per directory. LANG84: the prefix is the existing file
# stem, and `ecmascript/` takes none because its IDs already start es/js.
PAIR_PREFIX = {"ecmascript": ""}

RELEASE_RE = re.compile(r"^[A-Za-z0-9._-]+$")
DATE_RE = re.compile(r"^(\d{4})(?:-(\d{2})(?:-(\d{2}))?)?(\?)?$")
SHA_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
LEGACY_PAIRS_RE = re.compile(r"^#\s*legacy-minor-line-pairs:\s*(.+)$")


@dataclass
class Report:
    """Errors and notes for one inventory, printed by `main`."""

    path: Path
    errors: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    rows: int = 0

    def error(self, message: str) -> None:
        self.errors.append(message)


def date_key(value: str) -> str | None:
    """Sort key for a date cell, or None when the cell is malformed.

    Missing month or day become `00`, so a reduced-precision date sorts at the
    start of its period. Month and day are range-checked; a calendar check
    (February 30) is left to review because upstream sources use real dates.
    """
    match = DATE_RE.match(value)
    if not match:
        return None
    year, month, day, _uncertain = match.groups()
    if month is not None and not 1 <= int(month) <= 12:
        return None
    if day is not None and not 1 <= int(day) <= 31:
        return None
    return f"{year}-{month or '00'}-{day or '00'}"


def split_list(cell: str) -> set[str]:
    """`a;b` -> {"a", "b"}; `-` -> empty set."""
    if cell == "-":
        return set()
    return {part for part in cell.split(";") if part}


def read_inventory(path: Path) -> tuple[list[str], list[str], list[dict[str, str]]]:
    """Return (comment lines, header, rows) for one releases.csv.

    Leading `#` lines are provenance comments. They are only allowed before
    the header, matching LANG82/LANG84.
    """
    comments: list[str] = []
    body: list[str] = []
    with path.open(encoding="utf-8", newline="") as handle:
        for line in handle:
            if not body and line.startswith("#"):
                comments.append(line.rstrip("\n"))
            else:
                body.append(line)
    reader = csv.reader(body)
    header = next(reader, [])
    rows = []
    for number, cells in enumerate(reader, start=2 + len(comments)):
        if len(cells) != len(header):
            rows.append({"__line__": str(number), "__malformed__": "1"})
            continue
        row = dict(zip(header, cells))
        row["__line__"] = str(number)
        rows.append(row)
    return comments, header, rows


def legacy_pairs(comments: list[str]) -> set[str]:
    """Pair stems the inventory declares as legacy (not release pairs)."""
    stems: set[str] = set()
    for line in comments:
        match = LEGACY_PAIRS_RE.match(line)
        if match:
            stems.update(match.group(1).split())
    return stems


def known_subsets(directory: Path) -> set[str]:
    path = directory / "subsets.csv"
    if not path.is_file():
        return set()
    with path.open(encoding="utf-8", newline="") as handle:
        return {row["subset"] for row in csv.DictReader(handle) if row.get("subset")}


def known_profile(name: str) -> bool:
    """A profile is named in some IIR compiler crate's `profiles.rs`."""
    pattern = re.compile(rf"\b{re.escape(name)}\b")
    for path in (REPO / "code" / "packages" / "rust").glob("*/src/profiles.rs"):
        if pattern.search(path.read_text(encoding="utf-8")):
            return True
    return False


def check_rows(report: Report, rows: list[dict[str, str]], subsets: set[str]) -> None:
    """Per-row and cross-row invariants (everything but the pair files)."""
    position: dict[str, int] = {}
    previous_key = ""
    previous_release = ""
    for index, row in enumerate(rows):
        where = f"line {row['__line__']}"
        if row.get("__malformed__"):
            report.error(f"{where}: wrong number of fields")
            continue
        release = row["release"]
        where = f"line {row['__line__']} ({release})"
        if not RELEASE_RE.match(release):
            report.error(f"{where}: release ID must match [A-Za-z0-9._-]+")
        if release in position:
            report.error(f"{where}: duplicate release ID")
        position[release] = index

        key = date_key(row["date"])
        if key is None:
            report.error(f"{where}: date {row['date']!r} is not YYYY[-MM[-DD]][?]")
        elif key < previous_key:
            report.error(
                f"{where}: date {row['date']} is earlier than the previous row "
                f"({previous_release}); rows must be in inventory (date) order"
            )
        else:
            previous_key = key
            previous_release = release

        if row["channel"] not in CHANNELS:
            report.error(
                f"{where}: channel {row['channel']!r} is not one of {sorted(CHANNELS)}"
            )
        if not row["source_ref"].strip():
            report.error(f"{where}: source_ref is empty")
        if row["source_sha"] != "-" and not SHA_RE.match(row["source_sha"]):
            report.error(f"{where}: source_sha must be 40/64 lowercase hex or '-'")

        predecessor = row["predecessor"]
        if not predecessor:
            if index != 0:
                report.error(
                    f"{where}: predecessor is empty but this is not the first row"
                )
        elif predecessor not in position:
            report.error(
                f"{where}: predecessor {predecessor!r} is not an earlier row "
                "(it must exist and precede this row)"
            )
        elif predecessor == release:
            report.error(f"{where}: release is its own predecessor")

        check_status(report, where, row, subsets)


def check_status(
    report: Report, where: str, row: dict[str, str], subsets: set[str]
) -> None:
    """Allowed values plus LANG84's monotonic stage claims for one row."""
    status, stage = row["grammar_status"], row["grammar_stage"]
    subset, profile = row["grammar_subset"], row["profile"]
    iir, backend = split_list(row["iir_status"]), split_list(row["backend_status"])
    if status not in STATUSES:
        report.error(
            f"{where}: grammar_status {status!r} is not one of {sorted(STATUSES)}"
        )
    if stage not in STAGES:
        report.error(f"{where}: grammar_stage {stage!r} is not one of {sorted(STAGES)}")
    if status in {"partial", "complete"} and stage != "P":
        report.error(f"{where}: grammar_status {status} requires grammar_stage P")
    if status == "pending" and subset != "-":
        report.error(f"{where}: a pending row has grammar_subset '-'")
    if status == "complete" and subset != "full":
        report.error(f"{where}: a complete row has grammar_subset 'full'")
    if status == "partial" and subset in {"-", "full"}:
        report.error(f"{where}: a partial row names the subset it covers")
    if iir and stage != "P":
        report.error(f"{where}: iir_status claims need grammar_stage P")
    if not backend <= iir:
        report.error(
            f"{where}: backend_status {sorted(backend - iir)} not also in iir_status"
        )
    if (profile == "-") != (not iir):
        report.error(f"{where}: profile is '-' exactly when iir_status is '-'")
    named = iir | backend | ({subset} if status == "partial" else set())
    for name in sorted(named - subsets):
        report.error(f"{where}: subset {name!r} is not declared in subsets.csv")
    if profile != "-" and not known_profile(profile):
        report.error(f"{where}: profile {profile!r} is not defined in any profiles.rs")


def check_pairs(
    report: Report,
    directory: Path,
    rows: list[dict[str, str]],
    legacy: set[str],
) -> None:
    """Every claimed pair exists, and every versioned pair file has a row."""
    prefix = PAIR_PREFIX.get(directory.name, directory.name)
    releases = {row["release"]: row for row in rows if not row.get("__malformed__")}
    for release, row in releases.items():
        if row["grammar_stage"] == "none" and row["grammar_status"] == "pending":
            continue
        for suffix in (".tokens", ".grammar"):
            if not (directory / f"{prefix}{release}{suffix}").is_file():
                report.error(
                    f"{release}: claims a stage but {prefix}{release}{suffix} is missing"
                )
    stems = {path.stem for path in directory.glob("*.tokens")}
    stems |= {path.stem for path in directory.glob("*.grammar")}
    for stem in sorted(stems):
        if stem == prefix or stem in legacy:
            continue  # the unversioned pair, or a declared legacy pair
        if not stem.startswith(prefix) or stem[len(prefix) :] not in releases:
            report.error(f"pair file {stem}.* has no releases.csv row")
    for stem in sorted(legacy - stems):
        report.error(f"legacy pair {stem} is declared but has no files")


def check_inventory(path: Path) -> Report:
    report = Report(path)
    comments, header, rows = read_inventory(path)
    report.rows = len(rows)
    missing = [column for column in REQUIRED if column not in header]
    directory = path.parent
    if missing:
        if directory.name in LEGACY_SCHEMA:
            report.notes.append(f"skipped: {LEGACY_SCHEMA[directory.name]}")
            return report
        report.error(f"header is missing required column(s): {', '.join(missing)}")
        return report
    if header[: len(REQUIRED)] != REQUIRED:
        report.error(
            "required columns must come first, in LANG84 order: " + ",".join(REQUIRED)
        )
    duplicates = {column for column in header if header.count(column) > 1}
    if duplicates:
        report.error(f"duplicate header column(s): {', '.join(sorted(duplicates))}")
        return report
    if not rows:
        report.error("inventory has no rows")
        return report
    check_rows(report, rows, known_subsets(directory))
    check_pairs(report, directory, rows, legacy_pairs(comments))
    return report


def check(grammars: Path) -> int:
    paths = sorted(grammars.glob("*/releases.csv"))
    if not paths:
        print(f"no releases.csv under {grammars}", file=sys.stderr)
        return 1
    failed = False
    for path in paths:
        report = check_inventory(path)
        name = path.relative_to(grammars)
        for note in report.notes:
            print(f"{name}: {note}")
        for message in report.errors:
            print(f"{name}: {message}", file=sys.stderr)
        if report.errors:
            failed = True
        elif not report.notes:
            print(f"{name}: ok ({report.rows} rows)")
    return 1 if failed else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    check_parser = commands.add_parser("check", help="validate every releases.csv")
    check_parser.add_argument("grammars", nargs="?", type=Path, default=GRAMMARS)
    args = parser.parse_args(argv)
    if args.command == "check":
        return check(args.grammars)
    return 2  # pragma: no cover - argparse rejects unknown commands


if __name__ == "__main__":
    sys.exit(main())
