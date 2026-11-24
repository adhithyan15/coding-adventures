#!/usr/bin/env python3
"""Drift guard for the cross-lane "basic gates" Warrant document (LG02 section 4).

Why this exists
===============

``code/specs/logic-gates-basic.warrant`` (module ``logic_gates_basic``) is a
second Warrant document that holds ONLY the eleven scalar gates of
``code/specs/logic-gates.warrant``.  The assurance tooling wants one document
per package, so the Rust package keeps ``logic-gates.warrant`` and the other
language lanes replay the smaller basic document.  The price is that the basic
document is a *copy*.  A copy can silently drift: somebody edits a claim in one
file and forgets the other, and fifteen language lanes then prove a contract
the Rust package no longer has.  This script is the mechanical check that
fails the build when that happens.

Running it
==========

    python3 code/scripts/logic_gates_basic_drift.py

It takes NO arguments and reads NO environment variable.  The repository root
is derived from this file's own ``realpath``.  Exit 0 means every check below
passed; exit 1 means drift (one precise message on stderr, naming the first
difference only, never a whole file); exit 2 means the command line was misused.

The inputs, and how they are opened (the safety rules)
======================================================

Only these fixed repo-relative paths are ever read::

    code/specs/logic-gates.warrant
    code/specs/logic-gates-basic.warrant
    code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.profile   (once the fixture exists)
    code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.json      (once the fixture exists)
    code/packages/rust/logic-gates/BUILD
    code/packages/rust/logic-gates/evidence.bindings

A guard that could be pointed at another file, or that followed a symbolic link
out of the repository, could be made to compare a file with itself and pass.  So
for every input ``_read_input``:

* takes the ``realpath`` of the candidate and requires it to equal the plain
  ``root/rel`` path (stricter than "still under the root": ANY link in ANY
  parent directory, even one that points back inside the repository, is refused);
* requires ``lstat`` to say *regular file* (a link in the file position is not);
* opens with ``O_NOFOLLOW`` where the platform has it (Windows does not; there
  the ``lstat`` and ``realpath`` rules carry the guarantee) and reads from that
  descriptor, after ``fstat`` confirms it is still a regular file;
* refuses a missing file and any file larger than 1 MiB (1,048,576 bytes).

What is compared
================

The Warrant lexer (``code/grammars/warrant/warrant.tokens``) skips two things:
``COMMENT = /--[^\\n]*/`` and ``WHITESPACE = /[ \\t]+/``.  The only quoted token
it has is ``BYTES_LITERAL = /b"(?:[0-9a-f]{2})*"/``, whose body is hex digits, so
it can never contain a space, a dash or a quote.  ``_normalise`` therefore scans
every line left to right like the lexer:

1. a ``BYTES_LITERAL`` is consumed whole and kept;
2. otherwise ``--`` removes the rest of the line (a comment);
3. otherwise a ``"`` is *refused*: the grammar has no string literal, so a
   quoted ``" -- "`` cannot be used to hide a comment marker from the guard;
4. a ``--`` directly after a non-blank character (``a--b``) is refused, because
   what the lexer makes of it (a comment, or two minus signs) is not something
   a specification should depend on.

Identifier runs are consumed whole, so the ``b`` in ``ab"00"`` is not mistaken
for the start of a literal.  After that, trailing blanks are dropped and a line
that is empty is *blank*.  Nothing else is normalised.  Tabs, CR, a byte-order
mark, invalid UTF-8, a missing final LF and an indent that is not a multiple of
two spaces are all differences (checked before the scan).

Every non-blank line must then be either the one column-0 header line, or part of
a block.  A block starts at a two-space line whose first word is one of the nine
declaration keywords and runs over every following line indented more than two
spaces.  A stray line therefore cannot hide: it is an error, not a skip.

The twelve checks (numbers are LG02 section 4.2's)
==================================================

 1. strict file shape;                  2. the comment rule above;
 3. no line outside a block;            4. block extraction;
 5. the exact key set of the basic document (the script holds it, computing the
    39 example names from the full document by the ops each example calls);
 6. block text byte-identical to the full document's block;
 7. blocks of one kind in the same relative order;
 8. closure: no block of the basic document names an operation outside the 11;
 9. headers: ``warrant logic_gates_basic v1`` versus ``warrant logic_gates v1``;
10. per (op, fault): ``corpus.profile`` equals the ``faults=`` of ``evidence.bindings``;
11. per (op, fault, required claim): the 31 fault pins of the Rust BUILD equal the
    controls of ``corpus.json``, and every pinned claim is one of that control's
    ``required_claims`` (claim-to-op table embedded below, itself checked against
    the 26 claim names of check 5);
12. ``corpus.json``'s ``spec_sha256`` equals the SHA-256 of the basic document.

Checks 10 to 12 need the fixture ``code/specs/fixtures/logic-gates-basic-warrant-v2``
that item LGE-S1c adds.  While that directory does not exist they are skipped and
the script says so.  The moment the directory exists they are mandatory: a missing
or stale file then fails.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import stat
import sys

# ---------------------------------------------------------------------------
# Fixed facts (LG02 sections 2 and 4)
# ---------------------------------------------------------------------------

MAX_BYTES = 1024 * 1024  # 1 MiB cap on every input

FULL_SPEC = "code/specs/logic-gates.warrant"
BASIC_SPEC = "code/specs/logic-gates-basic.warrant"
FIXTURE_DIR = "code/specs/fixtures/logic-gates-basic-warrant-v2"
PROFILE = FIXTURE_DIR + "/corpus.profile"
CORPUS = FIXTURE_DIR + "/corpus.json"
BUILD_FILE = "code/packages/rust/logic-gates/BUILD"
BINDINGS = "code/packages/rust/logic-gates/evidence.bindings"

FULL_HEADER = "warrant logic_gates v1"
BASIC_HEADER = "warrant logic_gates_basic v1"

# The nine declaration keywords (warrant.grammar:3-4).
KEYWORDS = (
    "model",
    "error",
    "op",
    "example",
    "claim",
    "assume",
    "labels",
    "input",
    "bound",
)

# The eleven scalar operations; the number of parameters drives the input keys.
OPS: dict[str, tuple[str, ...]] = {
    "not_gate": ("a",),
    "and_gate": ("a", "b"),
    "or_gate": ("a", "b"),
    "xor_gate": ("a", "b"),
    "nand_gate": ("a", "b"),
    "nor_gate": ("a", "b"),
    "xnor_gate": ("a", "b"),
    "nand_not": ("a",),
    "nand_and": ("a", "b"),
    "nand_or": ("a", "b"),
    "nand_xor": ("a", "b"),
}

EXPECTED_EXAMPLE_COUNT = 39

# The explicit claim -> op table of check 11.  A claim that mentions two ops
# (nand_xor_equals_xor calls nand_xor and xor_gate) maps to the NAND-built op.
_STEMS = {
    "not": "not_gate",
    "and": "and_gate",
    "or": "or_gate",
    "xor": "xor_gate",
    "nand": "nand_gate",
    "nor": "nor_gate",
    "xnor": "xnor_gate",
    "nand_not": "nand_not",
    "nand_and": "nand_and",
    "nand_or": "nand_or",
    "nand_xor": "nand_xor",
}
CLAIM_TO_OP: dict[str, str] = {}
for _stem, _op in _STEMS.items():
    CLAIM_TO_OP[_stem + "_truth_table"] = _op
    CLAIM_TO_OP[_stem + "_rejects_non_bit"] = _op
CLAIM_TO_OP.update(
    {
        "nand_not_equals_not": "nand_not",
        "nand_and_equals_and": "nand_and",
        "nand_or_equals_or": "nand_or",
        "nand_xor_equals_xor": "nand_xor",
    }
)

_BYTES_LITERAL = re.compile(r'b"(?:[0-9a-f]{2})*"')
_WORD = re.compile(r"[A-Za-z0-9_]+")
_KEY_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)?")
_CALL = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\(")

Key = tuple[str, str]


class DriftError(Exception):
    """A check failed.  The message is the whole report."""


# ---------------------------------------------------------------------------
# Safe input reading
# ---------------------------------------------------------------------------


def _read_input(root: str, rel: str) -> bytes:
    """Read one fixed repo-relative input under the rules in the module docstring."""
    candidate = os.path.join(root, *rel.split("/"))
    try:
        real = os.path.realpath(candidate)
    except OSError as exc:  # pragma: no cover - realpath rarely raises
        raise DriftError(f"{rel}: cannot resolve ({exc})")
    if os.path.normcase(real) != os.path.normcase(candidate):
        raise DriftError(
            f"{rel}: refused, its real path differs from the repository path "
            "(a symbolic link in the file or in a parent directory)"
        )
    try:
        before = os.lstat(candidate)
    except FileNotFoundError:
        raise DriftError(f"{rel}: missing")
    except OSError as exc:
        raise DriftError(f"{rel}: cannot stat ({exc})")
    if not stat.S_ISREG(before.st_mode):
        raise DriftError(f"{rel}: refused, not a regular file")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_BINARY", 0)
    try:
        fd = os.open(candidate, flags)
    except OSError as exc:
        raise DriftError(f"{rel}: cannot open ({exc})")
    try:
        after = os.fstat(fd)
        if not stat.S_ISREG(after.st_mode):
            raise DriftError(f"{rel}: refused, not a regular file")
        if after.st_size > MAX_BYTES:
            raise DriftError(f"{rel}: refused, larger than {MAX_BYTES} bytes")
        data = b""
        while len(data) <= MAX_BYTES:
            chunk = os.read(fd, MAX_BYTES + 1 - len(data))
            if not chunk:
                break
            data += chunk
    finally:
        os.close(fd)
    if len(data) > MAX_BYTES:
        raise DriftError(f"{rel}: refused, larger than {MAX_BYTES} bytes")
    return data


# ---------------------------------------------------------------------------
# Checks 1-4: file shape, comment rule, block extraction
# ---------------------------------------------------------------------------


def _normalise_line(rel: str, lineno: int, line: str) -> str:
    """Scan one line the way the Warrant lexer does; return its content (no comment)."""
    out: list[str] = []
    i, n = 0, len(line)
    while i < n:
        literal = _BYTES_LITERAL.match(line, i)
        if literal:
            out.append(literal.group(0))
            i = literal.end()
            continue
        if line.startswith("--", i):
            if out and out[-1][-1:] not in (" ",):
                raise DriftError(
                    f"{rel}:{lineno}: '--' directly after a non-blank character; "
                    "write a space before a comment"
                )
            break
        ch = line[i]
        if ch == '"':
            raise DriftError(
                f'{rel}:{lineno}: a \'"\' that does not start a b"hex" literal '
                "(the Warrant grammar has no string literal)"
            )
        word = _WORD.match(line, i)
        if word:
            out.append(word.group(0))
            i = word.end()
        else:
            out.append(ch)
            i += 1
    return "".join(out).rstrip(" ")


class Block:
    """One extracted block: its key, its normalised non-blank lines, its position."""

    def __init__(self, key: Key, lineno: int, lines: list[str]) -> None:
        self.key = key
        self.lineno = lineno
        self.lines = lines

    @property
    def text(self) -> str:
        return "\n".join(self.lines)


class Document:
    """A parsed Warrant document: header text and its blocks in file order."""

    def __init__(self, rel: str, header: str, blocks: list[Block]) -> None:
        self.rel = rel
        self.header = header
        self.blocks = blocks

    def of_kind(self, kind: str) -> list[Block]:
        return [b for b in self.blocks if b.key[0] == kind]


def _block_key(rel: str, lineno: int, kind: str, rest: str) -> Key:
    match = _KEY_NAME.match(rest)
    if not match:
        raise DriftError(f"{rel}:{lineno}: cannot read the name of the {kind} block")
    return (kind, match.group(0))


def parse_document(rel: str, data: bytes) -> Document:
    """Checks 1 to 4 for one file."""
    if data.startswith(b"\xef\xbb\xbf"):
        raise DriftError(f"{rel}: starts with a byte-order mark")
    if b"\r" in data:
        raise DriftError(f"{rel}: contains a carriage return")
    if b"\t" in data:
        raise DriftError(f"{rel}: contains a tab character")
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise DriftError(f"{rel}: not valid UTF-8 ({exc})")
    if not text.endswith("\n"):
        raise DriftError(f"{rel}: no final line feed")

    header: str | None = None
    blocks: list[Block] = []
    current: Block | None = None
    for lineno, raw in enumerate(text.split("\n")[:-1], start=1):
        content = _normalise_line(rel, lineno, raw)
        if content == "":
            continue
        indent = len(content) - len(content.lstrip(" "))
        if indent % 2:
            raise DriftError(
                f"{rel}:{lineno}: indentation is not a multiple of two spaces"
            )
        if indent == 0:
            if header is not None or blocks:
                raise DriftError(
                    f"{rel}:{lineno}: a column-0 line outside the header: {content!r}"
                )
            if not content.startswith("warrant "):
                raise DriftError(
                    f"{rel}:{lineno}: the first line is not a 'warrant' header"
                )
            header = content
            continue
        if header is None:
            raise DriftError(f"{rel}:{lineno}: content before the 'warrant' header")
        if indent == 2:
            word, _, rest = content[2:].partition(" ")
            if word not in KEYWORDS:
                raise DriftError(
                    f"{rel}:{lineno}: a two-space line that starts no block: {content!r}"
                )
            key = _block_key(rel, lineno, word, rest)
            current = Block(key, lineno, [content])
            blocks.append(current)
        else:
            if current is None:
                raise DriftError(
                    f"{rel}:{lineno}: an indented line with no open block: {content!r}"
                )
            current.lines.append(content)
    if header is None:
        raise DriftError(f"{rel}: no 'warrant' header")
    return Document(rel, header, blocks)


# ---------------------------------------------------------------------------
# Checks 5-9: key set, text, order, closure, headers
# ---------------------------------------------------------------------------


def _called_ops(block: Block, declared: set) -> set:
    """The declared operation names an example body calls (its subject)."""
    return {m for line in block.lines[1:] for m in _CALL.findall(line) if m in declared}


def expected_keys(full: Document) -> dict[str, list[Key]]:
    """The key set check 5 requires of the basic document, in the full document's order."""
    declared = {b.key[1] for b in full.of_kind("op")}
    ops = set(OPS)
    wanted: dict[str, list[Key]] = {k: [] for k in KEYWORDS}
    for block in full.blocks:
        kind, name = block.key
        if kind in ("error", "assume"):
            ok = True
        elif kind == "model":
            ok = name == "b2u"
        elif kind in ("op", "labels", "bound"):
            ok = name in ops
        elif kind == "input":
            ok = name.split(".")[0] in ops
        elif kind == "example":
            called = _called_ops(block, declared)
            if called & ops and not called <= ops:
                raise DriftError(
                    f"{full.rel}:{block.lineno}: example {name} mixes basic and "
                    "non-basic operations; LG02 has no rule for it"
                )
            ok = bool(called) and called <= ops
        else:
            ok = name in CLAIM_TO_OP
        if ok:
            wanted[kind].append(block.key)
    # Sanity of the embedded tables against the full document itself.
    if [k[1] for k in wanted["op"]] != list(OPS):
        raise DriftError(
            f"{full.rel}: the eleven op blocks are not the expected ones in the expected order"
        )
    inputs = [(op, p) for op, params in OPS.items() for p in params]
    if [tuple(k[1].split(".")) for k in wanted["input"]] != inputs:
        raise DriftError(f"{full.rel}: the 20 input blocks are not the expected ones")
    if len(wanted["example"]) != EXPECTED_EXAMPLE_COUNT:
        raise DriftError(
            f"{full.rel}: {len(wanted['example'])} examples for the eleven ops, expected "
            f"{EXPECTED_EXAMPLE_COUNT}"
        )
    if {k[1] for k in wanted["claim"]} != set(CLAIM_TO_OP) or len(
        wanted["claim"]
    ) != len(CLAIM_TO_OP):
        raise DriftError(
            f"{full.rel}: the 26 claim names differ from the script's claim-to-op table"
        )
    return wanted


