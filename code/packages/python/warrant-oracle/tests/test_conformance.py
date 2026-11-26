"""Unit tests of ``conformance.py``: the derivation functions of the v2 conformance kit (slice S5a).

The tables themselves are tested in ``test_conformance_tables.py`` and the mutants in
``test_conformance_mutants.py``; here every function and every branch of the derivation module is
exercised on small, hand-built inputs.
"""

from __future__ import annotations

import copy
import hashlib

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle import ir_eval as E
from coding_adventures_warrant_oracle.errors import OracleError

ERR = {"E": {"kind": "error", "cases": ["BAD", "WORSE"]}}
REC = {
    "Point": {"kind": "record", "fields": [["x", "U8"], ["y", "Bool"]]},
    "Shape": {"kind": "variant", "ctors": [["Dot", []], ["Pair", ["U8", "Bool"]]]},
}


def hexj(text: str | bytes) -> str:
    return (text if isinstance(text, bytes) else text.encode()).hex()


def value_case(text, ty="Bool", types=None):
    return {"type": ty, "types": types if types is not None else {}, "input_hex": hexj(text)}


# ---------------------------------------------------------------------------------------------------
# the strict reader
# ---------------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "data, want",
    [
        (b"true", True),
        (b" \n\t\r false ", False),
        (b"null", None),
        (b"-12", -12),
        (b"0", 0),
        (b'"a\\n\\u00e9\\ud83d\\ude00"', "a\né\U0001f600"),
        (b'"\\/\\b\\f\\r\\t\\"\\\\"', '/\b\f\r\t"\\'),
        (b"[]", []),
        (b"{}", {}),
        (b'{"a": [1, {"b": null}]}', {"a": [1, {"b": None}]}),
        (str(2**53 - 1).encode(), 2**53 - 1),
    ],
)
def test_reader_accepts(data, want):
    assert C.read_json(data) == want


@pytest.mark.parametrize(
    "data",
    [
        b"",
        b"   ",
        b"\xff",
        b"\xef\xbb\xbftrue",
        b"\xc0\xaf",
        b"\xed\xa0\x80",
        b"tru",
        b"nul",
        b"01",
        b"-",
        b"-a",
        b"1.0",
        b"1e3",
        b"1E3",
        b"-0",
        b"+1",
        b"\xd9\xa3",  # a non-ASCII digit
        str(2**53).encode(),
        b"12345678901234567",
        b'"abc',
        b'"a\nb"',
        b'"\\q"',
        b'"\\',
        b'"\\u12"',
        b'"\\u12zz"',
        b'"\\ud800"',
        b'"\\ud800x"',
        b'"\\ud800\\u0041"',
        b'"\\udc00"',
        b"[1,]",
        b"[1 2]",
        b"[1",
        b"{",
        b'{"a"}',
        b'{"a":1,}',
        b"{a:1}",
        b'{"a":1 "b":2}',
        b'{"a":1,"a":2}',
        b'{"a":1,"\\u0061":2}',
        b"true false",
        b"NaN",
        b"'x'",
    ],
)
def test_reader_rejects(data):
    with pytest.raises(C.JsonReject):
        C.read_json(data)


def test_reader_depth_limit_is_256_containers():
    assert C.read_json(b"[" * 256 + b"]" * 256)
    with pytest.raises(C.JsonReject):
        C.read_json(b"[" * 257 + b"]" * 257)
    with pytest.raises(C.JsonReject):
        C.read_json(b'{"a":' * 257 + b"1" + b"}" * 257)


# ---------------------------------------------------------------------------------------------------
# type text
# ---------------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "text, head",
    [
        ("Bool", "Bool"),
        ("U8", "U8"),
        ("I64", "I64"),
        ("Nat", "Nat"),
        ("Bytes", "Bytes"),
        ("Bytes<0>", "Bytes"),
        ("Bytes<4096>", "Bytes"),
        ("(U8,Bool)", "()"),
        ("List<U8>", "List"),
        ("Option<Bytes<3>>", "Option"),
        ("Result<Bool,E>", "Result"),
        ("Result<Result<Bool,E>,E>", "Result"),
        ("Point", "Point"),
    ],
)
def test_parse_type_accepts(text, head):
    ty = C.parse_type(text, {**ERR, **REC})
    assert ty.head == head
    assert str(ty) == text


@pytest.mark.parametrize(
    "text",
    [
        "",
        "Int",
        "U128",
        "u8",
        "Foo",
        "Bool ",
        "Result<Bool, E>",
        "Result<Bool,E",
        "Result<Bool,X>",
        "Result<Bool,U8>",
        "Bytes<N>",
        "Bytes<07>",
        "Bytes<4097>",
        "Bytes<>",
        "Bytes<3",
        "(U8)",
        "(U8,U8,U8,U8,U8,U8,U8,U8,U8)",
        "(U8,U8",
        "List<>",
        "List<U8",
        "Result<Bool>",
        "Option<Int>",
        "Result<" * 17 + "Bool" + ",E>" * 17,
    ],
)
def test_parse_type_refuses(text):
    with pytest.raises(C.Refused) as exc:
        C.parse_type(text, ERR)
    assert exc.value.code == "type"


def test_parse_type_depth_boundary_and_non_text():
    # a bare name is depth 0: 16 wrappers are accepted, 17 refused
    assert C.parse_type("Bool", {}).head == "Bool"
    assert C.parse_type("Result<" * 16 + "Bool" + ",E>" * 16, ERR)
    with pytest.raises(C.Refused):
        C.parse_type("Result<" * 17 + "Bool" + ",E>" * 17, ERR)
    assert C.parse_type("(" * 16 + "Bool,Bool" + ",Bool)" * 16, {})
    deep_tuple = "(" * 17 + "Bool,Bool" + ",Bool)" * 17
    with pytest.raises(C.Refused):
        C.parse_type(deep_tuple, {})
    with pytest.raises(C.Refused):
        C.parse_type(7, {})


def test_parse_type_int_only_when_allowed_and_open_mode_for_error_names():
    assert C.parse_type("Int", {}, allow_int=True) == A.Ty("Int")
    assert C.parse_type("Result<Bool,Anything>", None).args[1] == A.Ty("Anything")
    with pytest.raises(C.Refused):
        C.parse_type("Foo", None)


