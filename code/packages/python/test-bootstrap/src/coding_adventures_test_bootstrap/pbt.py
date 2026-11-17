"""Property-based testing engine with shrinking and deterministic PRNG (VER05).

Pure Python implementation conforming to VER00/VER05/CWE-400 policies.
Zero external dependencies.
"""

from __future__ import annotations

import string
from collections.abc import Callable, Iterator, Sequence
from dataclasses import dataclass
from typing import Any

from coding_adventures_test_bootstrap.prng import SplitMix64

# CWE-400 Bounds
MAX_CASES = 100_000
MAX_SHRINK_STEPS = 10_000
MAX_INPUT_BYTES = 1024 * 1024
MAX_LIST_LEN = 100_000


def _check_strict_int(val: Any, name: str) -> None:
    if isinstance(val, bool) or not isinstance(val, int):
        raise TypeError(f"{name} must be an integer, not {type(val).__name__}")


@dataclass(frozen=True)
class PropertyResult:
    """Outcome of a property check run."""

    passed: bool
    cases_run: int
    seed: int
    counterexample: tuple[Any, ...] | None = None
    shrunken: tuple[Any, ...] | None = None
    error: str | None = None
    shrink_steps: int = 0


class Gen[T]:
    """Generator for property testing."""

    __slots__ = ("_sample_fn",)

    def __init__(self, sample_fn: Callable[[SplitMix64], T]) -> None:
        self._sample_fn = sample_fn

    def sample(self, rng: SplitMix64) -> T:
        """Draw one sample using the supplied PRNG."""
        return self._sample_fn(rng)

    def map[U](self, fn: Callable[[T], U]) -> Gen[U]:
        """Transform generated values."""
        return Gen(lambda rng: fn(self.sample(rng)))

    def filter(self, pred: Callable[[T], bool], max_retries: int = 100) -> Gen[T]:
        """Filter values by predicate, retrying up to max_retries."""
        _check_strict_int(max_retries, "max_retries")
        if max_retries <= 0:
            raise ValueError("max_retries must be positive")

        def _filtered(rng: SplitMix64) -> T:
            for _ in range(max_retries):
                val = self.sample(rng)
                if pred(val):
                    return val
            raise RuntimeError(f"Gen.filter exceeded max_retries ({max_retries})")

        return Gen(_filtered)

    def flat_map[U](self, fn: Callable[[T], Gen[U]]) -> Gen[U]:
        """Monadic bind for dependent generators."""
        return Gen(lambda rng: fn(self.sample(rng)).sample(rng))


# --- Generator Combinators ---


def gen_bool() -> Gen[bool]:
    """Generate a random boolean."""
    return Gen(lambda rng: rng.bounded(2) == 1)


def gen_int(min_val: int = -2_147_483_648, max_val: int = 2_147_483_647) -> Gen[int]:
    """Generate an integer in [min_val, max_val]."""
    _check_strict_int(min_val, "min_val")
    _check_strict_int(max_val, "max_val")
    if min_val > max_val:
        raise ValueError(f"min_val ({min_val}) cannot exceed max_val ({max_val})")
    return Gen(lambda rng: rng.bounded_range(min_val, max_val))


def gen_u8() -> Gen[int]:
    """Generate an 8-bit unsigned integer [0, 255]."""
    return Gen(lambda rng: rng.bounded(256))


def gen_u64() -> Gen[int]:
    """Generate a 64-bit unsigned integer [0, 2^64-1]."""
    return Gen(lambda rng: rng.next_u64())


def gen_float(min_val: float = -1e6, max_val: float = 1e6) -> Gen[float]:
    """Generate a finite float in [min_val, max_val]."""
    if min_val > max_val:
        raise ValueError("min_val cannot exceed max_val")
    span = max_val - min_val
    return Gen(lambda rng: min_val + rng.next_float() * span)


