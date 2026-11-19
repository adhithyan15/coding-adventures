"""Validate the neutral hash fixture against the exact independent oracle."""

from __future__ import annotations

import json
import sys

from generate import ROOT, build_fixture
from oracle import (
    MURMUR_SEED,
    POLY_BASE,
    POLY_MODULUS,
    SIPHASH_KEY,
    avalanche_counts,
    distribution_counts,
    djb2,
    fixed_hex,
    fnv1a32,
    fnv1a64,
    hash_string_fnv1a32,
    hash_string_siphash,
    murmur3_32,
    polynomial_rolling,
    siphash24,
)

WIDTHS = {
    "fnv1a_32": 8,
    "murmur3_32": 8,
    "murmur3_32_with_seed": 8,
    "fnv1a_64": 16,
    "djb2": 16,
    "polynomial_rolling": 16,
    "polynomial_rolling_with_params": 16,
    "siphash_2_4": 16,
}


def _number(text: str) -> int:
    return int(text, 0) if text.startswith("0x") else int(text)


def _check_record(record: dict) -> None:
    data = bytes.fromhex(record["data_hex"])
    params = record.get("parameters", {})
    expected = record["expected_hex"]
    actual = {
        "fnv1a_32": fixed_hex(fnv1a32(data), 32),
        "fnv1a_64": fixed_hex(fnv1a64(data), 64),
        "djb2": fixed_hex(djb2(data), 64),
        "polynomial_rolling": fixed_hex(polynomial_rolling(data), 64),
        "polynomial_rolling_with_params": fixed_hex(
            polynomial_rolling(
                data,
                _number(params.get("polynomial_rolling_with_params", {}).get("base", str(POLY_BASE))),
                _number(params.get("polynomial_rolling_with_params", {}).get("modulus", str(POLY_MODULUS))),
            ),
            64,
        ),
        "murmur3_32": fixed_hex(murmur3_32(data), 32),
        "murmur3_32_with_seed": fixed_hex(
            murmur3_32(
                data,
                _number(params.get("murmur3_32_with_seed", {}).get("seed", f"0x{MURMUR_SEED:08x}")),
            ),
            32,
        ),
        "siphash_2_4": fixed_hex(
            siphash24(
                data,
                bytes.fromhex(params.get("siphash_2_4", {}).get("key_hex", SIPHASH_KEY.hex())),
            ),
            64,
        ),
    }
    if expected != actual:
        raise AssertionError(f"{record['id']}: outputs differ from exact oracle")


def _check_single_known_answer(record: dict) -> None:
    op = record["op"]
    params = record.get("parameters", {})
    if "text" in record:
        text = record["text"]
        actual = (
            hash_string_fnv1a32(text)
            if op == "hash_str_fnv1a_32"
            else hash_string_siphash(text, bytes.fromhex(params["key_hex"]))
        )
    else:
        data = bytes.fromhex(record["data_hex"])
        actual = {
            "fnv1a_32": fnv1a32,
            "fnv1a_64": fnv1a64,
            "djb2": djb2,
            "polynomial_rolling": polynomial_rolling,
            "murmur3_32": murmur3_32,
            "siphash_2_4": lambda d: siphash24(d, bytes.fromhex(params.get("key_hex", SIPHASH_KEY.hex()))),
        }.get(op, lambda _: None)(data)
        if op == "polynomial_rolling_with_params":
            actual = polynomial_rolling(
                data,
                _number(params["base"]),
                _number(params["modulus"]),
            )
        elif op == "murmur3_32_with_seed":
            actual = murmur3_32(data, _number(params["seed"]))
    bits = 32 if op in {"fnv1a_32", "murmur3_32", "murmur3_32_with_seed", "hash_str_fnv1a_32"} else 64
    if fixed_hex(actual, bits) != record["expected_hex"]:
        raise AssertionError(f"{record['id']}: literal answer differs from exact oracle")


