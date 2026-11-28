"""Independent VER05/VER06 assurance for the HTTP/1.0 request-line slice."""

from __future__ import annotations

import json
from dataclasses import FrozenInstanceError
from itertools import product
from pathlib import Path
from typing import Any

import pytest
from coding_adventures_test_bootstrap import (
    FuzzOptions,
    PbtOptions,
    check_property,
    fuzz_function,
    gen_bytes,
    gen_choice,
    gen_list,
)

from historical_web import (
    Http10RequestLine,
    encode_http10_request_line,
    parse_http09_request,
    parse_http10_request_line,
)

_ROOT = Path(__file__).resolve().parents[4]
_HTTP10_FIXTURE_PATH = (
    _ROOT / "specs/fixtures/historical-web-http10-request-line-v1/vectors.json"
)
_HTTP09_FIXTURE_PATH = _ROOT / "specs/fixtures/historical-web-v1/vectors.json"
_ALLOWED_METHODS = frozenset({b"GET", b"HEAD", b"POST"})
_PATH_TAIL_BYTES = frozenset(
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789/._-"
)


def _load_object(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    assert type(value) is dict, f"fixture root must be an object: {path}"
    return value


HTTP10 = _load_object(_HTTP10_FIXTURE_PATH)
HTTP09 = _load_object(_HTTP09_FIXTURE_PATH)


def _check_fixture_schema() -> None:
    assert set(HTTP10) == {
        "schema_version",
        "profile",
        "accepted",
        "invalid_wire",
        "invalid_methods",
        "invalid_targets",
        "version_comparison",
    }
    assert type(HTTP10["schema_version"]) is int and HTTP10["schema_version"] == 1
    assert type(HTTP10["profile"]) is str
    assert HTTP10["profile"] == "http-1.0-request-line-1996"
    for category in ("accepted", "invalid_wire", "invalid_methods", "invalid_targets"):
        assert type(HTTP10[category]) is list and HTTP10[category]
    for case in HTTP10["accepted"]:
        assert set(case) == {"id", "method", "target", "wire_hex"}
        assert all(
            type(case[key]) is str for key in ("id", "method", "target", "wire_hex")
        )
        bytes.fromhex(case["wire_hex"])
    for case in HTTP10["invalid_wire"]:
        assert set(case) == {"id", "wire_hex"}
        assert all(type(case[key]) is str for key in ("id", "wire_hex"))
        bytes.fromhex(case["wire_hex"])
    assert all(type(value) is str for value in HTTP10["invalid_methods"])
    assert all(type(value) is str for value in HTTP10["invalid_targets"])
    comparison = HTTP10["version_comparison"]
    assert set(comparison) == {
        "legacy_fixture",
        "http10_rejects_legacy_accepted_requests",
        "http09_rejects_http10_accepted_requests",
    }
    assert comparison == {
        "legacy_fixture": "historical-web-v1/vectors.json",
        "http10_rejects_legacy_accepted_requests": True,
        "http09_rejects_http10_accepted_requests": True,
    }
    assert all(
        type(comparison[key]) is bool
        for key in (
            "http10_rejects_legacy_accepted_requests",
            "http09_rejects_http10_accepted_requests",
        )
    )
    assert set(HTTP09) == {"http-0.9", "html-tags-1992", "exchanges"}
    assert type(HTTP09["http-0.9"]["requests"]) is list
    assert HTTP09["http-0.9"]["requests"]


def _model_parse(wire: bytes) -> tuple[str, str] | None:
    """Small independent grammar oracle; does not use production helpers."""
    if type(wire) is not bytes or not wire.endswith(b"\r\n"):
        return None
    line = wire[:-2]
    fields = line.split(b" ")
    if len(fields) != 3 or fields[0] not in _ALLOWED_METHODS:
        return None
    method, target, version = fields
    if version != b"HTTP/1.0" or not target.startswith(b"/"):
        return None
    if any(byte not in _PATH_TAIL_BYTES for byte in target[1:]):
        return None
    return method.decode("ascii"), target.decode("ascii")


def _assert_parser_matches_oracle(wire: bytes) -> None:
    expected = _model_parse(wire)
    try:
        actual = parse_http10_request_line(wire)
    except ValueError:
        assert expected is None, (wire, expected)
    else:
        assert expected is not None, wire
        assert type(actual) is Http10RequestLine
        assert (actual.method, actual.target) == expected


def test_shared_fixture_replay_and_fail_closed_shape() -> None:
    _check_fixture_schema()
    for case in HTTP10["accepted"]:
        wire = bytes.fromhex(case["wire_hex"])
        expected = Http10RequestLine(case["method"], case["target"])
        assert parse_http10_request_line(wire) == expected, case["id"]
        assert encode_http10_request_line(case["method"], case["target"]) == wire
        assert _model_parse(wire) == (case["method"], case["target"])

    for case in HTTP10["invalid_wire"]:
        wire = bytes.fromhex(case["wire_hex"])
        assert _model_parse(wire) is None, case["id"]
        with pytest.raises(ValueError):
            parse_http10_request_line(wire)

    for method in HTTP10["invalid_methods"]:
        with pytest.raises(ValueError):
            encode_http10_request_line(method, "/ok")
    for target in HTTP10["invalid_targets"]:
        with pytest.raises(ValueError):
            encode_http10_request_line("GET", target)


def test_shared_h00_request_lines_are_a_distinct_versioned_profile() -> None:
    _check_fixture_schema()
    for case in HTTP09["http-0.9"]["requests"]:
        wire = bytes.fromhex(case["wire_hex"])
        assert parse_http09_request(wire) == case["address"]
        with pytest.raises(ValueError):
            parse_http10_request_line(wire)

    for case in HTTP10["accepted"]:
        wire = bytes.fromhex(case["wire_hex"])
        with pytest.raises(ValueError):
            parse_http09_request(wire)


def test_result_record_is_frozen_and_exact_output_is_preserved() -> None:
    record = parse_http10_request_line(b"GET /a/../b-1_2 HTTP/1.0\r\n")
    assert type(record) is Http10RequestLine
    assert record.method == "GET"
    assert record.target == "/a/../b-1_2"
    with pytest.raises(FrozenInstanceError):
        record.method = "POST"  # type: ignore[misc]


class _BytesSubclass(bytes):
    pass


class _StrSubclass(str):
    pass


@pytest.mark.parametrize(
    "wire",
    [
        bytearray(b"GET / HTTP/1.0\r\n"),
        memoryview(b"GET / HTTP/1.0\r\n"),
        "GET / HTTP/1.0\r\n",
        _BytesSubclass(b"GET / HTTP/1.0\r\n"),
    ],
)
def test_parser_requires_exact_builtin_bytes(wire: object) -> None:
    with pytest.raises(TypeError):
        parse_http10_request_line(wire)  # type: ignore[arg-type]


@pytest.mark.parametrize(
    ("method", "target"),
    [
        (b"GET", "/"),
        ("GET", b"/"),
        (_StrSubclass("GET"), "/"),
        ("GET", _StrSubclass("/")),
        (None, "/"),
        ("GET", None),
    ],
)
def test_encoder_requires_exact_builtin_strings(method: object, target: object) -> None:
    with pytest.raises(TypeError):
        encode_http10_request_line(method, target)  # type: ignore[arg-type]


def test_exhaustive_two_octet_path_suffix_domain() -> None:
    checked = 0
    valid = 0
    for first, second in product(range(256), repeat=2):
        wire = b"GET /" + bytes((first, second)) + b" HTTP/1.0\r\n"
        _assert_parser_matches_oracle(wire)
        checked += 1
        if first in _PATH_TAIL_BYTES and second in _PATH_TAIL_BYTES:
            valid += 1
    assert checked == 256**2
    assert valid == len(_PATH_TAIL_BYTES) ** 2


def test_exhaustive_single_octet_method_separator_version_and_framing() -> None:
    checked = 0
    base = b"GET / HTTP/1.0\r\n"
    for byte in range(256):
        _assert_parser_matches_oracle(b"GET " + bytes((byte,)) + b" HTTP/1.0\r\n")
        checked += 1
    for index in range(3):
        for byte in range(256):
            method = bytearray(b"GET")
            method[index] = byte
            _assert_parser_matches_oracle(bytes(method) + b" / HTTP/1.0\r\n")
            checked += 1
    for byte in range(256):
        _assert_parser_matches_oracle(b"GET" + bytes((byte,)) + b"/ HTTP/1.0\r\n")
        _assert_parser_matches_oracle(b"GET /" + bytes((byte,)) + b"HTTP/1.0\r\n")
        checked += 2
    version_offset = base.index(b"HTTP/1.0")
    for index in range(len(b"HTTP/1.0")):
        for byte in range(256):
            wire = bytearray(base)
            wire[version_offset + index] = byte
            _assert_parser_matches_oracle(bytes(wire))
            checked += 1
    for index in (-2, -1):
        for byte in range(256):
            wire = bytearray(base)
            wire[index] = byte
            _assert_parser_matches_oracle(bytes(wire))
            checked += 1
    assert checked == 256 + 3 * 256 + 2 * 256 + len(b"HTTP/1.0") * 256 + 2 * 256


def test_pbt_generated_wire_and_structured_round_trips() -> None:
    wire_verdict = check_property(
        0x485745423038,
        gen_bytes(0, 64),
        _assert_parser_matches_oracle,
        PbtOptions(num_tests=256, ci=False),
    )
    assert wire_verdict.passed, wire_verdict.message
    assert wire_verdict.cases_run == 256

    suffixes = gen_list(
        gen_choice(
            list("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789/._-")
        ),
        0,
        24,
    ).map("".join)
    targets = suffixes.map(lambda suffix: "/" + suffix)
    methods = gen_choice(["GET", "HEAD", "POST"])
    requests = methods.flat_map(
        lambda method: targets.map(lambda target: (method, target))
    )
    structured_verdict = check_property(
        0x19960501,
        requests,
        lambda pair: (
            parse_http10_request_line(encode_http10_request_line(*pair))
            == Http10RequestLine(*pair)
        ),
        PbtOptions(num_tests=256, ci=False),
    )
    assert structured_verdict.passed, structured_verdict.message
    assert structured_verdict.cases_run == 256


def _fuzz_request_line(data: bytes) -> None:
    _assert_parser_matches_oracle(data)
    expected = _model_parse(data)
    if expected is not None:
        assert encode_http10_request_line(*expected) == data
        with pytest.raises(ValueError):
            parse_http09_request(data)


def test_ver06_fuzz_mixed_wire_seeds_is_crash_clean() -> None:
    _check_fixture_schema()
    seeds = [bytes.fromhex(case["wire_hex"]) for case in HTTP10["accepted"]]
    seeds.extend(bytes.fromhex(case["wire_hex"]) for case in HTTP10["invalid_wire"])
    seeds.extend(
        bytes.fromhex(case["wire_hex"]) for case in HTTP09["http-0.9"]["requests"]
    )
    report = fuzz_function(
        _fuzz_request_line,
        seeds,
        FuzzOptions(max_iterations=2048, seed=0x19960501, max_input_len=128),
    )
    assert report.iterations == 2048
    assert report.is_clean
    assert not report.crashes and not report.timeouts and not report.ooms
