"""Independent exact-integer DT17 hash and analysis reference models.

This module is deliberately written in Python's unbounded integer arithmetic.
It does not import or call the Rust or Dart implementations.
"""

from __future__ import annotations

MASK32 = (1 << 32) - 1
MASK64 = (1 << 64) - 1
U32_MAX = MASK32
U64_MAX = MASK64
FNV32_OFFSET = 0x811C9DC5
FNV32_PRIME = 0x01000193
FNV64_OFFSET = 0xCBF29CE484222325
FNV64_PRIME = 0x00000100000001B3
POLY_BASE = 31
POLY_MODULUS = (1 << 61) - 1
MURMUR_SEED = 0x9747B28C
SIPHASH_KEY = bytes(range(16))


def fnv1a32(data: bytes) -> int:
    value = FNV32_OFFSET
    for byte in data:
        value = ((value ^ byte) * FNV32_PRIME) & MASK32
    return value


def fnv1a64(data: bytes) -> int:
    value = FNV64_OFFSET
    for byte in data:
        value = ((value ^ byte) * FNV64_PRIME) & MASK64
    return value


def djb2(data: bytes) -> int:
    value = 5381
    for byte in data:
        value = (value * 33 + byte) & MASK64
    return value


def polynomial_rolling(data: bytes, base: int = POLY_BASE, modulus: int = POLY_MODULUS) -> int:
    if not 0 <= base <= U64_MAX:
        raise ValueError("base outside u64")
    if not 1 <= modulus <= U64_MAX:
        raise ValueError("modulus outside positive u64")
    value = 0
    for byte in data:
        # Python int is unbounded: this is the mathematical intermediate, not
        # a wrapped U64 multiplication.
        value = (value * base + byte) % modulus
    return value


def _rotl32(value: int, count: int) -> int:
    count %= 32
    value &= MASK32
    return ((value << count) | (value >> (32 - count))) & MASK32


def murmur3_32(data: bytes, seed: int = 0) -> int:
    if not 0 <= seed <= U32_MAX:
        raise ValueError("seed outside u32")
    c1 = 0xCC9E2D51
    c2 = 0x1B873593
    value = seed
    end = len(data) & ~3
    for offset in range(0, end, 4):
        word = int.from_bytes(data[offset : offset + 4], "little")
        word = (word * c1) & MASK32
        word = _rotl32(word, 15)
        word = (word * c2) & MASK32
        value ^= word
        value = _rotl32(value, 13)
        value = (value * 5 + 0xE6546B64) & MASK32

    tail = data[end:]
    word = int.from_bytes(tail, "little")
    if tail:
        word = (word * c1) & MASK32
        word = _rotl32(word, 15)
        word = (word * c2) & MASK32
        value ^= word

    value ^= len(data) & MASK32
    value ^= value >> 16
    value = (value * 0x85EBCA6B) & MASK32
    value ^= value >> 13
    value = (value * 0xC2B2AE35) & MASK32
    value ^= value >> 16
    return value & MASK32


def _rotl64(value: int, count: int) -> int:
    count %= 64
    value &= MASK64
    return ((value << count) | (value >> (64 - count))) & MASK64


def _sipround(state: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
    v0, v1, v2, v3 = state
    v0 = (v0 + v1) & MASK64
    v1 = _rotl64(v1, 13) ^ v0
    v0 = _rotl64(v0, 32)
    v2 = (v2 + v3) & MASK64
    v3 = _rotl64(v3, 16) ^ v2
    v0 = (v0 + v3) & MASK64
    v3 = _rotl64(v3, 21) ^ v0
    v2 = (v2 + v1) & MASK64
    v1 = _rotl64(v1, 17) ^ v2
    v2 = _rotl64(v2, 32)
    return v0, v1, v2, v3


def siphash24(data: bytes, key: bytes) -> int:
    if len(key) != 16:
        raise ValueError("SipHash key must contain exactly 16 bytes")
    k0 = int.from_bytes(key[:8], "little")
    k1 = int.from_bytes(key[8:], "little")
    v0 = 0x736F6D6570736575 ^ k0
    v1 = 0x646F72616E646F6D ^ k1
    v2 = 0x6C7967656E657261 ^ k0
    v3 = 0x7465646279746573 ^ k1

    end = len(data) & ~7
    for offset in range(0, end, 8):
        word = int.from_bytes(data[offset : offset + 8], "little")
        v3 ^= word
        state = _sipround((v0, v1, v2, v3))
        state = _sipround(state)
        v0, v1, v2, v3 = state
        v0 ^= word

    tail = data[end:]
    last = (len(data) & 0xFF) << 56
    last |= int.from_bytes(tail, "little")
    v3 ^= last
    state = _sipround((v0, v1, v2, v3))
    state = _sipround(state)
    v0, v1, v2, v3 = state
    v0 ^= last
    v2 ^= 0xFF
    for _ in range(4):
        v0, v1, v2, v3 = _sipround((v0, v1, v2, v3))
    return (v0 ^ v1 ^ v2 ^ v3) & MASK64


def hash_string_fnv1a32(text: str) -> int:
    return fnv1a32(text.encode("utf-8"))


def hash_string_siphash(text: str, key: bytes) -> int:
    return siphash24(text.encode("utf-8"), key)


def expected_hashes(
    data: bytes,
    *,
    seed: int = MURMUR_SEED,
    key: bytes = SIPHASH_KEY,
    base: int = POLY_BASE,
    modulus: int = POLY_MODULUS,
) -> dict[str, str]:
    return {
        "fnv1a_32": fixed_hex(fnv1a32(data), 32),
        "fnv1a_64": fixed_hex(fnv1a64(data), 64),
        "djb2": fixed_hex(djb2(data), 64),
        "polynomial_rolling": fixed_hex(polynomial_rolling(data), 64),
        "polynomial_rolling_with_params": fixed_hex(polynomial_rolling(data, base, modulus), 64),
        "murmur3_32": fixed_hex(murmur3_32(data), 32),
        "murmur3_32_with_seed": fixed_hex(murmur3_32(data, seed), 32),
        "siphash_2_4": fixed_hex(siphash24(data, key), 64),
    }


def fixed_hex(value: int, bits: int) -> str:
    return f"{value:0{bits // 4}x}"


def avalanche_counts(
    hash_fn, samples: list[bytes], output_bits: int
) -> tuple[int, int]:
    if not samples or not 1 <= output_bits <= 64 or any(not sample for sample in samples):
        raise ValueError("invalid avalanche corpus")
    mask = (1 << output_bits) - 1
    changed = 0
    compared = 0
    for sample in samples:
        original = hash_fn(sample) & mask
        for bit in range(len(sample) * 8):
            mutated = bytearray(sample)
            mutated[bit // 8] ^= 1 << (bit % 8)
            changed += (original ^ (hash_fn(bytes(mutated)) & mask)).bit_count()
            compared += output_bits
    return changed, compared


def distribution_counts(hash_fn, inputs: list[bytes], bucket_count: int) -> tuple[list[int], int, int]:
    if not inputs or bucket_count <= 0:
        raise ValueError("invalid distribution corpus")
    buckets = [0] * bucket_count
    for data in inputs:
        buckets[hash_fn(data) % bucket_count] += 1
    total = len(inputs)
    numerator = sum((bucket_count * count - total) ** 2 for count in buckets)
    denominator = bucket_count * total
    divisor = __import__("math").gcd(numerator, denominator)
    return buckets, numerator // divisor, denominator // divisor