def test_type_levels():
    assert C.type_levels(C.parse_type("Result<Bool,E>", ERR), ERR) == set()
    assert C.type_levels(C.parse_type("(U8,Bool)", {})) == {"ir.tuple"}
    # the document-level oracle refuses a list whole (S11a); the kit names its level, the reserved ir.list (R69 b, D63)
    assert C.type_levels(C.parse_type("List<U8>", {})) == {"ir.list"}
    assert C.type_levels(C.parse_type("Option<U8>", {})) == {"ir.data"}
    assert C.type_levels(C.parse_type("Point", REC), REC) == {"ir.data"}


# ---------------------------------------------------------------------------------------------------
# values: decode, encode, write back
# ---------------------------------------------------------------------------------------------------


def decode(raw, text, types=None):
    types = {**ERR, **REC} if types is None else types
    return C.decode_value(raw, C.parse_type(text, types), types)


def test_decode_scalars():
    assert decode(True, "Bool") is True
    assert decode("-128", "I8") == -128
    with pytest.raises(C.Refused):  # an error type is not a value type
        decode("BAD", "E")
    assert decode("255", "U8") == 255
    assert decode("18446744073709551615", "Nat") == 2**64 - 1
    assert decode("0aff", "Bytes") == b"\x0a\xff"
    assert decode("0aff", "Bytes<2>") == b"\x0a\xff"
    assert decode("", "Bytes<0>") == b""


@pytest.mark.parametrize(
    "raw, ty",
    [
        (1, "Bool"),
        ("true", "Bool"),
        ("256", "U8"),
        (7, "U8"),
        ("-1", "U8"),
        ("01", "U8"),
        ("2" * 21, "U64"),
        ("0A", "Bytes"),
        ("0", "Bytes"),
        ([1], "Bytes"),
        ("00" * 4097, "Bytes"),
        ("00", "Bytes<2>"),
        ({"ok": True, "err": "BAD"}, "Result<Bool,E>"),
        ({"err": "NOPE"}, "Result<Bool,E>"),
        ({"err": 1}, "Result<Bool,E>"),
        ({"x": 1}, "Result<Bool,E>"),
        (True, "Result<Bool,E>"),
        ({"ok": 1}, "Result<Bool,E>"),
        ([True], "(U8,Bool)"),
        (["1", True, True], "(U8,Bool)"),
        ("x", "(U8,Bool)"),
        ("x", "List<U8>"),
        (["1"] * 4097, "List<U8>"),
        ({"none": False}, "Option<U8>"),
        ({"some": "1", "none": True}, "Option<U8>"),
        ({"other": 1}, "Option<U8>"),
        (["1", True, 5], "Point"),
        ({"ctor": "Dot"}, "Shape"),
        ({"ctor": "Nope", "fields": []}, "Shape"),
        ({"ctor": "Pair", "fields": ["1"]}, "Shape"),
        ({"ctor": "Dot", "fields": "x"}, "Shape"),
    ],
)
def test_decode_refuses(raw, ty):
    with pytest.raises(C.Refused) as exc:
        decode(raw, ty)
    assert exc.value.code == "value"


def test_decode_every_composite_form_and_its_stream():
    types = {**ERR, **REC}
    cases = [
        ({"ok": "7"}, "Result<U8,E>", ("ok", 7), "50" + "10" + (7).to_bytes(8, "little").hex()),
        ({"err": "BAD"}, "Result<U8,E>", ("err", "BAD"), "51" + "03000000" + b"BAD".hex()),
        (["1", True], "(U8,Bool)", (1, True), "64" + "02000000" + "10" + (1).to_bytes(8, "little").hex() + "01"),
        ({"none": True}, "Option<U8>", ("none",), "40"),
        ({"some": "2"}, "Option<U8>", ("some", 2), "41" + "10" + (2).to_bytes(8, "little").hex()),
        (["1", "2"], "List<U8>", ("list", (1, 2)),
         "30" + "02000000" + "10" + (1).to_bytes(8, "little").hex() + "10" + (2).to_bytes(8, "little").hex()),
        (["3", False], "Point", (3, False), "64" + "02000000" + "10" + (3).to_bytes(8, "little").hex() + "00"),
        ({"ctor": "Dot", "fields": []}, "Shape", ("Dot", ()), "65" + "03000000" + b"Dot".hex() + "00000000"),
        ({"ctor": "Pair", "fields": ["4", True]}, "Shape", ("Pair", (4, True)),
         "65" + "04000000" + b"Pair".hex() + "02000000" + "10" + (4).to_bytes(8, "little").hex() + "01"),
    ]
    for raw, ty_text, value, stream in cases:
        ty = C.parse_type(ty_text, types)
        assert C.decode_value(raw, ty, types) == value
        assert C.encode_value(value, ty, types).hex() == stream


def test_decode_result_without_a_types_table_accepts_any_case_name():
    ty = C.parse_type("Result<Bool,E>", None)
    assert C.decode_value({"err": "ANY"}, ty, {}) == ("err", "ANY")


def test_value_to_json_round_trip_and_refusals():
    ty = C.parse_type("Result<Bytes,E>", ERR)
    assert C.value_to_json(("ok", b"\x01"), ty) == {"ok": "01"}
    assert C.value_to_json(("err", "BAD"), ty) == {"err": "BAD"}
    assert C.value_to_json((1, True), A.Ty("()", (A.Ty("U8"), A.Ty("Bool")))) == ["1", True]
    assert C.value_to_json(-3, A.Ty("Int")) == "-3"
    assert C.value_to_json(0, A.Ty("Bool")) is False
    with pytest.raises(C.KitError):
        C.value_to_json(("some", 1), A.Ty("Option", (A.Ty("U8"),)))


# ---------------------------------------------------------------------------------------------------
# v2-value and v2-arith
# ---------------------------------------------------------------------------------------------------


def test_derive_value_accept_and_digest_only():
    out = C.derive_value(value_case('"ff"', "Bytes<1>"))
    assert out["outcome"] == "accept" and out["stream_hex"] == "2001000000ff"
    assert out["stream_sha256"] == hashlib.sha256(bytes.fromhex(out["stream_hex"])).hexdigest()
    big = C.derive_value(value_case('"' + "ab" * 4096 + '"', "Bytes"))
    assert big["outcome"] == "accept" and "stream_hex" not in big


