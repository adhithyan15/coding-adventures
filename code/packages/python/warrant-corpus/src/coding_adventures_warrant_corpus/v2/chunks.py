"""Chunks: validating a chunk file and enumerating one chunk alone (section 5.4 to 5.6).

A suite's product space is cut into **chunks of** ``chunk_size`` **product
ordinals** (a power of two in ``2^10..=2^16``, recorded in the corpus and never
assumed).  Chunk ``c`` covers ordinals ``[c*chunk_size, (c+1)*chunk_size)``, and
there are ``m = ceil(P / chunk_size)`` of them (at most 4,096).  Boundaries are
on *product* ordinals, not admitted ones, so a lane can **unrank**
``c * chunk_size``, enumerate from there, and never touch an earlier chunk::

    product ordinals   0 .... 65535 | 65536 ... 131071 | ...
    chunk              |<--- 0 ---->|<----- 1 ------->|
    admitted cases      (refinements discard some tuples; the discarded ones
                         take a product ordinal but no case number)

Each chunk has its own pair of SHA-256 streams.  The prefix is the context of
VER04i section 2.6 -- the header, the length-prefixed suite id, the binder count
and each binder's canonical type name, the result type name -- followed by the
u32-LE **chunk index**; the body is the typed values of that chunk's admitted
cases in order, with no per-case delimiter.  Chunk digests are therefore
independent and are combined by *listing* them (``warrant-chunks-v1``), never by
hashing one long stream.

The chunk file (section 5.5) has one line per chunk::

    <c> <cases> <first_admitted> <calls> <inputs_hex> [<expected_hex>]

and ``first_admitted`` -- the admitted-case ordinal of the chunk's first admitted
case -- is the running sum of the ``cases`` of earlier chunks.  A lane that
enumerates a chunk alone cannot compute it, so it reports it **from the
validated file** (this is what a slim run does).

Two obligations this module is built to be tested for
-----------------------------------------------------
* *chunk ``c`` alone equals sequential*: :func:`run_chunk` unranks and jumps,
  :func:`run_sequential` walks from ordinal 0 and never unranks; the digests of
  every chunk must agree, and agree with the committed chunk file.
* *every consumer treats the file as hostile*: line grammar by fixed field
  count (never a backtracking regular expression), every cross-check of
  sections 5.5 and 14 before the first case.
"""

from __future__ import annotations

import hashlib
import json
from collections.abc import Iterator
from dataclasses import dataclass
from itertools import islice

from .domains import Desc, is_descriptor_for
from .domains import build as build_descriptor
from .errors import KitError, Refusal
from .ir import (
    IMPLEMENTED_LEVELS,
    Compiled,
    Program,
    check_requires,
    evaluator,
    guarded,
    scan_tags,
    type_level,
)
from .semantics import STANDARD, Semantics
from .values import make_encoder
from .wtypes import BOOL, Ty, check_types_table, flows, parse_type, unify

MAX_CHUNKS = 4096
MAX_SLIM_CHUNKS = 32
MAX_SUITE_PRODUCT = 1 << 28  # section 14: chunks bound the work, not the product
MAX_BINDERS = 8
MAX_CHUNK_FILE_BYTES = (
    2 << 20
)  # 4,096 lines of at most ~330 bytes is 1.3 MB; 2 MiB is room
MAX_COUNT_DIGITS = 20  # a count in a chunk file is a decimal of at most 20 digits
HEADERS = ("warrant-stream-v1", "warrant-stream-v2")

# The exact keys of the ``corpus`` part of a v2-chunk case (section 11): the
# pieces of a corpus.json that one suite needs, and nothing else.
CHUNK_CORPUS_KEYS = frozenset(
    {"chunk_size", "max_ir_cost", "types", "ops", "models", "model_order"}
)


# ---------------------------------------------------------------------------
# The key set of a suite row (section 1.4)
# ---------------------------------------------------------------------------
# A suite row is a *closed* object.  Its keys are those every suite has, plus
# the ones that depend on two facts about the suite: whether it is a ``reference``
# suite (it has an expected stream, so it commits to expected digests) and
# whether it keeps the v1 stream header (it then also carries v1's whole-suite
# digests).  The table says which facts switch which keys on.
_KEYS_OF_EVERY_SUITE = frozenset(
    [
        "id",
        "kind",
        "claim",
        "ops",
        "requires",
        "binders",
        "demoted",
        "domain_label",
        "admit",
        "body",
        "oracle",
        "result_type",
        "stream",
        "product",
        "cases",
        "discarded",
        "expected_calls",
        "wraps",
        "ir_cost",
        "chunks",
        "chunks_sha256",
        "inputs_sha256",
        "slim_chunks",
        "slim_inputs_sha256",
    ]
)
_KEYS_ONLY_WITH_EXPECTED = frozenset({"expected_sha256", "slim_expected_sha256"})
_KEYS_ONLY_WITH_V1_HEADER = frozenset({"v1_inputs_sha256"})
_KEYS_ONLY_WITH_V1_HEADER_AND_EXPECTED = frozenset({"v1_expected_sha256"})
_V1_HEADER = "warrant-stream-v1"


