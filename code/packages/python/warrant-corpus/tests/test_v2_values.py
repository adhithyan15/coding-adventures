"""Types and values of VER04r section 2: parse, decode, encode, round-trip.

The conformance kit covers the ``ir.core`` types; the composite forms (tuples,
records, variants, ``Option``, ``List``) belong to later levels but the decoder
and encoder implement every form of sections 2.3 and 2.4, so they are tested
here from the spec's own text (the worked example of section 2.4 included).
"""

from __future__ import annotations

import hashlib

import pytest

from coding_adventures_warrant_corpus.v2.errors import Refusal
from coding_adventures_warrant_corpus.v2.semantics import STANDARD
from coding_adventures_warrant_corpus.v2.values import (
    decode_value,
    encode_value,
    make_encoder,
    read_json,
    value_to_json,
)
from coding_adventures_warrant_corpus.v2.wtypes import (
    BYTES,
    INT,
    NAT,
    Ty,
    fixed_range,
    flows,
    is_undetermined,
    parse_type,
    unify,
)

TYPES = {
    "E": {"kind": "error", "cases": ["BAD", "WORSE"]},
    "Point": {"kind": "record", "fields": [["x", "U8"], ["y", "U8"]]},
    "Shape": {
        "kind": "variant",
        "ctors": [["Dot", []], ["Line", ["U8", "Bytes"]], ["Pair", ["Point", "Bool"]]],
    },
}


def code_of(call, *args, **kwargs):
    with pytest.raises(Refusal) as caught:
        call(*args, **kwargs)
    return caught.value.code


# -- section 2.1: type names ---------------------------------------------------
@pytest.mark.parametrize(
    "text",
    [
        "Bool", "U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64", "Nat", "Bytes",
        "Bytes<0>", "Bytes<4096>", "(U8,Bool)", "(U8,(Bool,Bytes<2>),Nat)",
        "List<U8>", "Option<Bytes>", "Result<U8,E>", "Result<Option<(U8,Nat)>,E>",
        "Point", "Shape", "Option<Point>",
    ],
)  # fmt: skip
def test_a_type_prints_as_it_was_written(text):
    assert str(parse_type(text, TYPES)) == text


@pytest.mark.parametrize(
    "text",
    [
        "", "u8", "U128", "Foo", "Bool<", "Bytes<", "Bytes<>", "Bytes<07>", "Bytes<4097>",
        "Bytes<N>", "(U8)", "(U8,", "(U8,Bool", "List<U8", "Result<U8>", "Result<U8,X>",
        "Result<U8,U8>", "Result<U8,Point>", "E", "U8 ", "U8,Bool", "Int", "Result<Int,E>",
        "(U8,U8,U8,U8,U8,U8,U8,U8,U8)",
    ],
)  # fmt: skip
def test_text_that_is_not_a_type_is_refused_as_type(text):
    assert code_of(parse_type, text, TYPES) == "type"


def test_a_type_is_text_and_an_int_needs_the_ir_switch():
    assert code_of(parse_type, 7, TYPES) == "type"
    assert parse_type("Int", TYPES, allow_int=True) == INT


def test_a_named_width_is_allowed_only_when_the_signature_declares_it():
    ty = parse_type("Bytes<M>", TYPES, widths=frozenset({"M"}))
    assert ty == Ty("Bytes", ("M",)) and str(ty) == "Bytes<M>"
    assert code_of(parse_type, "Bytes<M>", TYPES) == "type"


def test_the_open_mode_has_no_table_and_names_only_an_error_freely():
    assert str(parse_type("Result<U8,Anything>", None)) == "Result<U8,Anything>"
    assert code_of(parse_type, "Point", None) == "type"


def test_type_depth_counts_enclosing_constructors():
    def nest(depth):
        return "Result<" * depth + "Bool" + ",E>" * depth

    assert str(parse_type(nest(16), TYPES)) == nest(16)
    assert code_of(parse_type, nest(17), TYPES) == "type"
    assert code_of(parse_type, "(" * 17 + "U8,U8" + ",U8)" * 17, TYPES) == "type"


def test_type_properties_and_ranges():
    u8, i8, nat = parse_type("U8", {}), parse_type("I8", {}), parse_type("Nat", {})
    assert (u8.width, u8.signed, i8.signed) == (8, False, True)
    assert u8.is_fixed and u8.is_number and not nat.is_fixed and nat.is_exact
    assert fixed_range(u8) == (0, 255) and fixed_range(i8) == (-128, 127)