@pytest.mark.parametrize(
    "case, code",
    [
        (value_case("[true,]"), "malformed-json"),
        (value_case(b"\xff"), "malformed-json"),
        (value_case("true", "Int"), "type"),
        (value_case('"1"', "U8 "), "type"),
        (value_case("true", "Result<Bool,E>", {}), "type"),
        (value_case('"300"', "U8"), "value"),
        (value_case("[", "Int"), "malformed-json"),
    ],
)
def test_derive_value_rejects_with_the_first_matching_code(case, code):
    assert C.derive_value(case) == {"outcome": "reject", "error": code}


def test_derive_arith_value_bool_and_overflow():
    assert C.derive_arith({"type": "U8", "op": "add", "lhs": "255", "rhs": "1"}) == {"type": "U8", "value": "0"}
    assert C.derive_arith({"type": "Nat", "op": "sub", "lhs": "3", "rhs": "5"}) == {"type": "Int", "value": "-2"}
    assert C.derive_arith({"type": "I8", "op": "neg", "lhs": "-128"}) == {"type": "I8", "value": "-128"}
    assert C.derive_arith({"type": "U8", "op": "lt", "lhs": "1", "rhs": "2"}) == {"type": "Bool", "value": True}
    big = str(2**64 - 1)
    assert C.derive_arith({"type": "Nat", "op": "mul", "lhs": big, "rhs": big}) == {"error": "ir-overflow"}


def test_derive_arith_unexpected_oracle_failure_is_a_table_error(monkeypatch):
    real = E.Program.compile

    def failing(self, ir, tenv, allow_subject=True):
        c = real(self, ir, tenv, allow_subject)

        def fn(env):
            raise OracleError("ir", "an evaluation failure")

        return E.Compiled(c.ty, fn, c.cost, c.subject)

    monkeypatch.setattr(E.Program, "compile", failing)
    with pytest.raises(C.KitError):
        C.derive_arith({"type": "U8", "op": "add", "lhs": "1", "rhs": "1"})


def test_derive_arith_refuses_what_is_not_a_kit_case():
    with pytest.raises(C.KitError):
        C.derive_arith({"type": "U8", "op": "div", "lhs": "1", "rhs": "1"})
    with pytest.raises(C.KitError):
        C.derive_arith({"type": "U8", "op": "add", "lhs": "256", "rhs": "1"})  # the literal does not fit
    with pytest.raises(C.KitError):
        C.derive_arith({"type": "Bool", "op": "add", "lhs": "1", "rhs": "1"})


# ---------------------------------------------------------------------------------------------------
# v2-ir
# ---------------------------------------------------------------------------------------------------

U8 = ["lit", {"type": "U8", "value": "1"}]


def ir_case(ir, **kw):
    case = {
        "levels": ["ir.core"],
        "ir": ir,
        "env": [],
        "types": {},
        "models": {},
        "model_order": [],
        "ops": {},
        "subjects": {},
    }
    case.update(kw)
    if "models" in kw and "model_order" not in kw:
        case["model_order"] = list(kw["models"])
    return case


def test_static_refusal_each_structural_code():
    assert C.static_refusal(ir_case(["wibble"])).code == "unknown-tag"
    assert C.static_refusal(ir_case(["bit_not", U8])).code == "level-missing"
    assert C.static_refusal(ir_case(["match", ["var", "x"], [[["wild"], U8]]])).code == "level-missing"
    assert C.static_refusal(ir_case(["match", ["var", "x"], [[["wild"], U8]]], levels=["ir.core", "ir.tuple"])) is None
    assert C.static_refusal(ir_case(["model_call", "ghost", []])).code == "model-order"
    two = {"a": {"params": [], "body": ["model_call", "b", []]}, "b": {"params": [], "body": U8}}
    assert C.static_refusal(ir_case(["model_call", "a", []], models=two, model_order=["a", "b"])).code == "model-order"
    assert C.static_refusal(ir_case(["model_call", "a", []], models=two, model_order=["b", "a"])) is None
    subj = {"m": {"params": [], "body": ["subject_op", "o", []]}}
    assert C.static_refusal(ir_case(U8, models=subj)).code == "subject-forbidden"
    assert C.static_refusal(ir_case(["subject_op", "o", []], role="fault")).code == "subject-forbidden"
    assert C.static_refusal(ir_case(["subject_op", "o", []], role="body")) is None


def test_rewrite_subjects_handles_tuples_and_non_nodes():
    assert C._rewrite_subjects("x", {}) == "x"
    assert C._rewrite_subjects(["tuple", [["subject_op", "o", []], ["lit", True]]], {"o": "m"}) == [
        "tuple",
        [["model_call", "m", []], ["lit", True]],
    ]
    with pytest.raises(C.KitError):
        C._rewrite_subjects(["subject_op", "o", []], {})


def test_a_stand_in_for_an_op_the_case_never_declares_is_ignored():
    case = ir_case(["lit", True], subjects={"ghost": "nowhere"})
    assert C.derive_ir(case) == {"type": "Bool", "value": True, "calls": 0}


def test_nodes_survives_a_malformed_node():
    assert list(C._nodes("not a node")) == []
    assert [n[0] for n in C._nodes(["subject_op"])] == ["subject_op"]
    assert [n[0] for n in C._nodes(["match", 1])] == ["match"]


def test_classify_ir_refusal_maps_every_oracle_code():
    cl = C.classify_ir_refusal
    assert cl(OracleError("generation", "W0701: a subject call under an impure guard"), set()).code == "impure-guard"
    assert cl(OracleError("limit", "ir-cost: expression exceeds MAX_IR_COST"), set()).code == "ir-cost"
    assert cl(OracleError("limit", "max_ir_cost exceeds the 65,536 ceiling"), set()).code == "ir-cost"
    assert cl(OracleError("limit", "IR nesting exceeds 64 levels"), set()).code == "depth"
    assert cl(OracleError("limit", "IR nests too deeply"), set()).code == "depth"
    assert cl(OracleError("level", "x", level="ir.data"), {"ir.core"}).code == "level-missing"
    assert cl(OracleError("ir", "an unbound IR variable"), set()).code == "ill-typed"
    assert cl(OracleError("type", "x"), set()).code == "ill-typed"
    for bad in (
        OracleError("corpus", "x"),
        OracleError("limit", "total work bound exceeded"),
        OracleError("level", "x", level="ir.data"),
        OracleError("generation", "something else"),
    ):
        with pytest.raises(C.KitError):
            cl(bad, {"ir.data"})


