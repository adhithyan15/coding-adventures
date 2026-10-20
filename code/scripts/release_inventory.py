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
`<name>.reject`.

THE MATRIX CHECK (`--matrix`)
=============================

LANG84 "CI bounding" runs every release on every backend only for boundary
releases. `check --matrix` verifies the one claim the per-row checks cannot:
a release that claims B(S) is backed by backend runs. For every such release R
and every case C of S's corpus, one of these must hold:

    1. `release_classes.csv` marks C as `rejected` under R (the case does not
       apply: R's grammar does not parse it);
    2. a `lang_matrix` row runs C on R itself;
    3. `release_classes.csv` puts (C, R) in an IIR-identity class whose
       representative -- its first release in inventory order -- has a
       `lang_matrix` row for C.

`release_classes.csv` (`subset,case,release,class`) is written by the release
sweep (LANG84 phase 4c); `class` is the hex digest of the IIR that R produces
for C, or `rejected`. A `lang_matrix` row (`Prog { .. }` in
`code/packages/rust/lang-aot/tests/lang_matrix.rs`) counts when it names
`release: Some("<id>")` and `case: Some("<subset>/<name>")`. Rows naming an
unknown release, subset or case fail the check, so stale rows cannot linger.

USAGE
=====

    python3 code/scripts/release_inventory.py check [--matrix] [GRAMMARS_DIR]
        [--matrix-source lang_matrix.rs]
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

RELEASE_RE = re.compile(r"^[A-Za-z0-9._-]+$")
DATE_RE = re.compile(r"^(\d{4})(?:-(\d{2})(?:-(\d{2}))?)?(\?)?$")
SHA_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
LEGACY_PAIRS_RE = re.compile(r"^#\s*legacy-minor-line-pairs:\s*(.+)$")
SUBSET_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
SPEC_RE = re.compile(r"^[A-Z]+[0-9]+[A-Za-z0-9]*$")
CASE_RE = re.compile(r"^[A-Za-z0-9_-]+$")
CLASS_RE = re.compile(r"^(?:[0-9a-f]{12,64}|rejected)$")


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


def spec_exists(name: str) -> bool:
    return (SPECS / f"{name}.md").is_file() or any(SPECS.glob(f"{name}-*.md"))


def read_subsets(report: Report, directory: Path) -> dict[str, dict[str, str]]:
    """Declared subsets of one grammar directory, validated, by name."""
    path = directory / "subsets.csv"
    if not path.is_file():
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
        subsets[name] = row
    return subsets


def corpus_cases(report: Report, directory: Path, subset: str) -> set[str] | None:
    """Positive case names of a subset's corpus, or None when it has none.

    A case is `<name>.<ext>` + `<name>.expect` + `<name>.cite`; a negative
    probe is `<name>.<ext>` + `<name>.reject`. Overrides `<name>.expect@<p>`
    need the plain `.expect`. Only positive cases need backend coverage.
    """
    corpus = directory / "corpus" / subset
    if not corpus.is_dir():
        return None
    files = [path.name for path in corpus.iterdir() if path.is_file()]
    stems: dict[str, set[str]] = {}
    for name in files:
        stem, dot, suffix = name.partition(".")
        stems.setdefault(stem, set()).add(suffix if dot else "")
    cases: set[str] = set()
    for stem, suffixes in sorted(stems.items()):
        where = f"corpus/{subset}/{stem}"
        if not CASE_RE.match(stem):
            report.error(f"{where}: case name must match [A-Za-z0-9_-]+")
            continue
        sources = {
            s
            for s in suffixes
            if s not in {"expect", "cite", "reject"} and not s.startswith("expect@")
        }
        positive, negative = "expect" in suffixes, "reject" in suffixes
        if positive == negative:
            report.error(f"{where}: needs exactly one of .expect or .reject")
        if len(sources) != 1:
            report.error(
                f"{where}: needs exactly one source file, found {sorted(sources)}"
            )
        if positive and "cite" not in suffixes:
            report.error(f"{where}: .expect has no .cite oracle record")
        if any(s.startswith("expect@") for s in suffixes) and not positive:
            report.error(f"{where}: a profile override needs the plain .expect")
        if positive:
            cases.add(stem)
    return cases


# ---------------------------------------------------------------------------
# Rows
# ---------------------------------------------------------------------------


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
    if profile != "-" and not known_profile(profile):
        report.error(f"{where}: profile {profile!r} is not defined in any profiles.rs")


def check_claimed_corpora(
    report: Report, directory: Path, rows: list[dict[str, str]], subsets: set[str]
) -> dict[str, set[str]]:
    """Stages I and B are measured on a corpus, so a claimed subset needs one.

    Returns the positive cases of every claimed subset (for `--matrix`).
    """
    claimed: set[str] = set()
    for row in rows:
        if not row.get("__malformed__"):
            claimed |= split_list(row["iir_status"]) | split_list(row["backend_status"])
    cases: dict[str, set[str]] = {}
    for subset in sorted(claimed & subsets):
        found = corpus_cases(report, directory, subset)
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


# ---------------------------------------------------------------------------
# --matrix
# ---------------------------------------------------------------------------

# A Rust string literal: "..." with escapes, or a raw r"..." / r#"..."#.
# Character literals ('"', '\'') are consumed too, so a quote inside one
# cannot open a phantom string; a lifetime ('static) matches neither.
RUST_STRING_RE = re.compile(
    r'r(#*)".*?"\1|"(?:[^"\\]|\\.)*"|\'(?:[^\'\\\n]|\\.)\'', re.DOTALL
)


def rust_string_value(literal: str) -> str | None:
    """The text of a plain or raw Rust string literal without escapes.

    Release IDs and case names never need escapes, so a literal that has one
    is reported as unreadable (None) instead of being half-decoded.
    """
    if literal.startswith("'"):
        return None
    if literal.startswith("r"):
        return literal.strip("r#")[1:-1]
    inner = literal[1:-1]
    return None if "\\" in inner else inner


def matrix_rows(source: str) -> list[dict[str, str]]:
    """`Prog { .. }` literals of `lang_matrix.rs` that name a release or case.

    String literals are swapped for placeholders before the field regexes
    run, so text inside a program's `src` (which may itself contain `{`, `}`
    or `release:`) can never be mistaken for a field.
    """
    strings: list[str] = []

    def stash(match: re.Match[str]) -> str:
        strings.append(match.group(0))
        # Keep the literal's newlines so line numbers stay true.
        return f"$S{len(strings) - 1}$" + "\n" * match.group(0).count("\n")

    text = RUST_STRING_RE.sub(stash, source)
    # Line comments may hold example rows; drop them after strings are safe.
    text = re.sub(r"//[^\n]*", "", text)
    rows = []
    for match in re.finditer(r"\bProg\s*\{", text):
        # Balanced scan to the matching `}`: a field such as a struct-variant
        # `expect: Expect::X { .. }` may nest braces of its own.
        depth, end = 1, match.end()
        while depth and end < len(text):
            depth += {"{": 1, "}": -1}.get(text[end], 0)
            end += 1
        body = text[match.end() : end - 1]
        # Only top-level fields count: blank out nested `{ .. }` groups.
        while re.search(r"\{[^{}]*\}", body):
            body = re.sub(r"\{[^{}]*\}", "", body)
        language = re.search(r"\blang\s*:\s*Language::(\w+)", body)
        if not language:
            continue  # `struct Prog { .. }` itself
        line = text.count("\n", 0, match.start()) + 1
        row = {"lang": language.group(1), "line": str(line)}
        for name in ("release", "case"):
            found = re.search(rf"\b{name}\s*:\s*Some\(\s*\$S(\d+)\$\s*\)", body)
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
    undeclared = 0
    for row in mine:
        where = f"lang_matrix.rs line {row['line']}"
        if "release" not in row:
            undeclared += 1
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
    if undeclared:
        report.notes.append(
            f"matrix: {undeclared} lang_matrix row(s) name no release "
            "(LANG84 requires one; Prog gains the field in phase 4c)"
        )

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
    subsets = set(read_subsets(report, directory))
    check_rows(report, rows, subsets)
    cases = check_claimed_corpora(report, directory, rows, subsets)
    check_pairs(report, directory, rows, legacy_pairs(comments))
    if matrix is not None:
        check_matrix(report, directory, rows, cases, matrix)
    return report


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
        print(f"{orphan}: has no releases.csv beside it", file=sys.stderr)
        failed = True
    matrix = None
    if matrix_source is not None:
        matrix = matrix_rows(matrix_source.read_text(encoding="utf-8"))
    for path in paths:
        report = check_inventory(path, matrix)
        name = path.relative_to(grammars)
        for message in report.errors:
            print(f"{name}: {message}", file=sys.stderr)
        if report.errors:
            failed = True
        else:
            print(f"{name}: ok ({report.rows} rows)")
        for note in report.notes:
            print(f"{name}: {note}")
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
