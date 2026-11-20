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
import re
from collections.abc import Iterator
from dataclasses import dataclass
from itertools import islice

from .domains import Desc, fits
from .domains import build as build_descriptor
from .errors import KitError, Refusal
from .ir import Compiled, Program, recursion_guard
from .semantics import STANDARD, Semantics
from .values import make_encoder
from .wtypes import BOOL, Ty, parse_type

MAX_CHUNKS = 4096
MAX_SLIM_CHUNKS = 32
MAX_SUITE_PRODUCT = 1 << 28  # section 14: chunks bound the work, not the product
MAX_BINDERS = 8
CHUNK_CORPUS_KEYS = frozenset(
    {"chunk_size", "max_ir_cost", "types", "ops", "models", "model_order"}
)
_ROW_BASE = frozenset(
    [
        *("id", "kind", "claim", "ops", "requires", "binders", "demoted"),
        *("domain_label", "admit", "body", "oracle", "result_type", "stream"),
        *("product", "cases", "discarded", "expected_calls", "wraps", "ir_cost"),
        *("chunks", "chunks_sha256", "inputs_sha256", "slim_chunks"),
        "slim_inputs_sha256",
    ]
)
_LINE = re.compile(
    r"(0|[1-9][0-9]*) (0|[1-9][0-9]*) (0|[1-9][0-9]*) (0|[1-9][0-9]*) "
    r"([0-9a-f]{64})(?: ([0-9a-f]{64}))?"
)
HEADERS = ("warrant-stream-v1", "warrant-stream-v2")


def _corpus_error(why: str) -> Refusal:
    return Refusal("corpus", why)


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


def row_keys(row: dict) -> frozenset[str]:
    """The exact key set of a suite row (section 1.4).

    The base keys; for a ``reference`` suite also the two expected digests; for a
    suite with the v1 stream header also the v1 whole-suite digests (the second
    only for a ``reference`` suite).
    """
    keys = set(_ROW_BASE)
    if is_reference(row):
        keys |= {"expected_sha256", "slim_expected_sha256"}
    if str(row.get("stream", "")).startswith("warrant-stream-v1"):
        keys.add("v1_inputs_sha256")
        if is_reference(row):
            keys.add("v1_expected_sha256")
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


def build_suite(corpus: object, row: object, sem: Semantics = STANDARD) -> Suite:
    """Validate the ``corpus`` part and the suite ``row`` and compile the suite.

    Refused with ``corpus`` for: an unknown or missing key, an invalid
    ``chunk_size``, a ``max_ir_cost`` outside ``2,000..=65,536``, a wrong
    descriptor ``size``, a wrong ``product``, a wrong chunk count, and any IR the
    checker refuses (section 11, R45).
    """
    if not isinstance(corpus, dict) or set(corpus) != CHUNK_CORPUS_KEYS:
        raise _corpus_error("the corpus part has the wrong keys")
    chunk_size = corpus["chunk_size"]
    if (
        not is_index(chunk_size)
        or chunk_size & (chunk_size - 1)
        or not 1 << 10 <= chunk_size <= 1 << 16
    ):
        raise _corpus_error("chunk_size is a power of two in 2^10..2^16")
    cost = corpus["max_ir_cost"]
    if not is_index(cost) or not 2000 <= cost <= 65_536:
        raise _corpus_error("max_ir_cost is in 2,000..=65,536")
    if not isinstance(row, dict) or set(row) != row_keys(row):
        raise _corpus_error("the suite row has an unknown or missing key")
    try:
        return _compile_suite(corpus, row, chunk_size, sem)
    except Refusal as error:
        if error.code == "corpus":
            raise
        raise _corpus_error(f"{error.code}: {error.message}") from error
    except (KeyError, IndexError, TypeError, AttributeError, ValueError) as error:
        raise _corpus_error("a field of the suite row has the wrong shape") from error


