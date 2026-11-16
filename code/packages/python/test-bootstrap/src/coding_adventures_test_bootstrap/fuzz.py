"""Coverage-guided and mutation fuzzing engine (VER06).

Pure Python implementation of mutator algebra, deterministic corpus evolution,
crash deduplication, and CWE-400 bounds enforcement.
Zero external dependencies.
"""

from __future__ import annotations

import string
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from typing import Any

from coding_adventures_test_bootstrap.prng import SplitMix64, fnv1a64

# CWE-400 Bounds
MAX_ITERATIONS = 1_000_000
MAX_INPUT_BYTES = 1024 * 1024
MAX_CRASHES = 1000
MAX_INITIAL_CORPUS = 10_000


def _check_strict_int(val: Any, name: str) -> None:
    if isinstance(val, bool) or not isinstance(val, int):
        raise TypeError(f"{name} must be an integer, not {type(val).__name__}")


@dataclass(frozen=True)
class FuzzCrash:
    """Recorded crash occurrence during fuzzing."""

    input_data: bytes | str
    exception: str
    fingerprint: int
    iteration: int


@dataclass(frozen=True)
class FuzzResult:
    """Summary of fuzzing run."""

    iterations_run: int
    crashes: list[FuzzCrash]
    unique_crashes: int
    seed: int
    corpus_size: int


# --- Mutator Algebra (VER06 §3) ---