def test_derive_ir_value_calls_and_stand_in():
    inc = {"params": [["a", "U8"]], "body": ["add", ["var", "a"], U8]}
    op = {"params": [["a", "U8"]], "ret": "U8", "requires": ["ir.core"]}
    case = ir_case(
        ["subject_op", "bump", [["subject_op", "bump", [["var", "a"]]]]],
        env=[["a", {"type": "U8", "value": "5"}]],
        models={"inc": inc},
        ops={"bump": op},
        subjects={"bump": "inc"},
    )
    assert C.derive_ir(case) == {"type": "U8", "value": "7", "calls": 2}


def test_derive_ir_stand_in_signature_must_match():
    wrong = {"params": [["a", "U8"]], "body": ["eq", ["var", "a"], U8]}  # returns Bool, the op returns U8
    op = {"params": [["a", "U8"]], "ret": "U8", "requires": ["ir.core"]}
    case = ir_case(["subject_op", "bump", [U8]], models={"inc": wrong}, ops={"bump": op}, subjects={"bump": "inc"})
    with pytest.raises(C.KitError):
        C.derive_ir(case)
    case["subjects"] = {"bump": "missing"}
    with pytest.raises(C.KitError):
        C.derive_ir(case)
    case["subjects"] = {}  # no stand-in at all for a called op
    with pytest.raises(C.KitError):
        C.derive_ir(case)


def test_derive_ir_unknown_error_type_is_a_table_error():
    case = ir_case(["ok", U8])
    with pytest.raises(C.KitError):
        C.derive_ir(case)


def test_derive_ir_undetermined_result_component_fails_the_build_r41():
    """R41: a kit case never has a result type with an undetermined component (an unknown error type or payload)."""
    for ir in (["ok", U8], ["err", "BAD"], ["if", ["lit", True], ["err", "BAD"], ["err", "WORSE"]]):
        with pytest.raises(C.KitError, match="R41"):
            C.derive_ir(ir_case(ir, types=ERR))
    # an intermediate may be undetermined: is_err of a bare ok has the determined type Bool
    assert C.derive_ir(ir_case(["is_err", ["ok", U8]]))["type"] == "Bool"


def test_undetermined_looks_inside_every_argument():
    q = A.Ty("?")
    assert C._undetermined(q)
    assert C._undetermined(A.Ty("Result", (A.Ty("U8"), q)))
    assert C._undetermined(A.Ty("Result", (A.Ty("Option", (q,)), A.Ty("E"))))
    assert not C._undetermined(A.Ty("Result", (A.Ty("U8"), A.Ty("E"))))
    assert not C._undetermined(A.Ty("Bytes", (4,)))


def test_derive_ir_level_error_of_the_oracle_is_level_missing():
    s = ["match", ["lit", {"type": "U8", "value": "1"}], [[["wild"], U8]]]
    case = ir_case(s, levels=["ir.core", "ir.tuple"])
    assert C.derive_ir(case) == {"error": "level-missing"}


def test_derive_ir_overflow_class_and_error_of_the_oracle():
    big = ["lit", {"type": "Nat", "value": str(2**64 - 1)}]
    assert C.derive_ir(ir_case(["mul", big, big])) == {"error": "ir-overflow", "class": "corpus"}
    keep = {"params": [["a", "Nat"]], "body": ["var", "a"]}
    op = {"params": [["a", "Nat"]], "ret": "Nat", "requires": ["ir.core"]}
    case = ir_case(
        ["mul", ["subject_op", "idn", [big]], big], models={"keep": keep}, ops={"idn": op}, subjects={"idn": "keep"}
    )
    assert C.derive_ir(case) == {"error": "ir-overflow", "class": "case"}


def test_derive_ir_unexpected_oracle_failure_during_evaluation_is_a_table_error(monkeypatch):
    def boom(self, tag, kids, cost, subj):
        raise AssertionError("not reached")

    case = ir_case(["not", ["lit", True]])
    real = E.Program.compile

    def failing(self, ir, tenv, allow_subject=True):
        c = real(self, ir, tenv, allow_subject)

        def fn(env):
            raise OracleError("ir", "an evaluation failure")

        return E.Compiled(c.ty, fn, c.cost, c.subject)

    monkeypatch.setattr(E.Program, "compile", failing)
    with pytest.raises(C.KitError):
        C.derive_ir(case)


def test_compile_ir_case_max_ir_cost_must_be_an_integer_in_range():
    for bad in (1999, 65537, "2000", 2000.0):
        with pytest.raises(C.Refused) as exc:
            C.compile_ir_case(ir_case(["lit", True], max_ir_cost=bad))
        assert exc.value.code == "ir-cost"
    assert C.compile_ir_case(ir_case(["lit", True], max_ir_cost=65536)).root.cost == 1


def test_models_are_typed_in_order_and_environment_is_decoded():
    models = {
        "inc": {"params": [["a", "U8"]], "body": ["add", ["var", "a"], U8]},
        "twice": {"params": [["a", "U8"]], "body": ["model_call", "inc", [["model_call", "inc", [["var", "a"]]]]]},
    }
    built = C.compile_ir_case(
        ir_case(["model_call", "twice", [["var", "x"]]], env=[["x", {"type": "U8", "value": "250"}]], models=models)
    )
    assert built.env == {"x": 250}
    assert built.prog.models["twice"].ret == A.Ty("U8")


# ---------------------------------------------------------------------------------------------------
# v2-domain
# ---------------------------------------------------------------------------------------------------


