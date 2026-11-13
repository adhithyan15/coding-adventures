#!/usr/bin/env python3
"""Check and materialize the Rust release pairs of RS01 from `pair-evidence.csv`.

WHY THIS EXISTS
===============

RS01 (`code/specs/RS01-rust-grammars.md`) gives every published Rust release,
and every edition that release supports, its own `.tokens` and `.grammar`
file: 347 pairs today. LANG84 forbids aliases, so no pair may point at another
pair's files; every pair is a real file. Most pairs are nevertheless identical
to a neighbour (a 1.x patch changes no syntax; many minors change none). Typing
them by hand invites exactly the copy errors the evidence is meant to rule
out. So a pair that is a COPY is produced by this script and proven by its
`check` mode, and a pair that is NOT a copy is hand-written and reviewed.

The three bases (RS01 section 6.3, LANG84 "Amendment RS01" R4), one per row of
`code/grammars/rust/pair-evidence.csv`:

    reviewed            written or changed by hand from the tag's sources, as
                        a reviewed delta from its predecessor. The script never
                        writes it; `check` only verifies its evidence row and
                        that its header cites the right commit.
    copy:<pair>         every recorded syntax blob equals the source pair's.
                        The body is copied byte for byte.
    semantic-copy:<pair>
                        a 1.x minor whose parser source changed although its
                        syntax did not. It copies its predecessor's pair only
                        when ALL THREE conditions hold and are RECORDED:
                          1. accepted.rs: no syntax-changing feature stabilized
                             in this release (blob and features checked);
                          2. RELEASES.md: no grammar-visible "Language" entry
                             (blob and section);
                          3. the oracle sweep passed (toolchain SHA-256 and
                             result digest).
                        Miss one and the pair needs a `reviewed` delta.

A "pair" is named `<release>` before 1.0 and `<release>-e<edition>` from 1.0,
for example `0.4` or `1.85.0-e2021`. Files are `rust<pair>.tokens` and
`rust<pair>.grammar`.

WHAT A COPY IS, BYTE FOR BYTE
=============================

A pair file is a header (the leading block of `#` lines) and then a blank line
and then the BODY. RS01 section 2.2 says each header names its OWN release and
edition and cites its own evidence, so a copy's header cannot equal its
source's. What is copied byte for byte is the BODY: everything after the first
blank line, including line endings and any bytes that are not valid text. The
header is generated, from `releases.csv` (date, source_ref, source_sha) and
the evidence row, so it too is deterministic.

    <generated header for this pair>
    <blank line>
    <body bytes, identical to the source's>

USAGE
=====

    python3 code/scripts/rust_release_pairs.py check [DIR]   # verify, exit 1 on a violation
    python3 code/scripts/rust_release_pairs.py write [DIR]   # (re)write the copy files

`DIR` defaults to `code/grammars/rust`. `check` verifies, and reports every
violation it finds rather than only the first:

    * the evidence file's header and every cell are well formed;
    * every evidence pair is a real (release, edition) of `releases.csv`;
    * each row's commit is the inventory's `source_sha`;
    * `copy:` blobs equal the source's, `semantic-copy:` records all three
      conditions, `reviewed` names its blobs, its note and its commit;
    * a source exists, has an evidence row, and PRECEDES the target in lineage
      (the copy graph has no cycle);
    * the rows cover EXACTLY the pair files on disk (no orphan either way, no
      pair with only one of its two files);
    * every copy file equals what `write` would produce.

With no pair files and a header-only evidence file the state is empty, and
`check` passes: RS01 slice A4 writes the first pairs.

HOSTILE INPUT
=============

The evidence file, the inventory and the pair files are all attacker-writable
in a pull request, so:

    * A path is only ever built from a stem that matched a strict pattern, and
      is then re-checked to sit directly inside the grammar directory. A
      release ID such as `../x` never reaches the filesystem.
    * Every file is opened with `O_NOFOLLOW` and must be a regular file, so a
      symlink (even one swapped in after a check) is refused, never read
      through. `write` replaces a file by renaming a new one over it, so it
      never writes through a link either. The directory itself must not be a
      symlink.
    * Reads are size-capped. Cells are length-capped and may hold no control
      or bidi character; messages are printed through `printable`, so no
      message can forge a workflow command or reorder the log line.
    * Nothing is executed, no subprocess is started, no network is used.
    * Output is a pure function of the files: no clock, no environment, no
      absolute path, sorted where a set is listed.
"""