def gen_bytes(min_len: int = 0, max_len: int = 64) -> Gen[bytes]:
    """Generate bytes of length between min_len and max_len."""
    _check_strict_int(min_len, "min_len")
    _check_strict_int(max_len, "max_len")
    if min_len < 0 or max_len < min_len:
        raise ValueError("Invalid length bounds")
    if max_len > MAX_INPUT_BYTES:
        raise ValueError(f"max_len exceeds MAX_INPUT_BYTES ({MAX_INPUT_BYTES})")

    def _sample(rng: SplitMix64) -> bytes:
        length = rng.bounded_range(min_len, max_len)
        return bytes(rng.bounded(256) for _ in range(length))

    return Gen(_sample)


def gen_text(
    min_len: int = 0,
    max_len: int = 64,
    alphabet: str = string.ascii_letters + string.digits,
) -> Gen[str]:
    """Generate string from alphabet with length between min_len and max_len."""
    _check_strict_int(min_len, "min_len")
    _check_strict_int(max_len, "max_len")
    if min_len < 0 or max_len < min_len:
        raise ValueError("Invalid length bounds")
    if not alphabet:
        raise ValueError("Alphabet cannot be empty")
    if max_len > MAX_INPUT_BYTES:
        raise ValueError(f"max_len exceeds MAX_INPUT_BYTES ({MAX_INPUT_BYTES})")

    alpha_len = len(alphabet)

    def _sample(rng: SplitMix64) -> str:
        length = rng.bounded_range(min_len, max_len)
        return "".join(alphabet[rng.bounded(alpha_len)] for _ in range(length))

    return Gen(_sample)


def gen_choice[T](elements: Sequence[T]) -> Gen[T]:
    """Pick one element uniformly from non-empty sequence."""
    if not elements:
        raise ValueError("gen_choice requires non-empty sequence")
    items = list(elements)
    count = len(items)
    return Gen(lambda rng: items[rng.bounded(count)])


def gen_list[T](elem_gen: Gen[T], min_len: int = 0, max_len: int = 20) -> Gen[list[T]]:
    """Generate a list of elements bounded by MAX_LIST_LEN."""
    _check_strict_int(min_len, "min_len")
    _check_strict_int(max_len, "max_len")
    if min_len < 0 or max_len < min_len:
        raise ValueError("Invalid length bounds")
    if max_len > MAX_LIST_LEN:
        raise ValueError(f"max_len exceeds MAX_LIST_LEN ({MAX_LIST_LEN})")

    def _sample(rng: SplitMix64) -> list[T]:
        length = rng.bounded_range(min_len, max_len)
        return [elem_gen.sample(rng) for _ in range(length)]

    return Gen(_sample)


def gen_one_of[T](*gens: Gen[T]) -> Gen[T]:
    """Pick one of the provided generators uniformly."""
    if not gens:
        raise ValueError("gen_one_of requires at least one generator")
    gen_list_copy = list(gens)
    count = len(gen_list_copy)
    return Gen(lambda rng: gen_list_copy[rng.bounded(count)].sample(rng))


# --- Shrinking Primitives ---


def shrink_int(val: int, target: int = 0) -> Iterator[int]:
    """Shrink an integer towards target by binary bisection."""
    _check_strict_int(val, "val")
    _check_strict_int(target, "target")
    if val == target:
        return
    diff = val - target
    yield target
    half = diff // 2
    while half != 0:
        yield val - half
        half = half // 2


def shrink_bytes(data: bytes) -> Iterator[bytes]:
    """Shrink bytes by removing chunks and simplifying values."""
    length = len(data)
    if length == 0:
        return
    yield b""
    for k in (2, 4, 8):
        if length >= k:
            chunk = length // k
            for i in range(k):
                yield data[: i * chunk] + data[(i + 1) * chunk :]
    for i in range(length):
        yield data[:i] + data[i + 1 :]
    for i in range(length):
        if data[i] != 0:
            yield data[:i] + b"\x00" + data[i + 1 :]