def test_desc_from_json_builds_every_kind():
    cases = [
        ({"d": "bool", "size": 2}, "Bool", 2),
        ({"d": "u8_all", "size": 256}, "U8", 256),
        ({"d": "u64_edge", "size": 377}, "U64", 377),
        ({"d": "nat_0_64", "size": 65}, "Nat", 65),
        ({"d": "nat_count", "k": 7, "size": 8}, "Nat", 8),
        ({"d": "nat_shift", "size": 73}, "Nat", 73),
        ({"d": "bytes_d1", "size": 347}, "Bytes", 347),
        ({"d": "bytes_n", "n": 2, "size": 16}, "Bytes<2>", 16),
        ({"d": "fx_full", "t": "I8", "size": 256}, "I8", 256),
        ({"d": "fx_edge", "t": "I16", "size": 527}, "I16", 527),
        ({"d": "result", "ok": {"d": "bool", "size": 2}, "errs": ["X"], "size": 3}, "Result<Bool,E>", 3),
        ({"d": "option", "of": {"d": "bool", "size": 2}, "size": 3}, "Option<Bool>", 3),
        ({"d": "list", "of": {"d": "bool", "size": 2}, "size": 1 + 2 + 4 + 8}, "List<Bool>", 15),
        (
            {"d": "tuple", "of": [{"d": "bool", "size": 2}, {"d": "bool", "size": 2}], "size": 4},
            "(Bool,Bool)",
            4,
        ),
        (
            {"d": "variant", "ctors": [["A", []], ["B", [{"d": "bool", "size": 2}]]], "size": 3},
            "Shape",
            3,
        ),
    ]
    for d, ty_text, size in cases:
        desc = C.desc_from_json(d, C.parse_type(ty_text, None if "Shape" not in ty_text else {"Shape": {"kind": "variant"}}))
        assert desc.size() == size
        assert desc.nth(size - 1) is not None


def test_desc_from_json_refusals():
    ty = A.Ty("Bool")
    for d in ({"d": "nope", "size": 1}, {"d": "bool"}, {"d": "bool", "size": 3}, [], None):
        with pytest.raises(C.Refused):
            C.desc_from_json(d, ty)
    with pytest.raises(C.Refused):  # a descriptor for another type
        C.desc_from_json({"d": "u8_all", "size": 256}, ty)
    for d, t in (
        ({"d": "tuple", "of": [{"d": "bool", "size": 2}], "size": 2}, "(Bool,Bool)"),  # arity
        ({"d": "option", "of": {"d": "bool", "size": 2}, "size": 3}, "List<Bool>"),
        ({"d": "list", "of": {"d": "bool", "size": 2}, "size": 15}, "Option<Bool>"),
        ({"d": "variant", "ctors": [["A", []]], "size": 1}, "Bool"),
    ):
        with pytest.raises(C.Refused):
            C.desc_from_json(d, C.parse_type(t, None))


def test_desc_from_json_disagreeing_built_size_is_refused(monkeypatch):
    monkeypatch.setattr("coding_adventures_warrant_oracle.domains.leaf_bool", lambda: type("D", (), {"size": lambda s: 9, "ty": A.Ty("Bool")})())
    with pytest.raises(C.Refused):
        C.desc_from_json({"d": "bool", "size": 2}, A.Ty("Bool"))


def test_derive_domain_index_range_and_tuple_stream():
    tup = {"d": "tuple", "of": [{"d": "bool", "size": 2}, {"d": "u8_all", "size": 256}], "size": 512}
    out = C.derive_domain({"descriptor": tup, "type": "(Bool,U8)", "index": 257})
    assert out["size"] == 512
    # index 257 is Bool true (1) with U8 1: the tuple tag 64, a count of two, then the two values
    assert out["values_hex"] == ["64" + "02000000" + "01" + "10" + (1).to_bytes(8, "little").hex()]
    assert C.derive_domain({"descriptor": tup, "type": "(Bool,U8)", "range": [3, 3]})["values_hex"] == []
    assert C.derive_domain({"descriptor": tup, "type": "(Bool,U8)", "range": [0, 513]}) == {"error": "index"}
    assert C.derive_domain({"descriptor": tup, "type": "(Bool,U8)", "index": 512}) == {"error": "index"}
    assert C.derive_domain({"descriptor": tup, "type": "Zzz", "index": 0}) == {"error": "descriptor"}


@pytest.mark.parametrize(
    "field",
    [
        {"index": -1},
        {"index": True},
        {"index": 1.5},
        {"index": "1"},
        {"range": [1.5, 2]},
        {"range": [-1, 2]},
        {"range": [0, "2"]},
        {"range": [0, True]},
        {"range": [0]},
        {"range": "01"},
    ],
)
def test_a_schema_violation_in_a_domain_case_is_a_kit_error_not_an_outcome(field):
    """R45 f: a negative or non-integer ``index`` or ``range`` bound is a broken row, never an ``index`` vector."""
    case = {"descriptor": {"d": "bool", "size": 2}, "type": "Bool", **field}
    with pytest.raises(C.KitError):
        C.derive_domain(case)


def test_descriptor_levels():
    leaf = {"d": "bool", "size": 2}
    assert C.descriptor_levels(leaf) == {"ir.core"}
    assert C.descriptor_levels({"d": "result", "ok": leaf, "errs": [], "size": 2}) == {"ir.core"}
    assert C.descriptor_levels({"d": "tuple", "of": [leaf, leaf], "size": 4}) == {"ir.core", "ir.tuple"}
    assert C.descriptor_levels({"d": "option", "of": leaf, "size": 3}) == {"ir.core", "ir.data"}
    assert C.descriptor_levels({"d": "list", "of": {"d": "tuple", "of": [leaf], "size": 2}, "size": 1}) == {
        "ir.core",
        "ir.data",
        "ir.tuple",
    }
    assert C.descriptor_levels("junk") == {"ir.core"}


# ---------------------------------------------------------------------------------------------------
# v2-fault
# ---------------------------------------------------------------------------------------------------

OPS = {
    "echo": {"params": [["a", "Bytes"]], "ret": "Bytes", "requires": ["ir.core"]},
    "swap": {"params": [["a", "U8"], ["b", "U8"]], "ret": "U8", "requires": ["ir.core"]},
}


def fault_case(fault, op="echo", args=("0a",), result="0b", **kw):
    case = {"fault": fault, "op": OPS[op], "types": {}, "levels": ["ir.core"], "args": list(args), "result": result}
    case.update(kw)
    return case