from __future__ import annotations

import argparse
import csv
import io
import os
import re
import stat
import sys
from dataclasses import dataclass, field
from pathlib import Path

import release_inventory as inventory

REPO = Path(__file__).resolve().parents[2]
RUST = REPO / "code" / "grammars" / "rust"

EVIDENCE_FILE = "pair-evidence.csv"
INVENTORY_FILE = "releases.csv"
PREFIX = "rust"
SUFFIXES = (".tokens", ".grammar")
KINDS = {".tokens": "tokens", ".grammar": "grammar"}

# `semantic_*` columns are the three recorded conditions of a semantic-copy.
COLUMNS = [
    "release",
    "edition",
    "commit",
    "syntax_blobs",
    "basis",
    "accepted_blob",
    "accepted_features",
    "releases_md_blob",
    "releases_md_section",
    "sweep_toolchain_sha256",
    "sweep_digest",
    "oracle",
    "note",
]
SEMANTIC_COLUMNS = [
    "accepted_blob",
    "accepted_features",
    "releases_md_blob",
    "releases_md_section",
    "sweep_toolchain_sha256",
    "sweep_digest",
]

# Caps: the whole real evidence file is a few hundred rows; anything near
# these limits is not a legitimate input.
MAX_EVIDENCE_BYTES = 4 * 1024 * 1024
MAX_INVENTORY_BYTES = 1024 * 1024
MAX_PAIR_BYTES = 16 * 1024 * 1024
MAX_ROWS = 4000
MAX_CELL = 2000

BLOB_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")
SHA256_RE = re.compile(r"^[0-9a-f]{64}\Z")
BLOB_PATH_RE = re.compile(r"^[A-Za-z0-9_][A-Za-z0-9_./-]*\Z")
FEATURE_RE = re.compile(r"^[a-z0-9_]+\Z")
ORACLE_RE = re.compile(r"^[A-Za-z0-9._:/@+=-]+\Z")
EDITION_RE = re.compile(r"^[0-9]{4}\Z")
MINOR_RE = re.compile(r"^1\.(?:0|[1-9][0-9]{0,2})\.0\Z")
# Control, DEL, C1 and bidi/zero-width characters: never legal in a cell, and
# `printable` keeps them out of messages for the cases that slip through.
CONTROL_RE = inventory.UNPRINTABLE_RE

printable = inventory.printable


# ---------------------------------------------------------------------------
# Safe file access
# ---------------------------------------------------------------------------


def read_plain(path: Path, limit: int) -> tuple[bytes | None, str | None]:
    """Read a regular file without ever following a symlink.

    Returns (bytes, None) or (None, reason). `O_NOFOLLOW` makes the open itself
    refuse a link, so there is no window between "is it a link?" and "read
    it". `O_NONBLOCK` keeps a FIFO from hanging the check; the `fstat` then
    rejects it as not a regular file.
    """
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    try:
        fd = os.open(path, flags)
    except OSError as error:
        if path.is_symlink():
            return None, "is a symlink"
        return None, f"cannot be opened ({error.strerror or 'error'})"
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode):
            return None, "is not a regular file"
        if info.st_size > limit:
            return None, f"is larger than {limit} bytes"
        with os.fdopen(fd, "rb", closefd=False) as handle:
            data = handle.read(limit + 1)
    finally:
        os.close(fd)
    if len(data) > limit:
        return None, f"is larger than {limit} bytes"
    return data, None


def write_plain(path: Path, data: bytes) -> None:
    """Replace `path` with `data`, never writing through a symlink.

    A temporary sibling is created exclusively (and refuses a link), then
    renamed over the target; `rename` replaces a link itself rather than
    following it. The caller has already refused an existing symlink target.
    """
    temporary = path.with_name(f".{path.name}.tmp")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
    try:
        fd = os.open(temporary, flags, 0o644)
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
        os.replace(temporary, path)
    finally:
        if temporary.exists() or temporary.is_symlink():
            temporary.unlink()


def pair_path(directory: Path, stem: str, suffix: str) -> Path:
    """`directory/stem+suffix`, proven to be a direct child of `directory`.

    Callers only pass stems built from validated release IDs and editions;
    this is the second lock on the same door.
    """
    name = f"{stem}{suffix}"
    if "/" in name or "\\" in name or name in {".", ".."} or "\x00" in name:
        raise ValueError(f"unsafe file name {printable(repr(name))}")
    path = directory / name
    if path.parent != directory:
        raise ValueError(f"unsafe file name {printable(repr(name))}")
    return path


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------