def is_index(value: object) -> bool:
    """A JSON integer that is not a ``bool`` (``True`` is an ``int`` in Python)."""
    return type(value) is int


def schema_index(value: object, what: str) -> int:
    """A case field that must be a non-negative JSON integer, else a *kit* error."""
    if not is_index(value) or value < 0:  # type: ignore[operator]
        raise KitError(f"{what} must be a non-negative JSON integer")
    return value  # type: ignore[return-value]


def is_reference(row: dict) -> bool:
    oracle = row.get("oracle")
    return isinstance(oracle, dict) and oracle.get("kind") == "reference"


def expected_row_keys(row: dict) -> frozenset[str]:
    """The exact keys a suite ``row`` of its own kind of suite must have."""
    keys = set(_KEYS_OF_EVERY_SUITE)
    has_expected = is_reference(row)
    if has_expected:
        keys |= _KEYS_ONLY_WITH_EXPECTED
    if row.get("stream") == _V1_HEADER:
        keys |= _KEYS_ONLY_WITH_V1_HEADER
        if has_expected:
            keys |= _KEYS_ONLY_WITH_V1_HEADER_AND_EXPECTED
    return frozenset(keys)


# ---------------------------------------------------------------------------
# Digests
# ---------------------------------------------------------------------------
def chunk_digest_framing(digests: list[bytes], sem: Semantics = STANDARD) -> str:
    """``SHA-256("warrant-chunks-v1\\0" || u32-LE m || each chunk digest, in order)``.

    This is the v2 suite commitment (section 5.4): the suite digest of a chunked
    suite is a hash of the chunks' own digests, so chunks merge by listing.
    """
    body = b"warrant-chunks-v1\x00" + sem.u32(len(digests)) + b"".join(digests)
    return hashlib.sha256(body).hexdigest()


def stream_context(row: dict, sem: Semantics = STANDARD) -> bytes:
    """The context both streams start with (VER04i 2.6, section 2.4).

    ``"warrant-stream-vN\\0"``, the length-prefixed suite id, the u32 binder count,
    each binder's canonical type name length-prefixed, and the result type name.
    Because the context fixes every type, the per-case bytes need no delimiters.
    """

    def field(text: str) -> bytes:
        raw = text.encode("utf-8")
        return sem.u32(len(raw)) + raw

    binders = row["binders"]
    return (
        row["stream"].encode("ascii")
        + b"\x00"
        + field(row["id"])
        + sem.u32(len(binders))
        + b"".join(field(binder[1]["type"]) for binder in binders)
        + field(row["result_type"])
    )


# ---------------------------------------------------------------------------
# Validation of the corpus part and the suite row  (code `corpus`)
# ---------------------------------------------------------------------------
@dataclass
class Suite:
    """A suite row, checked and compiled, ready to enumerate.

    ``names`` and ``types`` are the binders in order; ``descs`` their descriptor
    trees; ``admit`` and ``body`` the compiled refinement and claim; ``expected``
    the compiled pure side of a ``reference`` suite (``None`` for a predicate
    suite, which has no expected stream); ``chunk_size`` the corpus's recorded
    value; ``m`` the chunk count.
    """

    row: dict
    names: list[str]
    types: list[Ty]
    descs: list
    admit: Compiled
    body: Compiled
    expected: Compiled | None
    result_ty: Ty
    chunk_size: int
    m: int
    product: int
    context: bytes
    corpus_types: dict


def _corpus_error(why: str) -> Refusal:
    return Refusal("corpus", why)