def _compile_suite(corpus: dict, row: dict, chunk_size: int, sem: Semantics) -> Suite:
    types = corpus["types"]
    names, tys, descs = [], [], []
    product = 1
    if not 1 <= len(row["binders"]) <= MAX_BINDERS:
        raise _corpus_error("a claim has 1 to 8 binders")
    for name, binder in row["binders"]:
        desc = build_descriptor(binder["domain"], sem)
        ty = parse_type(binder["type"], types)
        if not fits(binder["domain"], ty, types):
            raise _corpus_error("a binder descriptor that is not one for its type")
        names.append(name)
        tys.append(ty)
        descs.append(desc)
        product *= desc.size
    if product > MAX_SUITE_PRODUCT:
        raise _corpus_error("the product exceeds 2^28 ordinals")
    if row["product"] != product:
        raise _corpus_error("product is not the product of the descriptor sizes")
    m = max(1, -(-product // chunk_size))
    if m > MAX_CHUNKS or row["chunks"] != m:
        raise _corpus_error("chunks is not ceil(product / chunk_size)")
    if row["stream"] not in HEADERS:
        raise _corpus_error("an unknown stream header")
    oracle = row["oracle"]
    if oracle["kind"] not in ("reference", "predicate"):
        raise _corpus_error("an unknown oracle kind")
    program = Program(
        types=types,
        ops=corpus["ops"],
        models=corpus["models"],
        model_order=corpus["model_order"],
        max_ir_cost=corpus["max_ir_cost"],
        sem=sem,
    )
    scope = dict(zip(names, tys, strict=True))
    admit = program.compile(row["admit"], scope, role="admit")
    body = program.compile(row["body"], scope, role="body")
    if admit.ty != BOOL or body.ty != BOOL:
        raise _corpus_error("admit and body are Bool expressions")
    expected = None
    if oracle["kind"] == "reference":
        expected = program.compile(oracle["expected"], scope, role="expected")
    cost = admit.cost + body.cost + (expected.cost if expected else 0)
    if type(row["ir_cost"]) is not int or row["ir_cost"] != cost:
        raise _corpus_error("ir_cost is not the sum of the worst-path costs")
    return Suite(
        row=row,
        names=names,
        types=tys,
        descs=descs,
        admit=admit,
        body=body,
        expected=expected,
        result_ty=parse_type(row["result_type"], types),
        chunk_size=chunk_size,
        m=m,
        product=product,
        context=stream_context(row, sem),
        corpus_types=types,
    )


def check_chunk_index(suite: Suite, chunk: int) -> None:
    """``chunk >= m`` is ``corpus`` (section 11, R45)."""
    if chunk >= suite.m:
        raise _corpus_error("chunk >= m")


def check_slim_chunks(row: dict, m: int) -> None:
    """``slim-chunks``: ascending, no duplicates, below ``m``, first and last in, <= 32.

    An entry that is negative or not a JSON integer is a kit error, not an outcome.
    """
    slim = row["slim_chunks"]
    for entry in slim if isinstance(slim, list) else []:
        schema_index(entry, "a slim_chunks entry")
    if (
        not isinstance(slim, list)
        or slim != sorted(set(slim))
        or not 0 < len(slim) <= MAX_SLIM_CHUNKS
        or slim[0] != 0
        or slim[-1] != m - 1
    ):
        raise Refusal("slim-chunks", "slim_chunks is not a valid slim set")


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


def check_chunk_file(
    data: bytes, suite: Suite, sem: Semantics = STANDARD
) -> list[ChunkLine]:
    """Validate a chunk file against its suite row (sections 5.5 and 14).

    ``m`` LF-terminated lines of single-space-separated canonical decimals and
    64-digit lowercase hex; the chunk index in sequence; ``first_admitted`` the
    running prefix sum of ``cases``; ``sum(cases)`` the row's ``cases`` and
    ``discarded`` the rest of the product; ``chunks_sha256`` the file's digest; the
    two suite digests the ``warrant-chunks-v1`` framing of the per-chunk digests.
    The expected column is present exactly when the suite is a ``reference`` suite.
    The row's *slim* digests are **not** compared (R46).
    """
    row = suite.row

    def bad(why: str) -> Refusal:
        return Refusal("chunk-file", why)

    if hashlib.sha256(data).hexdigest() != row["chunks_sha256"]:
        raise bad("chunks_sha256 is not the digest of the file")
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise bad("the file is not ASCII") from error
    lines = text.split("\n")
    if lines.pop() != "" or len(lines) != suite.m:
        raise bad("the file has exactly m LF-terminated lines")
    reference = is_reference(row)
    out: list[ChunkLine] = []
    running = 0
    for position, line in enumerate(lines):
        hit = _LINE.fullmatch(line)
        if not hit or (hit.group(6) is not None) != reference:
            raise bad("a line breaks the grammar of section 5.5")
        index, cases, first, calls = (int(hit.group(i)) for i in range(1, 5))
        if index != position:
            raise bad("a chunk index out of sequence")
        if first != running:
            raise bad("first_admitted is not the running prefix sum")
        running = sem.advance_admitted(running, cases, suite.chunk_size)
        out.append(ChunkLine(index, cases, first, calls, hit.group(5), hit.group(6)))
    if running != row["cases"] or row["discarded"] != row["product"] - running:
        raise bad("the cases do not sum to the row's cases")
    if (
        chunk_digest_framing([bytes.fromhex(x.inputs) for x in out], sem)
        != row["inputs_sha256"]
    ):
        raise bad("inputs_sha256 is not the framing of the chunk digests")
    if reference and (
        chunk_digest_framing([bytes.fromhex(x.expected or "") for x in out], sem)
        != row["expected_sha256"]
    ):
        raise bad("expected_sha256 is not the framing of the chunk digests")
    return out


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
        admit = None if suite.row["admit"] == ["lit", True] else suite.admit.fn
        expected_fn = suite.expected.fn if suite.expected else None
        calls = suite.body.calls
        tabulated = all(c.values is not None for c in columns)
        values = [c.values for c in columns]
        encoded = [c.encoded for c in columns]
        cases = total_calls = 0
        pending_in: list[bytes] = []
        pending_out: list[bytes] = []
        with recursion_guard():
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
                if not isinstance(calls, int):
                    total_calls += calls(env)
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