PairKey = tuple[str, "str | None"]  # (release, edition or None before 1.0)


def pair_id(key: PairKey) -> str:
    """`0.4` or `1.85.0-e2021`: a pair's name without the `rust` prefix."""
    release, edition = key
    return inventory.pair_stem("", release, edition)


@dataclass
class Row:
    """One validated line of `pair-evidence.csv`."""

    line: int
    key: PairKey
    cells: dict[str, str]
    source: PairKey | None = None  # the pair a copy copies, when it is one
    blobs: dict[str, str] = field(default_factory=dict)

    @property
    def basis(self) -> str:
        return self.cells["basis"]

    @property
    def kind(self) -> str:
        """`reviewed`, `copy` or `semantic-copy`."""
        return self.basis.split(":", 1)[0]

    @property
    def stem(self) -> str:
        return f"{PREFIX}{pair_id(self.key)}"


@dataclass
class Inventory:
    """What the tool needs from `releases.csv`."""

    order: dict[str, int]
    rows: dict[str, dict[str, str]]
    editions: dict[str, tuple[str, ...]]
    ancestors: dict[str, list[str]]


class Errors(list):
    """An ordered list of error strings with a one-line adder."""

    def add(self, message: str) -> None:
        self.append(printable(message))


# ---------------------------------------------------------------------------
# Reading the inventory
# ---------------------------------------------------------------------------


def load_inventory(directory: Path, errors: Errors) -> Inventory | None:
    """The slice of `releases.csv` this tool uses; reuses `release_inventory`.

    The stem parser, the editions header and the edition derivation are
    `release_inventory`'s, not copies: one definition of "what is a Rust pair"
    for both checkers. `release_inventory.py check` validates the inventory
    itself; this only needs it readable.
    """
    path = directory / INVENTORY_FILE
    data, reason = read_plain(path, MAX_INVENTORY_BYTES)
    if data is None:
        errors.add(f"{INVENTORY_FILE} {reason}")
        return None
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        errors.add(f"{INVENTORY_FILE} is not valid UTF-8")
        return None
    # `read_inventory` reads a path; hand it the vetted bytes through a
    # temporary stream instead of reopening the (possibly swapped) file.
    comments, header, rows = _parse_inventory(text)
    if "release" not in header or "predecessor" not in header:
        errors.add(f"{INVENTORY_FILE} header lacks release or predecessor")
        return None
    firsts, header_errors = inventory.parse_editions_header(comments)
    for message in header_errors:
        errors.add(f"{INVENTORY_FILE}: {message}")
    good = [row for row in rows if not row.get("__malformed__")]
    derived = inventory.derive_editions(good, firsts or {})
    order = {}
    by_release = {}
    for row in good:
        release = row["release"]
        if release in order:
            errors.add(f"{INVENTORY_FILE}: duplicate release {release}")
            continue
        order[release] = len(order)
        by_release[release] = row
    ancestors = {}
    for release in by_release:
        chain: list[str] = []
        seen = {release}
        current = by_release[release]["predecessor"]
        while current:
            if current in seen:
                errors.add(f"{INVENTORY_FILE}: predecessor cycle through {release}")
                break
            if current not in by_release:
                break
            seen.add(current)
            chain.append(current)
            current = by_release[current]["predecessor"]
        ancestors[release] = chain
    return Inventory(order, by_release, derived, ancestors)


def _parse_inventory(text: str) -> tuple[list[str], list[str], list[dict[str, str]]]:
    """`release_inventory.read_inventory`'s parse, over text already in hand."""
    comments: list[str] = []
    body: list[str] = []
    for line in text.splitlines(keepends=True):
        if not body and line.startswith("#"):
            comments.append(line.rstrip("\r\n"))
        else:
            body.append(line)
    reader = csv.reader(body)
    header = next(reader, [])
    rows = []
    for cells in reader:
        if len(cells) != len(header):
            rows.append({"__malformed__": "1"})
        else:
            rows.append(dict(zip(header, cells)))
    return comments, header, rows


# ---------------------------------------------------------------------------
# Reading and validating the evidence
# ---------------------------------------------------------------------------


