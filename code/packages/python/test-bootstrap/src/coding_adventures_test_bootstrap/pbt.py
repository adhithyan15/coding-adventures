"""Property-based testing engine: the Python lane of VER05 / VER05c.

A property-based test does not check one example, it checks a *claim* about
every input a generator can produce::

    for every list xs of small ints:   sorted(sorted(xs)) == sorted(xs)

The engine has three jobs, and this module is organised around them:

1. **Generate** candidate inputs from a deterministic random stream
   (``Gen``, ``gen_*``).  The stream is SplitMix64 (``prng.py``), so the same
   seed gives the same candidates on every machine and in every language lane;
   the shared ``pbt-v1`` corpus pins the first outputs.
2. **Shrink** a failing candidate to a small witness a human can read.  A
   counterexample of ``[913, -4, 77, 0, 2]`` is noise; ``[0]`` is a diagnosis.
   Shrinking here is *integrated*: the generator that made the value knows its
   own bounds, so a shrunk ``gen_int(5, 10)`` value never becomes ``0``.
3. **Judge** honestly (``check_property``).  A verdict is one of a closed set
   of failure ids, and the engine refuses to report ``pass`` unless every case
   actually ran and held (VER05 section 7.1, "verdict honesty").

Worked example (the corpus case ``pbt/shrink_int/54321_le_100``)::

    property:  x <= 100          falsified by x = 54321
    shrink:    54321 -> 27161 -> 13581 -> ... -> 101      (15 accepted moves)
    result:    (101, 15)   -- 101 is the smallest value that still fails

Python integers are arbitrary precision, so nothing here overflows; wherever a
fixed-width lane relies on wrap-around (SplitMix64, FNV-1a) this lane masks to
64 bits explicitly.

Zero dependencies, no I/O, no environment access (the package declares zero
capabilities).
"""

from __future__ import annotations

import string
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import Any

from coding_adventures_test_bootstrap.prng import MASK64, SplitMix64, fnv1a64

# --- CWE-400 bounds ----------------------------------------------------------
MAX_CASES = 100_000
MAX_SHRINK_STEPS = 10_000
MAX_INPUT_BYTES = 1024 * 1024
MAX_LIST_LEN = 100_000
# frequency() draws with bounded(total); bounded is specified up to 2^32 here.
MAX_FREQUENCY_TOTAL = 1 << 32

# --- Closed failure vocabulary (VER05 section 5.1) ---------------------------
FID_COUNTEREXAMPLE = "property-counterexample"  # falsified; minimal witness found
FID_EXHAUSTED = "property-exhausted"  # nothing (or no valid input) was evaluated
FID_TIMEOUT = "property-timeout"  # registered, never produced by this lane
FID_PANIC = "property-panic"  # the property itself raised
PBT_FAILURE_IDS = (FID_COUNTEREXAMPLE, FID_EXHAUSTED, FID_TIMEOUT, FID_PANIC)


def is_pbt_failure_id(candidate: object) -> bool:
    """True only for a ``str`` equal to one of the four registered ids.

    ``is_pbt_failure_id("property-panic")``  -> True
    ``is_pbt_failure_id("unregistered-id")`` -> False
    ``is_pbt_failure_id(None)``              -> False  (not a string at all)
    """
    return isinstance(candidate, str) and candidate in PBT_FAILURE_IDS


class ExhaustedError(RuntimeError):
    """A generator could not produce a value (e.g. a filter rejected everything).

    Subclasses ``RuntimeError`` so code written against the pre-0.2 engine,
    which raised a bare ``RuntimeError``, keeps catching it.
    """


def _check_strict_int(val: Any, name: str) -> None:
    # bool is a subclass of int in Python; ``True`` is never a valid count/seed.
    if isinstance(val, bool) or not isinstance(val, int):
        raise TypeError(f"{name} must be an integer, not {type(val).__name__}")


@dataclass(frozen=True)
class PropertyResult:
    """Outcome of a ``for_all`` run."""

    passed: bool
    cases_run: int
    seed: int
    counterexample: tuple[Any, ...] | None = None
    shrunken: tuple[Any, ...] | None = None
    error: str | None = None
    shrink_steps: int = 0


# A shrink function receives the failing value, a predicate ``falsifies(v)``
# that is True while ``v`` still makes the property fail, and a step budget.  It
# returns ``(smaller_value, accepted_steps)``.
ShrinkFn = Callable[[Any, Callable[[Any], bool], int], "tuple[Any, int]"]


