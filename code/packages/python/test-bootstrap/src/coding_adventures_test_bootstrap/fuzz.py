"""Coverage-guided fuzz engine for Python (VER06, Python lane VER06d).

This module is the Python lane of the Level 2 fuzz engine.  It conforms to the
shared ``fuzz-v1`` corpus (``code/specs/fixtures/fuzz-v1/cases.json``) and to the
campaign contracts audited in VER06 section 9.  Everything here is a pure
computation: no file, clock, network, environment or subprocess access.  The
only ambient facility used is the interpreter's own execution-monitoring hook
(``sys.monitoring`` or ``sys.settrace``), which is how a Python lane gets
coverage without a third-party package (VER06 section 7).

Reading guide, top to bottom::

    1. limits and the small helpers every part shares
    2. the mutator algebra            (VER06 section 3, pinned by the corpus)
    3. coverage bitmap and admission   (VER06 section 4.1, 4.2)
    4. power schedule                  (VER06 section 4.3)
    5. crash signatures + private TAP  (VER06 sections 5.2, 6)
    6. outcomes, options, reports
    7. minimisation                    (VER06 section 5.3)
    8. the campaign                    (``fuzz_target``)
    9. instrumentation of plain Python (``Harness``, ``fuzz_function``)
   10. compatibility wrappers          (``fuzz_bytes``, ``fuzz_text``)

A fuzz campaign is a *pure function* of (initial corpus, options, target).  Run
it twice with the same arguments and you get the same report, byte for byte.
"""

from __future__ import annotations

import abc as _abc
import bisect
import math
import sys
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass, field, replace
from typing import Any

from coding_adventures_test_bootstrap.prng import SplitMix64, fnv1a64

# =============================================================================
# 1. Limits and shared helpers
# =============================================================================

#: CWE-400 bounds.  A fuzz campaign is attacker-reachable work (a CI job fed a
#: hostile configuration), so every count and length that drives a loop or an
#: allocation has a hard ceiling.
MAX_ITERATIONS = 1_000_000
MAX_INPUT_BYTES = 1024 * 1024
MAX_CRASHES = 1000
MAX_INITIAL_CORPUS = 10_000
MAX_STEP_BUDGET = 100_000_000
#: Hard ceiling on target executions spent minimising ONE crash, and the
#: default for ``minimize_crashing_input``.
MAX_MINIMIZE_BUDGET = 100_000
DEFAULT_MINIMIZE_BUDGET = 2000
#: Most code objects a ``Harness`` remembers (code can be generated at run time).
MAX_CODE_CACHE = 10_000
DEFAULT_STEP_BUDGET = 200_000
#: Frames kept in a crash signature (the innermost ones survive).
MAX_FRAMES = 64

#: VER06 section 4.1: 8,192 bytes = 65,536 bits = 65,536 edges.
COVERAGE_MAP_SIZE = 65_536
COVERAGE_BYTES = COVERAGE_MAP_SIZE // 8

#: The closed failure vocabulary (VER06 section 6.2).  Nothing else is ever
#: printed in a public failure line.
FUZZ_FID_CRASH = "fuzz-crash"
FUZZ_FID_TIMEOUT = "fuzz-timeout"
FUZZ_FID_OOM = "fuzz-oom"
FUZZ_FID_LEAK = "fuzz-leak"
FUZZ_FID_PANIC = "fuzz-panic"
FUZZ_FAILURE_IDS: tuple[str, ...] = (
    FUZZ_FID_CRASH,
    FUZZ_FID_TIMEOUT,
    FUZZ_FID_OOM,
    FUZZ_FID_LEAK,
    FUZZ_FID_PANIC,
)

#: Coverage tiers a report can carry (VER06 section 7).
TIER_MONITORING = "sys.monitoring"
TIER_SETTRACE = "sys.settrace"
TIER_REPORTED = "caller-reported"
TIER_BLACK_BOX = "black-box-uninstrumented"


def _check_strict_int(val: Any, name: str) -> None:
    """Reject ``bool`` and non-``int``: ``True`` is an int to Python, not to us."""
    if isinstance(val, bool) or not isinstance(val, int):
        raise TypeError(f"{name} must be an integer, not {type(val).__name__}")


def _as_bytes(data: Any, name: str) -> bytes:
    """Accept bytes-like input and return immutable ``bytes``."""
    if isinstance(data, str) or not isinstance(data, (bytes, bytearray, memoryview)):
        raise TypeError(f"{name} must be bytes, got {type(data).__name__}")
    return bytes(data)


def _below(rng: SplitMix64, n: int) -> int:
    """Uniform in ``[0, n)`` that **always consumes one stream word**.

    ``SplitMix64.bounded(1)`` returns 0 without touching the stream, but the
    reference (``code/scripts/fuzz_conformance.py``), the Rust lane and VER05
    section 2.2 all draw a word even for ``n == 1``.  The fuzz stream position
    must match across lanes, so every draw in this module goes through here.
    """
    if n == 1:
        rng.next_u64()
        return 0
    return rng.bounded(n)


_INVERT = bytes(range(255, -1, -1))  # translate table: b -> b ^ 0xFF


def is_fuzz_failure_id(candidate: object) -> bool:
    """Is ``candidate`` one of the five registered failure ids?

    ``is_fuzz_failure_id("fuzz-crash")``           -> True
    ``is_fuzz_failure_id("fuzz-arbitrary-error")`` -> False
    ``is_fuzz_failure_id(None)``                    -> False (not a string at all)
    """
    return isinstance(candidate, str) and candidate in FUZZ_FAILURE_IDS


# =============================================================================
# 2. Mutator algebra (VER06 section 3)
#
# Every mutator takes an immutable input and returns a NEW ``bytes``.  Offsets
# wrap with Python's floor-modulo, which is already non-negative for a positive
# modulus (so ``-1 % 8 == 7``): no sign juggling is needed here.
# =============================================================================

#: Interest values of section 3.4, by width in bytes.
INTEREST_8: tuple[int, ...] = (0, 1, 127, 128, 255)
INTEREST_16: tuple[int, ...] = (0, 1, 255, 256, 32767, 32768, 65535)
INTEREST_32: tuple[int, ...] = (0, 1, 65535, 65536, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF)
INTEREST_64: tuple[int, ...] = (
    0,
    1,
    0xFFFFFFFF,
    0x100000000,
    0x7FFFFFFFFFFFFFFF,
    0x8000000000000000,
    0xFFFFFFFFFFFFFFFF,
)
INTEREST_BY_WIDTH: dict[int, tuple[int, ...]] = {
    1: INTEREST_8,
    2: INTEREST_16,
    4: INTEREST_32,
    8: INTEREST_64,
}
_WIDTHS = frozenset(INTEREST_BY_WIDTH)