def parse_pair(text: str, inv: Inventory) -> tuple[PairKey | None, str | None]:
    """A pair ID (`0.4`, `1.85.0-e2021`) as a key, or (None, why not).

    The release must match `release_inventory`'s release pattern and contain no
    `..`, be a row of the inventory, and carry exactly an edition its row lists
    (and none when it lists none).
    """
    split = inventory.split_pair_stem(text)
    if split is None:
        return None, "is not a pair ID"
    release, edition = split
    if not inventory.RELEASE_RE.match(release) or ".." in release:
        return None, "is not a release ID"
    if release not in inv.rows:
        return None, f"release {release} is not in {INVENTORY_FILE}"
    listed = inv.editions.get(release, ())
    if edition is None and listed:
        return None, f"needs an edition ({release} lists {';'.join(listed)})"
    if edition is not None and not listed:
        return None, f"names edition {edition}, but {release} has no editions"
    if edition is not None and edition not in listed:
        return None, f"names edition {edition}, which {release} does not list"
    return (release, edition), None


def read_evidence(directory: Path, inv: Inventory, errors: Errors) -> list[Row] | None:
    """Validated rows of `pair-evidence.csv`, or None when it is unreadable."""
    data, reason = read_plain(directory / EVIDENCE_FILE, MAX_EVIDENCE_BYTES)
    if data is None:
        errors.add(f"{EVIDENCE_FILE} {reason}")
        return None
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        errors.add(f"{EVIDENCE_FILE} is not valid UTF-8")
        return None
    lines = text.splitlines(keepends=True)
    index = 0
    while index < len(lines) and lines[index].startswith("#"):
        index += 1  # leading provenance comments, as in every LANG84 table
    try:
        reader = csv.reader(io.StringIO("".join(lines[index:])))
        table = list(reader)
    except csv.Error as error:
        errors.add(f"{EVIDENCE_FILE}: unreadable CSV ({error})")
        return None
    if not table or table[0] != COLUMNS:
        errors.add(f"{EVIDENCE_FILE}: header must be exactly {','.join(COLUMNS)}")
        return None
    if len(table) - 1 > MAX_ROWS:
        errors.add(f"{EVIDENCE_FILE}: more than {MAX_ROWS} rows")
        return None
    rows: list[Row] = []
    seen: set[PairKey] = set()
    for offset, cells in enumerate(table[1:], start=2 + index):
        where = f"{EVIDENCE_FILE} line {offset}"
        if len(cells) != len(COLUMNS):
            errors.add(f"{where}: {len(cells)} fields, expected {len(COLUMNS)}")
            continue
        mapping = dict(zip(COLUMNS, cells))
        if not check_cells(where, mapping, errors):
            continue
        row = validate_row(where, offset, mapping, inv, errors)
        if row is None:
            continue
        if row.key in seen:
            errors.add(f"{where}: duplicate row for {pair_id(row.key)}")
            continue
        seen.add(row.key)
        rows.append(row)
    return rows


def check_cells(where: str, cells: dict[str, str], errors: Errors) -> bool:
    """Length and character rules that hold for every cell of every row."""
    good = True
    for column, value in cells.items():
        if CONTROL_RE.search(value):
            errors.add(f"{where}: {column} holds a control character")
            good = False
        elif len(value) > MAX_CELL:
            errors.add(f"{where}: {column} is longer than {MAX_CELL} characters")
            good = False
    return good


def validate_row(
    where: str,
    line: int,
    cells: dict[str, str],
    inv: Inventory,
    errors: Errors,
) -> Row | None:
    """Everything decidable from one row and the inventory alone."""
    before = len(errors)
    edition_cell = cells["edition"]
    pair_text = cells["release"]
    if edition_cell != "-":
        if not EDITION_RE.match(edition_cell):
            errors.add(f"{where}: edition {edition_cell!r} is not four digits or -")
            return None
        pair_text = f"{pair_text}-e{edition_cell}"
    key, why = parse_pair(pair_text, inv)
    if key is None:
        errors.add(f"{where}: pair {pair_text!r} {why}")
        return None
    row = Row(line, key, cells)
    release = key[0]

    if cells["commit"] != inv.rows[release]["source_sha"]:
        errors.add(f"{where}: commit differs from releases.csv source_sha")
    if not BLOB_RE.match(cells["commit"]):
        errors.add(f"{where}: commit is not a 40- or 64-hex digest")
    row.blobs = parse_blobs(where, cells["syntax_blobs"], errors)
    if not cells["note"] or cells["note"] == "-":
        errors.add(f"{where}: note must say what changed, or why nothing did")
    if not cells["oracle"] or (
        cells["oracle"] != "-" and not ORACLE_RE.match(cells["oracle"])
    ):
        errors.add(f"{where}: oracle must be - or a record ID")

    basis = cells["basis"]
    if basis == "reviewed":
        pass
    elif basis.startswith(("copy:", "semantic-copy:")):
        _kind, _colon, source_text = basis.partition(":")
        source, why = parse_pair(source_text, inv)
        if source is None:
            errors.add(f"{where}: copy source {printable(repr(source_text))} {why}")
        else:
            row.source = source
    else:
        errors.add(
            f"{where}: basis {printable(repr(basis))} must be reviewed, "
            "copy:<pair> or semantic-copy:<pair>"
        )
        return None
    if row.kind == "semantic-copy":
        check_semantic(where, row, inv, errors)
    else:
        for column in SEMANTIC_COLUMNS:
            if cells[column] != "-":
                errors.add(
                    f"{where}: {column} is only a semantic-copy row's evidence; "
                    "use - here"
                )
    return row if len(errors) == before else None


