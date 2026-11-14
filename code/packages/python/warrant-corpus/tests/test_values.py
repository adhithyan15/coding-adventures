"""Types, value decoding, strict result checks and the stream encoding."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_corpus.errors import CorpusError
from coding_adventures_warrant_corpus.values import (
    check_value,
    decode_json,
    decode_tfv,
    encode_stream,
    parse_type,
    stream_prefix,
)

ERRORS = {"SelectError": ["CT_SELECT_LENGTH_MISMATCH"]}


def test_parse_type_accepts_the_closed_profile() -> None:
    assert parse_type("Bool").kind == "Bool"
    assert parse_type("Bytes").width is None
    assert parse_type("Bytes<4>").width == 4
    assert parse_type("Bytes<N>").width == "N"
    result = parse_type("Result<Bytes,SelectError>")
    assert result.kind == "Result" and result.error == "SelectError"
    assert result.ok is not None and result.ok.kind == "Bytes"


@pytest.mark.parametrize(
    "text",
    [
        "List<U8>",
        "Option<Bool>",
        "Float",
        "Bytes<99999>",
        "Result<Bytes>",
        "Result<Bytes,x>",
    ],
)
def test_parse_type_rejects_everything_else(text: str) -> None:
    with pytest.raises(CorpusError):
        parse_type(text)


def test_decode_json_scalars_are_tagged_objects() -> None:
    assert decode_json(True, parse_type("Bool"), ERRORS) is True
    assert (
        decode_json(
            {"type": "U64", "value": "18446744073709551615"}, parse_type("U64"), ERRORS
        )
        == (1 << 64) - 1
    )
    assert (
        decode_json({"type": "Bytes", "value": "00ff"}, parse_type("Bytes"), ERRORS)
        == b"\x00\xff"
    )
    assert decode_json({"type": "Nat", "value": "3"}, parse_type("Nat"), ERRORS) == 3


@pytest.mark.parametrize(
    ("raw", "type_text"),
    [
        (1, "Bool"),  # a bare number is not a Bool
        (5, "U64"),  # bare JSON numbers are refused: they are floats elsewhere
        ({"type": "U64", "value": "18446744073709551616"}, "U64"),  # 2**64
        ({"type": "U64", "value": "007"}, "U64"),  # not canonical
        ({"type": "U8", "value": "256"}, "U8"),
        ({"type": "Bytes", "value": "0G"}, "Bytes"),
        ({"type": "Bytes", "value": "0A"}, "Bytes"),  # uppercase hex
        ({"type": "Bytes<2>", "value": "00"}, "Bytes<2>"),  # wrong width
        ({"type": "Bytes", "value": "00"}, "Bytes<N>"),  # tag disagrees with type
    ],
)
def test_decode_json_rejects_malformed_values(raw: object, type_text: str) -> None:
    with pytest.raises(CorpusError):
        decode_json(raw, parse_type(type_text), ERRORS)


def test_decode_json_results_only_carry_declared_variants() -> None:
    ty = parse_type("Result<Bytes,SelectError>")
    ok = {"type": ty.text, "value": {"ok": {"type": "Bytes", "value": "00"}}}
    err = {"type": ty.text, "value": {"err": "CT_SELECT_LENGTH_MISMATCH"}}
    assert decode_json(ok, ty, ERRORS) == ("ok", b"\x00")
    assert decode_json(err, ty, ERRORS) == ("err", "CT_SELECT_LENGTH_MISMATCH")
    for bad in (
        {"type": ty.text, "value": {"err": "SOMETHING_ELSE"}},
        {"type": "Result<Bytes,Other>", "value": {"err": "X"}},
        {"type": ty.text, "value": {"ok": 1, "err": "X"}},
        {"type": ty.text, "value": 3},
    ):
        with pytest.raises(CorpusError):
            decode_json(bad, ty, ERRORS)


def test_check_value_is_exact_about_python_types() -> None:
    boolean, byte_string, u64 = (
        parse_type("Bool"),
        parse_type("Bytes"),
        parse_type("U64"),
    )
    assert check_value(True, boolean, ERRORS)
    assert not check_value(1, boolean, ERRORS)  # True == 1 must not pass
    assert check_value(b"a", byte_string, ERRORS)
    assert not check_value(bytearray(b"a"), byte_string, ERRORS)  # bytearray == bytes
    assert check_value(5, u64, ERRORS)
    assert not check_value(True, u64, ERRORS)  # bool is an int subclass
    assert not check_value(-1, u64, ERRORS)
    assert not check_value(1 << 64, u64, ERRORS)
    result = parse_type("Result<Bytes,SelectError>")
    assert check_value(("ok", b""), result, ERRORS)
    assert check_value(("err", "CT_SELECT_LENGTH_MISMATCH"), result, ERRORS)
    assert not check_value(("err", "NOPE"), result, ERRORS)
    assert not check_value(("ok", bytearray()), result, ERRORS)
    assert not check_value(b"bare", result, ERRORS)
    assert not check_value(("ok", b"", b""), result, ERRORS)


def test_stream_encoding_matches_the_specified_bytes() -> None:
    assert encode_stream(False, parse_type("Bool")) == b"\x00"
    assert encode_stream(True, parse_type("Bool")) == b"\x01"
    assert encode_stream(258, parse_type("U64")) == b"\x10" + bytes(
        [2, 1, 0, 0, 0, 0, 0, 0]
    )
    assert encode_stream(b"\xab", parse_type("Bytes")) == b"\x20\x01\x00\x00\x00\xab"
    result = parse_type("Result<Bytes,SelectError>")
    assert encode_stream(("ok", b""), result) == b"\x50\x20\x00\x00\x00\x00"
    assert encode_stream(("err", "AB"), result) == b"\x51\x02\x00\x00\x00AB"


def test_stream_prefix_layout() -> None:
    prefix = stream_prefix("s", ["Bytes", "Bool"], "Bool")
    expected = (
        b"warrant-stream-v1\x00"
        + b"\x01\x00\x00\x00s"
        + b"\x02\x00\x00\x00"
        + b"\x05\x00\x00\x00Bytes"
        + b"\x04\x00\x00\x00Bool"
        + b"\x04\x00\x00\x00Bool"
    )
    assert prefix == expected


def test_decode_tfv_follows_section_2_7() -> None:
    assert decode_tfv(("bool", True), parse_type("Bool"), ERRORS) is True
    assert decode_tfv(("int", 7), parse_type("U8"), ERRORS) == 7
    assert decode_tfv(("bytes", b"ab"), parse_type("Bytes<2>"), ERRORS) == b"ab"
    result = parse_type("Result<Bytes,SelectError>")
    ok = ("map", [(("text", "ok"), ("bytes", b"x"))])
    err = ("map", [(("text", "err"), ("text", "CT_SELECT_LENGTH_MISMATCH"))])
    assert decode_tfv(ok, result, ERRORS) == ("ok", b"x")
    assert decode_tfv(err, result, ERRORS) == ("err", "CT_SELECT_LENGTH_MISMATCH")
    bad_cases = [
        (("int", 1), "Bool"),
        (("bytes", b"a"), "Bytes<2>"),
        (("bool", True), "Bytes"),
        (("int", 256), "U8"),
        (("int", -1), "U64"),
        (("bool", True), "Result<Bytes,SelectError>"),
    ]
    for raw, text in bad_cases:
        with pytest.raises(CorpusError):
            decode_tfv(raw, parse_type(text), ERRORS)
    for bad in (
        ("map", [(("int", 1), ("bytes", b""))]),
        ("map", [(("text", "err"), ("text", "UNDECLARED"))]),
        ("map", [(("text", "nope"), ("bytes", b""))]),
    ):
        with pytest.raises(CorpusError):
            decode_tfv(bad, result, ERRORS)


def test_nested_angle_brackets_in_a_result_type() -> None:
    parsed = parse_type("Result<Bytes<2>,SelectError>")
    assert parsed.ok is not None and parsed.ok.width == 2