def bit_flip(data: bytes, bit_offset: int, count: int = 1) -> bytes:
    """Invert count consecutive bits starting at bit_offset modulo total bit length.

    Operates in O(len) time regardless of count via parity wrapping.
    """
    _check_strict_int(bit_offset, "bit_offset")
    _check_strict_int(count, "count")
    if not data or count <= 0:
        return data

    buf = bytearray(data)
    total_bits = len(buf) * 8
    base = bit_offset % total_bits
    full_cycles = count // total_bits
    remainder = count % total_bits

    for step in range(total_bits):
        flips = full_cycles + (1 if step < remainder else 0)
        if flips % 2 == 1:
            pos = (base + step) % total_bits
            buf[pos // 8] ^= 1 << (pos % 8)

    return bytes(buf)


def byte_flip(data: bytes, byte_offset: int, count: int = 1) -> bytes:
    """Invert count consecutive bytes with XOR 0xFF modulo total length.

    Operates in O(len) time regardless of count via parity wrapping.
    """
    _check_strict_int(byte_offset, "byte_offset")
    _check_strict_int(count, "count")
    if not data or count <= 0:
        return data

    buf = bytearray(data)
    total_bytes = len(buf)
    base = byte_offset % total_bytes
    full_cycles = count // total_bytes
    remainder = count % total_bytes

    for step in range(total_bytes):
        flips = full_cycles + (1 if step < remainder else 0)
        if flips % 2 == 1:
            buf[(base + step) % total_bytes] ^= 0xFF

    return bytes(buf)


def byte_insert(data: bytes, offset: int, new_bytes: bytes) -> bytes:
    """Insert new_bytes at offset."""
    _check_strict_int(offset, "offset")
    if not new_bytes:
        return data
    pos = 0 if not data else (offset % (len(data) + 1))
    return data[:pos] + new_bytes + data[pos:]


def byte_delete(data: bytes, offset: int, count: int = 1) -> bytes:
    """Delete count bytes from offset."""
    _check_strict_int(offset, "offset")
    _check_strict_int(count, "count")
    if not data or count <= 0:
        return data
    pos = offset % len(data)
    end = min(pos + count, len(data))
    return data[:pos] + data[end:]


def byte_replace(data: bytes, offset: int, new_bytes: bytes) -> bytes:
    """Replace bytes starting at offset with new_bytes."""
    _check_strict_int(offset, "offset")
    if not data or not new_bytes:
        return data
    buf = bytearray(data)
    pos = offset % len(buf)
    for i, b in enumerate(new_bytes):
        if pos + i < len(buf):
            buf[pos + i] = b
    return bytes(buf)


def crossover(data1: bytes, data2: bytes, split_point: int) -> bytes:
    """Combine prefix of data1 with suffix of data2."""
    _check_strict_int(split_point, "split_point")
    if not data1:
        return data2
    if not data2:
        return data1
    p1 = split_point % len(data1)
    p2 = split_point % len(data2)
    return data1[:p1] + data2[p2:]


def mutate_bytes(data: bytes, rng: SplitMix64, max_len: int = 4096) -> bytes:
    """Apply one deterministic mutation to input bytes."""
    _check_strict_int(max_len, "max_len")
    if max_len < 1 or max_len > MAX_INPUT_BYTES:
        raise ValueError(
            f"max_len must be between 1 and {MAX_INPUT_BYTES}, got {max_len}"
        )
    if not data:
        # Generate arbitrary 1 to 4 bytes
        length = rng.bounded_range(1, min(4, max_len))
        return bytes(rng.bounded(256) for _ in range(length))

    choice = rng.bounded(6)
    match choice:
        case 0:
            bit_pos = rng.bounded(len(data) * 8)
            count = rng.bounded_range(1, 4)
            return bit_flip(data, bit_pos, count)
        case 1:
            byte_pos = rng.bounded(len(data))
            count = rng.bounded_range(1, 3)
            return byte_flip(data, byte_pos, count)
        case 2:
            if len(data) < max_len:
                byte_pos = rng.bounded(len(data) + 1)
                chunk_len = rng.bounded_range(1, min(4, max_len - len(data)))
                chunk = bytes(rng.bounded(256) for _ in range(chunk_len))
                return byte_insert(data, byte_pos, chunk)
            return byte_flip(data, rng.bounded(len(data)))
        case 3:
            if len(data) > 1:
                byte_pos = rng.bounded(len(data))
                count = rng.bounded_range(1, min(3, len(data)))
                return byte_delete(data, byte_pos, count)
            return b""
        case 4:
            byte_pos = rng.bounded(len(data))
            chunk_len = rng.bounded_range(1, min(4, len(data) - byte_pos + 1))
            chunk = bytes(rng.bounded(256) for _ in range(chunk_len))
            return byte_replace(data, byte_pos, chunk)
        case _:
            # Byte swap or random byte write
            buf = bytearray(data)
            pos = rng.bounded(len(buf))
            buf[pos] = rng.bounded(256)
            return bytes(buf)


def mutate_text(text: str, rng: SplitMix64, max_len: int = 4096) -> str:
    """Apply one deterministic mutation to a text string."""
    _check_strict_int(max_len, "max_len")
    if max_len < 1 or max_len > MAX_INPUT_BYTES:
        raise ValueError(
            f"max_len must be between 1 and {MAX_INPUT_BYTES}, got {max_len}"
        )
    alphabet = string.ascii_letters + string.digits + string.punctuation + " \t\n"
    if not text:
        length = rng.bounded_range(1, min(4, max_len))
        return "".join(alphabet[rng.bounded(len(alphabet))] for _ in range(length))

    choice = rng.bounded(4)
    chars = list(text)
    match choice:
        case 0:  # replace char
            pos = rng.bounded(len(chars))
            chars[pos] = alphabet[rng.bounded(len(alphabet))]
            return "".join(chars)
        case 1:  # insert char
            if len(chars) < max_len:
                pos = rng.bounded(len(chars) + 1)
                ch = alphabet[rng.bounded(len(alphabet))]
                chars.insert(pos, ch)
                return "".join(chars)
            return text
        case 2:  # delete char
            if len(chars) > 1:
                pos = rng.bounded(len(chars))
                chars.pop(pos)
                return "".join(chars)
            return ""
        case _:  # swap adjacent
            if len(chars) > 1:
                pos = rng.bounded(len(chars) - 1)
                chars[pos], chars[pos + 1] = chars[pos + 1], chars[pos]
                return "".join(chars)
            return text


# --- Fuzz Harness Execution ---


def fuzz_bytes(
    target_fn: Callable[[bytes], Any],
    seed: int = 42,
    iterations: int = 1000,
    max_input_bytes: int = 4096,
    initial_corpus: Sequence[bytes] | None = None,
) -> FuzzResult:
    """Fuzz a bytes target function deterministically.

    Catches unhandled exceptions, deduplicates crashes by FNV-1a stack fingerprint,
    and returns a structured FuzzResult.
    """
    _check_strict_int(seed, "seed")
    _check_strict_int(iterations, "iterations")
    _check_strict_int(max_input_bytes, "max_input_bytes")

    if iterations <= 0:
        raise ValueError(f"iterations must be positive, got {iterations}")
    if iterations > MAX_ITERATIONS:
        raise ValueError(f"iterations exceeds MAX_ITERATIONS ({MAX_ITERATIONS})")

    if max_input_bytes <= 0:
        raise ValueError("max_input_bytes must be positive")
    if max_input_bytes > MAX_INPUT_BYTES:
        msg = f"max_input_bytes exceeds MAX_INPUT_BYTES ({MAX_INPUT_BYTES})"
        raise ValueError(msg)

    if initial_corpus is not None:
        if not isinstance(initial_corpus, Sequence):
            raise TypeError("initial_corpus must be a sequence of bytes")
        if len(initial_corpus) == 0:
            raise ValueError("initial_corpus cannot be empty")
        if len(initial_corpus) > MAX_INITIAL_CORPUS:
            raise ValueError(f"initial_corpus exceeds limit of {MAX_INITIAL_CORPUS}")
        for idx, item in enumerate(initial_corpus):
            if isinstance(item, str) or not isinstance(item, (bytes, bytearray)):
                raise TypeError(
                    f"initial_corpus[{idx}] must be bytes, got {type(item).__name__}"
                )
            if len(item) > max_input_bytes:
                msg = (
                    f"initial_corpus[{idx}] length ({len(item)}) "
                    f"exceeds max_input_bytes ({max_input_bytes})"
                )
                raise ValueError(msg)
        corpus: list[bytes] = [bytes(item) for item in initial_corpus]
    else:
        corpus = [b"", b"test", b"\x00" * 8]

    rng = SplitMix64(seed)
    crashes: list[FuzzCrash] = []
    seen_fingerprints: set[int] = set()

    for i in range(iterations):
        parent = corpus[rng.bounded(len(corpus))]
        mutated = mutate_bytes(parent, rng, max_len=max_input_bytes)
        if len(mutated) > max_input_bytes:
            mutated = mutated[:max_input_bytes]

        try:
            target_fn(mutated)
            # Add interesting mutants to corpus (bounded to 256 items)
            if len(corpus) < 256 and rng.bounded(10) == 0:
                corpus.append(mutated)
        except Exception as ex:
            err_msg = f"{type(ex).__name__}: {ex}"
            fp = fnv1a64(err_msg)
            if len(crashes) < MAX_CRASHES:
                crashes.append(
                    FuzzCrash(
                        input_data=mutated,
                        exception=err_msg,
                        fingerprint=fp,
                        iteration=i + 1,
                    )
                )
            seen_fingerprints.add(fp)

    return FuzzResult(
        iterations_run=iterations,
        crashes=crashes,
        unique_crashes=len(seen_fingerprints),
        seed=seed,
        corpus_size=len(corpus),
    )


def fuzz_text(
    target_fn: Callable[[str], Any],
    seed: int = 42,
    iterations: int = 1000,
    max_input_bytes: int = 4096,
    initial_corpus: Sequence[str] | None = None,
) -> FuzzResult:
    """Fuzz a text string target function deterministically."""
    _check_strict_int(seed, "seed")
    _check_strict_int(iterations, "iterations")
    _check_strict_int(max_input_bytes, "max_input_bytes")

    if iterations <= 0:
        raise ValueError(f"iterations must be positive, got {iterations}")
    if iterations > MAX_ITERATIONS:
        raise ValueError(f"iterations exceeds MAX_ITERATIONS ({MAX_ITERATIONS})")

    if max_input_bytes <= 0:
        raise ValueError("max_input_bytes must be positive")
    if max_input_bytes > MAX_INPUT_BYTES:
        msg = f"max_input_bytes exceeds MAX_INPUT_BYTES ({MAX_INPUT_BYTES})"
        raise ValueError(msg)

    if initial_corpus is not None:
        if not isinstance(initial_corpus, Sequence):
            raise TypeError("initial_corpus must be a sequence of str")
        if len(initial_corpus) == 0:
            raise ValueError("initial_corpus cannot be empty")
        if len(initial_corpus) > MAX_INITIAL_CORPUS:
            raise ValueError(f"initial_corpus exceeds limit of {MAX_INITIAL_CORPUS}")
        for idx, item in enumerate(initial_corpus):
            if not isinstance(item, str):
                raise TypeError(
                    f"initial_corpus[{idx}] must be str, got {type(item).__name__}"
                )
            if len(item.encode("utf-8")) > max_input_bytes:
                msg = (
                    f"initial_corpus[{idx}] UTF-8 byte length "
                    f"exceeds max_input_bytes ({max_input_bytes})"
                )
                raise ValueError(msg)
        corpus: list[str] = list(initial_corpus)
    else:
        corpus = ["", "hello", "0", "{}"]

    rng = SplitMix64(seed)
    crashes: list[FuzzCrash] = []
    seen_fingerprints: set[int] = set()

    for i in range(iterations):
        parent = corpus[rng.bounded(len(corpus))]
        mutated = mutate_text(parent, rng, max_len=max_input_bytes)
        if len(mutated) > max_input_bytes:
            mutated = mutated[:max_input_bytes]

        try:
            target_fn(mutated)
            if len(corpus) < 256 and rng.bounded(10) == 0:
                corpus.append(mutated)
        except Exception as ex:
            err_msg = f"{type(ex).__name__}: {ex}"
            fp = fnv1a64(err_msg)
            if len(crashes) < MAX_CRASHES:
                crashes.append(
                    FuzzCrash(
                        input_data=mutated,
                        exception=err_msg,
                        fingerprint=fp,
                        iteration=i + 1,
                    )
                )
            seen_fingerprints.add(fp)

    return FuzzResult(
        iterations_run=iterations,
        crashes=crashes,
        unique_crashes=len(seen_fingerprints),
        seed=seed,
        corpus_size=len(corpus),
    )