def parse_blobs(where: str, cell: str, errors: Errors) -> dict[str, str]:
    """`path=blob;path=blob` as a dict; every entry must be well formed."""
    blobs: dict[str, str] = {}
    if not cell or cell == "-":
        errors.add(f"{where}: syntax_blobs names no syntax file blob")
        return blobs
    for entry in cell.split(";"):
        path, equals, blob = entry.partition("=")
        if (
            not equals
            or not BLOB_PATH_RE.match(path)
            or ".." in path.split("/")
            or path.endswith("/")
            or not (BLOB_RE.match(blob) or blob == "-")
        ):
            errors.add(
                f"{where}: syntax_blobs entry {printable(repr(entry))} must be "
                "<path>=<40- or 64-hex blob, or ->"
            )
        elif path in blobs:
            errors.add(f"{where}: syntax_blobs lists {path} twice")
        else:
            blobs[path] = blob
    return blobs


def check_semantic(where: str, row: Row, inv: Inventory, errors: Errors) -> None:
    """A semantic-copy records all three RS01 conditions, and is eligible.

    The six columns are the three conditions; each missing or malformed one is
    its own message so a reviewer sees exactly which evidence is absent.
    """
    cells = row.cells
    release, edition = row.key
    # Eligibility (RS01 section 6.3): a 1.x minor, copying its predecessor.
    if not MINOR_RE.match(release):
        errors.add(f"{where}: only a 1.x minor release (1.Y.0) may be a semantic-copy")
    if row.source is not None:
        predecessor = inv.rows[release]["predecessor"]
        if row.source[0] != predecessor:
            errors.add(
                f"{where}: a semantic-copy's source must be the direct "
                f"predecessor release ({predecessor or 'none'})"
            )
        elif row.source[1] != edition:
            errors.add(f"{where}: a semantic-copy's source must have the same edition")

    # Condition 1: accepted.rs.
    if not BLOB_RE.match(cells["accepted_blob"]):
        errors.add(f"{where}: semantic-copy lacks the accepted.rs blob (accepted_blob)")
    features = cells["accepted_features"]
    if not features or features == "-":
        errors.add(
            f"{where}: semantic-copy lacks the accepted features checked "
            "(accepted_features; `none` if none had since = this release)"
        )
    elif features != "none" and not all(
        FEATURE_RE.match(name) for name in features.split(";")
    ):
        errors.add(
            f"{where}: accepted_features must be none or ;-separated "
            "feature identifiers [a-z0-9_]+"
        )
    # Condition 2: RELEASES.md.
    if not BLOB_RE.match(cells["releases_md_blob"]):
        errors.add(
            f"{where}: semantic-copy lacks the RELEASES.md blob (releases_md_blob)"
        )
    if not cells["releases_md_section"] or cells["releases_md_section"] == "-":
        errors.add(
            f"{where}: semantic-copy lacks the RELEASES.md section "
            "(releases_md_section)"
        )
    # Condition 3: the oracle sweep.
    if not SHA256_RE.match(cells["sweep_toolchain_sha256"]):
        errors.add(
            f"{where}: semantic-copy lacks the sweep toolchain SHA-256 "
            "(sweep_toolchain_sha256, 64 hex)"
        )
    if not SHA256_RE.match(cells["sweep_digest"]):
        errors.add(
            f"{where}: semantic-copy lacks the sweep result digest "
            "(sweep_digest, 64 hex)"
        )


# ---------------------------------------------------------------------------
# Relations between rows
# ---------------------------------------------------------------------------


