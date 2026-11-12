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
    subsets       every named subset is declared in `subsets.csv`, and a
                  subset claimed at stage I or B has a corpus with cases
    pair files    a row past stage `none` has both pair files, and every
                  versioned pair file in the directory has a row

Columns are always looked up by header NAME. A missing column is an error
rather than a silent shift of every later field (lessons.d: "read the
inventory header before grouping release statuses"). Every inventory uses the
common header; there is no legacy-schema exemption (Perl's LANG82 inventory
was migrated in LANG84 phase 2b).

RELEASES ARE NOT SEMANTIC VERSIONS
==================================

Ordering is by date, never by comparing release IDs. Ruby 1.8.7-p374 (June
2013) was released after 2.0.0-p0 (February 2013), so a version sort would
misplace it. A date that is only known to the month (`1997-06`) or year
(`1999`) stands for that whole period:

    date          earliest      latest
    1999          1999-00-00    1999-12-31
    1999-12       1999-12-00    1999-12-31
    1999-12-01?   1999-12-01    1999-12-01   (the `?` marks an uncertain date)

The order check asks whether some choice of a day inside each row's period
makes the column non-decreasing. Walking down the rows, `floor` is the latest
`earliest` seen so far; a row fails only if even its `latest` is before the
floor. That keeps perlhist's `5.000alpha4  1993-???-??` between alpha3
(1993-10-10) and alpha6 (1994-03-18), where upstream lists it, instead of
forcing it to the top of 1993. For fully dated rows it is the plain
"never decreases" rule.

SUBSETS (`subsets.csv`)
=======================

A subset is a named slice of a language (LANG84 "Status stages"):

    subset,description,corpus_dir,spec
    rb-int-puts,LANG80 integer puts subset,corpus/rb-int-puts,LANG80

`corpus_dir` is always `corpus/<subset>` (LANG84 "Corpora and oracles"), so a
CSV cell can never point outside the grammar directory. `spec` names a file
`code/specs/<spec>-*.md`. A corpus case is `<name>.<ext>` plus `<name>.expect`
and its oracle record `<name>.cite`; a negative case is `<name>.<ext>` plus
`<name>.reject`. A positive case may carry per-profile overrides:
`<name>.expect@<profile>` (different output under that profile) or
`<name>.reject@<profile>` (outside the subset under that profile, LANG84 4c),
and the profile must be defined in a `profiles.rs`.

THE EDITION AXIS (RS01, LANG84 "Amendment RS01" R1)
===================================================

Rust has a second selection input besides the release: the edition (2015,
2018, 2021, 2024). One release compiles under several editions, and an edition
changes the lexer and the parser, so a release has ONE PAIR PER EDITION:

    rust0.4.tokens                  pre-1.0: no editions, plain stem
    rust1.85.0-e2021.tokens         release 1.85.0, edition 2021

An inventory opts in by having an `editions` column and a header comment that
says where each edition starts:

    # editions: 2015@1.0.0-alpha 2018@1.31.0 2021@1.56.0 2024@1.85.0

The checker does not trust the column; it DERIVES it. A row's editions are its
`predecessor`'s editions plus any edition whose first release is this row.
Lineage carries the set, not the date, so a 1.30.1 patch released after 1.31.0
never gains 2018:

    1.30.0  2015
    1.31.0  2015;2018      (2018 starts here)
    1.30.1  2015           (predecessor is 1.30.0)

Three more rules follow from it:

    stems         split from the RIGHT: a trailing `-e` plus exactly four
                  digits is the edition. So a release ID ending in `-e<digits>`
                  would be ambiguous and is refused in an edition-axis
                  inventory.
    uniform       a row's status holds for EVERY edition it lists: a claimed
    claims        row needs both files for each edition, and a pair file for
                  an edition the row does not list is an error.
    overrides     a corpus override key may name a pair:
                  `<case>.reject@1.85.0-e2021`.

For the directory named `rust` the checker also enforces RS01 section 1:
release IDs are the published ones (nightly or beta-looking IDs are refused),
each row's channel, `predecessor` and `lineage` follow RS01's rules, and no two
rows continue the same predecessor on the same lineage. Every other inventory
is checked exactly as before.

SUBSET GATES (`allowlist` column)
=================================

`subsets.csv` may carry a fifth column, `allowlist`: `-` (no gate) or the name
of a `*.allowlist` file in the grammar directory itself (never under
`corpus/`, where it would read as a corpus case). Membership in a gated subset
is the release grammar plus that checked-in gate (RS01 section 7.1). The file
must exist only once a row claims the subset; until then (slice A2 declares
`rs-confined`, slice B1 writes its allowlist) a missing file is fine.

THE MATRIX CHECK (`--matrix`)
=============================

LANG84 "CI bounding" runs every release on every backend only for boundary
releases. `check --matrix` verifies the one claim the per-row checks cannot:
a release that claims B(S) is backed by backend runs. For every such release R
and every case C of S's corpus, one of these must hold:

    1. `release_classes.csv` marks C as `rejected` under R (the case does not
       apply: R's grammar does not parse it, or a `.reject@<profile>`
       override puts it outside R's profile);
    2. a `lang_matrix` row runs C on R itself;
    3. `release_classes.csv` puts (C, R) in an IIR-identity class whose
       representative -- its first release in inventory order -- has a
       `lang_matrix` row for C.

`release_classes.csv` (`subset,case,release,class`) is written by the release
sweep (LANG84 phase 4c); `class` is the hex digest of the IIR that R produces
for C, or `rejected`. A `lang_matrix` row (`Prog { .. }` in
`code/packages/rust/lang-aot/tests/lang_matrix.rs`) counts when it names
`release: Some("<id>")` and `case: Some("<subset>/<name>")`. Rows naming an
unknown release, subset or case fail the check, so stale rows cannot linger,
and so does a row of an inventoried language that names no release (since
LANG84 phase 4c; phase 2b only counted such rows in a note).

USAGE
=====

    python3 code/scripts/release_inventory.py check [--matrix] [GRAMMARS_DIR]
        [--matrix-source lang_matrix.rs]
"""

from __future__ import annotations

import argparse
import bisect
import csv
import itertools
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
GRAMMARS = REPO / "code" / "grammars"
SPECS = REPO / "code" / "specs"
LANG_MATRIX = (
    REPO / "code" / "packages" / "rust" / "lang-aot" / "tests" / "lang_matrix.rs"
)

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
SUBSET_COLUMNS = ["subset", "description", "corpus_dir", "spec"]
CLASS_COLUMNS = ["subset", "case", "release", "class"]

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

# Pair-file stem prefix per directory. LANG84: the prefix is the existing file
# stem, and `ecmascript/` takes none because its IDs already start es/js.
PAIR_PREFIX = {"ecmascript": ""}

# `Language::<Variant>` in lang-aot -> grammar directory, for `--matrix`.
MATRIX_LANGUAGES = {
    "Perl": "perl",
    "Ruby": "ruby",
    "Python": "python",
    "JavaScript": "ecmascript",
}

RELEASE_RE = re.compile(r"^[A-Za-z0-9._-]+\Z")
DATE_RE = re.compile(r"^(\d{4})(?:-(\d{2})(?:-(\d{2}))?)?(\?)?\Z")
SHA_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")
LEGACY_PAIRS_RE = re.compile(r"^#\s*legacy-minor-line-pairs:\s*(.+)$")
SUBSET_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*\Z")
SPEC_RE = re.compile(r"^[A-Z]+[0-9]+[A-Za-z0-9]*\Z")
CASE_RE = re.compile(r"^[A-Za-z0-9_-]+\Z")
CLASS_RE = re.compile(r"^(?:[0-9a-f]{12,64}|rejected)\Z")
# `\Z`, not `$`: Python's `$` also matches before a trailing newline, so
# `"1.0\n"` (a quoted CSV cell, or a file name) would pass as `1.0`.
PROFILE_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*\Z")
# Edition axis (RS01). `EDITION_STEM_RE` splits a pair stem from the right.
# `\Z`, not `$`, for the same reason as above.
EDITION_STEM_RE = re.compile(r"^(.*)-e([0-9]{4})\Z")
EDITION_RE = re.compile(r"^[0-9]{4}\Z")
EDITIONS_HEADER_RE = re.compile(r"^#\s*editions:(.*)$")
EDITION_FIRST_RE = re.compile(r"^([0-9]{4})@([A-Za-z0-9._-]+)\Z")
# A gate file name: a plain file name in the grammar directory.
ALLOWLIST_RE = re.compile(r"^[A-Za-z0-9_-][A-Za-z0-9._-]*\.allowlist\Z")

# Rust's published releases before 1.x, in lineage order (RS01 section 1.2).
# 0.3.1 is the one pre-1.0 patch. The three 1.0 pre-releases are the only
# published semver pre-release versions; there are no others, so a beta.2, an
# rc, or a nightly is not an inventory row.
RUST_PRE_1_0 = [
    "0.1",
    "0.2",
    "0.3",
    "0.3.1",
    "0.4",
    "0.5",
    "0.6",
    "0.7",
    "0.8",
    "0.9",
    "0.10",
    "0.11.0",
    "0.12.0",
]
RUST_PRE_RELEASES = ["1.0.0-alpha", "1.0.0-alpha.2", "1.0.0-beta"]
RUST_1X_RE = re.compile(r"^1\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\Z")
LINEAGES = {"first", "line", "branch-point"}

# A grammar directory name is the first thing on every printed line.
DIRECTORY_RE = re.compile(r"^[a-z0-9_-]+\Z")


@dataclass
class Report:
    """Errors and notes for one inventory, printed by `main`."""

    path: Path
    errors: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    rows: int = 0

    def error(self, message: str) -> None:
        self.errors.append(message)


# ---------------------------------------------------------------------------
# Dates
# ---------------------------------------------------------------------------


def date_key(value: str) -> str | None:
    """Earliest sort key for a date cell, or None when the cell is malformed.

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


def date_span(value: str) -> tuple[str, str] | None:
    """(earliest, latest) keys of the period a date cell names.

    `1993` -> ("1993-00-00", "1993-12-31"); `1993-07` -> ("1993-07-00",
    "1993-07-31"); a full date is a one-day period.
    """
    earliest = date_key(value)
    if earliest is None:
        return None
    year, month, day = earliest.split("-")
    latest = f"{year}-{'12' if month == '00' else month}-{'31' if day == '00' else day}"
    return earliest, latest


# ---------------------------------------------------------------------------
# Reading the CSV files
# ---------------------------------------------------------------------------


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


def read_table(
    report: Report, path: Path, columns: list[str]
) -> list[dict[str, str]] | None:
    """Rows of a small side table (`subsets.csv`, `release_classes.csv`).

    Columns are found by name; an absent one is an error and the table is
    treated as unreadable (None) so that no check runs on shifted fields.
    """
    with path.open(encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        header = reader.fieldnames or []
        missing = [column for column in columns if column not in header]
        if missing:
            report.error(
                f"{path.name}: header is missing column(s): {', '.join(missing)}"
            )
            return None
        rows = []
        for number, row in enumerate(reader, start=2):
            if None in row or any(row[column] is None for column in columns):
                report.error(f"{path.name} line {number}: wrong number of fields")
                continue
            row["__line__"] = str(number)
            rows.append(row)
        return rows


def legacy_pairs(comments: list[str]) -> set[str]:
    """Pair stems the inventory declares as legacy (not release pairs)."""
    stems: set[str] = set()
    for line in comments:
        match = LEGACY_PAIRS_RE.match(line)
        if match:
            stems.update(match.group(1).split())
    return stems


# ---------------------------------------------------------------------------
# Subsets and their corpora
# ---------------------------------------------------------------------------


def is_plain(path: Path, root: Path) -> bool:
    """`path` exists, and neither it nor any directory between `root` and it
    is a symlink. A PR could otherwise commit `releases.csv` (or a corpus
    directory) as a link to a file outside the repository, and its contents
    would be echoed into the public CI log as "malformed rows"."""
    current = path
    while True:
        if current.is_symlink():
            return False
        if current == root or current.parent == current:
            break
        current = current.parent
    return path.exists()


def spec_exists(name: str) -> bool:
    return (SPECS / f"{name}.md").is_file() or any(SPECS.glob(f"{name}-*.md"))


def read_subsets(report: Report, directory: Path) -> dict[str, dict[str, str]]:
    """Declared subsets of one grammar directory, validated, by name."""
    path = directory / "subsets.csv"
    if not path.is_file():
        return {}
    if not is_plain(path, directory.parent):
        report.error("subsets.csv: must not be a symlink")
        return {}
    rows = read_table(report, path, SUBSET_COLUMNS)
    subsets: dict[str, dict[str, str]] = {}
    for row in rows or []:
        name = row["subset"]
        where = f"subsets.csv line {row['__line__']} ({name})"
        if not SUBSET_RE.match(name) or name == "full":
            report.error(f"{where}: subset name must match [a-z0-9]+(-[a-z0-9]+)*")
            continue
        if name in subsets:
            report.error(f"{where}: duplicate subset")
            continue
        if not row["description"].strip():
            report.error(f"{where}: description is empty")
        if row["corpus_dir"] != f"corpus/{name}":
            report.error(f"{where}: corpus_dir must be corpus/{name}")
        if not SPEC_RE.match(row["spec"]) or not spec_exists(row["spec"]):
            report.error(f"{where}: spec {row['spec']!r} is not a file in code/specs")
        if "allowlist" in row:
            cell = row["allowlist"]
            if cell is None:
                report.error(f"{where}: allowlist cell is missing")
            elif cell != "-" and not ALLOWLIST_RE.match(cell):
                report.error(
                    f"{where}: allowlist {printable(repr(cell))} must be '-' or a "
                    "plain *.allowlist file name in the grammar directory"
                )
        subsets[name] = row
    return subsets


def corpus_cases(
    report: Report, directory: Path, subset: str, releases: set[str] | None = None
) -> set[str] | None:
    """Positive case names of a subset's corpus, or None when it has none.

    A case is `<name>.<ext>` + `<name>.expect` + `<name>.cite`; a negative
    probe is `<name>.<ext>` + `<name>.reject`. Overrides `<name>.expect@<p>`
    and `<name>.reject@<p>` need the plain `.expect`. `<p>` is a profile
    that some `profiles.rs` defines (Perl's `reject@perl_nv`) or a row of
    this language's `releases.csv` (Python's `int_underscores.reject@3.0a1`),
    or, on an edition axis, a (release, edition) pair written
    `<release>-e<edition>` (Rust's `x.reject@1.85.0-e2021`). The caller puts
    the valid pair names in `releases` next to the release IDs.
    Only positive cases need backend coverage.
    """
    corpus = directory / "corpus" / subset
    if not corpus.is_dir():
        return None
    # A corpus file is a tiny program or record; a large one is a mistake or
    # an attempt to make the oracle tests slow.
    max_bytes = 64 * 1024
    if not is_plain(corpus, directory.parent):
        report.error(f"corpus/{subset}: must not be or sit under a symlink")
        return None
    files = []
    for path in corpus.iterdir():
        shown = printable(repr(path.name))
        if path.is_symlink():
            report.error(f"corpus/{subset}: {shown} is a symlink")
        elif path.is_file():
            files.append(path.name)
            if path.stat().st_size > max_bytes:
                report.error(f"corpus/{subset}: {shown} exceeds {max_bytes} bytes")
                continue
            # CRLF line ends are allowed (a case may test them); a bare CR is
            # not, because it can hide text in terminal and diff views.
            data = path.read_bytes()
            if data.count(b"\r") != data.count(b"\r\n"):
                report.error(f"corpus/{subset}: {shown} has a CR not followed by LF")
    stems: dict[str, set[str]] = {}
    for name in files:
        stem, dot, suffix = name.partition(".")
        stems.setdefault(stem, set()).add(suffix if dot else "")
    cases: set[str] = set()
    for stem, suffixes in sorted(stems.items()):
        where = f"corpus/{subset}/{printable(stem)}"
        if not CASE_RE.match(stem):
            report.error(f"{where}: case name must match [A-Za-z0-9_-]+")
            continue
        overrides = {s for s in suffixes if s.startswith(("expect@", "reject@"))}
        sources = suffixes - overrides - {"expect", "cite", "reject"}
        positive, negative = "expect" in suffixes, "reject" in suffixes
        if positive == negative:
            report.error(f"{where}: needs exactly one of .expect or .reject")
        if len(sources) != 1:
            report.error(
                f"{where}: needs exactly one source file, found {sorted(sources)}"
            )
        if positive and "cite" not in suffixes:
            report.error(f"{where}: .expect has no .cite oracle record")
        if overrides and not positive:
            report.error(f"{where}: a profile override needs the plain .expect")
        targets = [o.partition("@")[2] for o in overrides]
        for target in sorted({t for t in targets if targets.count(t) > 1}):
            report.error(
                f"{where}: more than one override for {printable(repr(target))}"
            )
        for override in sorted(overrides):
            target = override.partition("@")[2]
            is_profile = bool(PROFILE_RE.match(target)) and known_profile(target)
            is_release = releases is not None and target in releases
            if not (is_profile or is_release):
                report.error(
                    f"{where}: override .{printable(repr(override))} names no "
                    "profile defined in a profiles.rs and no releases.csv row"
                )
        if positive:
            cases.add(stem)
    return cases


# ---------------------------------------------------------------------------
# Rows
# ---------------------------------------------------------------------------


# A profile is defined by a `name: "<id>"` field of a `ReleaseProfile` literal.
PROFILE_NAME_RE = re.compile(r'\bname\s*:\s*"([A-Za-z_][A-Za-z0-9_]*)"')


def profile_names(rust_packages: Path | None = None) -> set[str]:
    """Every profile some IIR compiler crate's `profiles.rs` defines.

    Only the string values of `name:` fields count. Matching the bare word
    anywhere in the file (as phase 2b did) let `pub`, `Double` or a word in a
    comment pass as a profile, so a misspelled `.expect@<p>` override was
    accepted here and then silently ignored by the frontend's tests.
    """
    root = rust_packages or REPO / "code" / "packages" / "rust"
    names: set[str] = set()
    for path in sorted(root.glob("*/src/profiles.rs")):
        if path.is_symlink():
            continue
        names.update(PROFILE_NAME_RE.findall(path.read_text(encoding="utf-8")))
    return names


def known_profile(name: str) -> bool:
    """`name` is a profile some IIR compiler crate's `profiles.rs` defines."""
    return name in profile_names()


def check_rows(report: Report, rows: list[dict[str, str]], subsets: set[str]) -> None:
    """Per-row and cross-row invariants (everything but the pair files)."""
    position: dict[str, int] = {}
    floor = ""  # the latest `earliest` date so far (see the module docstring)
    floor_release = ""
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

        span = date_span(row["date"])
        if span is None:
            report.error(f"{where}: date {row['date']!r} is not YYYY[-MM[-DD]][?]")
        elif span[1] < floor:
            report.error(
                f"{where}: date {row['date']} is earlier than the previous row "
                f"({floor_release}); rows must be in inventory (date) order"
            )
        elif span[0] > floor:
            floor, floor_release = span[0], release

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
    if profile != "-" and not PROFILE_RE.match(profile):
        report.error(f"{where}: profile {profile!r} is not a Rust identifier")
    elif profile != "-" and not known_profile(profile):
        report.error(f"{where}: profile {profile!r} is not defined in any profiles.rs")


def check_claimed_corpora(
    report: Report,
    directory: Path,
    rows: list[dict[str, str]],
    subsets: set[str],
    editions: dict[str, tuple[str, ...]] | None = None,
) -> dict[str, set[str]]:
    """Stages I and B are measured on a corpus, so a claimed subset needs one.

    Returns the positive cases of every claimed subset (for `--matrix`).
    """
    claimed: set[str] = set()
    for row in rows:
        if not row.get("__malformed__"):
            claimed |= split_list(row["iir_status"]) | split_list(row["backend_status"])
    releases = {row["release"] for row in rows if not row.get("__malformed__")}
    # On an edition axis an override may also name a pair (R1.6).
    for release, listed in (editions or {}).items():
        releases |= {f"{release}-e{edition}" for edition in listed}
    cases: dict[str, set[str]] = {}
    for subset in sorted(claimed & subsets):
        found = corpus_cases(report, directory, subset, releases)
        if not found:
            report.error(
                f"subset {subset!r} is claimed at stage I or B but corpus/{subset}/ "
                "has no positive case"
            )
        cases[subset] = found or set()
    return cases


def check_pairs(
    report: Report,
    directory: Path,
    rows: list[dict[str, str]],
    legacy: set[str],
    editions: dict[str, tuple[str, ...]] | None = None,
) -> None:
    """Every claimed pair exists, and every versioned pair file has a row.

    With `editions` (an edition-axis inventory, see the module docstring) a
    release has one pair per listed edition, and the claim is uniform: a
    claimed row needs ALL of them. Without it this is the original check.
    """
    prefix = PAIR_PREFIX.get(directory.name, directory.name)
    releases = {row["release"]: row for row in rows if not row.get("__malformed__")}
    for release, row in releases.items():
        if row["grammar_stage"] == "none" and row["grammar_status"] == "pending":
            continue
        if not RELEASE_RE.match(release) or ".." in release:
            continue  # already an error; never build a path from it
        # A release with no editions (or any release of an ordinary
        # inventory) has the single plain pair.
        listed: tuple[str | None, ...] = tuple((editions or {}).get(release, ())) or (
            None,
        )
        for edition in listed:
            stem = pair_stem(prefix, release, edition)
            note = f" (edition {edition})" if edition else ""
            for suffix in (".tokens", ".grammar"):
                if not (directory / f"{stem}{suffix}").is_file():
                    report.error(
                        f"{release}: claims a stage but {stem}{suffix} is missing{note}"
                    )
    stems = {path.stem for path in directory.glob("*.tokens")}
    stems |= {path.stem for path in directory.glob("*.grammar")}
    for stem in sorted(stems):
        if stem == prefix or stem in legacy:
            continue  # the unversioned pair, or a declared legacy pair
        if not stem.startswith(prefix):
            report.error(f"pair file {stem}.* has no releases.csv row")
        elif editions is None:
            if stem[len(prefix) :] not in releases:
                report.error(f"pair file {stem}.* has no releases.csv row")
        else:
            check_edition_stem(report, stem, stem[len(prefix) :], releases, editions)
    for stem in sorted(legacy - stems):
        report.error(f"legacy pair {stem} is declared but has no files")


def check_edition_stem(
    report: Report,
    stem: str,
    rest: str,
    releases: dict[str, dict[str, str]],
    editions: dict[str, tuple[str, ...]],
) -> None:
    """One pair file of an edition-axis inventory (`rest` is the stem minus
    the language prefix): its release is a row, and its edition (or lack of
    one) is exactly what that row lists."""
    split = split_pair_stem(rest)
    if split is None or split[0] not in releases:
        report.error(f"pair file {stem}.* has no releases.csv row")
        return
    release, edition = split
    listed = editions.get(release, ())
    if edition is None and listed:
        report.error(
            f"pair file {stem}.* needs an edition ({release} lists {';'.join(listed)})"
        )
    elif edition is not None and edition not in listed:
        report.error(
            f"pair file {stem}.* names edition {edition}, which {release} does not list"
        )


# ---------------------------------------------------------------------------
# The edition axis (RS01)
# ---------------------------------------------------------------------------


def split_pair_stem(text: str) -> tuple[str, str | None] | None:
    """Split a pair stem (without the language prefix) from the RIGHT.

        "0.4"                  -> ("0.4", None)
        "1.85.0-e2021"         -> ("1.85.0", "2021")
        "1.0.0-alpha.2-e2015"  -> ("1.0.0-alpha.2", "2015")
        "1.85.0-e21"           -> ("1.85.0-e21", None)   (not an edition)
        "-e2021"               -> None                   (no release)

    Only a trailing `-e` plus exactly four digits is an edition. Splitting
    from the right is what lets a release ID contain hyphens (`1.0.0-beta`).
    """
    match = EDITION_STEM_RE.match(text)
    if match:
        release, edition = match.groups()
        return (release, edition) if release else None
    return (text, None) if text else None


def pair_stem(prefix: str, release: str, edition: str | None) -> str:
    """The file stem of one pair: `rust0.4` or `rust1.85.0-e2021`."""
    return f"{prefix}{release}" if edition is None else f"{prefix}{release}-e{edition}"


def parse_editions_header(
    comments: list[str],
) -> tuple[dict[str, str] | None, list[str]]:
    """The `# editions: 2015@1.0.0-alpha 2018@1.31.0 ...` header comment.

    Returns ({edition: first release}, errors), or (None, []) when the
    inventory has no such comment (it is then not an edition-axis inventory).
    Editions must be listed ascending, each once.
    """
    lines = [m for c in comments if (m := EDITIONS_HEADER_RE.match(c))]
    if not lines:
        return None, []
    errors: list[str] = []
    if len(lines) > 1:
        errors.append("more than one `# editions:` header comment")
    firsts: dict[str, str] = {}
    entries = lines[0].group(1).split()
    if not entries:
        errors.append("`# editions:` declares no edition")
    previous = ""
    for entry in entries:
        match = EDITION_FIRST_RE.match(entry)
        if not match:
            errors.append(
                f"`# editions:` entry {printable(repr(entry))} must look like "
                "2015@<release>"
            )
            continue
        edition, first = match.groups()
        if edition in firsts:
            errors.append(f"`# editions:` edition {edition} is declared twice")
        elif edition < previous:
            errors.append("`# editions:` editions must be listed ascending")
        firsts.setdefault(edition, first)
        previous = max(previous, edition)
    return firsts, errors


def derive_editions(
    rows: list[dict[str, str]], firsts: dict[str, str]
) -> dict[str, tuple[str, ...]]:
    """Each release's editions, derived along `predecessor` lineage.

    A row has its predecessor's editions plus every edition that starts at
    it. Never by date or by comparing release IDs: a patch to an old line
    released after a newer minor keeps its own line's editions.
    """
    derived: dict[str, tuple[str, ...]] = {}
    for row in rows:
        if row.get("__malformed__"):
            continue
        editions = set(derived.get(row["predecessor"], ()))
        editions |= {
            edition for edition, first in firsts.items() if first == row["release"]
        }
        derived[row["release"]] = tuple(sorted(editions))
    return derived


def count_pairs(derived: dict[str, tuple[str, ...]]) -> int:
    """Pairs in the inventory: one per edition, one for an editionless release."""
    return sum(max(1, len(editions)) for editions in derived.values())


def check_editions(
    report: Report,
    comments: list[str],
    header: list[str],
    rows: list[dict[str, str]],
) -> dict[str, tuple[str, ...]] | None:
    """Validate the edition axis; return the derived editions per release, or
    None when the inventory has no edition axis (or its declaration is broken
    beyond use)."""
    has_column = "editions" in header
    firsts, problems = parse_editions_header(comments)
    if firsts is None and not has_column:
        return None
    if firsts is None:
        report.error(
            "the `editions` column needs a `# editions:` header comment "
            "declaring each edition's first release"
        )
        return None
    if not has_column:
        report.error("has an `# editions:` header but no `editions` column")
        return None
    for problem in problems:
        report.error(problem)
    known = {row["release"] for row in rows if not row.get("__malformed__")}
    for edition, first in sorted(firsts.items()):
        if first not in known:
            report.error(
                f"`# editions:` edition {edition}: first release {first!r} is "
                "not a releases.csv row"
            )
    derived = derive_editions(rows, firsts)
    for row in rows:
        if row.get("__malformed__"):
            continue
        release = row["release"]
        where = f"line {row['__line__']} ({release})"
        if EDITION_STEM_RE.match(release):
            report.error(
                f"{where}: release ID must not end in -e<edition>; a pair stem "
                "is split from the right, so the split would be ambiguous"
            )
        cell = row["editions"]
        parts = cell.split(";")
        ordered = all(a < b for a, b in itertools.pairwise(parts))
        if cell != "-" and not (all(EDITION_RE.match(p) for p in parts) and ordered):
            report.error(
                f"{where}: editions {printable(repr(cell))} must be '-' or "
                "';'-separated ascending four-digit editions"
            )
            continue
        expected = ";".join(derived[release]) or "-"
        if cell != expected:
            report.error(
                f"{where}: editions {cell!r} but lineage derives {expected!r} "
                "(an edition is inherited from the predecessor and starts at "
                "its `# editions:` first release)"
            )
    report.notes.append(
        f"editions: {len(derived)} releases, {count_pairs(derived)} pairs"
    )
    return derived


# ---------------------------------------------------------------------------
# Rust's release rules (RS01 section 1)
# ---------------------------------------------------------------------------


def rust_expected(release: str) -> tuple[str, str, str] | None:
    """(channel, predecessor, lineage) RS01 prescribes for a published Rust
    release, or None when `release` is not a published release ID.

    1.x minor X.Y.0     predecessor 1.(Y-1).0           branch-point
    1.x patch X.Y.1     predecessor X.Y.0               line
    1.x patch X.Y.z     predecessor X.Y.(z-1)           line
    1.0.0               predecessor 1.0.0-beta          line
    1.0.0-alpha         predecessor 0.12.0              branch-point
    0.3.1               predecessor 0.3                 line
    0.4                 predecessor 0.3 (not 0.3.1)     branch-point
    every other 0.N     the previous final              branch-point
    """
    if release in RUST_PRE_1_0:
        finals = [r for r in RUST_PRE_1_0 if r != "0.3.1"]
        if release == "0.1":
            return "final", "", "first"
        if release == "0.3.1":
            return "patch", "0.3", "line"
        return "final", finals[finals.index(release) - 1], "branch-point"
    if release in RUST_PRE_RELEASES:
        channel = "beta" if release.endswith("beta") else "alpha"
        if release == "1.0.0-alpha":
            return channel, "0.12.0", "branch-point"
        return channel, RUST_PRE_RELEASES[RUST_PRE_RELEASES.index(release) - 1], "line"
    match = RUST_1X_RE.match(release)
    if not match:
        return None
    minor, patch = int(match.group(1)), int(match.group(2))
    if patch == 0:
        if minor == 0:
            return "final", "1.0.0-beta", "line"
        return "final", f"1.{minor - 1}.0", "branch-point"
    return "patch", f"1.{minor}.{patch - 1 if patch > 1 else 0}", "line"


def check_rust(
    report: Report, header: list[str], rows: list[dict[str, str]], axis: bool
) -> None:
    """RS01 section 1: published releases only, and their prescribed channel,
    predecessor and lineage."""
    if not axis:
        report.error(
            "the Rust inventory declares an edition axis: an `editions` column "
            "and a `# editions:` header comment (RS01 section 2.3)"
        )
    if "lineage" not in header:
        report.error("the Rust inventory needs a `lineage` column (RS01 section 1.2)")
        return
    seen: dict[tuple[str, str], str] = {}
    for row in rows:
        if row.get("__malformed__"):
            continue
        release = row["release"]
        where = f"line {row['__line__']} ({release})"
        expected = rust_expected(release)
        if expected is None:
            report.error(
                f"{where}: not a published Rust release ID (0.1 ... 0.12.0, "
                "1.0.0-alpha, 1.0.0-alpha.2, 1.0.0-beta or 1.Y.Z; nightly, "
                "beta-channel and release-candidate versions are out of scope)"
            )
            continue
        channel, predecessor, lineage = expected
        if row["channel"] != channel:
            report.error(f"{where}: channel {row['channel']!r} must be {channel!r}")
        if row["predecessor"] != predecessor:
            report.error(
                f"{where}: predecessor {row['predecessor']!r} but RS01 lineage "
                f"says {predecessor!r}"
            )
        if row["lineage"] != lineage:
            report.error(
                f"{where}: lineage {printable(repr(row['lineage']))} but RS01 "
                f"lineage says {lineage!r}"
            )
        key = (row["predecessor"], row["lineage"])
        if row["predecessor"] and key in seen:
            report.error(
                f"{where}: same predecessor and lineage as {seen[key]} "
                f"({row['predecessor']!r}, {row['lineage']!r})"
            )
        seen.setdefault(key, release)


def check_allowlists(
    report: Report,
    directory: Path,
    rows: list[dict[str, str]],
    subsets: dict[str, dict[str, str]],
) -> None:
    """A claimed subset's gate file exists, and is a plain file.

    Declared-but-unclaimed gates may be absent: the declaration comes first,
    the gate with the subset's first claim.
    """
    claimed: set[str] = set()
    for row in rows:
        if row.get("__malformed__"):
            continue
        claimed |= split_list(row["iir_status"]) | split_list(row["backend_status"])
        if row["grammar_status"] == "partial":
            claimed.add(row["grammar_subset"])
    for name, declared in sorted(subsets.items()):
        cell = declared.get("allowlist")
        if name not in claimed or not cell or cell == "-":
            continue
        if not ALLOWLIST_RE.match(cell):
            continue  # already an error from read_subsets
        where = f"subsets.csv line {declared['__line__']} ({name})"
        path = directory / cell
        if path.is_symlink():
            report.error(f"{where}: allowlist {cell!r} is a symlink")
        elif not path.is_file():
            report.error(f"{where}: allowlist {cell!r} does not exist")


# ---------------------------------------------------------------------------
# --matrix
# ---------------------------------------------------------------------------

# `lang_matrix.rs` is read without a Rust parser, so the reader below is
# written to be LINEAR in the size of the file, whatever the file holds. A
# regex such as r'r(#*)".*?"\1' rescans to the end of the input from every
# unterminated `r#"`, and a brace scan started at every `Prog {` rescans
# the rest of the file for each one: both are quadratic on hostile input.
#
#   1. mask_rust_literals  one left-to-right pass; every string and char
#                          literal becomes a placeholder `$S<n>$`
#   2. top-level bodies    one pass over the braces with a stack; each
#                          character lands in exactly one frame's buffer
#   3. field regexes       run on those buffers, whose total size is the
#                          file size


def _is_ident(char: str) -> bool:
    return char.isalnum() or char == "_"


def mask_rust_literals(source: str) -> tuple[str, list[str]]:
    """Replace each Rust string/char literal with `$S<n>$`, linearly.

    Handles "..." with escapes, raw r"..." and r#"..."# (also after `b`),
    and char literals such as '"' or '\\''. A lifetime ('static) is not a
    literal. An unterminated string swallows the rest of the file, which is
    what rustc would report too. Each placeholder keeps the literal's
    newlines so that line numbers stay true.
    """
    out: list[str] = []
    strings: list[str] = []
    i, n = 0, len(source)

    def stash(start: int, end: int) -> None:
        literal = source[start:end]
        strings.append(literal)
        out.append(f"\x00S{len(strings) - 1}\x00" + "\n" * literal.count("\n"))

    while i < n:
        char = source[i]
        prev = source[i - 1] if i else ""
        if char == "r" and not (_is_ident(prev) and prev != "b"):
            j = i + 1
            while j < n and source[j] == "#":
                j += 1
            if j < n and source[j] == '"':
                close = source.find('"' + "#" * (j - i - 1), j + 1)
                end = n if close < 0 else close + 1 + (j - i - 1)
                stash(i, end)
                i = end
                continue
        if char == '"':
            j = i + 1
            while j < n and source[j] != '"':
                j += 2 if source[j] == "\\" else 1
            end = min(j + 1, n)
            stash(i, end)
            i = end
            continue
        if char == "'":
            if source.startswith("\\", i + 1) and source.startswith("'", i + 3):
                stash(i, i + 4)
                i += 4
                continue
            if i + 2 < n and source[i + 1] not in "\\\n'" and source[i + 2] == "'":
                stash(i, i + 3)
                i += 3
                continue
        out.append(char)
        i += 1
    return "".join(out), strings


def rust_string_value(literal: str) -> str | None:
    """The text of a plain or raw Rust string literal without escapes.

    Release IDs and case names never need escapes, so a literal that has one
    (or a char literal, or an unterminated string) is reported as unreadable
    (None) instead of being half-decoded.
    """
    match = re.fullmatch(r'b?r(#*)"(.*)"\1|b?"([^"\\]*)"', literal, re.DOTALL)
    if not match:
        return None
    return match.group(2) if match.group(2) is not None else match.group(3)


def matrix_rows(source: str) -> list[dict[str, str]]:
    """`Prog { .. }` literals of `lang_matrix.rs` that name a release or case.

    String literals are masked before the field regexes run, so text inside
    a program's `src` (which may itself contain `{`, `}` or `release:`) can
    never be mistaken for a field. Only a Prog's top-level fields count: a
    nested `{ .. }` such as a struct-variant `expect` is skipped.
    """
    # The placeholders are NUL-delimited, so no text in the file can forge
    # one: any NUL already in the source is replaced first.
    text, strings = mask_rust_literals(source.replace("\x00", "\ufffd"))
    # Line comments may hold example rows; drop them now strings are masked.
    text = re.sub(r"//[^\n]*", "", text)
    opens = {match.end() - 1 for match in re.finditer(r"\bProg\s*\{", text)}
    newlines = [i for i, char in enumerate(text) if char == "\n"]

    bodies: list[tuple[int, str]] = []
    stack: list[tuple[int, list[str]]] = []
    for index, char in enumerate(text):
        if char == "{":
            stack.append((index, []))
        elif char == "}" and stack:
            start, buffer = stack.pop()
            if start in opens:
                bodies.append((start, "".join(buffer)))
        elif stack:
            stack[-1][1].append(char)

    rows = []
    for start, body in sorted(bodies):
        language = re.search(r"\blang\s*:\s*Language::(\w+)", body)
        if not language:
            continue  # `struct Prog { .. }` itself
        line = bisect.bisect_left(newlines, start) + 1
        row = {"lang": language.group(1), "line": str(line)}
        for name in ("release", "case"):
            found = re.search(rf"\b{name}\s*:\s*Some\(\s*\x00S(\d+)\x00\s*\)", body)
            if found:
                value = rust_string_value(strings[int(found.group(1))])
                row[name] = value if value is not None else "\\"
        rows.append(row)
    return rows


def check_matrix(
    report: Report,
    directory: Path,
    rows: list[dict[str, str]],
    cases: dict[str, set[str]],
    matrix: list[dict[str, str]],
) -> None:
    """Every B(S) claim is covered case by case (see the module docstring)."""
    order = {
        row["release"]: i for i, row in enumerate(rows) if not row.get("__malformed__")
    }
    language = [
        lang for lang, name in MATRIX_LANGUAGES.items() if name == directory.name
    ]
    mine = [row for row in matrix if row["lang"] in language]

    # Matrix rows that name a release/case must name real ones.
    ran: set[tuple[str, str, str]] = set()
    for row in mine:
        where = f"lang_matrix.rs line {row['line']}"
        if "release" not in row:
            # LANG84 "CI bounding": `release` is required (`Some`) for every
            # language with an inventory. 2b only noted such rows, until 4c
            # gave the 11 Perl rows the field; from 4c on it is an error.
            report.error(
                f"{where}: a Language::{row['lang']} row names no release "
                "(LANG84 requires one for a language with releases.csv)"
            )
            continue
        release, case = row["release"], row.get("case")
        if release not in order:
            report.error(f"{where}: release {release!r} is not a releases.csv row")
            continue
        if case is None:
            continue
        subset, slash, name = case.partition("/")
        if not slash or subset not in cases or name not in cases[subset]:
            report.error(
                f"{where}: case {case!r} is not a case of a claimed subset's corpus"
            )
            continue
        ran.add((release, subset, name))

    classes = read_classes(report, directory, order, cases)
    representative: dict[tuple[str, str, str], str] = {}
    for (subset, name, release), digest in sorted(
        classes.items(), key=lambda item: order[item[0][2]]
    ):
        representative.setdefault((subset, name, digest), release)

    claims = direct = by_class = skipped = 0
    for row in rows:
        if row.get("__malformed__"):
            continue
        release = row["release"]
        for subset in sorted(split_list(row["backend_status"])):
            claims += 1
            for name in sorted(cases.get(subset, set())):
                digest = classes.get((subset, name, release))
                if digest == "rejected":
                    skipped += 1
                elif (release, subset, name) in ran:
                    direct += 1
                elif (
                    digest is not None
                    and (representative[(subset, name, digest)], subset, name) in ran
                ):
                    by_class += 1
                else:
                    report.error(
                        f"{release}: claims B({subset}) but case {subset}/{name} has "
                        "no lang_matrix row on this release or on its IIR class "
                        "representative"
                    )
    report.notes.append(
        f"matrix: {claims} backend claim(s); case coverage {direct} direct, "
        f"{by_class} by IIR class, {skipped} not applicable"
    )


def read_classes(
    report: Report,
    directory: Path,
    order: dict[str, int],
    cases: dict[str, set[str]],
) -> dict[tuple[str, str, str], str]:
    """`release_classes.csv` as {(subset, case, release): class}."""
    path = directory / "release_classes.csv"
    if not path.is_file():
        return {}
    if not is_plain(path, directory.parent):
        report.error("release_classes.csv: must not be a symlink")
        return {}
    classes: dict[tuple[str, str, str], str] = {}
    for row in read_table(report, path, CLASS_COLUMNS) or []:
        where = f"release_classes.csv line {row['__line__']}"
        subset, name, release, digest = (row[c] for c in CLASS_COLUMNS)
        if subset not in cases or name not in cases[subset]:
            report.error(f"{where}: {subset}/{name} is not a case of a claimed subset")
        elif release not in order:
            report.error(f"{where}: release {release!r} is not a releases.csv row")
        elif not CLASS_RE.match(digest):
            report.error(f"{where}: class must be 12-64 lowercase hex or 'rejected'")
        elif (subset, name, release) in classes:
            report.error(f"{where}: duplicate entry for {release} {subset}/{name}")
        else:
            classes[(subset, name, release)] = digest
    return classes


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def check_inventory(path: Path, matrix: list[dict[str, str]] | None = None) -> Report:
    """Check one releases.csv; with `matrix`, also its backend coverage."""
    report = Report(path)
    comments, header, rows = read_inventory(path)
    report.rows = len(rows)
    missing = [column for column in REQUIRED if column not in header]
    directory = path.parent
    if missing:
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
    declared = read_subsets(report, directory)
    subsets = set(declared)
    check_rows(report, rows, subsets)
    editions = check_editions(report, comments, header, rows)
    if directory.name == "rust":
        check_rust(report, header, rows, editions is not None)
    check_allowlists(report, directory, rows, declared)
    cases = check_claimed_corpora(report, directory, rows, subsets, editions)
    check_pairs(report, directory, rows, legacy_pairs(comments), editions)
    if matrix is not None:
        check_matrix(report, directory, rows, cases, matrix)
    return report


# C0/C1 controls, DEL, and the bidi/zero-width format characters. A CSV
# cell is untrusted text that ends up in CI logs: a quoted cell holding a
# newline followed by `::error::` would otherwise start a GitHub Actions
# workflow command, and bidi controls could make a message read differently
# from what it says.
UNPRINTABLE_RE = re.compile(
    "[\x00-\x1f\x7f-\x9f\u061c\u200b-\u200f\u2028-\u202e\u2066-\u2069\ufeff]"
)


def printable(text: str) -> str:
    """`text` with every control or bidi character shown as `\\uXXXX`."""
    return UNPRINTABLE_RE.sub(lambda m: f"\\u{ord(m.group(0)):04x}", text)


def check(grammars: Path, matrix_source: Path | None = None) -> int:
    paths = sorted(grammars.glob("*/releases.csv"))
    if not paths:
        print(f"no releases.csv under {grammars}", file=sys.stderr)
        return 1
    failed = False
    orphans = sorted(
        path.relative_to(grammars)
        for path in grammars.glob("*/subsets.csv")
        if not (path.parent / "releases.csv").is_file()
    )
    for orphan in orphans:
        print(
            f"release_inventory: {printable(repr(str(orphan)))} has no "
            "releases.csv beside it",
            file=sys.stderr,
        )
        failed = True
    matrix = None
    if matrix_source is not None:
        matrix = matrix_rows(matrix_source.read_text(encoding="utf-8"))
    for path in paths:
        directory = path.parent.name
        if not DIRECTORY_RE.match(directory) or not is_plain(path, grammars):
            # Never echo such a name at the start of a line: a directory
            # called `::warning::x` would be read as a workflow command.
            print(
                "release_inventory: grammar directory "
                f"{printable(repr(directory))} is not [a-z0-9_-]+ or is a symlink",
                file=sys.stderr,
            )
            failed = True
            continue
        report = check_inventory(path, matrix)
        name = path.relative_to(grammars)
        for message in report.errors:
            print(f"{name}: {printable(message)}", file=sys.stderr)
        if report.errors:
            failed = True
        else:
            print(f"{name}: ok ({report.rows} rows)")
        for note in report.notes:
            print(f"{name}: {printable(note)}")
    return 1 if failed else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    check_parser = commands.add_parser("check", help="validate every releases.csv")
    check_parser.add_argument("grammars", nargs="?", type=Path, default=GRAMMARS)
    check_parser.add_argument(
        "--matrix",
        action="store_true",
        help="also check that every backend_status claim has lang_matrix coverage",
    )
    check_parser.add_argument(
        "--matrix-source",
        type=Path,
        default=LANG_MATRIX,
        help="the lang_matrix test source to read rows from (with --matrix)",
    )
    args = parser.parse_args(argv)
    if args.command == "check":
        return check(args.grammars, args.matrix_source if args.matrix else None)
    return 2  # pragma: no cover - argparse rejects unknown commands


if __name__ == "__main__":
    sys.exit(main())