class Gen[T]:
    """A composable deterministic generator of ``T``.

    ``sample_fn(rng)`` draws one value; the optional ``shrink_fn`` makes
    shrinking *integrated* (it knows the generator's bounds).  A ``Gen`` built
    from a bare lambda has no shrinker and its values are reported unshrunk.
    """

    __slots__ = ("_sample_fn", "_shrink_fn")

    def __init__(
        self,
        sample_fn: Callable[[SplitMix64], T],
        shrink_fn: ShrinkFn | None = None,
    ) -> None:
        self._sample_fn = sample_fn
        self._shrink_fn = shrink_fn

    def sample(self, rng: SplitMix64) -> T:
        """Draw one sample using the supplied PRNG."""
        return self._sample_fn(rng)

    def shrink(
        self, value: T, falsifies: Callable[[T], bool], max_steps: int
    ) -> tuple[T, int]:
        """Shrink ``value`` while ``falsifies`` holds; ``(value, 0)`` if unable."""
        if self._shrink_fn is None:
            return value, 0
        return self._shrink_fn(value, falsifies, max_steps)

    def map[U](self, fn: Callable[[T], U]) -> Gen[U]:
        """Transform generated values (drops shrinking: ``fn`` has no inverse)."""
        return Gen(lambda rng: fn(self.sample(rng)))

    def filter(self, pred: Callable[[T], bool], max_retries: int = 100) -> Gen[T]:
        """Keep only values satisfying ``pred``; ``ExhaustedError`` after
        exactly ``max_retries`` rejected samples.

        Shrinking is preserved: a shrink candidate that fails ``pred`` is simply
        not a valid input, so it is treated as "does not falsify".
        """
        _check_strict_int(max_retries, "max_retries")
        if max_retries <= 0:
            raise ValueError("max_retries must be positive")

        def _filtered(rng: SplitMix64) -> T:
            for _ in range(max_retries):
                val = self.sample(rng)
                if pred(val):
                    return val
            raise ExhaustedError(f"Gen.filter exceeded max_retries ({max_retries})")

        def _shrink(
            value: T, falsifies: Callable[[T], bool], max_steps: int
        ) -> tuple[T, int]:
            return self.shrink(
                value, lambda c: bool(pred(c)) and falsifies(c), max_steps
            )

        return Gen(_filtered, _shrink)

    def flat_map[U](self, fn: Callable[[T], Gen[U]]) -> Gen[U]:
        """Dependent generation: sample, then sample from ``fn(value)``."""
        return Gen(lambda rng: fn(self.sample(rng)).sample(rng))


# =============================================================================
# Primitive generators (VER05 section 3.1)
#
# Draw order is the compatibility contract between language lanes: the pbt-v1
# vectors pin exact outputs per seed, so reordering a draw breaks conformance.
# =============================================================================


def _below(rng: SplitMix64, n: int) -> int:
    """Uniform in ``[0, n)`` that **always consumes at least one stream word**.

    ``SplitMix64.bounded(1)`` returns 0 without touching the stream (the fuzz
    harness depends on that), but VER05 section 2.2's loop, the Rust lane and
    the reference draw one word even for ``n == 1``.  Using this helper keeps
    every generator's stream position identical across lanes: a fixed-length
    ``gen_bytes(4, 4)`` still advances the stream by one word for its length.
    (The Dart lane returns early for ``n <= 1``; see VER05c section 2.2.)
    """
    if n == 1:
        rng.next_u64()
        return 0
    return rng.bounded(n)


def _gen_bool(rng: SplitMix64) -> bool:
    return rng.bounded(2) == 1


def _gen_u8(rng: SplitMix64) -> int:
    return rng.bounded(256)


def _gen_u64(rng: SplitMix64) -> int:
    return rng.next_u64()


def _gen_i128(rng: SplitMix64) -> int:
    """Two stream words, high word first, read as a signed 128-bit integer.

    words 0x8000...0 : 0   -> -2^127   (top bit set => subtract 2^128)
    words 0x7fff...f : f.. ->  2^127 - 1
    """
    w1 = rng.next_u64()
    w2 = rng.next_u64()
    val = (w1 << 64) | w2
    if val >= 1 << 127:
        val -= 1 << 128
    return val