def inventory_position(key: PairKey, inv: Inventory) -> tuple[int, str]:
    return (inv.order[key[0]], key[1] or "")


def check_order(rows: list[Row], inv: Inventory, errors: Errors) -> None:
    """Rows are listed in inventory order (then edition), so output is stable."""
    previous: Row | None = None
    for row in rows:
        if previous is not None and inventory_position(
            row.key, inv
        ) < inventory_position(previous.key, inv):
            errors.add(
                f"{EVIDENCE_FILE} line {row.line}: row {pair_id(row.key)} is out "
                "of order (rows follow releases.csv, then edition)"
            )
        previous = row


def check_sources(rows: list[Row], inv: Inventory, errors: Errors) -> None:
    """Sources exist, precede their target in lineage, and form no cycle."""
    by_key = {row.key: row for row in rows}
    for row in rows:
        if row.source is None:
            continue
        where = f"{EVIDENCE_FILE} line {row.line}"
        source = row.source
        if source not in by_key:
            errors.add(f"{where}: copy source {pair_id(source)} has no evidence row")
            continue
        if source == row.key:
            continue  # reported as a cycle below
        if not precedes(source, row.key, inv):
            errors.add(
                f"{where}: copy source {pair_id(source)} does not precede "
                f"{pair_id(row.key)} in lineage"
            )
        elif row.kind == "copy" and by_key[source].blobs != row.blobs:
            errors.add(
                f"{where}: syntax blobs differ from {pair_id(source)}'s, so "
                f"{pair_id(row.key)} cannot be a copy (use reviewed, or "
                "semantic-copy with its evidence)"
            )
    report_cycles(rows, errors)


def precedes(source: PairKey, target: PairKey, inv: Inventory) -> bool:
    """Is `source` an earlier pair of the same lineage as `target`?

    Two shapes are allowed, both conservative:
      * a SIBLING: the same release, an earlier edition (a 1.31.0-e2018 pair
        may copy 1.31.0-e2015);
      * an ANCESTOR: a release on the target's `predecessor` chain, the SAME
        edition. Editions never change across a copy between releases, and an
        edition-less pre-1.0 pair cannot seed an edition pair.
    """
    src_release, src_edition = source
    release, edition = target
    if src_release == release:
        return src_edition is not None and edition is not None and src_edition < edition
    return src_release in inv.ancestors.get(release, []) and src_edition == edition


def report_cycles(rows: list[Row], errors: Errors) -> None:
    """Every cycle in the copy graph, each reported once (a self-copy is one)."""
    edges = {row.key: row.source for row in rows if row.source is not None}
    reported: set[frozenset[PairKey]] = set()
    for start in (row.key for row in rows):
        path: list[PairKey] = []
        current: PairKey | None = start
        while current is not None and current not in path:
            path.append(current)
            current = edges.get(current)
        if current is None:
            continue
        loop = path[path.index(current) :]
        members = frozenset(loop)
        if members in reported:
            continue
        reported.add(members)
        names = " -> ".join(pair_id(key) for key in [*loop, loop[0]])
        errors.add(f"{EVIDENCE_FILE}: copy cycle {names}")


# ---------------------------------------------------------------------------
# The pair files
# ---------------------------------------------------------------------------


def disk_stems(directory: Path, errors: Errors) -> set[str]:
    """Stems of every `rust*.tokens` / `rust*.grammar` entry in the directory.

    Symlinks and non-files are listed too (they are refused when read), so a
    link can never hide from the orphan check.
    """
    stems: set[str] = set()
    try:
        names = sorted(os.listdir(directory))
    except OSError as error:
        errors.add(f"cannot list the grammar directory ({error.strerror})")
        return stems
    for name in names:
        for suffix in SUFFIXES:
            if name.startswith(PREFIX) and name.endswith(suffix):
                stems.add(name[: -len(suffix)])
    return stems


def split_file(data: bytes) -> tuple[bytes, bytes] | None:
    """(header, body) of a pair file, or None when it has no header block.

    The header is the leading run of lines starting with `#`; it must be
    followed by a blank line. The body is everything after that blank line,
    untouched. The header keeps its trailing newline off, as the generator
    produces it.
    """
    position = 0
    while data.startswith(b"#", position):
        end = data.find(b"\n", position)
        if end == -1:
            return None
        position = end + 1
    if position == 0 or not data.startswith(b"\n", position):
        return None
    return data[: position - 1], data[position + 1 :]