def validate() -> tuple[int, int]:
    fixture_path = ROOT / "cases.json"
    actual_bytes = fixture_path.read_bytes()
    fixture = json.loads(actual_bytes)
    expected_bytes = (
        json.dumps(build_fixture(), ensure_ascii=False, separators=(",", ":")) + "\n"
    ).encode("utf-8")
    if actual_bytes != expected_bytes:
        raise AssertionError("cases.json is stale; run `python generate.py` and review the change")
    if fixture.get("schema") != "dt17-hash-functions-v1" or fixture.get("oracle") != "dt17-exact-arithmetic-v1":
        raise AssertionError("schema/oracle identifier differs from DT17 v1 contract")
    domain = fixture["exhaustive"]["domain"]
    if domain != {"lengths": [0, 1, 2], "order": "length-then-lexicographic-bytes", "count": 65793}:
        raise AssertionError("exhaustive domain or input ordering differs from DT17 v1 contract")

    for record in fixture["known_answers"]:
        _check_single_known_answer(record)
    for record in fixture["utf8_cases"]:
        _check_single_known_answer(record)
    for section in ("directed",):
        for record in fixture[section]:
            _check_record(record)
    for record in fixture["generated"]["cases"]:
        _check_record(record)
    for pair in fixture["mutations"]["cases"]:
        _check_record(pair["original"])
        _check_record(pair["mutated"])

    streams = fixture["exhaustive"]["expected_hex_streams"]
    count = fixture["exhaustive"]["domain"]["count"]
    expected_functions = {
        "fnv1a_32": (fnv1a32, 8),
        "fnv1a_64": (fnv1a64, 16),
        "djb2": (djb2, 16),
        "polynomial_rolling": (polynomial_rolling, 16),
        "polynomial_rolling_with_params": (
            lambda data: polynomial_rolling(
                data,
                _number(fixture["exhaustive"]["fixed_parameters"]["polynomial_base"]),
                _number(fixture["exhaustive"]["fixed_parameters"]["polynomial_modulus"]),
            ),
            16,
        ),
        "murmur3_32": (murmur3_32, 8),
        "murmur3_32_with_seed": (lambda data: murmur3_32(data, MURMUR_SEED), 8),
        "siphash_2_4": (lambda data: siphash24(data, SIPHASH_KEY), 16),
    }
    if set(streams) != set(expected_functions):
        raise AssertionError("exhaustive op set differs from schema contract")
    for op, (function, width) in expected_functions.items():
        stream = streams[op]
        if len(stream) != count * width or any(c not in "0123456789abcdef" for c in stream):
            raise AssertionError(f"{op}: expected a lowercase hex stream of {count * width} characters")
        index = 0
        for length in range(3):
            if length == 0:
                inputs = [b""]
            elif length == 1:
                inputs = (bytes([value]) for value in range(256))
            else:
                inputs = (bytes([left, right]) for left in range(256) for right in range(256))
            for data in inputs:
                expected = fixed_hex(function(data), width * 4)
                start = index * width
                if stream[start : start + width] != expected:
                    raise AssertionError(f"{op}: mismatch at exhaustive index {index}")
                index += 1
        if index != count:
            raise AssertionError(f"{op}: consumed {index} expected outputs, declared {count}")
    dispatch_probe = bytes.fromhex("0100")
    dispatch_base = _number(fixture["exhaustive"]["fixed_parameters"]["polynomial_base"])
    dispatch_modulus = _number(fixture["exhaustive"]["fixed_parameters"]["polynomial_modulus"])
    if polynomial_rolling(dispatch_probe) == polynomial_rolling(
        dispatch_probe, dispatch_base, dispatch_modulus
    ):
        raise AssertionError("parameterized polynomial dispatch negative control is not discriminating")

    # Keep the stream contract itself under negative control: consumers must
    # reject malformed encoding, truncation, and any changed result entry.
    probe_op = "fnv1a_32"
    probe = streams[probe_op]
    probe_width = WIDTHS[probe_op]
    if len(probe) != count * probe_width:
        raise AssertionError("negative-control setup: unexpected probe length")
    _expect_stream_rejected(probe[:-1], probe, count, probe_width, probe_op, "truncated")
    _expect_stream_rejected("A" + probe[1:], probe, count, probe_width, probe_op, "uppercase")
    changed_digit = "0" if probe[0] != "0" else "1"
    _expect_stream_rejected(changed_digit + probe[1:], probe, count, probe_width, probe_op, "mutated")

    for item in fixture["analysis"]:
        callback = item["callback"]
        if callback == {"kind": "synthetic", "name": "first_byte"}:
            hash_fn = lambda data: data[0]
        elif callback == {"kind": "hash", "op": "fnv1a_32"}:
            hash_fn = fnv1a32
        else:
            raise AssertionError(f"unknown analysis callback {callback}")
        if item["kind"] == "avalanche":
            samples = [bytes.fromhex(value) for value in item["samples_hex"]]
            changed, compared = avalanche_counts(hash_fn, samples, item["output_bits"])
            actual = {"flipped_bits": str(changed), "compared_bits": str(compared)}
        else:
            inputs = [bytes.fromhex(value) for value in item["inputs_hex"]]
            buckets, numerator, denominator = distribution_counts(hash_fn, inputs, item["bucket_count"])
            actual = {
                "bucket_counts": buckets,
                "chi_squared_numerator": str(numerator),
                "chi_squared_denominator": str(denominator),
            }
        if item["expected"] != actual:
            raise AssertionError(f"{item['id']}: analysis expectation differs from exact oracle")

    _check_invalid_analysis_inputs(fixture["dart_only_invalid"])

    return count, len(fixture["known_answers"])


def _check_invalid_analysis_inputs(cases: list[dict]) -> None:
    for item in cases:
        if item["op"] not in {"avalanche", "distribution"}:
            continue
        callback_result = _number(item.get("callback_result", "0"))
        callback = lambda _data, value=callback_result: value
        try:
            if item["op"] == "avalanche":
                samples = [bytes.fromhex(value) for value in item.get("samples_hex", [])]
                avalanche_counts(callback, samples, item.get("output_bits", 32))
            else:
                inputs = [bytes.fromhex(value) for value in item.get("inputs_hex", [])]
                distribution_counts(callback, inputs, item.get("bucket_count", 1))
        except ValueError:
            continue
        raise AssertionError(f"{item['id']}: invalid analysis input was accepted by exact oracle")


def _expect_stream_rejected(
    stream: str, canonical: str, count: int, width: int, op: str, label: str
) -> None:
    try:
        if len(stream) != count * width:
            raise ValueError("length")
        if any(c not in "0123456789abcdef" for c in stream):
            raise ValueError("encoding")
        # Compare every entry to the independent answer stream. The oracle
        # fixture is immutable for this check, so one altered digit must fail.
        if stream != canonical:
            raise ValueError("answer")
    except ValueError:
        return
    raise AssertionError(f"{op}: stream negative control {label} was not detected")


if __name__ == "__main__":
    try:
        exhaustive_count, known_count = validate()
    except (AssertionError, KeyError, TypeError, ValueError) as error:
        print(f"invalid DT17 fixture: {error}", file=sys.stderr)
        raise SystemExit(1)
    print(
        f"valid DT17 v1 fixture: {exhaustive_count} inputs per each of 8 ops; "
        f"{known_count} literal known answers; independent oracle matches"
    )