# ---------------------------------------------------------------------------
# Claim names and chunk file names (section 5.5)
# ---------------------------------------------------------------------------
MAX_CLAIM_NAME = 64  # each of the (one or two) names: 1 + 63 more characters
MAX_CHUNK_FILE_NAME = 133  # two 64-character names, the ".", and ".txt"
_NAME_FIRST = frozenset("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_")
_NAME_REST = _NAME_FIRST | frozenset("0123456789")
# A file with one of these as a dot-separated segment cannot be created on Windows.
_DEVICE_NAMES = frozenset(
    {"con", "prn", "aux", "nul"}
    | {f"com{n}" for n in range(1, 10)}
    | {f"lpt{n}" for n in range(1, 10)}
)


def chunk_file_name(claim: object) -> str:
    """The name of a suite's chunk file, derived from its claim field (section 5.5).

    A claim field is one or two names ``[A-Za-z_][A-Za-z0-9_]{0,63}`` joined by a
    single ``/``; the file is ``<claim with "/" as ".">.txt``.  Anything else --
    ``a/b/c``, ``../x``, the empty string, a name of 65 characters, a space, a
    backslash, a NUL -- is refused as ``corpus``.  So the derived name holds no
    path separator, no ``..`` and no leading dot, and **nothing read from the
    corpus is ever used as a path**: the only text that reaches a file name is
    one this function has accepted character by character.

    A segment that is a Windows device name (``con``, ``nul``, ``com1`` ...,
    any case) is refused as well, as is a name over 133 bytes, though neither
    can happen to a claim that passed the first rule except the device names.
    Two claims equal ignoring case is a fact about *two* rows and is the
    corpus reader's to check.

    This library never opens the file (the corpus reader, slice S6b, does); it
    checks the claim here because it reads the suite row anyway and a hostile
    row must not be half-accepted by one consumer and refused by the next.
    """
    if type(claim) is not str:
        raise _corpus_error("a claim is text")
    names = claim.split("/")
    if len(names) > 2:
        raise _corpus_error("a claim is one or two names joined by a single slash")
    for name in names:
        if not 1 <= len(name) <= MAX_CLAIM_NAME:
            raise _corpus_error("a claim name has one to 64 characters")
        if name[0] not in _NAME_FIRST or any(c not in _NAME_REST for c in name):
            raise _corpus_error("a claim name is [A-Za-z_][A-Za-z0-9_]*")
    file_name = ".".join(names) + ".txt"
    if len(file_name) > MAX_CHUNK_FILE_NAME:  # pragma: no cover - 64+1+64+4 = 133
        raise _corpus_error("a chunk file name is over 133 bytes")
    if any(name.lower() in _DEVICE_NAMES for name in names):
        raise _corpus_error("a claim name is a Windows device name")
    return file_name


def build_suite(
    corpus: object,
    row: object,
    sem: Semantics = STANDARD,
    levels: frozenset[str] | set[str] = IMPLEMENTED_LEVELS,
) -> Suite:
    """Validate the ``corpus`` part and the suite ``row`` and compile the suite.

    Everything that is wrong with the *document* is ``corpus`` (section 11,
    R45): a key too many or too few, an invalid ``chunk_size``, a
    ``max_ir_cost`` outside ``2,000..=65,536``, a wrong descriptor ``size``,
    ``product`` or chunk count, and any IR the checker refuses.  One thing is
    **not** a defect of the document: a level this runner lacks.  A suite whose
    binders, literals or call closure need one is ``level-missing`` -- the lane
    records that suite not-live -- and the refusal passes through unchanged
    ([D64]).  ``levels`` are the levels the runner implements.
    """
    chunk_size, max_cost = _read_corpus_part(corpus)
    if not isinstance(row, dict) or set(row) != expected_row_keys(row):
        raise _corpus_error(
            "the suite row carries keys that do not "
            "match the kind of suite it describes"
        )
    # Both are facts about the row that no level can excuse: an undefined level
    # name is a corpus error and never `level-missing` (section 4.2 rule 7), and a
    # claim that is not a legal path name must never reach a file name (5.5).
    check_requires(row.get("requires"), "corpus", "a suite's requires")
    chunk_file_name(row.get("claim"))
    try:
        return _assemble_suite(corpus, row, chunk_size, max_cost, sem, levels)  # type: ignore[arg-type]
    except Refusal as error:
        if error.code == "level-missing" and _row_is_live(row, levels):
            # The row says the lane can run it and it cannot: the row is wrong.
            raise _corpus_error(
                f"the row claims levels it does not stay within: {error.message}"
            ) from error
        if error.code in ("corpus", "level-missing"):
            raise
        raise _corpus_error(f"{error.code}: {error.message}") from error
    except (KeyError, IndexError, TypeError, AttributeError, ValueError) as error:
        raise _corpus_error(
            "some field of the suite row is not the shape it must be"
        ) from error