def render_header(row: Row, inv: Inventory, suffix: str) -> bytes:
    """The header of a copy or semantic-copy pair file, deterministically."""
    release, edition = row.key
    inv_row = inv.rows[release]
    title = f"# Rust {release} ({inv_row.get('date', '?')})"
    if edition is not None:
        title += f", edition {edition}"
    blobs = "; ".join(f"{path}={blob}" for path, blob in row.blobs.items())
    lines = [
        f"{title}: {KINDS[suffix]}.",
        "# @version 1",
        f"# source_ref: {inv_row.get('source_ref', '-')}",
        f"# source_sha: {inv_row['source_sha']}",
        f"# evidence: {blobs}",
        f"# basis: {row.basis}",
    ]
    if row.kind == "semantic-copy":
        cells = row.cells
        accepted = f"accepted.rs blob {cells['accepted_blob']}"
        features = f"features checked: {cells['accepted_features']}"
        notes = f"RELEASES.md blob {cells['releases_md_blob']}"
        section = f"section: {cells['releases_md_section']}"
        sweep = f"toolchain sha256 {cells['sweep_toolchain_sha256']}"
        digest = f"result digest {cells['sweep_digest']}"
        lines += [
            "# semantic-copy conditions (RS01 section 6.3), all three recorded:",
            f"#   1. {accepted}; {features}",
            f"#   2. {notes}; {section}",
            f"#   3. oracle sweep: {sweep}; {digest}",
        ]
    # Header text comes from validated cells and the inventory, whose own
    # content was only vetted for control characters when it reached a row.
    return "\n".join(printable(line) for line in lines).encode("utf-8")


def read_pair_file(
    directory: Path, stem: str, suffix: str
) -> tuple[bytes | None, str | None]:
    return read_plain(pair_path(directory, stem, suffix), MAX_PAIR_BYTES)


def check_files(
    directory: Path, rows: list[Row], inv: Inventory, errors: Errors
) -> None:
    """Coverage, then the content of each copy and reviewed pair."""
    on_disk = disk_stems(directory, errors)
    expected = {row.stem: row for row in rows}
    names = set(os.listdir(directory))
    for stem in sorted(on_disk - set(expected)):
        for suffix in SUFFIXES:
            name = f"{stem}{suffix}"
            if name in names:
                errors.add(f"{name} has no {EVIDENCE_FILE} row")
    for row in rows:
        for suffix in SUFFIXES:
            if not os.path.lexists(pair_path(directory, row.stem, suffix)):
                errors.add(
                    f"{row.stem}{suffix} is missing "
                    f"({EVIDENCE_FILE} line {row.line} claims it)"
                )
    for row in rows:
        for suffix in SUFFIXES:
            if not os.path.lexists(pair_path(directory, row.stem, suffix)):
                continue
            data, reason = read_pair_file(directory, row.stem, suffix)
            name = f"{row.stem}{suffix}"
            if data is None:
                errors.add(f"{name} {reason}")
            elif row.kind == "reviewed":
                check_reviewed_file(name, data, row, errors)
            else:
                check_copy_file(directory, name, data, row, suffix, inv, errors)


def check_reviewed_file(name: str, data: bytes, row: Row, errors: Errors) -> None:
    """A hand-written pair: a header block that cites the commit and the basis."""
    parts = split_file(data)
    if parts is None:
        errors.add(f"{name} has no `#` header block followed by a blank line")
        return
    header = parts[0].decode("utf-8", "replace").split("\n")
    if f"# source_sha: {row.cells['commit']}" not in header:
        errors.add(f"{name} header lacks `# source_sha: {row.cells['commit']}`")
    if not any(line.startswith("# basis: reviewed") for line in header):
        errors.add(f"{name} header lacks `# basis: reviewed`")


def check_copy_file(
    directory: Path,
    name: str,
    data: bytes,
    row: Row,
    suffix: str,
    inv: Inventory,
    errors: Errors,
) -> None:
    """A copy equals the generated header plus its source's body, byte for byte."""
    assert row.source is not None
    expected = expected_copy(directory, row, suffix, inv)
    if isinstance(expected, str):
        errors.add(f"{name}: {expected}")
        return
    if data == expected:
        return
    mine = split_file(data)
    theirs = split_file(expected)
    source_name = f"{PREFIX}{pair_id(row.source)}{suffix}"
    if mine is None:
        errors.add(f"{name} has no `#` header block followed by a blank line")
    elif mine[1] != theirs[1]:  # type: ignore[index]
        errors.add(
            f"{name}: body differs from {source_name} (a {row.kind} must match "
            "its source byte for byte; run `write`)"
        )
    else:
        errors.add(f"{name}: header differs from the generated header (run `write`)")


