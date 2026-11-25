"""Tests for TFV line format parser and serializer."""

import pytest

from coding_adventures_test_bootstrap.tfv import (
    HEADER,
    MAX_DEPTH,
    MAX_INPUT_BYTES,
    MAX_INT_DIGITS,
    TFVRecord,
    format_record,
    format_tfv,
    format_value,
    parse_tfv,
)


def test_format_value_primitives() -> None:
    assert format_value(None) == "N"
    assert format_value(True) == "T"
    assert format_value(False) == "F"
    assert format_value(0) == "I0"
    assert format_value(42) == "I42"
    assert format_value(-10) == "I-10"
    assert format_value(b"abc") == "X616263"
    assert format_value("hello") == "S68656c6c6f"

    # Float IEEE-754 hex
    f_str = format_value(1.0)
    assert f_str.startswith("D")
    assert len(f_str) == 17  # 'D' + 16 hex chars

    # List & Map
    assert format_value([]) == "L0"
    assert format_value([1, True]) == "L2 I1 T"
    assert format_value({}) == "M0"
    assert format_value({"a": 1}) == "M1 S61 I1"


def test_format_value_errors() -> None:
    with pytest.raises(TypeError):
        format_value(object())

    with pytest.raises(ValueError, match="MAX_INT_DIGITS"):
        huge_int = int("9" * (MAX_INT_DIGITS + 10))
        format_value(huge_int)

    with pytest.raises(ValueError, match="MAX_DEPTH"):
        nested: list[object] = []
        curr = nested
        for _ in range(MAX_DEPTH + 5):
            new_list: list[object] = []
            curr.append(new_list)
            curr = new_list
        format_value(nested)


def test_format_and_parse_tfv_roundtrip() -> None:
    records = [
        TFVRecord(
            record_id="pkg/test-1",
            fields={
                "op": "equal",
                "val": 123,
                "flag": True,
                "raw": b"\x01\x02",
                "ratio": 3.1415,
                "none": None,
                "list": [1, 2, 3],
                "dict": {"k": "v"},
            },
        ),
        TFVRecord(
            record_id="pkg/test-2",
            fields={"empty_l": [], "empty_m": {}},
        ),
    ]

    tfv_text = format_tfv(records)
    parsed = parse_tfv(tfv_text)

    assert len(parsed) == 2
    assert parsed[0].record_id == "pkg/test-1"
    assert parsed[0].fields["op"] == "equal"
    assert parsed[0].fields["val"] == 123
    assert parsed[0].fields["flag"] is True
    assert parsed[0].fields["raw"] == b"\x01\x02"
    assert abs(parsed[0].fields["ratio"] - 3.1415) < 1e-6
    assert parsed[0].fields["none"] is None
    assert parsed[0].fields["list"] == [1, 2, 3]
    assert parsed[0].fields["dict"] == {"k": "v"}

    assert parsed[1].record_id == "pkg/test-2"
    assert parsed[1].fields["empty_l"] == []
    assert parsed[1].fields["empty_m"] == {}


def test_parse_tfv_errors() -> None:
    assert parse_tfv("") == []
    assert parse_tfv("   \n  ") == []

    with pytest.raises(ValueError, match="Invalid TFV header"):
        parse_tfv("tfv 2\nsomething")

    with pytest.raises(ValueError, match="Malformed TFV record"):
        parse_tfv(f"{HEADER}\nonly_one_token")

    with pytest.raises(ValueError, match="Expected R<count>"):
        parse_tfv(f"{HEADER}\nrec_id NOT_R")

    with pytest.raises(ValueError, match="Expected K<name>"):
        parse_tfv(f"{HEADER}\nrec_id R1 NOT_K I1")

    with pytest.raises(ValueError, match="Unexpected end of tokens"):
        parse_tfv(f"{HEADER}\nrec_id R1 Kfield")

    with pytest.raises(ValueError, match="Unknown TFV tag"):
        parse_tfv(f"{HEADER}\nrec_id R1 Kfield Zbadtag")

    with pytest.raises(ValueError, match="canonical integer"):
        parse_tfv(f"{HEADER}\nrec_id R1 Kfield I042")

    with pytest.raises(ValueError, match="Float must have exactly 16 hex"):
        parse_tfv(f"{HEADER}\nrec_id R1 Kfield D1234")


def test_format_record() -> None:
    rec = TFVRecord("test/rec", {"x": 5})
    assert format_record(rec) == "test/rec R1 Kx I5"


def test_format_record_rejections() -> None:
    with pytest.raises(ValueError, match="Invalid record_id"):
        format_record(TFVRecord("", {"x": 1}))
    with pytest.raises(ValueError, match="Invalid record_id"):
        format_record(TFVRecord("rec with space", {"x": 1}))
    with pytest.raises(ValueError, match="Invalid record_id"):
        format_record(TFVRecord("rec\nnewline", {"x": 1}))
    with pytest.raises(ValueError, match="Invalid field key"):
        format_record(TFVRecord("rec", {"": 1}))
    with pytest.raises(ValueError, match="Invalid field key"):
        format_record(TFVRecord("rec", {"key with space": 1}))
    with pytest.raises(ValueError, match="Invalid field key"):
        format_record(TFVRecord("rec", {"key\nnewline": 1}))