def shrink_text(s: str) -> Iterator[str]:
    """Shrink a string by removing segments and simplifying characters."""
    length = len(s)
    if length == 0:
        return
    yield ""
    for k in (2, 4):
        if length >= k:
            chunk = length // k
            for i in range(k):
                yield s[: i * chunk] + s[(i + 1) * chunk :]
    for i in range(length):
        yield s[:i] + s[i + 1 :]
    for i in range(length):
        if s[i] != "a":
            yield s[:i] + "a" + s[i + 1 :]


def shrink_list(items: list[Any]) -> Iterator[list[Any]]:
    """Shrink a list by removing elements and shrinking individual elements."""
    length = len(items)
    if length == 0:
        return
    yield []
    for k in (2, 4):
        if length >= k:
            chunk = length // k
            for i in range(k):
                yield items[: i * chunk] + items[(i + 1) * chunk :]
    for i in range(length):
        yield items[:i] + items[i + 1 :]
    for i in range(length):
        for shrunk_elem in shrink_value(items[i]):
            cand = list(items)
            cand[i] = shrunk_elem
            yield cand


def shrink_value(val: Any) -> Iterator[Any]:
    """Dispatch shrinking based on value type."""
    if isinstance(val, bool):
        if val:
            yield False
    elif isinstance(val, int):
        yield from shrink_int(val)
    elif isinstance(val, bytes):
        yield from shrink_bytes(val)
    elif isinstance(val, str):
        yield from shrink_text(val)
    elif isinstance(val, list):
        yield from shrink_list(val)
    elif isinstance(val, tuple):
        for shrunk_list in shrink_list(list(val)):
            yield tuple(shrunk_list)


def _shrink_counterexample(
    property_fn: Callable[..., Any],
    args: tuple[Any, ...],
    max_steps: int,
) -> tuple[tuple[Any, ...], int]:
    """Iteratively shrink failing arguments toward minimal failing case."""
    current = args
    steps = 0

    improved = True
    while improved and steps < max_steps:
        improved = False
        for arg_idx in range(len(current)):
            for shrunk_val in shrink_value(current[arg_idx]):
                steps += 1
                candidate = list(current)
                candidate[arg_idx] = shrunk_val
                cand_tuple = tuple(candidate)
                try:
                    res = property_fn(*cand_tuple)
                    still_fails = res is False
                except Exception:
                    still_fails = True

                if still_fails:
                    current = cand_tuple
                    improved = True
                    break
            if improved or steps >= max_steps:
                break

    return current, steps


# --- Main Property Runner ---


def for_all(
    property_fn: Callable[..., Any],
    *generators: Gen[Any],
    count: int = 100,
    seed: int = 42,
    max_shrink_steps: int = 100,
) -> PropertyResult:
    """Run property-based test with deterministic SplitMix64 PRNG and shrinking.

    Enforces CWE-400 bounds and rejects invalid / non-positive parameters.
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

    rng = SplitMix64(seed)

    for i in range(count):
        args = tuple(g.sample(rng) for g in generators)
        try:
            res = property_fn(*args)
            if res is False:
                shrunken, steps = _shrink_counterexample(
                    property_fn, args, max_shrink_steps
                )
                return PropertyResult(
                    passed=False,
                    cases_run=i + 1,
                    seed=seed,
                    counterexample=args,
                    shrunken=shrunken,
                    error="Property returned False",
                    shrink_steps=steps,
                )
        except Exception as ex:
            shrunken, steps = _shrink_counterexample(
                property_fn, args, max_shrink_steps
            )
            return PropertyResult(
                passed=False,
                cases_run=i + 1,
                seed=seed,
                counterexample=args,
                shrunken=shrunken,
                error=f"{type(ex).__name__}: {ex}",
                shrink_steps=steps,
            )

    return PropertyResult(
        passed=True,
        cases_run=count,
        seed=seed,
    )