def expected_copy(
    directory: Path, row: Row, suffix: str, inv: Inventory
) -> bytes | str:
    """The bytes `write` would produce, or a reason string when impossible."""
    assert row.source is not None
    source_name = f"{PREFIX}{pair_id(row.source)}{suffix}"
    data, reason = read_pair_file(directory, f"{PREFIX}{pair_id(row.source)}", suffix)
    if data is None:
        return f"source {source_name} {reason}"
    parts = split_file(data)
    if parts is None:
        return f"source {source_name} has no `#` header block and blank line"
    return render_header(row, inv, suffix) + b"\n\n" + parts[1]


# ---------------------------------------------------------------------------
# Entry points
# ---------------------------------------------------------------------------


def load(directory: Path, errors: Errors) -> tuple[Inventory, list[Row]] | None:
    """Inventory and evidence rows after every check that needs no pair file."""
    if directory.is_symlink():
        errors.add("the grammar directory is a symlink")
        return None
    if not directory.is_dir():
        errors.add("the grammar directory does not exist")
        return None
    inv = load_inventory(directory, errors)
    if inv is None:
        return None
    rows = read_evidence(directory, inv, errors)
    if rows is None:
        return None
    check_order(rows, inv, errors)
    check_sources(rows, inv, errors)
    return inv, rows


def check_directory(directory: Path) -> list[str]:
    """Every violation under `directory`; empty means the tree is clean."""
    errors = Errors()
    loaded = load(directory, errors)
    if loaded is not None:
        inv, rows = loaded
        check_files(directory, rows, inv, errors)
    return list(errors)


def write_directory(directory: Path) -> tuple[list[str], list[str]]:
    """(errors, files written). Writes nothing unless the evidence is valid.

    Rows are in lineage order, so a copy's source is already final on disk
    when the copy is made. Only `copy` and `semantic-copy` files are written;
    a file already holding the right bytes is left alone, so a second run
    writes nothing.
    """
    errors = Errors()
    loaded = load(directory, errors)
    if loaded is None or errors:
        return list(errors), []
    inv, rows = loaded
    plan: list[tuple[Row, str]] = [
        (row, suffix) for row in rows if row.kind != "reviewed" for suffix in SUFFIXES
    ]
    # Refuse a symlinked or non-file destination before writing anything.
    for row, suffix in plan:
        path = pair_path(directory, row.stem, suffix)
        if path.is_symlink() or (path.exists() and not path.is_file()):
            errors.add(f"{row.stem}{suffix} is a symlink or not a regular file")
    if errors:
        return list(errors), []
    wrote: list[str] = []
    for row, suffix in plan:
        expected = expected_copy(directory, row, suffix, inv)
        name = f"{row.stem}{suffix}"
        if isinstance(expected, str):
            errors.add(f"{name}: {expected}")
            continue
        path = pair_path(directory, row.stem, suffix)
        current, _reason = (
            read_plain(path, MAX_PAIR_BYTES) if path.exists() else (None, None)
        )
        if current != expected:
            write_plain(path, expected)
            wrote.append(name)
    return list(errors), sorted(wrote)


def report(errors: list[str]) -> None:
    for message in errors:
        print(f"rust_release_pairs: {message}", file=sys.stderr)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    for name, text in (
        ("check", "verify pair-evidence.csv against the pair files"),
        ("write", "write the copy and semantic-copy pair files"),
    ):
        sub = commands.add_parser(name, help=text)
        sub.add_argument("directory", nargs="?", type=Path, default=RUST)
    args = parser.parse_args(argv)
    if args.command == "check":
        errors = check_directory(args.directory)
        report(errors)
        if errors:
            return 1
        rows = _count_rows(args.directory)
        print(f"rust/{EVIDENCE_FILE}: ok ({rows} pairs)")
        return 0
    errors, wrote = write_directory(args.directory)
    report(errors)
    for name in wrote:
        print(f"wrote: {name}")
    return 1 if errors else 0


def _count_rows(directory: Path) -> int:
    """Number of evidence rows, for the success line (the file just passed)."""
    errors = Errors()
    loaded = load(directory, errors)
    return len(loaded[1]) if loaded else 0


if __name__ == "__main__":
    sys.exit(main())