def test_parse_tfv_cwe400_and_canonical_strictness() -> None:
    # MAX_INPUT_BYTES
    huge_input = HEADER + "\n" + "x" * (MAX_INPUT_BYTES + 10)
    with pytest.raises(ValueError, match="exceeds MAX_INPUT_BYTES"):
        parse_tfv(huge_input)

    # N, T, F must not have body
    with pytest.raises(ValueError, match="Unexpected body for tag N"):
        parse_tfv(f"{HEADER}\nrec R1 Kk Nextra")
    with pytest.raises(ValueError, match="Unexpected body for tag T"):
        parse_tfv(f"{HEADER}\nrec R1 Kk Ttrue")
    with pytest.raises(ValueError, match="Unexpected body for tag F"):
        parse_tfv(f"{HEADER}\nrec R1 Kk Ffalse")

    # Canonical integers
    with pytest.raises(ValueError, match="Empty integer body"):
        parse_tfv(f"{HEADER}\nrec R1 Kk I")
    with pytest.raises(ValueError, match="Invalid canonical integer: -0"):
        parse_tfv(f"{HEADER}\nrec R1 Kk I-0")
    with pytest.raises(ValueError, match="Invalid canonical integer"):
        parse_tfv(f"{HEADER}\nrec R1 Kk I+1")
    with pytest.raises(ValueError, match="Invalid canonical integer"):
        parse_tfv(f"{HEADER}\nrec R1 Kk I1_000")
    with pytest.raises(ValueError, match="Leading zero"):
        parse_tfv(f"{HEADER}\nrec R1 Kk I-01")
    with pytest.raises(ValueError, match="Leading zero"):
        parse_tfv(f"{HEADER}\nrec R1 Kk I01")
    with pytest.raises(ValueError, match="exceeds MAX_INT_DIGITS"):
        parse_tfv(f"{HEADER}\nrec R1 Kk I" + "9" * (MAX_INT_DIGITS + 5))
    with pytest.raises(ValueError, match="exceeds MAX_INT_DIGITS"):
        parse_tfv(f"{HEADER}\nrec R1 Kk I-" + "9" * (MAX_INT_DIGITS + 5))
    with pytest.raises(ValueError):
        parse_tfv(f"{HEADER}\nrec R1 Kk I-")
    with pytest.raises(ValueError):
        parse_tfv(f"{HEADER}\nrec R1 Kk I-abc")
    with pytest.raises(ValueError):
        parse_tfv(f"{HEADER}\nrec R1 Kk Iabc")

    # Canonical L and M count
    with pytest.raises(ValueError, match="Invalid non-negative integer for L count"):
        parse_tfv(f"{HEADER}\nrec R1 Kk L-1")
    with pytest.raises(ValueError, match="Leading zero in L count"):
        parse_tfv(f"{HEADER}\nrec R1 Kk L01 I1")
    with pytest.raises(ValueError, match="Invalid non-negative integer for M count"):
        parse_tfv(f"{HEADER}\nrec R1 Kk M-1")

    # Canonical R field count
    with pytest.raises(
        ValueError, match="Invalid non-negative integer for R field count"
    ):
        parse_tfv(f"{HEADER}\nrec R-1 Kk I1")
    with pytest.raises(ValueError, match="Leading zero in R field count"):
        parse_tfv(f"{HEADER}\nrec R01 Kk I1")

    # Trailing tokens
    with pytest.raises(ValueError, match="Trailing tokens"):
        parse_tfv(f"{HEADER}\nrec R0 extra_token")
    with pytest.raises(ValueError, match="Trailing tokens"):
        parse_tfv(f"{HEADER}\nrec R1 Kk I1 extra_token")

    # Empty field key
    with pytest.raises(ValueError, match="Invalid field key"):
        parse_tfv(f"{HEADER}\nrec R1 K I1")


def test_hex_bodies_must_be_even_length_lowercase() -> None:
    """TF01: uppercase, odd-length and space-separated hex are all rejected."""
    for bad in (
        "tfv 1\nr R1 Kdata XFF\n",
        "tfv 1\nr R1 Kdata Xf\n",
        "tfv 1\nr R1 Kdata S4G\n",
        "tfv 1\nr R1 Kdata DBFF0000000000000\n",
    ):
        with pytest.raises(ValueError):
            parse_tfv(bad)
    assert parse_tfv("tfv 1\nr R1 Kdata Xff\n")[0].fields["data"] == b"\xff"
    assert parse_tfv("tfv 1\nr R1 Kdata X\n")[0].fields["data"] == b""