def _parity_flip(data: bytes, unit_bits: int, offset: int, count: int) -> bytes:
    """Invert ``count`` consecutive *units* (bits or bytes) starting at ``offset``.

    ``unit_bits`` is 1 for bit flips and 8 for byte flips.  The window wraps
    around the buffer, so with ``n`` units a window of ``count`` covers every
    unit ``count // n`` times and the first ``count % n`` units (counting from
    the wrapped start) one extra time.  A unit flipped an even number of times
    is back where it started (XOR twice), so only the *parity* of each unit's
    flips matters::

        flips(step) = count // n + (1 if step < count % n else 0)

    That is O(len(data)) work for ANY count, where a loop of ``count`` single
    flips would never terminate for ``count = 10**18`` (VER06 section 9.2).

    Worked example, 2 bytes = 16 bits, ``bit_flip(offset=0, count=17)``:
    ``17 // 16 = 1`` (every bit once, odd) and ``17 % 16 = 1`` (bit 0 once
    more, so twice, even): the result inverts bits 1..15 and leaves bit 0.
    """
    if not data or count <= 0:
        return data
    n = len(data) * (8 if unit_bits == 1 else 1)
    base = offset % n
    full, rem = divmod(count, n)
    # If every unit was hit an odd number of times, start from "all flipped";
    # then toggle the ``rem`` extra units.  XOR is commutative, so the order
    # in which we apply the two layers does not matter.
    buf = bytearray(data.translate(_INVERT) if full % 2 else data)
    for step in range(rem):
        pos = (base + step) % n
        if unit_bits == 1:
            buf[pos // 8] ^= 1 << (pos % 8)
        else:
            buf[pos] ^= 0xFF
    return bytes(buf)


def bit_flip(data: bytes, bit_offset: int, count: int = 1) -> bytes:
    """Invert ``count`` consecutive bits from ``bit_offset mod (8 * len)``.

    Bits are numbered LSB-first within each byte: bit 9 of ``00 00`` is bit 1
    of byte 1, so ``bit_flip(b"\\x00\\x00", 9)`` is ``00 02``.
    """
    _check_strict_int(bit_offset, "bit_offset")
    _check_strict_int(count, "count")
    return _parity_flip(_as_bytes(data, "data"), 1, bit_offset, count)


def byte_flip(data: bytes, byte_offset: int, count: int = 1) -> bytes:
    """XOR ``count`` consecutive bytes with 0xFF from ``byte_offset mod len``."""
    _check_strict_int(byte_offset, "byte_offset")
    _check_strict_int(count, "count")
    return _parity_flip(_as_bytes(data, "data"), 8, byte_offset, count)


def _byteorder(endian: str) -> str:
    """Fail closed on anything but the two spellings the corpus uses."""
    if endian not in ("little", "big"):
        raise ValueError(f"endian must be 'little' or 'big', got {endian!r}")
    return endian


def byte_arithmetic(
    data: bytes,
    byte_offset: int,
    delta: int,
    width: int = 1,
    endian: str = "little",
) -> bytes:
    """Add signed ``delta`` modulo 2^(8*width) to the integer at the offset.

    ``width`` is in {1, 2, 4, 8}; any other width, or ``len(data) < width``,
    returns the input unchanged.  The offset wraps over the ``len - width + 1``
    positions where a whole integer fits.

    Worked example (u16, little-endian): ``ff 00`` is 0x00ff; adding 1 gives
    0x0100, stored little-endian as ``00 01``.
    """
    _check_strict_int(byte_offset, "byte_offset")
    _check_strict_int(delta, "delta")
    _check_strict_int(width, "width")
    order = _byteorder(endian)
    raw = _as_bytes(data, "data")
    if width not in _WIDTHS or len(raw) < width:
        return raw
    buf = bytearray(raw)
    at = byte_offset % (len(buf) - width + 1)
    old = int.from_bytes(buf[at : at + width], order)
    new = (old + delta) % (1 << (8 * width))
    buf[at : at + width] = new.to_bytes(width, order)
    return bytes(buf)


def boundary_insert(
    data: bytes,
    byte_offset: int,
    width: int,
    val: int,
    endian: str = "little",
) -> bytes:
    """Overwrite ``width`` bytes with ``val mod 2^(8*width)`` at the offset.

    Same width and short-buffer rules as :func:`byte_arithmetic`.  Worked
    example: ``boundary_insert(00 00 00 00, 0, 4, 0x80000000, "little")`` is
    ``00 00 00 80``.
    """
    _check_strict_int(byte_offset, "byte_offset")
    _check_strict_int(width, "width")
    _check_strict_int(val, "val")
    order = _byteorder(endian)
    raw = _as_bytes(data, "data")
    if width not in _WIDTHS or len(raw) < width:
        return raw
    buf = bytearray(raw)
    at = byte_offset % (len(buf) - width + 1)
    buf[at : at + width] = (val % (1 << (8 * width))).to_bytes(width, order)
    return bytes(buf)


def block_delete(data: bytes, offset: int, length: int) -> bytes:
    """Delete up to ``length`` bytes at ``offset mod len``; clamp at the end."""
    _check_strict_int(offset, "offset")
    _check_strict_int(length, "length")
    raw = _as_bytes(data, "data")
    if not raw or length <= 0:
        return raw
    at = offset % len(raw)
    return raw[:at] + raw[at + min(length, len(raw) - at) :]


def block_insert(data: bytes, offset: int, insert_bytes: bytes) -> bytes:
    """Insert at ``offset mod (len + 1)``; an empty input becomes the insert."""
    _check_strict_int(offset, "offset")
    raw = _as_bytes(data, "data")
    ins = _as_bytes(insert_bytes, "insert_bytes")
    if not raw:
        return ins
    at = offset % (len(raw) + 1)
    return raw[:at] + ins + raw[at:]


def block_duplicate(
    data: bytes, src_offset: int, length: int, dst_offset: int
) -> bytes:
    """Copy up to ``length`` bytes from ``src_offset mod len`` to ``dst_offset``.

    The destination wraps modulo ``len + 1`` (the end of the buffer is a valid
    insertion point); the copy is clamped at the end of the source.
    """
    _check_strict_int(src_offset, "src_offset")
    _check_strict_int(length, "length")
    _check_strict_int(dst_offset, "dst_offset")
    raw = _as_bytes(data, "data")
    if not raw or length <= 0:
        return raw
    src = src_offset % len(raw)
    chunk = raw[src : src + min(length, len(raw) - src)]
    dst = dst_offset % (len(raw) + 1)
    return raw[:dst] + chunk + raw[dst:]


def splice(target: bytes, donor: bytes, target_split: int, donor_split: int) -> bytes:
    """``target[:ts] + donor[ds:]`` with both splits clamped into their buffers."""
    _check_strict_int(target_split, "target_split")
    _check_strict_int(donor_split, "donor_split")
    tgt = _as_bytes(target, "target")
    don = _as_bytes(donor, "donor")
    t = max(0, min(target_split, len(tgt)))
    d = max(0, min(donor_split, len(don)))
    return tgt[:t] + don[d:]


_DELTAS = (-35, -8, -1, 1, 8, 35)


def mutate_sequence(
    data: bytes, seed: int, steps: int, max_len: int = MAX_INPUT_BYTES
) -> bytes:
    """``steps`` seeded mutations, each chosen by one ``bounded(7)`` draw.

    ====  =========================================================
    draw  step
    ====  =========================================================
    0     bit_flip, count 1
    1     byte_flip, count 1
    2     byte_arithmetic, width 1, delta from (-35 -8 -1 1 8 35)
    3     boundary_insert, width 1, value from (0 1 127 128 255)
    4     block_delete (length 1..len//2); skipped if len <= 1
    5     block_insert of one random byte
    6     block_duplicate (length 1..len//2); skipped if len <= 1
    ====  =========================================================

    An empty buffer is refilled to ``00`` before every step.  The draw order is
    the cross-lane contract: the same ``(data, seed, steps)`` must give the same
    bytes in every language.

    ``max_len`` is VER06 section 3.5's "maximum length budget": a block insert or
    duplicate that would grow the buffer past it is *skipped after* its draws
    are consumed, so the stream stays aligned and memory stays bounded (each
    duplicate can add len/2, so 64 of them would otherwise be exponential).
    """
    _check_strict_int(seed, "seed")
    _check_strict_int(steps, "steps")
    _check_strict_int(max_len, "max_len")
    if steps > MAX_ITERATIONS:
        raise ValueError(f"steps exceeds MAX_ITERATIONS ({MAX_ITERATIONS})")
    rng = SplitMix64(seed)
    current = _as_bytes(data, "data")
    for _ in range(max(0, steps)):
        if not current:
            current = b"\x00"
        choice = rng.bounded(7)
        n = len(current)
        if choice == 0:
            current = bit_flip(current, _below(rng, n * 8), 1)
        elif choice == 1:
            current = byte_flip(current, _below(rng, n), 1)
        elif choice == 2:
            delta = _DELTAS[rng.bounded(len(_DELTAS))]
            current = byte_arithmetic(current, _below(rng, n), delta, 1)
        elif choice == 3:
            val = INTEREST_8[rng.bounded(len(INTEREST_8))]
            current = boundary_insert(current, _below(rng, n), 1, val)
        elif choice == 4:
            if n > 1:
                del_len = 1 + _below(rng, max(1, n // 2))
                current = block_delete(current, _below(rng, n), del_len)
        elif choice == 5:
            ins = bytes([rng.bounded(256)])
            at = _below(rng, n + 1)
            if n + 1 <= max_len:
                current = block_insert(current, at, ins)
        elif n > 1:
            dup_len = 1 + _below(rng, max(1, n // 2))
            src = _below(rng, n)
            dst = _below(rng, n + 1)
            if n + min(dup_len, n - src) <= max_len:
                current = block_duplicate(current, src, dup_len, dst)
    return current


# =============================================================================
# 3. Coverage bitmap and corpus admission (VER06 sections 4.1, 4.2)
# =============================================================================


def corpus_admit(
    existing_edges: Iterable[int], trace_edges: Iterable[int]
) -> tuple[bool, int]:
    """Pure novelty check: does the trace hold an edge absent from ``existing``?

    Returns ``(admitted, new_edges)`` where ``new_edges`` counts *unique* novel
    edges (a trace that repeats one novel edge three times still adds one).

    ``corpus_admit([1, 2, 3], [2, 4])``  -> ``(True, 1)``    (edge 4 is new)
    ``corpus_admit([1, 2, 3], [1, 2])``  -> ``(False, 0)``
    """
    new = set(trace_edges) - set(existing_edges)
    return (True, len(new)) if new else (False, 0)


class CoverageBitmap:
    """A 65,536-bit execution map; bit ``e`` set means edge ``e`` was executed.

    Edge ids are reduced modulo 65,536 (``edge % 65536``).  Collisions are the
    known price of a fixed-size map; AFL pays the same one.
    """

    __slots__ = ("_edges", "_map")

    def __init__(self) -> None:
        self._map = bytearray(COVERAGE_BYTES)
        # The set of masked edge ids, kept beside the map so that listing the
        # edges of one execution costs O(edges hit), not O(8192).
        self._edges: set[int] = set()

    def record_edge(self, edge: int) -> None:
        _check_strict_int(edge, "edge")
        e = edge % COVERAGE_MAP_SIZE
        self._map[e // 8] |= 1 << (e % 8)
        self._edges.add(e)

    def has_edge(self, edge: int) -> bool:
        _check_strict_int(edge, "edge")
        e = edge % COVERAGE_MAP_SIZE
        return bool(self._map[e // 8] & (1 << (e % 8)))

    def admit_trace(self, trace: Iterable[int]) -> tuple[bool, int]:
        """OR the trace into this (cumulative) map; return ``(has_new, count_new)``."""
        count_new = 0
        for edge in trace:
            _check_strict_int(edge, "edge")
            e = edge % COVERAGE_MAP_SIZE
            mask = 1 << (e % 8)
            if not self._map[e // 8] & mask:
                self._map[e // 8] |= mask
                self._edges.add(e)
                count_new += 1
        return count_new > 0, count_new

    def covered_edges(self) -> list[int]:
        """Every covered edge, ascending."""
        return sorted(self._edges)

    def count(self) -> int:
        """Number of distinct covered edges (a population count)."""
        return int.from_bytes(self._map, "little").bit_count()

    def merge(self, other: CoverageBitmap) -> None:
        """OR another bitmap into this one."""
        for i, byte in enumerate(other._map):
            self._map[i] |= byte
        self._edges |= other._edges

    @property
    def raw(self) -> bytes:
        """The 8,192 bytes of the map (LSB-first within each byte)."""
        return bytes(self._map)


# =============================================================================
# 4. Power schedule (VER06 section 4.3)
# =============================================================================


def calculate_energy(
    base_energy: int, seed_len: int, new_edges: int, total_edges: int
) -> int:
    """Mutation budget: ``max(1, round(base * (1 + new/total) / sqrt(len)))``.

    Short seeds that found a large share of the coverage get the most energy.
    The edge bonus is 1.0 while nothing is covered (``total_edges == 0``).

    Rounding is half *away from zero* (``floor(x + 0.5)``), as in Rust and
    Dart.  Python's ``round`` would be banker's rounding and disagree on exact
    halves, which would silently desynchronise the lanes' campaigns.

    Worked example: base 10, len 4, 3 new of 6 total edges:
    ``10 * 1.5 / 2 = 7.5`` rounds to ``8``.
    """
    base = float(max(1, base_energy))
    bonus = 1.0 + new_edges / total_edges if total_edges > 0 else 1.0
    length_factor = math.sqrt(max(1, seed_len))
    return max(1, math.floor((base * bonus) / length_factor + 0.5))


# =============================================================================
# 5. Crash signatures and private reporting (VER06 sections 5.2, 6)
# =============================================================================


def crash_signature(kind: str, frames: Sequence[str]) -> str:
    """Low 32 bits of ``fnv1a64(kind ":" frames joined by ":")`` as 8 lowercase hex.

    Frames are *not* escaped (VER06 section 9.4 item 6): one frame ``x:y`` and
    two frames ``x``, ``y`` hash alike.  This lane never produces a frame with
    a colon, so the ambiguity cannot arise from Python targets.
    """
    if not isinstance(kind, str):
        raise TypeError("kind must be a string")
    if isinstance(frames, str) or not all(isinstance(f, str) for f in frames):
        raise TypeError("frames must be a sequence of strings")
    raw = f"{kind}:" + ":".join(frames)
    return f"{fnv1a64(raw) & 0xFFFFFFFF:08x}"


def is_duplicate_crash(
    kind_a: str, frames_a: Sequence[str], kind_b: str, frames_b: Sequence[str]
) -> bool:
    """Do two crash reports share one signature?"""
    return crash_signature(kind_a, frames_a) == crash_signature(kind_b, frames_b)


def _token(text: object, allowed: Callable[[str], bool]) -> str:
    return "".join(c if allowed(c) else "_" for c in str(text))


def _is_id_char(c: str) -> bool:
    return (c.isascii() and c.isalnum()) or c in "._/-"


def _is_hex_char(c: str) -> bool:
    return c in "0123456789abcdef"


def format_tap_failure(
    test_num: int, target_id: str, failure_id: str, sig_hash: str
) -> str:
    """The only public failure line: ``not ok N - target [failure-id:hex8]``.

    It carries no input bytes, seed, message or stack trace (VER00 7.3).
    ``failure_id`` must be one of the five registered ids (``ValueError``
    otherwise).  The target and hash are reduced to the characters their
    grammar allows (target ``[A-Za-z0-9._/-]``, hash lowercase hex), every
    other character becoming ``_``.  Without that, a target id holding a
    newline would let a caller forge a second TAP line (``ok 2 - forged``)
    inside "an opaque token".
    Well-formed arguments pass through unchanged, so the pinned vectors hold.
    """
    _check_strict_int(test_num, "test_num")
    if not is_fuzz_failure_id(failure_id):
        raise ValueError("failure_id must be one of the registered FUZZ_FAILURE_IDS")
    return (
        f"not ok {test_num} - {_token(target_id, _is_id_char)} "
        f"[{_token(failure_id, _is_id_char)}:{_token(sig_hash, _is_hex_char)}]"
    )


# =============================================================================
# 6. Outcomes, options, reports
# =============================================================================


@dataclass(frozen=True)
class OutcomePass:
    """The target returned normally."""


@dataclass(frozen=True)
class OutcomeCrash:
    """The target failed with ``kind`` and the normalised ``frames``."""

    kind: str
    frames: tuple[str, ...] = ()


@dataclass(frozen=True)
class OutcomeTimeout:
    """The target exceeded its (deterministic) step budget."""


@dataclass(frozen=True)
class OutcomeOom:
    """The target ran out of memory (``MemoryError``)."""


_OUTCOMES = (OutcomePass, OutcomeCrash, OutcomeTimeout, OutcomeOom)
FuzzOutcome = OutcomePass | OutcomeCrash | OutcomeTimeout | OutcomeOom


@dataclass(frozen=True)
class FuzzOptions:
    """Bounds of a campaign; every field is validated (CWE-400).

    ``minimize_budget`` is the most target executions spent minimising ONE
    novel crash (ceiling ``MAX_MINIMIZE_BUDGET``).  On top of it the whole
    campaign spends at most ``max_iterations`` minimisation executions in
    total, so minimisation can at most double a campaign's work, however many
    distinct crashes it finds.  A crash found after that total is spent is
    still recorded, with ``CrashRecord.minimized == False``.  The Rust lane
    has neither cap (recorded in VER06d).
    """

    max_iterations: int = 1000
    seed: int = 42
    max_input_len: int = 4096
    base_energy: int = 10
    minimize_budget: int = 2000

    def __post_init__(self) -> None:
        for name in (
            "max_iterations",
            "seed",
            "max_input_len",
            "base_energy",
            "minimize_budget",
        ):
            _check_strict_int(getattr(self, name), name)
        if not 0 <= self.max_iterations <= MAX_ITERATIONS:
            raise ValueError(f"max_iterations must be in [0, {MAX_ITERATIONS}]")
        if not 0 <= self.max_input_len <= MAX_INPUT_BYTES:
            raise ValueError(f"max_input_len must be in [0, {MAX_INPUT_BYTES}]")
        if not 0 <= self.base_energy <= MAX_ITERATIONS:
            raise ValueError(f"base_energy must be in [0, {MAX_ITERATIONS}]")
        if not 0 <= self.minimize_budget <= MAX_MINIMIZE_BUDGET:
            raise ValueError(f"minimize_budget must be in [0, {MAX_MINIMIZE_BUDGET}]")


@dataclass
class CrashRecord:
    """One deduplicated crash.

    ``minimal_input`` is the shortest input known to reproduce the signature.
    It is PRIVATE (VER06 section 6.3): it is excluded from ``repr`` and must
    never reach a public log; public output goes through
    :func:`format_tap_failure` only.
    """

    signature: str
    kind: str
    frames: tuple[str, ...]
    minimal_input: bytes = field(repr=False)
    hits: int = 1
    #: False when minimisation was skipped or cut short by a budget, so
    #: ``minimal_input`` is only the shortest input seen, not a fixed point.
    minimized: bool = True


@dataclass(frozen=True)
class FuzzReport:
    """What a campaign found.  Every execution lands in exactly one bucket.

    ``iterations`` counts *mutation* executions (triage is not counted);
    ``crashes``/``timeouts``/``ooms`` count triage and mutation executions;
    anything that passed is in the corpus or was redundant.
    """

    iterations: int
    admitted_seeds: int
    total_edges: int
    crashes: list[CrashRecord]
    timeouts: int = 0
    ooms: int = 0
    #: Novel signatures dropped after ``MAX_CRASHES`` distinct ones were held.
    crashes_dropped: int = 0
    coverage_tier: str = TIER_BLACK_BOX
    seed: int = field(default=42, repr=False)  # seeds are never public (VER06 s6)
    #: Admitted seeds (PRIVATE, like ``CrashRecord.minimal_input``).
    corpus: tuple[bytes, ...] = field(default=(), repr=False)

    @property
    def is_clean(self) -> bool:
        """Can this report be read as "the campaign found nothing"?

        Only if at least one mutation iteration ran and no execution, triage
        included, crashed, timed out or ran out of memory.  A campaign that
        never got past triage, or ran zero iterations, fuzzed nothing.
        """
        return (
            self.iterations > 0
            and not self.crashes
            and self.timeouts == 0
            and self.ooms == 0
            and self.crashes_dropped == 0
        )

    # Names the pre-0.3 ``FuzzResult`` exposed; kept as read-only aliases.
    @property
    def iterations_run(self) -> int:
        return self.iterations

    @property
    def corpus_size(self) -> int:
        return self.admitted_seeds

    @property
    def unique_crashes(self) -> int:
        return len(self.crashes) + self.crashes_dropped


#: Back-compatible name of the report type.
FuzzResult = FuzzReport


def tap_failures(report: FuzzReport, target_id: str, first_test: int = 1) -> list[str]:
    """Public, opaque failure lines for a report (none for a clean one).

    One line per distinct crash signature, plus one for timeouts and one for
    OOMs.  A timeout or OOM has no stack to hash, so its ``sig_hash`` is the
    signature of the bare kind (``crash_signature("timeout", [])``).
    """
    lines = [
        format_tap_failure(first_test + i, target_id, FUZZ_FID_CRASH, rec.signature)
        for i, rec in enumerate(report.crashes)
    ]
    for count, fid, kind in (
        (report.timeouts, FUZZ_FID_TIMEOUT, "timeout"),
        (report.ooms, FUZZ_FID_OOM, "oom"),
    ):
        if count:
            sig = crash_signature(kind, [])
            lines.append(
                format_tap_failure(first_test + len(lines), target_id, fid, sig)
            )
    return lines


# =============================================================================
# 7. Minimisation (VER06 section 5.3)
# =============================================================================


def _same_crash(outcome: object, target_sig: str) -> bool:
    return (
        isinstance(outcome, OutcomeCrash)
        and crash_signature(outcome.kind, outcome.frames) == target_sig
    )


def _minimize(
    data: bytes,
    target_sig: str,
    test_fn: Callable[[bytes], FuzzOutcome],
    budget: int,
) -> tuple[bytes, int, bool]:
    """Minimise; return ``(bytes, executions spent, finished)``.

    ``finished`` is False when the budget stopped the search early.
    """
    current = _as_bytes(data, "data")
    spent = 0

    def reproduces(candidate: bytes) -> bool:
        nonlocal spent
        spent += 1
        return _same_crash(test_fn(candidate), target_sig)

    if not current:
        return current, 0, True
    chunk = max(1, len(current) // 2)
    while chunk > 0:
        offset = 0
        while offset < len(current):
            if spent >= budget:
                return current, spent, False
            end = offset + min(chunk, len(current) - offset)
            candidate = current[:offset] + current[end:]
            if reproduces(candidate):
                current = candidate  # same offset again: the tail slid left
                continue
            offset += chunk
        chunk //= 2
    buf = bytearray(current)
    for i in range(len(buf)):
        if buf[i] == 0:
            continue
        if spent >= budget:
            return bytes(buf), spent, False
        original = buf[i]
        buf[i] = 0
        if not reproduces(bytes(buf)):
            buf[i] = original
    return bytes(buf), spent, True


def minimize_crashing_input(
    data: bytes,
    target_sig: str,
    test_fn: Callable[[bytes], FuzzOutcome],
    budget: int = DEFAULT_MINIMIZE_BUDGET,
) -> bytes:
    """Shrink ``data`` while it keeps the **exact** crash signature.

    Pass 1 deletes blocks, halving the block size each round; pass 2 zeroes
    individual non-essential bytes.  A candidate that crashes *differently*
    (another signature) is rejected, so minimisation never trades one bug for
    another.  Pass 1 starts at ``max(1, len // 2)``: with a one-byte input
    ``len // 2`` is 0 and the empty input would never be tried.

    ``budget`` caps the number of ``test_fn`` executions; it is always finite
    (default ``DEFAULT_MINIMIZE_BUDGET``, ceiling ``MAX_MINIMIZE_BUDGET``).
    """
    _check_strict_int(budget, "budget")
    if not 0 <= budget <= MAX_MINIMIZE_BUDGET:
        raise ValueError(f"budget must be in [0, {MAX_MINIMIZE_BUDGET}]")
    return _minimize(data, target_sig, test_fn, budget)[0]


# =============================================================================
# 8. The campaign (VER06 section 7, contracts of section 9.1)
# =============================================================================


@dataclass
class _Seed:
    data: bytes
    new_edges: int
    generation: int


class _Campaign:
    """Mutable state of one ``fuzz_target`` run (kept off the public surface)."""

    def __init__(
        self,
        target: Callable[[bytes, CoverageBitmap], FuzzOutcome],
        opts: FuzzOptions,
    ) -> None:
        self.target = target
        self.opts = opts
        self.rng = SplitMix64(opts.seed)
        self.cumulative = CoverageBitmap()
        self.corpus: list[_Seed] = []
        self.crashes: list[CrashRecord] = []
        self.timeouts = 0
        self.ooms = 0
        self.dropped = 0
        # Campaign-wide minimisation allowance (see FuzzOptions).
        self.minimize_left = opts.max_iterations

    def execute(self, data: bytes) -> tuple[FuzzOutcome, CoverageBitmap]:
        step = CoverageBitmap()
        outcome = self.target(data, step)
        if not isinstance(outcome, _OUTCOMES):
            raise TypeError(
                f"target must return a FuzzOutcome, got {type(outcome).__name__}"
            )
        return outcome, step

    def probe(self, data: bytes) -> FuzzOutcome:
        """Re-run for minimisation: coverage is discarded, nothing is counted."""
        return self.execute(data)[0]

    def file(self, outcome: FuzzOutcome, data: bytes) -> None:
        """Put one non-passing execution in its bucket."""
        if isinstance(outcome, OutcomeTimeout):
            self.timeouts += 1
        elif isinstance(outcome, OutcomeOom):
            self.ooms += 1
        elif isinstance(outcome, OutcomeCrash):
            self._record_crash(outcome, data)

    def admit(self, step: CoverageBitmap, data: bytes, generation: int) -> bool:
        has_new, new_count = self.cumulative.admit_trace(step.covered_edges())
        # The first passing seed is always admitted, even with no coverage, so
        # there is something to mutate in black-box mode.
        if has_new or (generation == 0 and not self.corpus):
            self.corpus.append(_Seed(data, new_count, generation))
            return True
        return False

    def _record_crash(self, outcome: OutcomeCrash, data: bytes) -> None:
        sig = crash_signature(outcome.kind, outcome.frames)
        for existing in self.crashes:
            if existing.signature == sig:
                existing.hits += 1
                if len(data) < len(existing.minimal_input):
                    existing.minimal_input = data
                return
        if len(self.crashes) >= MAX_CRASHES:
            self.dropped += 1
            return
        allowance = min(self.opts.minimize_budget, self.minimize_left)
        minimal, spent, finished = _minimize(data, sig, self.probe, allowance)
        self.minimize_left -= spent
        self.crashes.append(
            CrashRecord(sig, outcome.kind, tuple(outcome.frames), minimal, 1, finished)
        )


def fuzz_target(
    initial_corpus: Sequence[bytes],
    options: FuzzOptions | None,
    target: Callable[[bytes, CoverageBitmap], FuzzOutcome],
) -> FuzzReport:
    """Run a bounded coverage-guided campaign.

    ``target(data, cov)`` runs the code under test on ``data`` and returns an
    outcome.  It reports the edges it executed by calling ``cov.record_edge``
    (a caller-supplied edge reporter); :func:`fuzz_function` does that for
    plain Python functions by instrumenting them.

    Contracts (VER06 section 9.1), each pinned by a test:

    * a pure function of (corpus, options, target): same inputs, same report;
    * with a live corpus ``iterations == options.max_iterations`` exactly, and
      no mutated candidate exceeds ``options.max_input_len`` (initial seeds are
      run as given);
    * an empty corpus is replaced by one ``00`` seed; seeds that crash, time
      out or exhaust memory are never mutated, so a corpus of crashers fuzzes
      nothing (``iterations == 0``);
    * every execution lands in ``crashes``, ``timeouts``, ``ooms`` or the
      corpus; ``is_clean`` needs at least one iteration and no failure at all;
    * one ``CrashRecord`` per signature; a novel crash is minimised.
    """
    opts = FuzzOptions() if options is None else options
    if not isinstance(opts, FuzzOptions):
        raise TypeError("options must be a FuzzOptions")
    if isinstance(initial_corpus, (str, bytes, bytearray)) or not isinstance(
        initial_corpus, Sequence
    ):
        raise TypeError("initial_corpus must be a sequence of bytes")
    if len(initial_corpus) > MAX_INITIAL_CORPUS:
        raise ValueError(f"initial_corpus exceeds limit of {MAX_INITIAL_CORPUS}")
    seeds = [_as_bytes(s, f"initial_corpus[{i}]") for i, s in enumerate(initial_corpus)]
    for i, s in enumerate(seeds):
        if len(s) > MAX_INPUT_BYTES:
            raise ValueError(f"initial_corpus[{i}] exceeds MAX_INPUT_BYTES")
    if not seeds:
        seeds = [b"\x00"]

    camp = _Campaign(target, opts)

    # --- 1. triage: run every seed once --------------------------------------
    for data in seeds:
        outcome, step = camp.execute(data)
        if isinstance(outcome, OutcomePass):
            camp.admit(step, data, 0)
        else:
            camp.file(outcome, data)

    # --- 2. the mutation loop -------------------------------------------------
    iterations = 0
    rng = camp.rng
    while iterations < opts.max_iterations and camp.corpus:
        seed = camp.corpus[_below(rng, len(camp.corpus))]
        energy = calculate_energy(
            opts.base_energy, len(seed.data), seed.new_edges, camp.cumulative.count()
        )
        for _ in range(energy):
            if iterations >= opts.max_iterations:
                break
            iterations += 1
            steps = 1 + rng.bounded(4)
            sub_seed = rng.next_u64()
            candidate = mutate_sequence(seed.data, sub_seed, steps)
            if len(camp.corpus) > 1 and rng.bounded(4) == 0:
                donor = camp.corpus[_below(rng, len(camp.corpus))].data
                t_split = _below(rng, len(candidate) + 1)
                d_split = _below(rng, len(donor) + 1)
                candidate = splice(candidate, donor, t_split, d_split)
            candidate = candidate[: opts.max_input_len]

            outcome, step = camp.execute(candidate)
            if isinstance(outcome, OutcomePass):
                camp.admit(step, candidate, seed.generation + 1)
            else:
                camp.file(outcome, candidate)

    total = camp.cumulative.count()
    return FuzzReport(
        iterations=iterations,
        admitted_seeds=len(camp.corpus),
        total_edges=total,
        crashes=camp.crashes,
        timeouts=camp.timeouts,
        ooms=camp.ooms,
        crashes_dropped=camp.dropped,
        coverage_tier=TIER_REPORTED if total > 0 else TIER_BLACK_BOX,
        seed=opts.seed,
        corpus=tuple(s.data for s in camp.corpus),
    )


# =============================================================================
# 9. Coverage of plain Python code (VER06 section 7, Python row)
#
# Two mechanisms, same edge ids:
#
#   sys.monitoring (3.12+)  LINE and BRANCH events, switched on only while the
#                           target runs, returning DISABLE for code we do not
#                           want so each such location costs one event, ever.
#   sys.settrace            the fallback: a line tracer that records
#                           (previous line, line) pairs per frame.
#
# An edge id is FNV-1a of ``"<file basename>:<qualname>:<from line>:<to line>"``
# reduced mod 65,536.  Nothing in it depends on ``id()``, memory addresses,
# absolute paths or hash randomisation, so a campaign finds the same edges on
# every run and every machine.  (Two files with the same basename, qualname and
# lines collide; with 16 bits of map the collision is already accepted.)
# =============================================================================

_THIS_FILE = __file__
_PACKAGE_DIR = _THIS_FILE.replace("\\", "/").rsplit("/", 1)[0]
_STDLIB_DIR = (_abc.__file__ or "").replace("\\", "/").rsplit("/", 1)[0]


class _StepBudgetExceeded(BaseException):
    """Raised into the target when it exceeds its step budget.

    A ``BaseException`` so a target's ``except Exception`` cannot swallow it.
    (A bare ``except:`` can: the budget keeps firing on every later line, but a
    target that loops on a bare except can still hang the campaign.  Recorded
    as a limit in VER06d.)
    """


def default_scope(filename: str, qualname: str) -> bool:
    """Which code counts towards coverage and the step budget by default.

    Excluded: module top-level code (``<module>`` runs at import time, so it
    would make the first execution of a campaign differ from the rest),
    synthetic files (``<frozen ...>``, ``<string>``), this package, and the
    standard library.  Everything else, including site-packages, is in scope.
    """
    if qualname == "<module>" or filename.startswith("<"):
        return False
    path = filename.replace("\\", "/")
    if path.startswith(_PACKAGE_DIR):
        return False
    in_stdlib = path.startswith(_STDLIB_DIR)
    third_party = "site-packages" in path or "dist-packages" in path
    return not (in_stdlib and not third_party)


class _CodeInfo:
    """Per-code-object facts, computed once per campaign."""

    __slots__ = ("_edges", "_lines", "_starts", "code", "prefix", "scope")

    def __init__(self, code: Any, scope: bool) -> None:
        self.code = code  # keeps the object alive so ``id(code)`` stays unique
        self.scope = scope
        name = getattr(code, "co_qualname", code.co_name)
        base = code.co_filename.replace("\\", "/").rsplit("/", 1)[-1]
        self.prefix = f"{base}:{name}"
        self._edges: dict[tuple[int, int], int] = {}
        self._starts: list[int] | None = None
        self._lines: list[int] = []

    def line_of(self, offset: int) -> int:
        """Source line of a bytecode offset (for BRANCH destinations)."""
        if self._starts is None:
            table = sorted(
                (start, line) for start, _end, line in self.code.co_lines() if line
            )
            self._starts = [start for start, _ in table]
            self._lines = [line for _, line in table]
        if not self._lines:
            return 0
        return self._lines[max(bisect.bisect_right(self._starts, offset) - 1, 0)]

    def edge(self, a: int, b: int) -> int:
        key = (a, b)
        e = self._edges.get(key)
        if e is None:
            e = fnv1a64(f"{self.prefix}:{a}:{b}") % COVERAGE_MAP_SIZE
            self._edges[key] = e
        return e


#: Shared stand-in for every out-of-scope code object (never stored per code).
_OUT_OF_SCOPE = _CodeInfo(compile("", "<out-of-scope>", "exec"), False)


def _frame_label(text: str) -> str:
    """A frame or kind with nothing outside ``[A-Za-z0-9._<>$/-]``, bounded."""
    keep = "._<>$/-"
    return "".join(
        c if (c.isascii() and c.isalnum()) or c in keep else "_" for c in text
    )[:200]


def frames_of(tb: Any) -> tuple[str, ...]:
    """Normalise a traceback into signature frames.

    Each frame is ``<module>.<qualname>``: no absolute path (machines differ),
    no line number (edits shift it, and a signature should survive a reformat).
    Frames of this module are dropped (they are the harness, not the target);
    runs of identical frames collapse to one so a ``RecursionError`` hashes the
    same at any depth; at most the innermost ``MAX_FRAMES`` survive.
    """
    out: list[str] = []
    while tb is not None:
        code = tb.tb_frame.f_code
        if code.co_filename != _THIS_FILE:
            module = tb.tb_frame.f_globals.get("__name__", "?")
            name = getattr(code, "co_qualname", code.co_name)
            label = _frame_label(f"{module}.{name}")
            if not out or out[-1] != label:
                out.append(label)
        tb = tb.tb_next
    return tuple(out[-MAX_FRAMES:])


_COVERAGE_MODES = ("auto", "monitoring", "settrace", "reporter", "none")


class Harness:
    """Runs a plain function as a fuzz target and classifies what happens.

    Use it as a context manager (``with Harness(fn) as target: ...``); the
    context claims a ``sys.monitoring`` tool id for its lifetime and gives it
    back.  The instance is itself the ``target(data, cov)`` callable that
    :func:`fuzz_target` expects.

    ``coverage`` picks the mechanism:

    ``"auto"``        sys.monitoring if available and a tool id is free, else
                      sys.settrace
    ``"monitoring"``  sys.monitoring or ``RuntimeError``
    ``"settrace"``    force the fallback
    ``"reporter"``    ``fn(data, cov)`` reports its own edges into ``cov``
    ``"none"``        no coverage: black-box mode

    Timeouts come from the step budget (traced lines), so they exist only in
    the two traced modes.  Outcomes: ``Exception`` or ``SystemExit`` is a
    crash, ``MemoryError`` is OOM, budget exhaustion is a timeout.
    ``KeyboardInterrupt`` and other ``BaseException`` propagate.
    """

    def __init__(
        self,
        fn: Callable[..., Any],
        *,
        coverage: str = "auto",
        step_budget: int = DEFAULT_STEP_BUDGET,
        scope: Callable[[str, str], bool] | None = None,
    ) -> None:
        if not callable(fn):
            raise TypeError("fn must be callable")
        if coverage not in _COVERAGE_MODES:
            raise ValueError(f"coverage must be one of {_COVERAGE_MODES}")
        _check_strict_int(step_budget, "step_budget")
        if not 1 <= step_budget <= MAX_STEP_BUDGET:
            raise ValueError(f"step_budget must be in [1, {MAX_STEP_BUDGET}]")
        self._fn = fn
        self._requested = coverage
        self._budget = step_budget
        self._scope = scope or default_scope
        self._codes: dict[int, _CodeInfo] = {}
        self._edges: set[int] = set()
        self._steps = 0
        self._tool: int | None = None
        self._mask = 0
        # A private subclass per instance: an inner Harness that sees an outer
        # one's budget exception re-raises it instead of claiming the timeout.
        self._budget_exc: type[_StepBudgetExceeded] = type(
            "_HarnessBudgetExceeded", (_StepBudgetExceeded,), {}
        )
        self.mechanism: str | None = (
            "none" if coverage in ("none", "reporter") else None
        )

    # -- lifecycle ---------------------------------------------------------

    def __enter__(self) -> Harness:
        want = self._requested
        if want in ("auto", "monitoring"):
            self._tool = self._claim_monitoring()
            if self._tool is not None:
                self.mechanism = TIER_MONITORING
            elif want == "monitoring":
                raise RuntimeError("sys.monitoring is unavailable (needs Python 3.12+)")
            else:
                self.mechanism = TIER_SETTRACE
        elif want == "settrace":
            self.mechanism = TIER_SETTRACE
        return self

    def __exit__(self, *exc: object) -> None:
        if self._tool is not None:
            mon = sys.monitoring  # type: ignore[attr-defined]
            mon.set_events(self._tool, 0)
            for event in self._events():
                mon.register_callback(self._tool, event, None)
            mon.free_tool_id(self._tool)
            self._tool = None

    @staticmethod
    def _events() -> list[int]:
        ev = sys.monitoring.events  # type: ignore[attr-defined]
        if hasattr(ev, "BRANCH"):
            return [ev.LINE, ev.BRANCH]
        return [ev.LINE, ev.BRANCH_LEFT, ev.BRANCH_RIGHT]  # pragma: no cover

    def _claim_monitoring(self) -> int | None:
        mon = getattr(sys, "monitoring", None)
        if mon is None:
            return None
        for tool in (3, 4, 5):  # 0 debugger, 1 coverage, 2 profiler are taken
            try:
                mon.use_tool_id(tool, "coding_adventures_test_bootstrap.fuzz")
            except ValueError:
                continue
            try:
                events = self._events()
                mon.register_callback(tool, events[0], self._on_line)
                for event in events[1:]:
                    mon.register_callback(tool, event, self._on_branch)
                self._mask = 0
                for event in events:
                    self._mask |= event
                mon.restart_events()  # re-arm locations another campaign disabled
            except BaseException:
                # Never leak a claimed tool id (it is process-global).
                mon.free_tool_id(tool)
                raise
            return tool
        return None

    # -- per-code bookkeeping ---------------------------------------------

    def _info(self, code: Any) -> _CodeInfo:
        """Facts about ``code``; only in-scope code is remembered.

        Out-of-scope code (the standard library, ``<string>`` code built by
        ``compile``/``eval``) shares one sentinel and is never stored, and the
        cache is capped, so a target that generates code cannot grow it
        without bound.
        """
        info = self._codes.get(id(code))
        if info is not None:
            return info
        name = getattr(code, "co_qualname", code.co_name)
        if not self._scope(code.co_filename, name):
            return _OUT_OF_SCOPE
        info = _CodeInfo(code, True)
        if len(self._codes) < MAX_CODE_CACHE:
            self._codes[id(code)] = info
        return info

    def _tick(self) -> None:
        self._steps += 1
        if self._steps > self._budget:
            raise self._budget_exc

    # -- sys.monitoring callbacks -----------------------------------------

    def _on_line(self, code: Any, line: int) -> Any:
        info = self._info(code)
        if not info.scope:
            return sys.monitoring.DISABLE  # type: ignore[attr-defined]
        self._edges.add(info.edge(line, line))
        self._tick()
        return None

    def _on_branch(self, code: Any, offset: int, dest: int) -> Any:
        info = self._info(code)
        if not info.scope:
            return sys.monitoring.DISABLE  # type: ignore[attr-defined]
        self._edges.add(info.edge(info.line_of(offset), info.line_of(dest)))
        return None

    # -- sys.settrace callbacks -------------------------------------------

    def _global_trace(self, frame: Any, event: str, _arg: Any) -> Any:
        if event != "call":
            return None
        info = self._info(frame.f_code)
        if not info.scope:
            return None
        previous = -1

        def local(frame: Any, event: str, _arg: Any) -> Any:
            nonlocal previous
            if event == "line":
                line = frame.f_lineno
                self._edges.add(info.edge(line, line))
                if previous >= 0:
                    self._edges.add(info.edge(previous, line))
                previous = line
                self._tick()
            return local

        return local

    # -- executing ---------------------------------------------------------

    def _run(self, data: bytes, cov: CoverageBitmap) -> None:
        self._edges = set()
        self._steps = 0
        if self.mechanism == TIER_MONITORING:
            mon = sys.monitoring  # type: ignore[attr-defined]
            mon.set_events(self._tool, self._mask)
            try:
                self._fn(data)
            finally:
                mon.set_events(self._tool, 0)
        elif self.mechanism == TIER_SETTRACE:
            previous = sys.gettrace()
            sys.settrace(self._global_trace)
            try:
                self._fn(data)
            finally:
                sys.settrace(previous)
        elif self._requested == "reporter":
            self._fn(data, cov)
        else:
            self._fn(data)

    def __call__(self, data: bytes, cov: CoverageBitmap) -> FuzzOutcome:
        if self.mechanism is None:
            raise RuntimeError("Harness must be used as a context manager")
        try:
            self._run(data, cov)
        except self._budget_exc:
            return OutcomeTimeout()
        except MemoryError:
            return OutcomeOom()
        except (Exception, SystemExit) as exc:
            kind = _frame_label(type(exc).__name__)
            return OutcomeCrash(kind, frames_of(exc.__traceback__))
        finally:
            for edge in self._edges:
                cov.record_edge(edge)
        return OutcomePass()


def fuzz_function(
    fn: Callable[..., Any],
    initial_corpus: Sequence[bytes] = (),
    options: FuzzOptions | None = None,
    *,
    coverage: str = "auto",
    step_budget: int = DEFAULT_STEP_BUDGET,
    scope: Callable[[str, str], bool] | None = None,
) -> FuzzReport:
    """Fuzz ``fn(data: bytes)``; any ``Exception`` it raises is a crash.

    The report's ``coverage_tier`` says what guided the run: ``"sys.monitoring"``,
    ``"sys.settrace"``, ``"caller-reported"`` or, when no edge was collected at
    all, ``"black-box-uninstrumented"`` (VER06 section 7).
    """
    with Harness(fn, coverage=coverage, step_budget=step_budget, scope=scope) as h:
        report = fuzz_target(initial_corpus, options, h)
    if report.total_edges == 0:
        tier = TIER_BLACK_BOX
    elif h.mechanism in (TIER_MONITORING, TIER_SETTRACE):
        tier = h.mechanism
    else:
        tier = TIER_REPORTED
    return replace(report, coverage_tier=tier)


# =============================================================================
# 10. Compatibility wrappers (the pre-0.3 entry points)
# =============================================================================


def fuzz_bytes(
    target_fn: Callable[[bytes], Any],
    seed: int = 42,
    iterations: int = 1000,
    max_input_bytes: int = 4096,
    initial_corpus: Sequence[bytes] | None = None,
    *,
    coverage: str = "auto",
) -> FuzzReport:
    """Fuzz a bytes target: a thin shell over :func:`fuzz_function`.

    ``iterations`` must be positive; an empty or missing corpus is replaced by
    a single ``00`` seed (it used to be an error or a built-in list).
    """
    _check_strict_int(iterations, "iterations")
    if iterations <= 0:
        raise ValueError(f"iterations must be positive, got {iterations}")
    opts = FuzzOptions(
        max_iterations=iterations, seed=seed, max_input_len=max_input_bytes
    )
    return fuzz_function(
        target_fn, tuple(initial_corpus or ()), opts, coverage=coverage
    )


def fuzz_text(
    target_fn: Callable[[str], Any],
    seed: int = 42,
    iterations: int = 1000,
    max_input_bytes: int = 4096,
    initial_corpus: Sequence[str] | None = None,
    *,
    coverage: str = "auto",
) -> FuzzReport:
    """Fuzz a text target: bytes are decoded as UTF-8 with ``errors="replace"``."""
    corpus: list[bytes] = []
    for i, item in enumerate(initial_corpus or ()):
        if not isinstance(item, str):
            raise TypeError(
                f"initial_corpus[{i}] must be str, got {type(item).__name__}"
            )
        corpus.append(item.encode("utf-8"))

    def run(data: bytes) -> Any:
        return target_fn(data.decode("utf-8", errors="replace"))

    return fuzz_bytes(run, seed, iterations, max_input_bytes, corpus, coverage=coverage)