def test_instantiate_fault_matches_the_templates_and_levels():
    row = C.instantiate_fault("truncate-last", "echo", OPS["echo"], {})
    assert row["kind"] == "post" and row["requires"] == ["ir.core"] and row["op"] == "echo"
    xor = C.instantiate_fault("xor-lsb", "swap", OPS["swap"], {})
    assert xor["requires"] == ["ir.fixed"]


def test_derive_fault_each_kind_and_error():
    post = C.instantiate_fault("truncate-last", "echo", OPS["echo"], {})
    assert C.derive_fault(fault_case(post, result="0a0b")) == {"result": "0a"}
    swap = {"fault": "s", "op": "swap", "kind": "pre", "requires": ["ir.core"],
            "rewrites": [{"param": "a", "expr": ["var", "b"]}, {"param": "b", "expr": ["var", "a"]}]}
    assert C.derive_fault(fault_case(swap, "swap", ("3", "4"), "7")) == {"args": ["4", "3"]}
    replace = {"fault": "r", "op": "echo", "kind": "replace", "requires": ["ir.core"], "expr": ["var", "a"]}
    assert C.derive_fault(fault_case(replace, result="ff")) == {"result": "0a"}
    xor = C.instantiate_fault("xor-lsb", "swap", OPS["swap"], {})
    assert C.derive_fault(fault_case(xor, "swap", ("3", "4"), "7")) == {"error": "level-missing"}
    assert C.derive_fault(fault_case(xor, "swap", ("3", "4"), "7", levels=["ir.core", "ir.fixed"])) == {"result": "6"}
    calls = {"fault": "c", "op": "echo", "kind": "post", "requires": ["ir.core"], "expr": ["subject_op", "echo", [["var", "a"]]]}
    assert C.derive_fault(fault_case(calls)) == {"error": "subject-forbidden"}


def test_derive_fault_table_errors():
    post = C.instantiate_fault("truncate-last", "echo", OPS["echo"], {})
    with pytest.raises(C.KitError):  # the row's requires disagrees with what its tags use
        C.derive_fault(fault_case({**post, "requires": ["ir.fixed"]}))
    with pytest.raises(C.KitError):  # requires not sorted
        C.derive_fault(fault_case({**post, "requires": ["ir.core", "ir.core"]}))
    with pytest.raises(C.KitError):
        C.derive_fault(fault_case(post, args=[]))
    bad = {"fault": "b", "op": "echo", "kind": "post", "requires": ["ir.core"], "expr": ["not", ["var", "a"]]}
    # the expression does not type: [D74], R110 ``ill-typed`` (it was a kit error before 16.22)
    assert C.derive_fault(fault_case(bad)) == {"error": "ill-typed"}
    with pytest.raises(C.Refused):  # a result that is not a value of the op's type
        C.derive_fault(fault_case(post, result="zz"))


# ---------------------------------------------------------------------------------------------------
# v2-chunk: the checks, on a tiny suite derived from the kit-small document
# ---------------------------------------------------------------------------------------------------


@pytest.fixture(scope="module")
def small_kit():
    import pathlib

    here = pathlib.Path(__file__).resolve().parent / "fixtures" / "conformance-v2"
    return C.build_kit((here / "kit-small.warrant").read_bytes(), (here / "kit-small.profile").read_bytes())


def chunk_case(kit, claim, chunk):
    return {
        "corpus": copy.deepcopy(kit.corpus_part()),
        "suite": copy.deepcopy(kit.row(claim)),
        "chunk_file_hex": kit.chunk_file(claim).hex(),
        "chunk": chunk,
    }


def test_derive_chunk_accepts_and_reports_first_admitted_from_the_file(small_kit):
    out = C.derive_chunk(chunk_case(small_kit, "gap", 63), small_kit)
    assert (out["cases"], out["first_admitted"], out["calls"]) == (512, 512, 512)
    assert len(out["inputs_sha256"]) == 64 and len(out["expected_sha256"]) == 64
    line = C.check_chunk_file(small_kit.chunk_file("gap"), small_kit.row("gap"), 64, 1024)[63]
    assert C.chunk_line_expect(line) == out


def test_kit_helpers(small_kit):
    assert set(small_kit.corpus_part()) == C.CHUNK_CORPUS_KEYS
    assert small_kit.row("ragged")["chunks"] == 17
    assert small_kit.chunk_file("ragged").endswith(b"\n")
    assert C.row_keys({"oracle": {"kind": "predicate"}, "stream": "warrant-stream-v2"}) == C._ROW_BASE
    ref_v1 = C.row_keys({"oracle": {"kind": "reference"}, "stream": "warrant-stream-v1"})
    assert {"expected_sha256", "slim_expected_sha256", "v1_inputs_sha256", "v1_expected_sha256"} <= ref_v1
    assert "v1_expected_sha256" not in C.row_keys({"oracle": {"kind": "predicate"}, "stream": "warrant-stream-v1"})
    assert set(small_kit.row("grid")) == C.row_keys(small_kit.row("grid"))


def test_check_corpus_and_row_each_refusal(small_kit):
    base = chunk_case(small_kit, "ragged", 3)
    assert C.check_corpus_and_row(base["corpus"], base["suite"], 3) == (17, 1024)

    def refuse(mutate):
        case = copy.deepcopy(base)
        mutate(case)
        with pytest.raises(C.Refused) as exc:
            C.check_corpus_and_row(case["corpus"], case["suite"], case["chunk"])
        assert exc.value.code == "corpus"

    refuse(lambda c: c.__setitem__("corpus", []))
    refuse(lambda c: c["corpus"].pop("types"))
    refuse(lambda c: c["corpus"].__setitem__("chunk_size", 3000))
    refuse(lambda c: c["corpus"].__setitem__("chunk_size", 0))
    refuse(lambda c: c["corpus"].__setitem__("max_ir_cost", 1999))
    refuse(lambda c: c["corpus"].__setitem__("max_ir_cost", 5))
    refuse(lambda c: c.__setitem__("suite", 7))
    refuse(lambda c: c["suite"].pop("cases"))
    refuse(lambda c: c["suite"].__setitem__("binders", [[]]))
    refuse(lambda c: c["suite"].__setitem__("binders", 5))
    refuse(lambda c: c["suite"].__setitem__("product", 1))
    refuse(lambda c: c["suite"].__setitem__("chunks", 9))
    refuse(lambda c: c.__setitem__("chunk", 17))
    refuse(lambda c: c["corpus"].__setitem__("chunk_size", 1024) or c["suite"].__setitem__("chunks", 5000))