# -- relations -------------------------------------------------------------------
def test_flows_is_the_checkers_assignability():
    u8, nat, int_ = Ty("U8"), NAT, INT
    sized = Ty("Bytes", (3,))
    ok_nat, ok_int = Ty("Result", (nat, Ty("E"))), Ty("Result", (int_, Ty("E")))
    assert flows(nat, int_) and not flows(int_, nat) and not flows(u8, nat)
    assert flows(sized, BYTES) and not flows(BYTES, sized)
    assert flows(ok_nat, ok_int) and not flows(ok_int, ok_nat)
    assert not flows(ok_nat, Ty("Result", (nat, Ty("F"))))
    assert flows(Ty("Option", (nat,)), Ty("Option", (int_,)))
    assert not flows(Ty("Option", (int_,)), Ty("Option", (nat,)))
    hole = Ty("Result", (Ty("?"), Ty("E")))
    assert flows(hole, ok_nat) and flows(ok_nat, hole)
    assert not flows(Ty("Bool"), u8)


def test_unify_finds_the_common_type_or_nothing():
    assert unify(NAT, INT) == INT and unify(INT, NAT) == INT
    assert unify(Ty("U8"), Ty("U8")) == Ty("U8")
    assert unify(Ty("U8"), Ty("Bool")) is None and unify(Ty("U8"), NAT) is None
    assert unify(Ty("Bytes", (2,)), Ty("Bytes", (3,))) == BYTES
    ok_u8 = Ty("Result", (Ty("U8"), Ty("?")))
    err_e = Ty("Result", (Ty("?"), Ty("E")))
    assert unify(ok_u8, err_e) == Ty("Result", (Ty("U8"), Ty("E")))
    assert (
        unify(Ty("Result", (Ty("U8"), Ty("E"))), Ty("Result", (Ty("U8"), Ty("F"))))
        is None
    )
    assert (
        unify(Ty("Result", (Ty("U8"), Ty("E"))), Ty("Result", (Ty("Bool"), Ty("E"))))
        is None
    )
    assert unify(Ty("?"), Ty("U8")) == Ty("U8") and unify(Ty("U8"), Ty("?")) == Ty("U8")
    assert unify(Ty("Option", (NAT,)), Ty("Option", (INT,))) == Ty("Option", (INT,))
    assert unify(Ty("Option", (NAT,)), Ty("Option", (Ty("Bool"),))) is None
    assert unify(Ty("Option", (NAT,)), NAT) is None


def test_an_undetermined_type_holds_a_question_mark():
    assert is_undetermined(Ty("Result", (Ty("U8"), Ty("?"))))
    assert not is_undetermined(Ty("Result", (Ty("U8"), Ty("E"))))
    assert is_undetermined(Ty("?"))


# -- section 2.3: JSON -> value -> JSON ----------------------------------------------
ROUND_TRIPS = [
    ("Bool", True),
    ("U8", "255"),
    ("I8", "-128"),
    ("Nat", "18446744073709551615"),
    ("Bytes", "0a0b"),
    ("Bytes<2>", "0a0b"),
    ("Result<U8,E>", {"ok": "7"}),
    ("Result<U8,E>", {"err": "WORSE"}),
    ("(U8,Bool)", ["1", True]),
    ("Point", ["1", "2"]),
    ("Option<U8>", {"none": True}),
    ("Option<Option<U8>>", {"some": {"some": "3"}}),
    ("Option<Option<U8>>", {"some": {"none": True}}),
    ("List<U8>", []),
    ("List<U8>", ["1", "2"]),
    ("Shape", {"ctor": "Dot", "fields": []}),
    ("Shape", {"ctor": "Line", "fields": ["4", "ff"]}),
    ("Shape", {"ctor": "Pair", "fields": [["1", "2"], False]}),
]


@pytest.mark.parametrize(("text", "raw"), ROUND_TRIPS)
def test_a_value_decodes_and_writes_back_to_the_same_json(text, raw):
    ty = parse_type(text, TYPES)
    assert value_to_json(decode_value(raw, ty, TYPES), ty, TYPES) == raw


@pytest.mark.parametrize(
    ("text", "raw"),
    [
        ("Bool", "true"), ("Bool", 1), ("U8", 5), ("U8", "256"), ("U8", "-1"), ("I8", "-129"),
        ("I8", "-0"), ("U8", "05"), ("U8", "+5"), ("U8", " 5"), ("U8", ""), ("U8", "1.0"),
        ("Nat", "18446744073709551616"), ("U64", "1" * 21), ("I64", "-" + "9" * 21),
        ("Bytes", "abc"), ("Bytes", "AB"), ("Bytes", 7), ("Bytes", "ab" * 4097),
        ("Bytes<2>", "ab"), ("Result<U8,E>", {"ok": "1", "err": "BAD"}),
        ("Result<U8,E>", {}), ("Result<U8,E>", {"err": "NOPE"}), ("Result<U8,E>", {"err": 1}),
        ("Result<U8,E>", "ok"), ("(U8,Bool)", ["1"]), ("(U8,Bool)", "1"), ("Point", ["1"]),
        ("Option<U8>", {"some": "1", "none": True}), ("Option<U8>", {"none": False}),
        ("Option<U8>", "none"), ("List<U8>", ["1"] * 4097), ("List<U8>", "1"),
        ("Shape", {"ctor": "Dot"}), ("Shape", {"ctor": "Nope", "fields": []}),
        ("Shape", {"ctor": "Line", "fields": ["1"]}), ("Shape", {"ctor": "Dot", "fields": "x"}),
        ("Shape", {"ctor": 3, "fields": []}), ("Shape", []),
    ],
)  # fmt: skip
def test_a_value_that_is_not_the_declared_type_is_refused_as_value(text, raw):
    assert code_of(decode_value, raw, parse_type(text, TYPES), TYPES) == "value"


