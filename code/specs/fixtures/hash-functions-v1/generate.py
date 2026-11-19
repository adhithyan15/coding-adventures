"""Generate the checked-in DT17 v1 fixture from the independent oracle."""

from __future__ import annotations

import json
from pathlib import Path

from oracle import (
    MASK64,
    MURMUR_SEED,
    POLY_BASE,
    POLY_MODULUS,
    SIPHASH_KEY,
    U32_MAX,
    U64_MAX,
    avalanche_counts,
    distribution_counts,
    djb2,
    expected_hashes,
    fixed_hex,
    fnv1a32,
    fnv1a64,
    hash_string_fnv1a32,
    hash_string_siphash,
    murmur3_32,
    polynomial_rolling,
    siphash24,
)

ROOT = Path(__file__).resolve().parent
SEED = 0x445431375F7631  # "DT17_v1", SplitMix64 stream seed.
PARAM_POLY_BASE = 257


class SplitMix64:
    def __init__(self, seed: int):
        self.state = seed & MASK64

    def next(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK64
        value = self.state
        value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
        value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & MASK64
        return (value ^ (value >> 31)) & MASK64


def _bytes_from_rng(rng: SplitMix64, size: int) -> bytes:
    output = bytearray()
    while len(output) < size:
        output.extend(rng.next().to_bytes(8, "little"))
    return bytes(output[:size])


def _record(case_id: str, data: bytes, *, seed: int, key: bytes, base: int, modulus: int) -> dict:
    return {
        "id": case_id,
        "data_hex": data.hex(),
        "parameters": {
            "murmur3_32_with_seed": {"seed": f"0x{seed:08x}"},
            "polynomial_rolling_with_params": {
                "base": str(base),
                "modulus": str(modulus),
            },
            "siphash_2_4": {"key_hex": key.hex()},
        },
        "expected_hex": expected_hashes(data, seed=seed, key=key, base=base, modulus=modulus),
    }


# Literal answers transcribed from the published vectors and Warrant's
# independently sourced known-answer examples. These values are not generated
# from the oracle below; validate.py checks the independent model against them,
# and the Rust consumer checks the actual public API against both.
_KNOWN = [
    ("fnv32-empty", "fnv1a_32", "", {}, "2166136261"),
    ("fnv32-a", "fnv1a_32", "61", {}, "3826002220"),
    ("fnv32-abc", "fnv1a_32", "616263", {}, "440920331"),
    ("fnv32-hello", "fnv1a_32", "68656c6c6f", {}, "1335831723"),
    ("fnv32-foobar", "fnv1a_32", "666f6f626172", {}, "3214735720"),
    ("fnv32-fox", "fnv1a_32", "54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f67", {}, "76545936"),
    ("fnv64-empty", "fnv1a_64", "", {}, "14695981039346656037"),
    ("fnv64-a", "fnv1a_64", "61", {}, "12638187200555641996"),
    ("fnv64-abc", "fnv1a_64", "616263", {}, "16654208175385433931"),
    ("fnv64-hello", "fnv1a_64", "68656c6c6f", {}, "11831194018420276491"),
    ("fnv64-foobar", "fnv1a_64", "666f6f626172", {}, "9625390261332436968"),
    ("fnv64-fox", "fnv1a_64", "54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f67", {}, "17580284887202820368"),
    ("djb2-empty", "djb2", "", {}, "5381"),
    ("djb2-a", "djb2", "61", {}, "177670"),
    ("djb2-abc", "djb2", "616263", {}, "193485963"),
    ("djb2-hello", "djb2", "68656c6c6f", {}, "210714636441"),
    ("djb2-foobar", "djb2", "666f6f626172", {}, "6953516687550"),
    ("djb2-fox", "djb2", "54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f67", {}, "3950289020261251294"),
    ("poly-empty", "polynomial_rolling", "", {}, "0"),
    ("poly-a", "polynomial_rolling", "61", {}, "97"),
    ("poly-ab", "polynomial_rolling", "6162", {}, "3105"),
    ("poly-abc", "polynomial_rolling", "616263", {}, "96354"),
    ("poly-fox", "polynomial_rolling", "54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f67", {}, "1508044031921644550"),
    ("poly-params-abc", "polynomial_rolling_with_params", "616263", {"base": "37", "modulus": "1000000007"}, "136518"),
    ("poly-params-fox", "polynomial_rolling_with_params", "54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f67", {"base": "257", "modulus": "65521"}, "420"),
    ("poly-params-u64-max", "polynomial_rolling_with_params", "6162", {"base": str(U64_MAX), "modulus": str(U64_MAX)}, "98"),
    ("poly-params-modulus-one", "polynomial_rolling_with_params", "616263", {"base": "31", "modulus": "1"}, "0"),
    ("murmur-empty", "murmur3_32", "", {}, "0"),
    ("murmur-a", "murmur3_32", "61", {}, "1009084850"),
    ("murmur-abc", "murmur3_32", "616263", {}, "3017643002"),
    ("murmur-abcd", "murmur3_32", "61626364", {}, "1139631978"),
    ("murmur-hello", "murmur3_32", "68656c6c6f", {}, "613153351"),
    ("murmur-fox", "murmur3_32", "54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f67", {}, "776992547"),
    ("murmur-seed-empty-1", "murmur3_32_with_seed", "", {"seed": "1"}, "1364076727"),
    ("murmur-seed-empty-max", "murmur3_32_with_seed", "", {"seed": str(U32_MAX)}, "2180083513"),
    ("murmur-seed-hello-world", "murmur3_32_with_seed", "48656c6c6f2c20776f726c6421", {"seed": str(MURMUR_SEED)}, "612912314"),
    ("murmur-seed-fox", "murmur3_32_with_seed", "54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f67", {"seed": str(MURMUR_SEED)}, "799549133"),
    ("murmur-five-blocks", "murmur3_32", "0102030405060708090a0b0c0d0e0f1011121314", {}, "2631643533"),
    ("murmur-five-blocks-seed", "murmur3_32_with_seed", "0102030405060708090a0b0c0d0e0f1011121314", {"seed": str(MURMUR_SEED)}, "937835026"),
    ("siphash-zero-bytes", "siphash_2_4", "", {"key_hex": SIPHASH_KEY.hex()}, "8246050544436514353"),
    ("siphash-one-zero-byte", "siphash_2_4", "00", {"key_hex": SIPHASH_KEY.hex()}, "8428550223375919101"),
    ("siphash-15", "siphash_2_4", bytes(range(15)).hex(), {"key_hex": SIPHASH_KEY.hex()}, "11613035633349379557"),
    ("siphash-16", "siphash_2_4", bytes(range(16)).hex(), {"key_hex": SIPHASH_KEY.hex()}, "4551675220716592091"),
    ("siphash-17", "siphash_2_4", bytes(range(17)).hex(), {"key_hex": SIPHASH_KEY.hex()}, "7609651759622801300"),
    ("siphash-24", "siphash_2_4", bytes(range(24)).hex(), {"key_hex": SIPHASH_KEY.hex()}, "13307381289415454612"),
    ("siphash-32", "siphash_2_4", bytes(range(32)).hex(), {"key_hex": SIPHASH_KEY.hex()}, "8153574914611379406"),
    ("siphash-63", "siphash_2_4", bytes(range(63)).hex(), {"key_hex": SIPHASH_KEY.hex()}, "10775480364379293042"),
    ("siphash-zero-key-hello", "siphash_2_4", "68656c6c6f", {"key_hex": "00" * 16}, "10142490492830962361"),
]


def _known_answers() -> list[dict]:
    widths = {
        "fnv1a_32": 32,
        "murmur3_32": 32,
        "murmur3_32_with_seed": 32,
        "fnv1a_64": 64,
        "djb2": 64,
        "polynomial_rolling": 64,
        "polynomial_rolling_with_params": 64,
        "siphash_2_4": 64,
    }
    output = []
    for case_id, op, data_hex, params, expected_decimal in _KNOWN:
        row = {
            "id": case_id,
            "op": op,
            "data_hex": data_hex,
            "expected_hex": fixed_hex(int(expected_decimal), widths[op]),
            "provenance": "DT17 Warrant known-answer literal; public Rust checked",
        }
        if params:
            row["parameters"] = params
        output.append(row)

    return output


def _utf8_cases() -> list[dict]:
    output = []
    for case_id, op, text, expected in [
        ("hash-string-fnv32-cafe", "hash_str_fnv1a_32", "café", hash_string_fnv1a32("café")),
        ("hash-string-fnv32-supplementary", "hash_str_fnv1a_32", "🧪", hash_string_fnv1a32("🧪")),
        ("hash-string-siphash-cafe", "hash_str_siphash", "café", hash_string_siphash("café", SIPHASH_KEY)),
        ("hash-string-siphash-embedded-null", "hash_str_siphash", "a\u0000b", hash_string_siphash("a\u0000b", SIPHASH_KEY)),
    ]:
        bits = 32 if op == "hash_str_fnv1a_32" else 64
        row = {
            "id": case_id,
            "op": op,
            "text": text,
            "expected_hex": fixed_hex(expected, bits),
            "provenance": "independent UTF-8 byte oracle; Rust public helper checked",
        }
        if op == "hash_str_siphash":
            row["parameters"] = {"key_hex": SIPHASH_KEY.hex()}
        output.append(row)
    return output


def _directed() -> list[dict]:
    rows = []
    boundaries = list(range(12)) + [15, 16, 17, 24, 32, 63, 64, 65, 127]
    for length in boundaries:
        data = bytes((index * 37 + 0x80) & 0xFF for index in range(length))
        key = bytes((0xFF - index) & 0xFF for index in range(16)) if length % 2 else SIPHASH_KEY
        seed = U32_MAX if length == 11 else MURMUR_SEED
        base = U64_MAX if length >= 64 else POLY_BASE + length
        modulus = U64_MAX if length >= 64 else max(1, POLY_MODULUS - length)
        rows.append(_record(f"boundary-length-{length}", data, seed=seed, key=key, base=base, modulus=modulus))

    rows.extend(
        [
            _record("wrap-all-ff-256", bytes([0xFF]) * 256, seed=U32_MAX, key=bytes([0xFF]) * 16, base=U64_MAX, modulus=U64_MAX),
            _record("all-byte-values", bytes(range(256)), seed=0, key=bytes(range(15, -1, -1)), base=U64_MAX, modulus=U64_MAX),
            _record("polynomial-large-intermediate", bytes([0xFE]) * 32, seed=MURMUR_SEED, key=SIPHASH_KEY, base=U64_MAX, modulus=U64_MAX),
            _record("polynomial-modulus-one", bytes(range(64)), seed=MURMUR_SEED, key=SIPHASH_KEY, base=U64_MAX, modulus=1),
            _record("murmur-max-seed", bytes(range(251)), seed=U32_MAX, key=SIPHASH_KEY, base=POLY_BASE, modulus=POLY_MODULUS),
            _record("siphash-high-bit-key", bytes(range(31)), seed=MURMUR_SEED, key=bytes([0x80]) * 16, base=POLY_BASE, modulus=POLY_MODULUS),
            _record("multi-block-deterministic", bytes((index * 131 + 17) & 0xFF for index in range(1024)), seed=MURMUR_SEED, key=SIPHASH_KEY, base=POLY_BASE, modulus=POLY_MODULUS),
        ]
    )
    return rows


def _generated_cases() -> list[dict]:
    rng = SplitMix64(SEED)
    cases = []
    for index in range(128):
        size = rng.next() % 513
        data = _bytes_from_rng(rng, size)
        seed = rng.next() & U32_MAX
        key = _bytes_from_rng(rng, 16)
        base = rng.next()
        modulus = 1 + rng.next() % U64_MAX
        cases.append(_record(f"splitmix-{index:03}", data, seed=seed, key=key, base=base, modulus=modulus))
    return cases


def _mutations() -> list[dict]:
    rng = SplitMix64(SEED ^ 0x4D55544154494F4E)
    rows = []
    for index in range(32):
        size = 1 + rng.next() % 64
        data = bytearray(_bytes_from_rng(rng, size))
        bit = rng.next() % (size * 8)
        changed = bytearray(data)
        changed[bit // 8] ^= 1 << (bit % 8)
        seed = rng.next() & U32_MAX
        key = _bytes_from_rng(rng, 16)
        base = rng.next()
        modulus = 1 + rng.next() % U64_MAX
        rows.append(
            {
                "id": f"byte-flip-{index:02}",
                "bit_index": bit,
                "original": _record(f"byte-flip-{index:02}-before", bytes(data), seed=seed, key=key, base=base, modulus=modulus),
                "mutated": _record(f"byte-flip-{index:02}-after", bytes(changed), seed=seed, key=key, base=base, modulus=modulus),
            }
        )
    return rows


def _analysis() -> list[dict]:
    first_byte = lambda data: data[0]
    avalanche_samples = [bytes.fromhex(value) for value in ("01", "23", "7f")]
    flip_count, compared = avalanche_counts(first_byte, avalanche_samples, 8)
    synthetic_avalanche = {
        "id": "synthetic-first-byte-avalanche",
        "kind": "avalanche",
        "reference_scope": "profile-only-synthetic",
        "callback": {"kind": "synthetic", "name": "first_byte"},
        "output_bits": 8,
        "samples_hex": [sample.hex() for sample in avalanche_samples],
        "expected": {"flipped_bits": str(flip_count), "compared_bits": str(compared)},
    }

    fnv_samples = [bytes.fromhex(value) for value in ("0001020304050607", "1020304050607080", "ffffffffffffffff")]
    flip_count, compared = avalanche_counts(fnv1a32, fnv_samples, 32)
    fnv_avalanche = {
        "id": "fnv1a32-explicit-sample-avalanche",
        "kind": "avalanche",
        "reference_scope": "public-hash-callback-only",
        "callback": {"kind": "hash", "op": "fnv1a_32"},
        "output_bits": 32,
        "samples_hex": [sample.hex() for sample in fnv_samples],
        "expected": {"flipped_bits": str(flip_count), "compared_bits": str(compared)},
    }

    synthetic_inputs = [bytes([value]) for value in (0, 1, 2, 3, 4, 5, 6, 7)]
    bucket_counts, numerator, denominator = distribution_counts(first_byte, synthetic_inputs, 4)
    synthetic_distribution = {
        "id": "synthetic-first-byte-distribution",
        "kind": "distribution",
        "reference_scope": "profile-only-synthetic",
        "callback": {"kind": "synthetic", "name": "first_byte"},
        "inputs_hex": [item.hex() for item in synthetic_inputs],
        "bucket_count": 4,
        "expected": {
            "bucket_counts": bucket_counts,
            "chi_squared_numerator": str(numerator),
            "chi_squared_denominator": str(denominator),
        },
    }

    dist_inputs = [bytes.fromhex(value) for value in ("00", "01", "02", "03", "10", "11", "fe", "ff", "0001", "0100")]
    bucket_counts, numerator, denominator = distribution_counts(fnv1a32, dist_inputs, 7)
    fnv_distribution = {
        "id": "fnv1a32-explicit-sample-distribution",
        "kind": "distribution",
        "reference_scope": "public-hash-callback-only",
        "callback": {"kind": "hash", "op": "fnv1a_32"},
        "inputs_hex": [item.hex() for item in dist_inputs],
        "bucket_count": 7,
        "expected": {
            "bucket_counts": bucket_counts,
            "chi_squared_numerator": str(numerator),
            "chi_squared_denominator": str(denominator),
        },
    }
    return [synthetic_avalanche, fnv_avalanche, synthetic_distribution, fnv_distribution]


def build_fixture() -> dict:
    cases = []
    for length in range(3):
        if length == 0:
            inputs = [b""]
        elif length == 1:
            inputs = [bytes([value]) for value in range(256)]
        else:
            inputs = [bytes([left, right]) for left in range(256) for right in range(256)]
        cases.extend(inputs)

    return {
        "schema": "dt17-hash-functions-v1",
        "oracle": "dt17-exact-arithmetic-v1",
        "exhaustive": {
            "domain": {
                "lengths": [0, 1, 2],
                "order": "length-then-lexicographic-bytes",
                "count": 65793,
            },
            "fixed_parameters": {
                "murmur3_32_with_seed": f"0x{MURMUR_SEED:08x}",
                "siphash_2_4_key_hex": SIPHASH_KEY.hex(),
                "polynomial_rolling_default_base": str(POLY_BASE),
                "polynomial_base": str(PARAM_POLY_BASE),
                "polynomial_modulus": str(POLY_MODULUS),
            },
            "expected_hex_streams": {
                name: "".join(
                    fixed_hex(function(data), bits)
                    for data in cases
                )
                for name, function, bits in [
                    ("fnv1a_32", fnv1a32, 32),
                    ("fnv1a_64", fnv1a64, 64),
                    ("djb2", djb2, 64),
                    ("polynomial_rolling", polynomial_rolling, 64),
                    (
                        "polynomial_rolling_with_params",
                        lambda data: polynomial_rolling(data, PARAM_POLY_BASE, POLY_MODULUS),
                        64,
                    ),
                    ("murmur3_32", murmur3_32, 32),
                    ("murmur3_32_with_seed", lambda data: murmur3_32(data, MURMUR_SEED), 32),
                    ("siphash_2_4", lambda data: siphash24(data, SIPHASH_KEY), 64),
                ]
            },
        },
        "known_answers": _known_answers(),
        "utf8_cases": _utf8_cases(),
        "directed": _directed(),
        "generated": {
            "algorithm": "SplitMix64-v1",
            "seed_hex": f"0x{SEED:016x}",
            "count": 128,
            "cases": _generated_cases(),
        },
        "mutations": {
            "algorithm": "SplitMix64-v1",
            "seed_hex": f"0x{(SEED ^ 0x4D55544154494F4E):016x}",
            "bit_flips": 32,
            "cases": _mutations(),
        },
        "analysis": _analysis(),
        "dart_only_invalid": [
            {"id": "negative-byte", "op": "fnv1a_32", "data": [-1], "expected_error": "ArgumentError", "rust_reference": "not-representable-in-u8-slice"},
            {"id": "byte-above-255", "op": "fnv1a_32", "data": [256], "expected_error": "ArgumentError", "rust_reference": "not-representable-in-u8-slice"},
            {"id": "seed-above-u32", "op": "murmur3_32_with_seed", "seed": str(U32_MAX + 1), "expected_error": "ArgumentError", "rust_reference": "not-representable-in-u32"},
            {"id": "negative-poly-base", "op": "polynomial_rolling_with_params", "base": "-1", "modulus": "17", "expected_error": "ArgumentError", "rust_reference": "not-representable-in-u64"},
            {"id": "poly-modulus-zero", "op": "polynomial_rolling_with_params", "base": "31", "modulus": "0", "expected_error": "ArgumentError", "rust_reference": "panics-assertion-no-native-error-value"},
            {"id": "siphash-short-key", "op": "siphash_2_4", "key_hex": "00" * 15, "expected_error": "ArgumentError", "rust_reference": "not-representable-in-array-u8-16"},
            {"id": "string-unpaired-surrogate", "op": "hash_str_fnv1a_32", "code_points": [0xD800], "expected_error": "ArgumentError", "rust_reference": "not-representable-in-valid-utf8-str"},
            {"id": "avalanche-empty-sample-list", "op": "avalanche", "samples_hex": [], "expected_error": "ArgumentError", "rust_reference": "analysis-api-has-rng-sample-count-not-explicit-samples"},
            {"id": "distribution-empty-input-list", "op": "distribution", "inputs_hex": [], "bucket_count": 4, "expected_error": "ArgumentError", "rust_reference": "Rust-analysis-panics-but-accepts-iterator"},
            {"id": "distribution-zero-buckets", "op": "distribution", "inputs_hex": ["61"], "bucket_count": 0, "expected_error": "ArgumentError", "rust_reference": "Rust-analysis-panics-but-has-no-Dart-error-contract"},
            {"id": "avalanche-zero-output-bits", "op": "avalanche", "samples_hex": ["01"], "output_bits": 0, "expected_error": "ArgumentError", "rust_reference": "profile-only-analysis-contract"},
            {"id": "avalanche-output-bits-over-u64", "op": "avalanche", "samples_hex": ["01"], "output_bits": 65, "expected_error": "ArgumentError", "rust_reference": "profile-only-analysis-contract"},
            {"id": "avalanche-negative-callback", "op": "avalanche", "samples_hex": ["01"], "output_bits": 8, "callback_result": "-1", "expected_error": "ArgumentError", "rust_reference": "profile-only-analysis-contract"},
            {"id": "avalanche-callback-too-wide", "op": "avalanche", "samples_hex": ["01"], "output_bits": 8, "callback_result": "256", "expected_error": "ArgumentError", "rust_reference": "profile-only-analysis-contract"},
            {"id": "distribution-negative-callback", "op": "distribution", "inputs_hex": [""], "bucket_count": 4, "callback_result": "-1", "expected_error": "ArgumentError", "rust_reference": "profile-only-analysis-contract"},
            {"id": "distribution-callback-over-u64", "op": "distribution", "inputs_hex": [""], "bucket_count": 4, "callback_result": str(U64_MAX + 1), "expected_error": "ArgumentError", "rust_reference": "profile-only-analysis-contract"},
        ],
        "rust_panics": [
            {"id": "poly-modulus-zero-rust-reference", "op": "polynomial_rolling_with_params", "data_hex": "616263", "base": "31", "modulus": "0", "behavior": "panics"}
        ],
    }


if __name__ == "__main__":
    (ROOT / "cases.json").write_text(
        json.dumps(build_fixture(), ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print("wrote cases.json from exact independent arithmetic model")