def gen_bounded_int(rng: SplitMix64, min_val: int, max_val: int) -> int:
    """Uniform integer over the inclusive range ``[min_val, max_val]``.

    The width of the range is ``span = max - min + 1``.  The *offset* from
    ``min_val`` is drawn by span (VER05 section 7.2)::

        span <= 2^32            rng.bounded(span)
        2^32 < span < 2^64      rng.bounded(span)   (rejection threshold
                                                     (2^64 - span) mod span)
        span == 2^64            one raw next_u64()
        span  > 2^64            k-word rejection sampling (below)

    Wide spans.  Take ``k = max(2, ceil(bit_length(span - 1) / 64))`` stream
    words, first draw most significant, giving ``raw`` uniform on
    ``[0, 2^(64k))``.  A plain ``raw % span`` over-weights the low residues
    (for ``span = 3 * 2^126`` the low third would be drawn half the time, not a
    third), so raws below ``t = (2^(64k) - span) mod span`` are redrawn.  For
    ``2^64 < span <= 2^128`` this is exactly the Rust lane's 128-bit rejection.
    """
    _check_strict_int(min_val, "min_val")
    _check_strict_int(max_val, "max_val")
    if min_val > max_val:
        raise ValueError(f"min_val ({min_val}) cannot exceed max_val ({max_val})")
    span = max_val - min_val + 1
    if span <= 1 << 64:
        # bounded() handles 1 <= n < 2^64 with rejection and n == 2^64 raw.
        return min_val + _below(rng, span)
    words = max(2, ((span - 1).bit_length() + 63) // 64)
    modulus = 1 << (64 * words)
    threshold = (modulus - span) % span
    while True:
        raw = 0
        for _ in range(words):
            raw = (raw << 64) | rng.next_u64()
        if raw >= threshold:
            return min_val + raw % span


def _int_shrinker(min_val: int, max_val: int) -> ShrinkFn:
    """Shrinker for ``gen_int(min_val, max_val)``: toward the in-range point
    nearest zero, never leaving ``[min_val, max_val]``."""
    target = min(max(0, min_val), max_val)

    def _shrink(
        value: int, falsifies: Callable[[int], bool], max_steps: int
    ) -> tuple[int, int]:
        return _shrink_int_core(value, target, min_val, max_val, falsifies, max_steps)

    return _shrink


def gen_bool() -> Gen[bool]:
    """Generate a random boolean (shrinks ``True`` to ``False``)."""

    def _shrink(
        value: bool, falsifies: Callable[[bool], bool], max_steps: int
    ) -> tuple[bool, int]:
        if value and max_steps > 0 and falsifies(False):
            return False, 1
        return value, 0

    return Gen(_gen_bool, _shrink)


def gen_int(min_val: int = -2_147_483_648, max_val: int = 2_147_483_647) -> Gen[int]:
    """Generate an integer in ``[min_val, max_val]`` (any width)."""
    _check_strict_int(min_val, "min_val")
    _check_strict_int(max_val, "max_val")
    if min_val > max_val:
        raise ValueError(f"min_val ({min_val}) cannot exceed max_val ({max_val})")
    return Gen(
        lambda rng: gen_bounded_int(rng, min_val, max_val),
        _int_shrinker(min_val, max_val),
    )


def gen_u8() -> Gen[int]:
    """Generate an 8-bit unsigned integer [0, 255]."""
    return Gen(_gen_u8, _int_shrinker(0, 255))


def gen_u64() -> Gen[int]:
    """Generate a 64-bit unsigned integer [0, 2^64-1]."""
    return Gen(_gen_u64, _int_shrinker(0, MASK64))


def gen_i128() -> Gen[int]:
    """Generate a signed 128-bit integer from two stream words."""
    return Gen(_gen_i128, _int_shrinker(-(1 << 127), (1 << 127) - 1))


def gen_constant[T](value: T) -> Gen[T]:
    """Always generate ``value`` (draws nothing)."""
    return Gen(lambda rng: value)


def gen_float(min_val: float = -1e6, max_val: float = 1e6) -> Gen[float]:
    """Generate a finite float in [min_val, max_val] (no shrinker)."""
    if min_val > max_val:
        raise ValueError("min_val cannot exceed max_val")
    span = max_val - min_val
    return Gen(lambda rng: min_val + rng.next_float() * span)


def _check_lengths(min_len: int, max_len: int, cap: int, cap_name: str) -> None:
    _check_strict_int(min_len, "min_len")
    _check_strict_int(max_len, "max_len")
    if min_len < 0 or max_len < min_len:
        raise ValueError("Invalid length bounds")
    if max_len > cap:
        raise ValueError(f"max_len exceeds {cap_name} ({cap})")


def gen_bytes(min_len: int = 0, max_len: int = 64) -> Gen[bytes]:
    """Generate bytes: ``L = min_len + bounded(span)``, then ``L`` byte draws."""
    _check_lengths(min_len, max_len, MAX_INPUT_BYTES, "MAX_INPUT_BYTES")
    span = max_len - min_len + 1

    def _sample(rng: SplitMix64) -> bytes:
        length = min_len + _below(rng, span)
        return bytes(rng.bounded(256) for _ in range(length))

    def _shrink(
        value: bytes, falsifies: Callable[[bytes], bool], max_steps: int
    ) -> tuple[bytes, int]:
        shrunk, steps = _shrink_sequence(
            list(value),
            lambda items: falsifies(bytes(items)),
            max_steps,
            min_len,
            _byte_walk,
        )
        return bytes(shrunk), steps

    return Gen(_sample, _shrink)


def gen_text(
    min_len: int = 0,
    max_len: int = 64,
    alphabet: str = string.ascii_letters + string.digits,
) -> Gen[str]:
    """Generate text over ``alphabet``; shrinks toward ``alphabet[0]``."""
    _check_lengths(min_len, max_len, MAX_INPUT_BYTES, "MAX_INPUT_BYTES")
    if not alphabet:
        raise ValueError("Alphabet cannot be empty")
    alpha_len = len(alphabet)
    span = max_len - min_len + 1

    def _sample(rng: SplitMix64) -> str:
        length = min_len + _below(rng, span)
        return "".join(alphabet[_below(rng, alpha_len)] for _ in range(length))

    def _shrink(
        value: str, falsifies: Callable[[str], bool], max_steps: int
    ) -> tuple[str, int]:
        # Shrink the *positions in the alphabet*: shorter, then lower indices.
        indices = [alphabet.index(c) for c in value]

        def walk(
            item: int, still_fails: Callable[[int], bool], budget: int
        ) -> tuple[int, int]:
            return _int_walk(item, 0, 0, alpha_len - 1, still_fails, budget)

        shrunk, steps = _shrink_sequence(
            indices,
            lambda idx: falsifies("".join(alphabet[i] for i in idx)),
            max_steps,
            min_len,
            walk,
        )
        return "".join(alphabet[i] for i in shrunk), steps

    return Gen(_sample, _shrink)


def gen_choice[T](elements: Sequence[T]) -> Gen[T]:
    """Pick one element uniformly from a non-empty sequence."""
    if not elements:
        raise ValueError("gen_choice requires non-empty sequence")
    items = list(elements)
    count = len(items)
    return Gen(lambda rng: items[_below(rng, count)])


def gen_list[T](elem_gen: Gen[T], min_len: int = 0, max_len: int = 20) -> Gen[list[T]]:
    """Generate a list: ``L = min_len + bounded(span)``, then ``L`` elements."""
    _check_lengths(min_len, max_len, MAX_LIST_LEN, "MAX_LIST_LEN")
    span = max_len - min_len + 1

    def _sample(rng: SplitMix64) -> list[T]:
        length = min_len + _below(rng, span)
        return [elem_gen.sample(rng) for _ in range(length)]

    def _shrink(
        value: list[T], falsifies: Callable[[list[T]], bool], max_steps: int
    ) -> tuple[list[T], int]:
        return _shrink_sequence(value, falsifies, max_steps, min_len, elem_gen.shrink)

    return Gen(_sample, _shrink)


def gen_one_of[T](*gens: Gen[T]) -> Gen[T]:
    """Pick one generator uniformly, then sample it (no shrinker)."""
    if not gens:
        raise ValueError("gen_one_of requires at least one generator")
    arms = list(gens)
    count = len(arms)
    return Gen(lambda rng: arms[_below(rng, count)].sample(rng))


one_of = gen_one_of


def frequency[T](*weighted: tuple[int, Gen[T]]) -> Gen[T]:
    """Weighted choice among ``(weight, gen)`` arms.

    ``pick = bounded(total)``; arms are walked in order, subtracting each
    weight, so ``frequency((1, a), (3, b))`` picks ``b`` three times in four.
    A zero-weight arm can never be selected (``pick < 0`` is never true); a
    negative weight, an empty list, a zero total or a total above 2^32 is an
    error.
    """
    if not weighted:
        raise ValueError("frequency requires at least one generator")
    arms = list(weighted)
    for weight, _ in arms:
        _check_strict_int(weight, "weight")
        if weight < 0:
            raise ValueError("frequency weights cannot be negative")
    total = sum(weight for weight, _ in arms)
    if total <= 0:
        raise ValueError("frequency total weight must be > 0")
    if total > MAX_FREQUENCY_TOTAL:
        raise ValueError(f"frequency total weight exceeds {MAX_FREQUENCY_TOTAL}")

    def _sample(rng: SplitMix64) -> T:
        pick = _below(rng, total)
        for weight, arm in arms:
            if pick < weight:
                return arm.sample(rng)
            pick -= weight
        raise AssertionError("unreachable: pick < total")  # pragma: no cover

    return Gen(_sample)


def gen_tuple(*gens: Gen[Any]) -> Gen[tuple[Any, ...]]:
    """Sample each generator left to right; shrink one coordinate at a time."""

    def _sample(rng: SplitMix64) -> tuple[Any, ...]:
        return tuple(g.sample(rng) for g in gens)

    def _shrink(
        value: tuple[Any, ...],
        falsifies: Callable[[tuple[Any, ...]], bool],
        max_steps: int,
    ) -> tuple[tuple[Any, ...], int]:
        current = list(value)
        steps = 0
        # Repeat passes until a whole pass makes no progress: shrinking
        # coordinate 2 can unlock more shrinking of coordinate 1.  Every pass
        # that continues accepted at least one move, so the budget bounds it.
        progress = True
        while progress and steps < max_steps:
            progress = False
            for index, gen in enumerate(gens):
                remaining = max_steps - steps
                if remaining <= 0:
                    break
                shrunk, used = gen.shrink(
                    current[index], _slot(current, index, falsifies), remaining
                )
                if used > 0:
                    current[index] = shrunk
                    steps += used
                    progress = True
        return tuple(current), steps

    return Gen(_sample, _shrink)


def _slot(
    current: list[Any], index: int, falsifies: Callable[[tuple[Any, ...]], bool]
) -> Callable[[Any], bool]:
    """Predicate over one tuple coordinate, the others held at ``current``."""

    def _check(candidate: Any) -> bool:
        probe = list(current)
        probe[index] = candidate
        return falsifies(tuple(probe))

    return _check


# =============================================================================
# Corpus entry points (pbt-v1)
# =============================================================================


def gen_primitive(seed: int, type_name: str, count: int) -> list[Any]:
    """``count`` primitives of ``type_name`` (bool/u8/u64/i128) from ``seed``."""
    rng = SplitMix64(seed)
    makers: dict[str, Callable[[SplitMix64], Any]] = {
        "bool": _gen_bool,
        "u8": _gen_u8,
        "u64": _gen_u64,
        "i128": _gen_i128,
    }
    if type_name not in makers:
        raise ValueError(f"unknown primitive type: {type_name}")
    return [makers[type_name](rng) for _ in range(count)]


def gen_bounded(seed: int, min_val: int, max_val: int, count: int) -> list[int]:
    """``count`` uniform integers over ``[min_val, max_val]`` from ``seed``."""
    rng = SplitMix64(seed)
    return [gen_bounded_int(rng, min_val, max_val) for _ in range(count)]


def gen_bytes_corpus(seed: int, min_len: int, max_len: int, count: int) -> list[bytes]:
    """``count`` byte strings of length in ``[min_len, max_len]`` from ``seed``."""
    gen = gen_bytes(min_len, max_len)
    rng = SplitMix64(seed)
    return [gen.sample(rng) for _ in range(count)]


# =============================================================================
# Integrated shrinking (VER05 section 4)
#
# Shrinking turns a failing candidate into the smallest witness that still
# fails.  Step counts are part of the conformance contract, so the phase order
# and the rule "count accepted moves only" match the reference exactly.
# =============================================================================


def _in_bounds(value: int, lo: int | None, hi: int | None) -> bool:
    return (lo is None or lo <= value) and (hi is None or value <= hi)


def _int_walk(
    current: int,
    target: int,
    lo: int | None,
    hi: int | None,
    falsifies: Callable[[int], bool],
    budget: int,
) -> tuple[int, int]:
    """Bisect then unit-step ``current`` toward ``target``.

    Each loop turn tries, in order,

    1. the *bisection* candidate ``current - diff // 2`` (``diff = current -
       target``; floor division equals Euclidean division by 2), when
       ``|diff| > 1``;
    2. the *unit* candidate ``current -/+ 1`` toward the target.

    A candidate is accepted only if it is inside ``[lo, hi]`` (``None`` means
    unbounded), differs from ``current`` and still falsifies.  Only accepted
    moves count as steps.  No acceptable candidate ends the walk.

        current=54321 target=0 prop x<=100:  54321 -> 27161 -> 13581 -> ...
    """
    steps = 0
    while steps < budget and current != target:
        diff = current - target
        moved = False
        if abs(diff) > 1:
            candidate = current - diff // 2
            if (
                candidate != current
                and _in_bounds(candidate, lo, hi)
                and falsifies(candidate)
            ):
                current, steps, moved = candidate, steps + 1, True
        if not moved:
            candidate = current - (1 if diff > 0 else -1)
            if (
                candidate != current
                and _in_bounds(candidate, lo, hi)
                and falsifies(candidate)
            ):
                current, steps, moved = candidate, steps + 1, True
        if not moved:
            break
    return current, steps


def _shrink_int_core(
    initial: int,
    target: int,
    lo: int,
    hi: int,
    falsifies: Callable[[int], bool],
    max_steps: int,
) -> tuple[int, int]:
    """Target probe, then the bisect/unit walk (VER05 section 4.2)."""
    if max_steps == 0:
        return initial, 0  # no budget, no shrinking
    if lo <= target <= hi and target != initial and falsifies(target):
        return target, 1  # the target itself fails: cannot do better
    return _int_walk(initial, target, lo, hi, falsifies, max_steps)


def _byte_walk(
    value: int, falsifies: Callable[[int], bool], budget: int
) -> tuple[int, int]:
    return _int_walk(value, 0, 0, 255, falsifies, budget)


def _with_element(
    current: list[Any], index: int, falsifies: Callable[[list[Any]], bool]
) -> Callable[[Any], bool]:
    """Predicate over one list element, the rest held at ``current``."""

    def _check(candidate: Any) -> bool:
        probe = list(current)
        probe[index] = candidate
        return falsifies(probe)

    return _check


def _shrink_sequence(
    initial: list[Any],
    falsifies: Callable[[list[Any]], bool],
    max_steps: int,
    min_len: int,
    shrink_elem: Callable[[Any, Callable[[Any], bool], int], tuple[Any, int]] | None,
) -> tuple[list[Any], int]:
    """Shrink a falsifying sequence (VER05 section 4.3, corpus phases).

    Phase 0  probe.  min_len == 0: does the empty list fail?  Then it is the
             answer, one step.  (min_len > 0: probe the length-min_len prefix
             and carry on from it.)
    Phase 1  bisection: keep the left half, else the right half, while one of
             them still fails.
    Phase 2  single deletion, right to left.
    Phase 3  element-wise, left to right, via ``shrink_elem``.

    Worked example (corpus ``list_sum_le_20``, initial [10, 5, 8, 4]):
    the empty list passes; bisection fails on both halves; deleting index 3
    leaves [10, 5, 8] (sum 23, still failing); element-wise 10 -> 8 gives the
    1-minimal [8, 5, 8].
    """
    current = list(initial)
    steps = 0
    if max_steps == 0:
        return current, 0

    if len(current) > min_len:
        probe = current[:min_len]
        if falsifies(probe):
            if min_len == 0:
                return probe, 1
            current, steps = probe, 1

    floor = max(1, min_len)

    # Phase 1: bisection.
    while len(current) > floor and steps < max_steps:
        mid = len(current) // 2
        left = current[:mid]
        if len(left) >= min_len and falsifies(left):
            current, steps = left, steps + 1
            continue
        right = current[mid:]
        if len(right) >= min_len and falsifies(right):
            current, steps = right, steps + 1
            continue
        break

    # Phase 2: single-element deletion, right to left.
    i = len(current) - 1
    while i >= 0 and len(current) > floor and steps < max_steps:
        candidate = current[:i] + current[i + 1 :]
        if falsifies(candidate):
            current, steps = candidate, steps + 1
            i = min(i, len(current) - 1)
        else:
            i -= 1

    # Phase 3: element-wise, left to right.
    if shrink_elem is not None:
        for index in range(len(current)):
            remaining = max_steps - steps
            if remaining <= 0:
                break
            new_value, used = shrink_elem(
                current[index], _with_element(current, index, falsifies), remaining
            )
            if used > 0:
                current = current[:index] + [new_value] + current[index + 1 :]
                steps += used

    return current, steps


def _check_budget(max_steps: int) -> None:
    _check_strict_int(max_steps, "max_steps")
    if max_steps < 0:
        raise ValueError("max_steps cannot be negative")
    if max_steps > MAX_SHRINK_STEPS:
        raise ValueError(f"max_steps exceeds MAX_SHRINK_STEPS ({MAX_SHRINK_STEPS})")


def shrink_int(
    initial: int,
    target: int,
    min_val: int,
    max_val: int,
    prop: Callable[[int], bool],
    max_steps: int = 1000,
) -> tuple[int, int]:
    """Shrink a falsifying integer toward ``target`` inside ``[min_val, max_val]``.

    ``prop(x)`` is True when the property *holds*.  Returns ``(minimal, steps)``.
    Raises ``ValueError`` if ``initial`` satisfies ``prop`` (a caller bug) or is
    outside the bounds.  Python integers never overflow, so unlike fixed-width
    lanes no saturating subtraction is needed at the extremes.
    """
    for name, number in (
        ("initial", initial),
        ("target", target),
        ("min_val", min_val),
        ("max_val", max_val),
    ):
        _check_strict_int(number, name)
    _check_budget(max_steps)
    if min_val > max_val:
        raise ValueError("min_val cannot exceed max_val")
    if not min_val <= initial <= max_val:
        raise ValueError("initial value must lie within [min_val, max_val]")
    if prop(initial):
        raise ValueError(f"initial value {initial} must falsify the property")
    return _shrink_int_core(
        initial, target, min_val, max_val, lambda v: not prop(v), max_steps
    )


def shrink_list(
    initial: Sequence[int],
    elem_target: int,
    prop: Callable[[list[int]], bool],
    max_steps: int = 1000,
) -> tuple[list[int], int]:
    """Shrink a falsifying list of integers (empty probe, halves, deletion,
    then each element toward ``elem_target``).  ``prop`` is True when it holds."""
    _check_strict_int(elem_target, "elem_target")
    _check_budget(max_steps)
    items = list(initial)
    for item in items:
        _check_strict_int(item, "element")
    if prop(list(items)):
        raise ValueError("initial list must falsify the property")

    def walk(
        value: int, falsifies: Callable[[int], bool], budget: int
    ) -> tuple[int, int]:
        return _int_walk(value, elem_target, None, None, falsifies, budget)

    return _shrink_sequence(
        items, lambda candidate: not prop(candidate), max_steps, 0, walk
    )


def shrink_bytes(
    initial: bytes,
    prop: Callable[[bytes], bool],
    max_steps: int = 1000,
) -> tuple[bytes, int]:
    """Shrink falsifying bytes: list shrinking over byte values toward ``0x00``."""
    _check_budget(max_steps)
    data = bytes(initial)
    if prop(data):
        raise ValueError("initial bytes must falsify the property")
    shrunk, steps = _shrink_sequence(
        list(data), lambda items: not prop(bytes(items)), max_steps, 0, _byte_walk
    )
    return bytes(shrunk), steps


def simulate_replay(seed: int, prop: Callable[[int], bool]) -> tuple[int, int]:
    """Replay a failure from nothing but ``seed``.

    Draw up to 1000 values ``bounded(1000)``; the first that falsifies ``prop``
    is ``initial_fail``; shrink it toward 0 over ``[0, 1000]``.  Returns
    ``(initial_fail, minimal)``.  Same seed, same pair, forever.
    """
    rng = SplitMix64(seed)
    initial_fail: int | None = None
    for _ in range(1000):
        value = rng.bounded(1000)
        if not prop(value):
            initial_fail = value
            break
    if initial_fail is None:
        raise ValueError("property was not falsified within 1000 draws")
    minimal, _ = shrink_int(initial_fail, 0, 0, 1000, prop)
    return initial_fail, minimal


# =============================================================================
# Seeds
# =============================================================================


def sub_seed(run_seed: int, property_id: str) -> int:
    """Isolated per-property seed (VER05 section 2.3).

        sub_seed = SplitMix64( (run_seed & MASK64) XOR fnv1a64(utf8(id)) ).next_u64()

    Two properties in one run never share a stream, and adding or reordering
    properties never changes another's draws.  The TypeScript and Dart lanes
    omit the final ``next_u64()``; this lane follows the spec, the Rust lane and
    the reference (see VER05c section 2.1).
    """
    _check_strict_int(run_seed, "run_seed")
    if not isinstance(property_id, str):
        raise TypeError("property_id must be a string")
    return SplitMix64((run_seed & MASK64) ^ fnv1a64(property_id)).next_u64()


# =============================================================================
# Verdicts and the honest runner (VER05 sections 5 and 7.1)
# =============================================================================


@dataclass(frozen=True)
class PbtOptions:
    """Options for ``check_property``.

    ``ci``: True / False, or None (the default) which means *redact*.  See
    VER05c section 5.1: this package cannot read the environment, so the caller
    decides (``ci_from_environ(os.environ)``), and the safe default is to hide
    the witness and seed.
    """

    num_tests: int = 100
    max_shrink_steps: int = 1000
    ci: bool | None = None


@dataclass(frozen=True)
class Verdict:
    """The outcome of ``check_property``.

    In redacted (CI) mode ``seed``, ``counterexample`` and ``minimal`` are
    ``None``; only the message, iteration, step count and digest remain.
    """

    passed: bool
    failure_id: str | None = None
    message: str = ""
    cases_run: int = 0
    iteration: int | None = None
    shrink_steps: int = 0
    seed: int | None = None
    counterexample: Any = None
    minimal: Any = None
    digest: str | None = None

    @property
    def failed(self) -> bool:
        return not self.passed


class PbtFailure(AssertionError):
    """Raised by ``assert_property``; carries the closed failure id."""

    def __init__(self, failure_id: str, message: str) -> None:
        super().__init__(f"[{failure_id}] {message}")
        self.failure_id = failure_id
        self.verdict_message = message


def ci_from_environ(environ: Mapping[str, str]) -> bool:
    """Pure CI detection: pass ``os.environ``.  ``CI`` set to anything except
    empty / ``0`` / ``false`` means CI."""
    value = environ.get("CI")
    return value is not None and value.strip().lower() not in ("", "0", "false")


_CANON_MAX_DEPTH = 16


def _canon(value: Any, depth: int = 0) -> str:
    """A stable text form of a witness (used in messages and the CI digest).

    Total: it never raises.  Nesting deeper than ``_CANON_MAX_DEPTH`` renders as
    ``...`` (a custom generator can build arbitrarily deep values, and this runs
    outside any ``try`` in redacted mode), and an object whose ``repr`` raises
    renders as its type name.
    """
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, (bytes, bytearray)):
        return "0x" + bytes(value).hex()
    if isinstance(value, (list, tuple)):
        if depth >= _CANON_MAX_DEPTH:
            return "..."
        return "[" + ",".join(_canon(v, depth + 1) for v in value) + "]"
    try:
        return repr(value)
    except Exception:
        return f"<{type(value).__name__}>"