def compare_documents(full: Document, basic: Document) -> None:
    """Checks 5 to 9."""
    if full.header != FULL_HEADER:
        raise DriftError(
            f"{full.rel}: header is {full.header!r}, expected {FULL_HEADER!r}"
        )
    if basic.header != BASIC_HEADER:
        raise DriftError(
            f"{basic.rel}: header is {basic.header!r}, expected {BASIC_HEADER!r}"
        )
    wanted = expected_keys(full)
    full_by_key = {b.key: b for b in full.blocks}
    for kind in KEYWORDS:
        got = [b.key for b in basic.of_kind(kind)]
        want = wanted[kind]
        if got == want:
            continue
        # Report the first precise difference: missing, extra, duplicate or order.
        seen = set()
        for b in basic.of_kind(kind):
            if b.key in seen:
                raise DriftError(
                    f"{basic.rel}:{b.lineno}: duplicate {kind} block {b.key[1]}"
                )
            seen.add(b.key)
            if b.key not in want:
                raise DriftError(
                    f"{basic.rel}:{b.lineno}: extra {kind} block {b.key[1]} (not in the basic set)"
                )
        for key in want:
            if key not in seen:
                raise DriftError(f"{basic.rel}: missing {kind} block {key[1]}")
        for index, (g, w) in enumerate(zip(got, want)):
            if g != w:
                raise DriftError(
                    f"{basic.rel}: {kind} blocks out of order at position {index + 1}: "
                    f"found {g[1]}, expected {w[1]}"
                )
        raise DriftError(
            f"{basic.rel}: {kind} blocks differ from the expected set"
        )  # pragma: no cover
    # Check 6 (text).  Kind order, then file order, so the first difference is stable.
    for kind in KEYWORDS:
        for block in basic.of_kind(kind):
            source = full_by_key[block.key]
            if block.text != source.text:
                first = next(
                    (
                        i
                        for i, (x, y) in enumerate(zip(block.lines, source.lines))
                        if x != y
                    ),
                    min(len(block.lines), len(source.lines)),
                )
                have = (
                    block.lines[first] if first < len(block.lines) else "<end of block>"
                )
                need = (
                    source.lines[first]
                    if first < len(source.lines)
                    else "<end of block>"
                )
                raise DriftError(
                    f"{kind} block {block.key[1]} differs: {basic.rel}:{block.lineno} has "
                    f"{have!r} at its line {first + 1} where {full.rel}:{source.lineno} has {need!r}"
                )
    # Check 8 (closure): no identifier of the basic document is a non-basic operation.
    outside = {b.key[1] for b in full.of_kind("op")} - set(OPS)
    for block in basic.blocks:
        for line in block.lines:
            for word in _WORD.findall(line):
                if word in outside:
                    raise DriftError(
                        f"{basic.rel}:{block.lineno}: {block.key[0]} block {block.key[1]} "
                        f"names {word}, an operation outside the basic eleven"
                    )