@pytest.mark.parametrize(
    "slim, ok",
    [
        ([0, 4, 16], True),
        ([0, 16], True),
        ([4, 16], False),
        ([0, 4], False),
        ([0, 4, 4, 16], False),
        ([0, 8, 4, 16], False),
        ([0, 17], False),
        ([], False),
        ("0,16", False),
        (list(range(17)) * 2, False),
    ],
)
def test_check_slim_chunks(slim, ok):
    if ok:
        C.check_slim_chunks({"slim_chunks": slim}, 17)
    else:
        with pytest.raises(C.Refused) as exc:
            C.check_slim_chunks({"slim_chunks": slim}, 17)
        assert exc.value.code == "slim-chunks"


@pytest.mark.parametrize("slim", [[0, -4, 16], [0, "4", 16], [0, 4.0, 16], [0, True, 16]])
def test_a_slim_chunks_entry_that_is_not_a_non_negative_integer_is_a_kit_error(slim):
    with pytest.raises(C.KitError):
        C.check_slim_chunks({"slim_chunks": slim}, 17)


def test_a_chunk_schema_violation_is_a_kit_error_not_an_outcome(small_kit):
    """R45 f: a negative or non-integer ``chunk`` or ``chunk_size`` is a broken row, never a ``corpus`` vector."""
    for field, bad in (("chunk", -1), ("chunk", False), ("chunk", "3"), ("chunk", 2.0)):
        case = chunk_case(small_kit, "ragged", 3)
        case[field] = bad
        with pytest.raises(C.KitError):
            C.derive_chunk(case, small_kit)
    for bad in (-1024, True, 1024.0, "1024"):
        case = chunk_case(small_kit, "ragged", 3)
        case["corpus"]["chunk_size"] = bad
        with pytest.raises(C.KitError):
            C.derive_chunk(case, small_kit)
    case = chunk_case(small_kit, "ragged", 3)
    case["suite"]["slim_chunks"] = [0, -1, 16]
    with pytest.raises(C.KitError):
        C.derive_chunk(case, small_kit)
    case["suite"]["slim_chunks"] = [0, 5000, 16]  # a plain slim-chunks outcome is not a schema violation
    assert C.derive_chunk(case, small_kit) == {"error": "slim-chunks"}


def test_check_slim_chunks_more_than_32_entries():
    with pytest.raises(C.Refused):
        C.check_slim_chunks({"slim_chunks": [*range(32), 255]}, 256)
    C.check_slim_chunks({"slim_chunks": [*range(31), 255]}, 256)


def test_check_chunk_file_each_refusal(small_kit):
    row = small_kit.row("ragged")
    good = small_kit.chunk_file("ragged")
    assert len(C.check_chunk_file(good, row, 17, 1024)) == 17

    def seal(data):
        r = dict(row)
        r["chunks_sha256"] = hashlib.sha256(data).hexdigest()
        return data, r

    lines = good.decode().split("\n")[:-1]
    for data in (
        b"",
        good[:-1],
        good.replace(b"\n", b"\r\n"),
        "\n".join(lines[:-1]).encode() + b"\n",
        b"\n".join(line.replace(b" ", b"  ") for line in good.split(b"\n")[:-1]) + b"\n",
        good.replace(b"1 ", b"01 ", 1),
        good + b"\xc3\xa9",
        good.upper(),
    ):
        data, r = seal(data)
        with pytest.raises(C.Refused) as exc:
            C.check_chunk_file(data, r, 17, 1024)
        assert exc.value.code == "chunk-file"
    with pytest.raises(C.Refused):  # a stale chunks_sha256
        C.check_chunk_file(good, {**row, "chunks_sha256": "0" * 64}, 17, 1024)
    for key in ("cases", "inputs_sha256", "expected_sha256"):
        r = dict(row)
        r[key] = r[key] + 1 if key == "cases" else "f" * 64
        with pytest.raises(C.Refused):
            C.check_chunk_file(good, r, 17, 1024)
    with pytest.raises(C.Refused):
        C.check_chunk_file(good, {**row, "discarded": 5}, 17, 1024)
    swapped = lines[:]
    swapped[2], swapped[3] = swapped[3], swapped[2]
    data, r = seal(("\n".join(swapped) + "\n").encode())
    with pytest.raises(C.Refused):
        C.check_chunk_file(data, r, 17, 1024)


def test_advance_admitted_is_a_running_sum_of_admitted_cases():
    assert C.advance_admitted(10, 3, 1024) == 13


# ---------------------------------------------------------------------------------------------------
# the envelope
# ---------------------------------------------------------------------------------------------------


def test_contains_is_a_type_strict_sub_object_test():
    assert C.contains({"a": 1}, {"a": 1, "b": 2})
    assert not C.contains({"a": 1}, {"a": True})
    assert not C.contains({"a": 1}, {"b": 1})
    assert C.contains([1, {"x": 2}], [1, {"x": 2, "y": 3}])
    assert not C.contains([1], [1, 2])
    assert not C.contains([1], {"a": 1})
    assert not C.contains({"a": 1}, [1])
    assert C.contains("s", "s") and not C.contains("s", 1)


def row_of(category, slug="ok-case", intent=None, **fields):
    return C.Row(slug, "a summary", fields, intent or {})