def _validate_options(options: PbtOptions) -> None:
    _check_strict_int(options.num_tests, "num_tests")
    _check_strict_int(options.max_shrink_steps, "max_shrink_steps")
    if options.num_tests < 0:
        raise ValueError("num_tests cannot be negative")
    if options.num_tests > MAX_CASES:
        raise ValueError(f"num_tests exceeds MAX_CASES ({MAX_CASES})")
    _check_budget(options.max_shrink_steps)


def check_property(
    seed: int,
    gen: Gen[Any],
    prop: Callable[[Any], Any],
    options: PbtOptions | None = None,
    shrink: ShrinkFn | None = None,
) -> Verdict:
    """Run ``prop`` over ``options.num_tests`` candidates from ``seed``.

    ``prop(value)`` fails when it returns exactly ``False``; any other return
    value passes (so assertion-style properties returning ``None`` work), and
    an exception is ``property-panic``.  ``shrink`` overrides the generator's
    own shrinker (and exists so a defective shrinker can be tested).

    Verdict honesty (VER05 section 7.1):

    * zero cases evaluated is ``property-exhausted``, never a pass;
    * a generator error is ``property-exhausted``;
    * the reported minimal witness is re-verified: if the shrinker raises,
      returns something that no longer fails, or returns a value the property
      panics on, the *original* candidate is reported with 0 steps;
    * redacted mode never emits the witness or the seed.
    """
    _check_strict_int(seed, "seed")
    opts = options if options is not None else PbtOptions()
    _validate_options(opts)
    redact = opts.ci is not False

    if opts.num_tests == 0:
        return Verdict(
            passed=False,
            failure_id=FID_EXHAUSTED,
            message=(
                "no cases were evaluated (num_tests is 0); "
                "a property that never ran is not a pass"
            ),
        )

    rng = SplitMix64(seed)
    for iteration in range(1, opts.num_tests + 1):
        try:
            candidate = gen.sample(rng)
        except Exception as err:
            if isinstance(err, ExhaustedError):
                text = (
                    "filter precondition rejected all candidates "
                    f"at iteration {iteration}"
                )
            elif redact:
                text = (
                    f"generator error at iteration {iteration} ({type(err).__name__})"
                )
            else:
                text = (
                    f"generator error at iteration {iteration}: "
                    f"{type(err).__name__}: {err}"
                )
            return Verdict(
                passed=False,
                failure_id=FID_EXHAUSTED,
                message=text,
                cases_run=iteration - 1,
                iteration=iteration,
            )

        try:
            outcome = prop(candidate)
        except Exception as err:
            if redact:
                text = (
                    f"property evaluation panicked at iteration {iteration} "
                    f"({type(err).__name__})"
                )
            else:
                text = (
                    f"property evaluation panicked at iteration {iteration}: "
                    f"{type(err).__name__}: {err}"
                )
            return Verdict(
                passed=False,
                failure_id=FID_PANIC,
                message=text,
                cases_run=iteration,
                iteration=iteration,
                seed=None if redact else seed,
            )

        if outcome is False:
            return _falsified(
                seed, iteration, candidate, gen, prop, shrink, opts, redact
            )

    return Verdict(passed=True, cases_run=opts.num_tests)