# ---------------------------------------------------------------------------
# Checks 10-12: fault list, fault pins, corpus digest
# ---------------------------------------------------------------------------


def _parse_profile(text: str) -> set:
    pairs = set()
    for number, line in enumerate(text.split("\n"), start=1):
        parts = line.split()
        if parts and parts[0] == "fault":
            if len(parts) != 3:
                raise DriftError(f"{PROFILE}:{number}: expected 'fault <op> <fault>'")
            pairs.add((parts[1], parts[2]))
    return pairs


def _parse_bindings(text: str) -> set:
    pairs = set()
    for line in text.split("\n"):
        parts = line.split()
        if len(parts) >= 2 and parts[0] == "op" and parts[1] in OPS:
            faults = [p[len("faults=") :] for p in parts[2:] if p.startswith("faults=")]
            if len(faults) != 1:
                raise DriftError(
                    f"{BINDINGS}: op {parts[1]} has no single faults= list"
                )
            for fault in filter(None, faults[0].split(",")):
                pairs.add((parts[1], fault))
    return pairs


_PIN = re.compile(
    r"WARRANT_EVIDENCE_FAULT=(\S+)\s+WARRANT_EVIDENCE_FILTER=/exhaustive/(\S+)"
)


def _parse_pins(text: str) -> list[tuple[str, str, str]]:
    """(op, fault, claim) for every BUILD fault pin whose claim is one of the 26."""
    pins = []
    for line in text.split("\n"):
        if line.lstrip().startswith("#"):
            continue
        match = _PIN.search(line)
        if match and match.group(2) in CLAIM_TO_OP:
            pins.append((CLAIM_TO_OP[match.group(2)], match.group(1), match.group(2)))
    return pins