def _row_is_live(row: dict, levels: frozenset[str] | set[str]) -> bool:
    """Does the row's own ``requires`` lie inside ``levels``? (R69 e)"""
    needs = row.get("requires")
    return isinstance(needs, list) and set(needs) <= set(levels)


def _read_corpus_part(corpus: object) -> tuple[int, int]:
    """The recorded ``chunk_size`` and ``max_ir_cost`` of a valid corpus part."""
    if not isinstance(corpus, dict) or set(corpus) != CHUNK_CORPUS_KEYS:
        raise _corpus_error(
            "the corpus part must hold the six named pieces and nothing more"
        )
    size, cost = corpus["chunk_size"], corpus["max_ir_cost"]
    if not is_index(size) or size not in {1 << power for power in range(10, 17)}:
        raise _corpus_error("chunk_size must be 1024, 2048, ... or 65536")
    if not is_index(cost) or not 2_000 <= cost <= 65_536:
        raise _corpus_error("max_ir_cost lies outside the permitted budget range")
    check_types_table(corpus["types"], "corpus")  # section 2.2, every consumer
    return size, cost


def _read_binders(
    row: dict, types: dict, sem: Semantics
) -> tuple[list[str], list[Ty], list[Desc], int]:
    """Names, types, built descriptors and the product size of a suite's binders."""
    entries = row["binders"]
    if type(entries) is not list or len(entries) > MAX_BINDERS:
        raise _corpus_error("a suite may bind at most eight variables")
    names: list[str] = []
    tys: list[Ty] = []
    descs: list[Desc] = []
    product = 1
    for entry in entries:
        if type(entry) is not list or len(entry) != 2 or type(entry[0]) is not str:
            raise _corpus_error(
                "each binder must be a name paired with its type and domain"
            )
        name, body = entry
        if type(body) is not dict or set(body) != {"type", "domain"}:
            raise _corpus_error(
                "a binder's second element must have only a type and a domain"
            )
        desc = build_descriptor(body["domain"], sem)
        ty = parse_type(body["type"], types)
        if not is_descriptor_for(body["domain"], ty, types):
            raise _corpus_error(
                "a binder's domain does not enumerate values of the binder's type"
            )
        names.append(name)
        tys.append(ty)
        descs.append(desc)
        product *= desc.size
    if len(set(names)) != len(names):
        raise _corpus_error("a variable name is bound twice")
    return names, tys, descs, product