def _falsified(
    seed: int,
    iteration: int,
    candidate: Any,
    gen: Gen[Any],
    prop: Callable[[Any], Any],
    shrink: ShrinkFn | None,
    opts: PbtOptions,
    redact: bool,
) -> Verdict:
    def falsifies(value: Any) -> bool:
        # A candidate the property panics on is not a *counterexample to the
        # claim*; it is a different failure, so it is not accepted as smaller.
        try:
            return prop(value) is False
        except Exception:
            return False

    minimal, steps = candidate, 0
    shrinker = shrink if shrink is not None else gen.shrink
    try:
        shrunk, used = shrinker(candidate, falsifies, opts.max_shrink_steps)
        valid_steps = (
            isinstance(used, int)
            and not isinstance(used, bool)
            and 0 <= used <= opts.max_shrink_steps
        )
        # Re-verify: the witness we report must really fail the property.
        if valid_steps and falsifies(shrunk):
            minimal, steps = shrunk, used
    except Exception:
        pass  # untrusted shrinker: fall back to the original, zero steps

    if redact:
        preimage = f"iter:{iteration};steps:{steps};shrunk:{_canon(minimal)}"
        digest = f"{fnv1a64(preimage):016x}"
        return Verdict(
            passed=False,
            failure_id=FID_COUNTEREXAMPLE,
            message=(
                f"property falsified after {iteration} iterations "
                f"(shrunk in {steps} steps, repro-fnv: {digest})"
            ),
            cases_run=iteration,
            iteration=iteration,
            shrink_steps=steps,
            digest=digest,
        )
    return Verdict(
        passed=False,
        failure_id=FID_COUNTEREXAMPLE,
        message=(
            f"property falsified at iteration {iteration} (seed={seed}): "
            f"initial={_canon(candidate)}, minimal={_canon(minimal)} "
            f"after {steps} shrink steps"
        ),
        cases_run=iteration,
        iteration=iteration,
        shrink_steps=steps,
        seed=seed,
        counterexample=candidate,
        minimal=minimal,
    )