def _describe(pairs: set) -> str:
    return ", ".join(f"{op}/{fault}" for op, fault in sorted(pairs))


def check_fixture(root: str, basic_bytes: bytes) -> None:
    """Checks 10, 11 and 12 (the fixture exists)."""
    profile = _parse_profile(_read_input(root, PROFILE).decode("utf-8", "replace"))
    bindings = _parse_bindings(_read_input(root, BINDINGS).decode("utf-8", "replace"))
    if profile != bindings:
        raise DriftError(
            f"corpus.profile faults differ from evidence.bindings: only in profile "
            f"[{_describe(profile - bindings)}], only in bindings [{_describe(bindings - profile)}]"
        )
    try:
        corpus = json.loads(_read_input(root, CORPUS).decode("utf-8"))
        controls = corpus["controls"]
        digest = corpus["spec_sha256"]
    except (ValueError, KeyError, TypeError) as exc:
        raise DriftError(
            f"{CORPUS}: unreadable or missing controls/spec_sha256 ({exc!r})"
        )
    required: dict[tuple[str, str], list[str]] = {}
    for control in controls:
        try:
            pair = (control["op"], control["fault"])
            required[pair] = list(control["required_claims"])
        except (KeyError, TypeError) as exc:
            raise DriftError(
                f"{CORPUS}: a control lacks op, fault or required_claims ({exc!r})"
            )
    pins = _parse_pins(_read_input(root, BUILD_FILE).decode("utf-8", "replace"))
    pinned = {(op, fault) for op, fault, _ in pins}
    if pinned != set(required):
        raise DriftError(
            f"BUILD fault pins differ from corpus controls: only in BUILD "
            f"[{_describe(pinned - set(required))}], only in corpus [{_describe(set(required) - pinned)}]"
        )
    if set(required) != profile:
        raise DriftError(
            f"corpus controls differ from corpus.profile: [{_describe(set(required) ^ profile)}]"
        )
    for op, fault, claim in pins:
        if not any(
            r == claim or r.endswith("/exhaustive/" + claim)
            for r in required[(op, fault)]
        ):
            raise DriftError(
                f"BUILD pins claim {claim} for {op}/{fault} but corpus required_claims "
                f"are {required[(op, fault)]}"
            )
    actual = hashlib.sha256(basic_bytes).hexdigest()
    if actual != digest:
        raise DriftError(
            f"{CORPUS}: spec_sha256 {digest} does not match {BASIC_SPEC} ({actual}); "
            "regenerate the fixture"
        )


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def run(root: str) -> list[str]:
    """Run every check against the tree at ``root``; raise DriftError on drift.

    Returns the informational lines to print on success.  Tests call this with a
    temporary tree; the command line passes only the realpath-derived root.
    """
    root = os.path.realpath(root)
    full = parse_document(FULL_SPEC, _read_input(root, FULL_SPEC))
    basic_bytes = _read_input(root, BASIC_SPEC)
    basic = parse_document(BASIC_SPEC, basic_bytes)
    compare_documents(full, basic)
    notes = [
        f"documents match: {len(basic.blocks)} blocks, {EXPECTED_EXAMPLE_COUNT} examples, {len(CLAIM_TO_OP)} claims"
    ]
    if os.path.lexists(os.path.join(root, *FIXTURE_DIR.split("/"))):
        check_fixture(root, basic_bytes)
        notes.append(
            "fixture checks 10-12 passed (fault lists, fault pins, corpus digest)"
        )
    else:
        notes.append(
            f"checks 10-12 skipped: {FIXTURE_DIR} does not exist yet (LGE-S1c adds it); "
            "they become mandatory the moment it does"
        )
    return notes


def main(argv: list[str]) -> int:
    if len(argv) != 1:
        print("logic_gates_basic_drift: takes no arguments", file=sys.stderr)
        return 2
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.realpath(__file__))))
    try:
        notes = run(root)
    except DriftError as exc:
        print(f"logic_gates_basic_drift: DRIFT: {exc}", file=sys.stderr)
        return 1
    for note in notes:
        print(f"logic_gates_basic_drift: {note}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