def _assemble_suite(
    corpus: dict,
    row: dict,
    chunk_size: int,
    max_cost: int,
    sem: Semantics,
    levels: frozenset[str] | set[str],
) -> Suite:
    types = corpus["types"]
    # R107 (d): the checks that read tag names alone run over every expression
    # the suite holds before any of them is recorded as needing a missing level.
    oracle = row.get("oracle")
    sides = (
        [oracle.get("subject"), oracle.get("expected")] if type(oracle) is dict else []
    )
    scan_tags(row.get("admit"), "admit")
    scan_tags(row.get("body"), "body")
    scan_tags(sides[1] if sides else None, "expected")
    scan_tags(sides[0] if sides else None, "body")
    names, tys, descs, product = _read_binders(row, types, sem)
    if product > MAX_SUITE_PRODUCT:
        raise _corpus_error(
            "the domains multiply to more ordinals than a suite may hold"
        )
    if not is_index(row["product"]) or row["product"] != product:
        raise _corpus_error("the recorded product disagrees with the domain sizes")
    count = max(1, -(-product // chunk_size))
    if count > MAX_CHUNKS or not is_index(row["chunks"]) or row["chunks"] != count:
        raise _corpus_error(
            "the recorded chunk count disagrees with product and chunk_size"
        )
    if row["stream"] not in HEADERS:
        raise _corpus_error("the stream header is not one this runner knows")
    if row["kind"] != "exhaustive":
        raise _corpus_error("the only kind of suite is the exhaustive one")

    program = Program(
        types=types,
        ops=corpus["ops"],
        models=corpus["models"],
        model_order=corpus["model_order"],
        levels=levels,
        max_ir_cost=max_cost,
        sem=sem,
    )
    scope = dict(zip(names, tys, strict=True))
    result_ty = parse_type(row["result_type"], types)
    program.need_levels(type_level(result_ty, types), "the result type")
    admit = program.compile(row["admit"], scope, role="admit")
    body = program.compile(row["body"], scope, role="body")
    if admit.ty != BOOL or body.ty != BOOL:
        raise _corpus_error("both admit and body must be true-or-false expressions")
    expected = _compile_oracle(row, program, scope, body, result_ty)
    cost = admit.cost + body.cost + (expected.cost if expected else 0)
    if not is_index(row["ir_cost"]) or row["ir_cost"] != cost:
        raise _corpus_error(
            "the recorded ir_cost disagrees with the recomputed worst-path costs"
        )
    return Suite(
        row=row,
        names=names,
        types=tys,
        descs=descs,
        admit=admit,
        body=body,
        expected=expected,
        result_ty=result_ty,
        chunk_size=chunk_size,
        m=count,
        product=product,
        context=stream_context(row, sem),
        corpus_types=types,
    )


def _is_reference_body(body: object, subject: object, expected: object) -> bool:
    """Section 4.3: a ``reference`` suite means ``subject == pure``.

    The body is the ``eq`` node over the oracle's two sides, in either order.
    Sides are compared as canonical JSON text so that ``true`` and ``1`` (equal
    in Python, different in JSON) are not confused.
    """

    def text(node: object) -> str:
        return json.dumps(node, sort_keys=True, separators=(",", ":"))

    if type(body) is not list or len(body) != 3 or body[0] != "eq":
        return False
    sides = (text(body[1]), text(body[2]))
    return sides in ((text(subject), text(expected)), (text(expected), text(subject)))


def _compile_oracle(
    row: dict, program: Program, scope: dict[str, Ty], body: Compiled, result_ty: Ty
) -> Compiled | None:
    """Check the ``oracle`` of a suite; compile and return its ``expected`` side.

    Section 4.3: a ``predicate`` suite has no expected side.  A ``reference``
    suite has two sides, ``subject`` (which calls the library) and ``expected``
    (which does not).  The expected side must produce values of the declared
    result type, because its values are what the expected stream encodes (F5).
    """
    oracle = row["oracle"]
    if type(oracle) is not dict:
        raise _corpus_error("the oracle entry must be an object")
    if oracle.get("kind") == "predicate" and set(oracle) == {"kind"}:
        return None
    if oracle.get("kind") != "reference" or set(oracle) != {
        "kind",
        "subject",
        "expected",
    }:
        raise _corpus_error(
            "the oracle entry is neither a bare predicate nor a full reference"
        )
    if not _is_reference_body(row["body"], oracle["subject"], oracle["expected"]):
        raise _corpus_error(
            "a reference suite's body must be eq(subject, expected), in either order"
        )
    subject = program.compile(oracle["subject"], scope, role="body")
    expected = program.compile(oracle["expected"], scope, role="expected")
    if not flows(expected.ty, result_ty) or unify(subject.ty, result_ty) is None:
        raise _corpus_error(
            "a reference suite's sides do not produce its declared result type"
        )
    return expected


def check_chunk_index(suite: Suite, chunk: int) -> None:
    """``chunk >= m`` is ``corpus`` (section 11, R45)."""
    if chunk >= suite.m:
        raise _corpus_error("chunk >= m")


def check_slim_chunks(row: dict, m: int) -> None:
    """``slim-chunks``: is the row's ``slim_chunks`` a legal slim set for ``m`` chunks?

    Section 5.6: ascending and free of duplicates, every index below ``m``, the
    first chunk and the last chunk both in, and no more than 32 entries.  An
    entry that is negative or not a JSON integer breaks the *case's* schema and
    is a kit error, not an outcome (R45).
    """
    chosen = row["slim_chunks"]
    if not isinstance(chosen, list):
        raise Refusal("slim-chunks", "slim_chunks must be a list")
    for entry in chosen:
        schema_index(entry, "a slim_chunks entry")
    reasons = (
        (not chosen or len(chosen) > MAX_SLIM_CHUNKS, "from 1 to 32 entries"),
        (
            any(a >= b for a, b in zip(chosen, chosen[1:], strict=False)),
            "strictly ascending",
        ),
        (bool(chosen) and chosen[0] != 0, "it must include chunk 0"),
        (bool(chosen) and chosen[-1] != m - 1, "it must end at the last chunk"),
    )
    for broken, wanted in reasons:
        if broken:
            raise Refusal("slim-chunks", f"slim_chunks is not a slim set: {wanted}")


# ---------------------------------------------------------------------------
# The chunk file  (code `chunk-file`)
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class ChunkLine:
    """One line of a chunk file."""

    index: int
    cases: int
    first_admitted: int
    calls: int
    inputs: str
    expected: str | None


_LOWER_HEX = frozenset("0123456789abcdef")


def _bad_file(why: str) -> Refusal:
    return Refusal("chunk-file", why)


def _count_field(text: str) -> int:
    """A canonical decimal count: ASCII digits, no leading zero, at most 20 of them."""
    if (
        not 1 <= len(text) <= MAX_COUNT_DIGITS
        or not text.isascii()
        or not text.isdigit()
        or (text[0] == "0" and len(text) > 1)
    ):
        raise _bad_file("a count must be written as a plain decimal number")
    return int(text)


def _digest_field(text: str) -> str:
    """A SHA-256 digest as 64 lowercase hex digits."""
    if len(text) != 64 or not _LOWER_HEX.issuperset(text):
        raise _bad_file("a digest must be sixty-four lowercase hex characters")
    return text


def parse_chunk_line(line: str, has_expected: bool) -> ChunkLine:
    """One line, ``<c> <cases> <first_admitted> <calls> <inputs> [<expected>]``.

    The line is cut at its single spaces and each field is judged on its own, so
    no pattern ever runs over an attacker's text: a field of a million digits is
    refused by its length before anything reads it.
    """
    fields = line.split(" ")
    if len(fields) != (6 if has_expected else 5):
        raise _bad_file(
            "a line has a different number of columns than this suite needs"
        )
    index, cases, first, calls = (_count_field(f) for f in fields[:4])
    inputs = _digest_field(fields[4])
    expected = _digest_field(fields[5]) if has_expected else None
    return ChunkLine(index, cases, first, calls, inputs, expected)


def check_chunk_file(
    data: bytes, suite: Suite, sem: Semantics = STANDARD
) -> list[ChunkLine]:
    """Validate a chunk file against its suite row (sections 5.5 and 14).

    The file is judged in this order, cheapest first, and every failure is
    ``chunk-file``: its size, its digest (``chunks_sha256``), its ASCII text and
    line count (``m`` lines, each ended by LF), each line's grammar, the chunk
    index in sequence, ``first_admitted`` as the running total of ``cases``, the
    total against the row's ``cases`` and ``discarded``, the ``calls`` total
    against the row's ``expected_calls`` ([D73]), and finally the two suite
    digests, which are the ``warrant-chunks-v1`` framing of the per-chunk
    digests.  The expected column is there exactly when the suite is a
    ``reference`` suite.  The row's *slim* digests are **not** compared (R46).
    """
    row = suite.row
    if len(data) > MAX_CHUNK_FILE_BYTES:
        raise _bad_file("the file exceeds the size a chunk file may have")
    if hashlib.sha256(data).hexdigest() != row["chunks_sha256"]:
        raise _bad_file("the file does not hash to the recorded chunks_sha256")
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise _bad_file("the file contains bytes outside ASCII") from error
    pieces = text.split("\n")
    if pieces[-1] != "" or len(pieces) - 1 != suite.m:
        raise _bad_file("the file does not consist of one newline-ended line per chunk")
    has_expected = is_reference(row)
    lines: list[ChunkLine] = []
    admitted_so_far = 0
    total_calls = 0
    for position, piece in enumerate(pieces[:-1]):
        line = parse_chunk_line(piece, has_expected)
        if line.index != position:
            raise _bad_file("a line carries the wrong chunk number for its place")
        if line.first_admitted != admitted_so_far:
            raise _bad_file("first_admitted does not equal the cases counted so far")
        admitted_so_far = sem.advance_admitted(
            admitted_so_far, line.cases, suite.chunk_size
        )
        total_calls += line.calls
        lines.append(line)
    if total_calls != row["expected_calls"]:  # [D73]
        raise _bad_file("the chunk calls do not sum to the row's expected_calls")
    if (
        admitted_so_far != row["cases"]
        or row["discarded"] != row["product"] - admitted_so_far
    ):
        raise _bad_file(
            "the chunk case counts disagree with the row's cases and discarded"
        )
    inputs_digest = chunk_digest_framing([bytes.fromhex(x.inputs) for x in lines], sem)
    if inputs_digest != row["inputs_sha256"]:
        raise _bad_file("inputs_sha256 is not what the chunk input digests combine to")
    if has_expected:
        framed = [bytes.fromhex(x.expected or "") for x in lines]
        if chunk_digest_framing(framed, sem) != row["expected_sha256"]:
            raise _bad_file(
                "expected_sha256 is not what the chunk expected digests combine to"
            )
    return lines


# ---------------------------------------------------------------------------
# Enumeration
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class ChunkResult:
    """What enumerating one chunk produced: counts and the two digests (raw bytes)."""

    cases: int
    calls: int
    inputs: bytes
    expected: bytes | None


class _Column:
    """One binder's values and their stream bytes.

    A domain of up to ``MEMO_LIMIT`` values (and ``MEMO_BYTES`` of stream bytes)
    is tabulated once, up front: a chunk of three ``U8`` binders would otherwise
    decode and encode the same 256 values 65,536 times each.  A larger domain (a
    single 2^24-value binder) is decoded on demand by ``nth``, which is
    ``O(depth)`` and never ``O(index)``.
    """

    MEMO_LIMIT = 1 << 17
    MEMO_BYTES = 8 << 20  # a table of long Bytes values is not worth its memory

    def __init__(self, desc: Desc, ty: Ty, types: dict, sem: Semantics) -> None:
        self.desc = desc
        self.encode = make_encoder(ty, types, sem)
        self.values: list | None = None
        self.encoded: list | None = None
        if desc.size <= self.MEMO_LIMIT:
            values: list = []
            encoded: list[bytes] = []
            spent = 0
            for index in range(desc.size):
                values.append(desc.nth(index))
                encoded.append(self.encode(values[-1]))
                spent += len(encoded[-1])
                if spent > self.MEMO_BYTES:
                    return  # too big to keep: decode on demand instead
            self.values, self.encoded = values, encoded

    def get(self, index: int) -> tuple[object, bytes]:
        value = self.desc.nth(index)
        return value, self.encode(value)


class _Enumerator:
    """Runs a suite's chunks; one instance serves any number of chunks."""

    def __init__(self, suite: Suite, sem: Semantics) -> None:
        self.suite = suite
        self.sem = sem
        types = suite.corpus_types
        self.columns = [
            _Column(d, t, types, sem)
            for d, t in zip(suite.descs, suite.types, strict=True)
        ]
        self.sizes = [d.size for d in suite.descs]
        self.encode_result = make_encoder(suite.result_ty, types, sem)

    def chunk_range(self, chunk: int) -> tuple[int, int]:
        """The product ordinals ``[start, stop)`` of ``chunk``."""
        suite = self.suite
        start = chunk * suite.chunk_size
        return start, min(suite.product, start + suite.chunk_size)

    def consume(
        self, indices: Iterator[list[int]], count: int, chunk: int
    ) -> ChunkResult:
        """Enumerate the next ``count`` index vectors as chunk number ``chunk``."""
        suite, sem = self.suite, self.sem
        prefix = sem.chunk_prefix(suite.context, chunk)
        inputs = hashlib.sha256(prefix)
        expected = hashlib.sha256(prefix) if suite.expected is not None else None
        names, columns = suite.names, self.columns
        # IR runs through ``evaluator``/``guarded``: only an expression that could
        # overflow the ordinary stack is given more of it, and only for its own
        # call, so the rest of this loop never runs under a raised limit (F14).
        admit = None if suite.row["admit"] == ["lit", True] else evaluator(suite.admit)
        expected_fn = evaluator(suite.expected) if suite.expected else None
        calls = suite.body.calls
        per_case = None if isinstance(calls, int) else guarded(calls, suite.body.nest)
        tabulated = all(c.values is not None for c in columns)
        values = [c.values for c in columns]
        encoded = [c.encoded for c in columns]
        cases = total_calls = 0
        pending_in: list[bytes] = []
        pending_out: list[bytes] = []
        for vector in islice(indices, count):
            if tabulated:
                row = [t[i] for t, i in zip(values, vector, strict=True)]  # type: ignore[index]
                pairs = None
            else:
                pairs = [c.get(i) for c, i in zip(columns, vector, strict=True)]
                row = [p[0] for p in pairs]
            env = dict(zip(names, row, strict=True))
            if admit is not None and not admit(env):
                continue
            cases += 1
            if pairs is None:
                pending_in.append(
                    b"".join([t[i] for t, i in zip(encoded, vector, strict=True)])  # type: ignore[index]
                )
            else:
                pending_in.append(b"".join([p[1] for p in pairs]))
            if expected_fn is not None:
                pending_out.append(self.encode_result(expected_fn(env)))
            if per_case is not None:
                total_calls += per_case(env)  # type: ignore[operator]
            if len(pending_in) >= 8192:
                inputs.update(b"".join(pending_in))
                pending_in.clear()
                if expected is not None:
                    expected.update(b"".join(pending_out))
                    pending_out.clear()
        if isinstance(calls, int):
            total_calls = calls * cases
        inputs.update(b"".join(pending_in))
        if expected is not None:
            expected.update(b"".join(pending_out))
        return ChunkResult(
            cases,
            total_calls,
            inputs.digest(),
            expected.digest() if expected is not None else None,
        )


def run_chunk(suite: Suite, chunk: int, sem: Semantics = STANDARD) -> ChunkResult:
    """Enumerate chunk ``chunk`` **alone**: unrank its first ordinal and start there.

    Nothing of any earlier chunk is touched; this is what a ``slim`` run does for
    each chunk it was told to run.
    """
    enumerator = _Enumerator(suite, sem)
    start, stop = enumerator.chunk_range(chunk)
    indices = sem.enumerate_indices(enumerator.sizes, start, stop)
    return enumerator.consume(indices, stop - start, chunk)


def run_sequential(
    suite: Suite, last: int, sem: Semantics = STANDARD
) -> list[ChunkResult]:
    """Enumerate chunks ``0..=last`` **in sequence**, as one walk from ordinal 0.

    The odometer is started once, at ordinal 0, and runs through every chunk
    boundary, cutting a new pair of digests at each; it never unranks a chunk's
    first ordinal.  Comparing this with :func:`run_chunk` is the lane's own test of
    "chunk ``c`` alone equals sequential" (section 11, R21).
    """
    enumerator = _Enumerator(suite, sem)
    _start, stop = enumerator.chunk_range(last)
    indices = sem.enumerate_indices(enumerator.sizes, 0, stop)
    results = []
    for chunk in range(last + 1):
        start, end = enumerator.chunk_range(chunk)
        results.append(enumerator.consume(indices, end - start, chunk))
    return results


def validate_and_run(
    corpus: object,
    row: object,
    file_bytes: bytes,
    chunk: int,
    sem: Semantics = STANDARD,
) -> dict:
    """The ``v2-chunk`` outcome: validate everything, then run chunk ``chunk`` alone.

    Order of the codes (section 11): ``corpus`` (1), ``slim-chunks`` (2),
    ``chunk-file`` (3).  The lane (a) validates the chunk file and ``slim_chunks``,
    (b) enumerates the chunk alone and recomputes ``cases``, ``calls`` and the
    digests, and (c) reports ``first_admitted`` **from the validated file**.
    """
    schema_index(chunk, "chunk")
    suite = build_suite(corpus, row, sem)
    check_chunk_index(suite, chunk)
    check_slim_chunks(suite.row, suite.m)
    lines = check_chunk_file(file_bytes, suite, sem)
    result = run_chunk(suite, chunk, sem)
    return outcome(result, lines[chunk].first_admitted)


def sequential_disagreements(
    suite: Suite,
    lines: list[ChunkLine],
    last: int,
    sem: Semantics = STANDARD,
    walked: list[ChunkResult] | None = None,
) -> list[str]:
    """Where "chunk ``c`` alone" and "chunks ``0..=c`` in sequence" disagree.

    Three things must agree for every chunk ``i <= last``: the sequential walk,
    the committed chunk file (``cases``, ``calls`` and both digests), and -- for
    ``i == last`` -- the chunk enumerated alone by unranking.  ``walked`` lets a
    caller that has already walked chunks ``0..=k`` (``k >= last``) reuse it.
    """
    walked = walked if walked is not None else run_sequential(suite, last, sem)
    alone = run_chunk(suite, last, sem)
    problems = []
    for chunk in range(last + 1):
        got, line = walked[chunk], lines[chunk]
        if (
            got.cases,
            got.calls,
            got.inputs.hex(),
            got.expected and got.expected.hex(),
        ) != (
            line.cases,
            line.calls,
            line.inputs,
            line.expected,
        ):
            problems.append(
                f"chunk {chunk}: the sequential walk disagrees with the chunk file"
            )
    if alone != walked[last]:
        problems.append(
            f"chunk {last}: enumerated alone it differs from the sequential walk"
        )
    return problems


def outcome(result: ChunkResult, first_admitted: int) -> dict:
    """The kit's outcome object for one chunk."""
    out = {
        "cases": result.cases,
        "first_admitted": first_admitted,
        "calls": result.calls,
        "inputs_sha256": result.inputs.hex(),
    }
    if result.expected is not None:
        out["expected_sha256"] = result.expected.hex()
    return out