def test_finish_case_checks_every_envelope_rule():
    value_fields = value_case("true")
    good = C.finish_case("v2-value", row_of("v2-value", intent={"outcome": "accept"}, **value_fields), {"outcome": "accept"})
    assert good["id"] == "v2-value/ok-case" and good["requires"] == ["ir.core"]
    assert good["expect"] == {"outcome": "accept"} and good["input_hex"] == value_fields["input_hex"]

    def bad(row, expect, category="v2-value"):
        with pytest.raises(C.KitError) as exc:
            C.finish_case(category, row, expect)
        return str(exc.value)

    assert "v2-value/Bad_Slug" in bad(C.Row("Bad_Slug", "s", value_fields, {}), {})
    assert bad(C.Row("a--b", "s", value_fields, {}), {})
    assert bad(C.Row("a-", "s", value_fields, {}), {})
    assert bad(C.Row("a", "", value_fields, {}), {})
    assert bad(C.Row("a", "two\nlines", value_fields, {}), {})
    assert bad(C.Row("a", "café", value_fields, {}), {})
    assert bad(C.Row("a", "x" * 241, value_fields, {}), {})
    assert "intends" in bad(row_of("v2-value", intent={"outcome": "reject"}, **value_fields), {"outcome": "accept"})
    assert "closed error set" in bad(row_of("v2-value", **value_fields), {"error": "nope"})
    assert "closed error set" in bad(row_of("v2-value", **value_fields), {"error": "depth"})
    ir_fields = ir_case(["lit", True])
    assert "class" in bad(row_of("v2-ir", **ir_fields), {"error": "ir-overflow"}, "v2-ir")
    assert "class" in bad(row_of("v2-ir", **ir_fields), {"error": "ir-cost", "class": "corpus"}, "v2-ir")
    assert "2,000" in bad(row_of("v2-ir", **ir_case(["lit", True], max_ir_cost=2000)), {"type": "Bool"}, "v2-ir")
    wide = ir_case(["bit_not", U8], levels=["ir.core", "ir.fixed"])
    assert "requires" in bad(row_of("v2-ir", **wide), {"type": "U8"}, "v2-ir")


def test_requires_of_every_category():
    r = C.requires_of
    assert r("v2-ir", ir_case(["wibble"]), {"error": "unknown-tag"}) == ["ir.core"]
    assert r("v2-ir", ir_case(["bit_not", U8]), {"error": "level-missing"}) == ["ir.core"]  # the case's levels
    assert r("v2-ir", ir_case(["bit_not", U8], levels=["ir.core", "ir.fixed"]), {"type": "U8"}) == ["ir.core", "ir.fixed"]
    tup = ir_case(["lit", True], env=[["t", {"type": "(U8,U8)", "value": ["1", "2"]}]])
    assert r("v2-ir", tup, {"type": "Bool"}) == ["ir.core", "ir.tuple"]
    tuple_op = {"o": {"params": [["a", "(U8,U8)"]], "ret": "U8"}}
    # R69 g: an op that ir does not reach is not counted; one it calls is
    assert r("v2-ir", ir_case(["lit", True], ops=tuple_op), {}) == ["ir.core"]
    assert r("v2-ir", ir_case(["subject_op", "o", []], ops=tuple_op), {}) == ["ir.core", "ir.tuple"]
    assert r("v2-value", value_case("1", "(U8,U8)"), {}) == ["ir.core", "ir.tuple"]
    assert r("v2-value", value_case("1", "Zzz"), {}) == ["ir.core"]
    assert r("v2-domain", {"descriptor": {"d": "option", "of": {"d": "bool", "size": 2}, "size": 3}}, {}) == ["ir.core", "ir.data"]
    assert r("v2-chunk", {"suite": {"requires": ["ir.core", "ir.fixed"]}}, {}) == ["ir.core", "ir.fixed"]
    assert r("v2-fault", {"fault": {"requires": ["ir.fixed"]}, "levels": ["ir.core"]}, {"error": "level-missing"}) == ["ir.core"]
    assert r("v2-fault", {"fault": {"requires": ["ir.core"]}, "levels": ["ir.core"]}, {"result": "x"}) == ["ir.core"]
    assert r("v2-arith", {}, {}) == ["ir.core"]


def test_build_category_checks_ids_and_chunk_agreement(monkeypatch):
    rows = [C.Row("same", "s", value_case("true"), {"outcome": "accept"})] * 2
    with pytest.raises(C.KitError, match="duplicate id"):
        C.build_category("v2-value", rows)
    with pytest.raises(C.KitError, match="v2-value/same"):
        C.build_category("v2-value", [C.Row("same", "s", value_case("true"), {"outcome": "reject"})])
    with pytest.raises(C.KitError, match="v2-value/boom"):
        monkeypatch.setitem(C._DERIVERS, "v2-value", lambda f: (_ for _ in ()).throw(C.KitError("x")))
        C.build_category("v2-value", [C.Row("boom", "s", value_case("true"), {})])


def test_build_category_chunk_enumeration_must_agree_with_the_file(small_kit, monkeypatch):
    fields = {**chunk_case(small_kit, "grid", 5), "_kit": "small"}
    row = C.Row("grid-five", "s", fields, {"cases": 1024})
    cases = C.build_category("v2-chunk", [row], {"small": small_kit})
    assert cases[0]["id"] == "v2-chunk/grid-five" and "_kit" not in cases[0]
    real = C.derive_chunk
    monkeypatch.setitem(C._DERIVERS, "v2-chunk", None)  # derive_expect special-cases v2-chunk

    def lying(case, kit):
        out = real(case, kit)
        return {**out, "calls": out["calls"] + 1}

    monkeypatch.setattr(C, "derive_chunk", lying)
    with pytest.raises(C.KitError, match="disagrees"):
        C.build_category("v2-chunk", [row], {"small": small_kit})


def test_render_and_build_files_orders_categories_and_rejects_clashes():
    rows = {c: [] for c in C.CATEGORIES}
    rows["v2-value"] = [C.Row("a", "s", value_case("true"), {"outcome": "accept"})]
    rows["v2-arith"] = [C.Row("a", "s", {"type": "U8", "op": "neg", "lhs": "1"}, {"type": "U8"})]
    files, index = C.build_files(rows, {})
    assert list(files) == [f"{c}.json" for c in C.CATEGORIES]
    assert [r["category"] for r in index] == list(C.CATEGORIES)
    assert index[0]["cases"] == 1 and index[2]["cases"] == 0
    assert index[0]["sha256"] == hashlib.sha256(files["v2-value.json"].encode()).hexdigest()
    text = files["v2-value.json"]
    assert text.endswith("}\n") and text.startswith('{\n  "cases": [\n') and '"schema": "warrant-corpus-conformance-v1"' in text
    rows["v2-arith"] = [C.Row("a", "s", value_case("true"), {"outcome": "accept"})]
    # the same slug in two categories is two ids; the same id twice would be a clash
    C.build_files({**rows, "v2-arith": [C.Row("a", "s", {"type": "U8", "op": "neg", "lhs": "1"}, {})]}, {})