def assert_property(
    gen: Gen[Any],
    prop: Callable[[Any], Any],
    options: PbtOptions | None = None,
    seed: int = 42,
) -> None:
    """``check_property`` that raises ``PbtFailure`` instead of returning a fail."""
    verdict = check_property(seed, gen, prop, options)
    if verdict.failed:
        raise PbtFailure(verdict.failure_id or FID_PANIC, verdict.message)


# =============================================================================
# for_all: the convenience runner (local, structured result)
# =============================================================================


def for_all(
    property_fn: Callable[..., Any],
    *generators: Gen[Any],
    count: int = 100,
    seed: int = 42,
    max_shrink_steps: int = 100,
) -> PropertyResult:
    """Run ``property_fn(*args)`` over ``count`` tuples drawn from ``generators``.

    Failure means returning exactly ``False`` or raising.  The failing tuple is
    shrunk coordinate-wise with each generator's own (bounded) shrinker, within
    ``max_shrink_steps`` accepted moves.  Unlike ``check_property`` this returns
    the raw counterexample, so use it for local development, not CI logs.
    """
    _check_strict_int(count, "count")
    _check_strict_int(seed, "seed")
    _check_strict_int(max_shrink_steps, "max_shrink_steps")

    if count <= 0:
        raise ValueError(f"count must be positive, got {count}")
    if count > MAX_CASES:
        raise ValueError(f"count exceeds MAX_CASES ({MAX_CASES})")
    if max_shrink_steps < 0:
        raise ValueError("max_shrink_steps cannot be negative")
    if max_shrink_steps > MAX_SHRINK_STEPS:
        msg = f"max_shrink_steps exceeds MAX_SHRINK_STEPS ({MAX_SHRINK_STEPS})"
        raise ValueError(msg)
    if not generators:
        raise ValueError("for_all requires at least one generator")

    args_gen = gen_tuple(*generators)
    rng = SplitMix64(seed)

    def still_fails(args: tuple[Any, ...]) -> bool:
        try:
            return property_fn(*args) is False
        except Exception:
            return True  # for_all: a raising property is a failing property

    for i in range(count):
        args = args_gen.sample(rng)
        error: str | None = None
        try:
            if property_fn(*args) is False:
                error = "Property returned False"
        except Exception as ex:
            error = f"{type(ex).__name__}: {ex}"
        if error is not None:
            shrunken, steps = args_gen.shrink(args, still_fails, max_shrink_steps)
            return PropertyResult(
                passed=False,
                cases_run=i + 1,
                seed=seed,
                counterexample=args,
                shrunken=shrunken,
                error=error,
                shrink_steps=steps,
            )

    return PropertyResult(passed=True, cases_run=count, seed=seed)