def test_a_value_of_an_undecodable_type_is_refused():
    assert code_of(decode_value, "x", Ty("?"), TYPES) == "value"


def test_an_open_table_accepts_any_error_case_name():
    assert decode_value({"err": "ANY"}, parse_type("Result<U8,E>", None), None) == (
        "err",
        "ANY",
    )


def test_int_text_follows_the_d58_window():
    """``Int`` is an IR literal type only: its text is decoded by the semantics."""
    assert STANDARD.decode_int("-99999999999999999999", INT) == -(10**20) + 1
    assert code_of(STANDARD.decode_int, "-100000000000000000000", INT) == "value"
    assert code_of(STANDARD.decode_int, "-1", NAT) == "value"
    assert code_of(STANDARD.decode_int, "1" * 21, INT) == "value"


# -- section 2.4: the stream -----------------------------------------------------------
def test_the_worked_example_of_the_spec_is_the_record_point_1_2():
    ty = parse_type("Point", TYPES)
    stream = encode_value((1, 2), ty, TYPES)
    assert stream.hex() == "6402000000100100000000000000100200000000000000"


def test_a_signed_value_is_sign_extended_to_64_bits():
    assert encode_value(-1, Ty("I8")).hex() == "11ffffffffffffffff"
    assert encode_value(-128, Ty("I8")).hex() == "1180ffffffffffffff"
    assert encode_value(127, Ty("I8")).hex() == "117f00000000000000"


@pytest.mark.parametrize(
    ("text", "raw", "hex_"),
    [
        ("Bool", False, "00"),
        ("Bool", True, "01"),
        ("U8", "7", "100700000000000000"),
        ("Bytes", "ab", "2001000000ab"),
        ("Result<Bool,E>", {"ok": True}, "5001"),
        ("Result<Bool,E>", {"err": "BAD"}, "5103000000" "424144"),
        ("Option<U8>", {"none": True}, "40"),
        ("Option<U8>", {"some": "1"}, "41" "100100000000000000"),
        ("List<U8>", ["1", "2"], "30" "02000000" "100100000000000000" "100200000000000000"),
        ("(U8,Bool)", ["1", True], "64" "02000000" "100100000000000000" "01"),
        ("Shape", {"ctor": "Dot", "fields": []}, "65" "03000000" "446f74" "00000000"),
        (
            "Shape",
            {"ctor": "Line", "fields": ["4", "ff"]},
            "65" "04000000" "4c696e65" "02000000" "100400000000000000" "2001000000ff",
        ),
    ],
)  # fmt: skip
def test_every_stream_form_of_section_2_4(text, raw, hex_):
    ty = parse_type(text, TYPES)
    assert encode_value(decode_value(raw, ty, TYPES), ty, TYPES).hex() == hex_


def test_a_value_outside_its_type_cannot_be_encoded():
    assert code_of(encode_value, 256, Ty("U8")) == "value"
    assert code_of(encode_value, -1, Ty("U8")) == "value"
    assert code_of(encode_value, 128, Ty("I8")) == "value"
    assert code_of(encode_value, True, Ty("U8")) == "value"
    assert code_of(encode_value, 2**64, Ty("Nat")) == "value"


def test_a_type_with_no_stream_form_is_refused():
    assert code_of(make_encoder, Ty("?")) == "type"


def test_an_encoder_is_built_once_and_called_per_value():
    encode = make_encoder(Ty("U8"))
    assert [encode(v).hex() for v in (0, 1)] == [
        "100000000000000000",
        "100100000000000000",
    ]


# -- the strict reader ---------------------------------------------------------------------
def test_the_strict_reader_accepts_json_and_refuses_everything_else_by_name():
    assert read_json(b' {"a": [1, "x", true]} ') == {"a": [1, "x", True]}
    for bad in (
        b"",
        b"[1,]",
        b"\xff",
        b'{"a":1,"a":2}',
        b"1.5",
        b"NaN",
        b"9007199254740992",
    ):
        assert code_of(read_json, bad) == "malformed-json"
    assert code_of(read_json, b"[" * 300 + b"]" * 300) == "malformed-json"


def test_a_stream_digest_is_the_sha256_of_its_bytes():
    stream = encode_value(True, Ty("Bool"))
    assert hashlib.sha256(stream).hexdigest() == (
        "4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a"
    )
