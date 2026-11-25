"""Slice S11c-1: evaluation of ``ir.data`` (VER04r 13, 16.14, Appendix A.2, 8.2, 16.17 R67, 16.19 R94).

Until this slice the oracle lowered, typed and costed records, variants and ``Option`` and refused to run them
(``level ir.data``).  These tests hold what it computes now: the four tags and ``match`` with the data patterns on
values written by hand, structural equality, the three value encodings (JSON, stream, TFV), the two faults that
need ``ir.data`` against the exact JSON of section 8.2, the first comparison of a fault row's type with a declared
type (R67, R94), the fault gate that opens here (a ``{"deferred": "ir.data"}`` witness is rejected, the S10c
pattern) and ``emit-v2``/``verify-v2`` on a synthetic data document.  The committed dumps of
``tests/fixtures/data-eval-v1`` are tested in ``test_data_eval_dump.py``.
"""

from __future__ import annotations

import json
import pathlib

import pytest
from conftest import adopter_bytes, write_lf

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle import patterns as P
from coding_adventures_warrant_oracle import tfv
from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.corpus_v2 import (
    _over_limit,
    canonical,
    derive,
    value_json,
)
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.faults import (
    DATA_LEVEL,
    IMPLEMENTED,
    REGISTRY,
    _compile_fault,
    _fault_unify,
    derive_controls,
    fault_row,
)
from coding_adventures_warrant_oracle.ir_eval import Program
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.stream import make_encoder
from coding_adventures_warrant_oracle.verify_v2 import verify_v2

GEN = {"name": "test", "version": "0"}
CLOSED = ("ir.core", "ir.fixed", "ir.tuple")  # the gate before this slice: every ir.data fault is deferred

DECLS = """  error E = A | B
  record Point(x: U8, y: U8)
  record Pair(a: Bool, n: Nat)
  record Slot(o: Option<U8>, t: (Bool,Nat))
  variant Shape = Circle(r: U8) | Rect(w: U8, h: U8) | Dot
  variant Mode = Add | Sub | Neg
"""


def document(models: str = "", ops: str = "") -> bytes:
    return f"warrant t v1\n{DECLS}{models}{ops}".encode()


def spec_and_program(models: str = ""):
    spec = lower_document(parse_source(document(models)))
    return spec, build_program(spec)


def evaluate(body: str, params: str, ret: str, env: dict | None = None, extra: str = ""):
    """The value of ``model m(params) -> ret = body`` at ``env`` (a dict, parameter name to value)."""
    _spec, prog = spec_and_program(f"{extra}  model m({params}) -> {ret} = {body}\n")
    return prog.model("m").fn(env or {})


NONE = ("none",)


def some(v):
    return ("some", v)


# -- the four tags of Appendix A.2 -------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "body,params,ret,env,expected",
    [
        ("Some(a)", "a: U8", "Option<U8>", {"a": 7}, some(7)),
        ("None", "u: Bool", "Option<U8>", {"u": True}, NONE),
        ("Some(Some(a))", "a: Bool", "Option<Option<Bool>>", {"a": False}, some(some(False))),
        ("Some(None)", "u: Bool", "Option<Option<Bool>>", {"u": True}, some(NONE)),
        ("if c then Some(a) else None", "c: Bool, a: U8", "Option<U8>", {"c": False, "a": 1}, NONE),
        ("Some((a, 2))", "a: Bool", "Option<(Bool,Nat)>", {"a": True}, some((True, 2))),
        ("Point(a, b)", "a: U8, b: U8", "Point", {"a": 1, "b": 2}, (1, 2)),
        ("Pair(a, 3)", "a: Bool", "Pair", {"a": True}, (True, 3)),
        ("Slot(o, (a, 2))", "o: Option<U8>, a: Bool", "Slot", {"o": some(9), "a": False}, (some(9), (False, 2))),
        ("Circle(r)", "r: U8", "Shape", {"r": 3}, ("Circle", (3,))),
        ("Rect(w, h)", "w: U8, h: U8", "Shape", {"w": 3, "h": 4}, ("Rect", (3, 4))),
        ("Dot", "u: Bool", "Shape", {"u": True}, ("Dot", ())),
        ("Neg", "u: Bool", "Mode", {"u": True}, ("Neg", ())),
        ("p.x", "p: Point", "U8", {"p": (5, 6)}, 5),
        ("p.y", "p: Point", "U8", {"p": (5, 6)}, 6),
        ("s.t.0", "s: Slot", "Bool", {"s": (NONE, (True, 1))}, True),
        ("s.o", "s: Slot", "Option<U8>", {"s": (some(4), (True, 1))}, some(4)),
        ("Point(a, 1u8).x", "a: U8", "U8", {"a": 9}, 9),
        ("unwrap_or(o, 5u8)", "o: Option<U8>", "U8", {"o": NONE}, 5),
        ("unwrap_or(o, 5u8)", "o: Option<U8>", "U8", {"o": some(2)}, 2),
        ("unwrap_or(o, None)", "o: Option<Option<Bool>>", "Option<Bool>", {"o": NONE}, NONE),
        ("unwrap_or(checked_add(a, b), 0u8)", "a: U8, b: U8", "U8", {"a": 200, "b": 100}, 0),
        ("checked_add(a, b) == Some(a)", "a: U8, b: U8", "Bool", {"a": 1, "b": 0}, True),
    ],
)
def test_the_four_tags_evaluate(body, params, ret, env, expected):
    assert evaluate(body, params, ret, env) == expected


def test_a_none_is_typed_by_its_context_and_an_option_is_an_ordinary_operand():
    # ``["none", "T"]`` carries the canonical type name; ``some`` is Option<type(e)> (D60: it lifts like Result)
    _spec, prog = spec_and_program()
    tenv = {"o": A.Ty("Option", (A.Ty("U8"),))}
    c = prog.compile(["eq", ["var", "o"], ["none", "U8"]], tenv, False)
    assert c.ty == A.Ty("Bool") and c.fn({"o": NONE}) and not c.fn({"o": some(1)})
    c = prog.compile(["some", ["lit", {"type": "Nat", "value": "3"}]], {}, False)  # a top-level Option result
    assert c.ty == A.Ty("Option", (A.Ty("Nat"),)) and c.fn({}) == some(3)


def test_costs_are_one_plus_the_operands_and_none_is_one():
    _spec, prog = spec_and_program()
    v = ["var", "a"]
    assert prog.compile(["none", "U8"], {}, False).cost == 1
    assert prog.compile(["some", v], {"a": A.Ty("U8")}, False).cost == 2
    assert prog.compile(["ctor", "Point", [v, v]], {"a": A.Ty("U8")}, False).cost == 3
    assert prog.compile(["field", ["ctor", "Point", [v, v]], 1], {"a": A.Ty("U8")}, False).cost == 4


def test_ir_errors_of_the_data_tags_are_corpus_errors_with_the_code_ir():
    _spec, prog = spec_and_program()
    u8 = ["lit", {"type": "U8", "value": "1"}]
    point = A.Ty("Point")
    for bad in (
        ["none"],  # malformed
        ["none", "Wide"],  # an undeclared type name
        ["none", "List<U8>"],  # a list is the reserved level (below)
        ["some"],
        ["ctor", "Nowhere", []],  # an undeclared constructor
        ["ctor", "Point", [u8]],  # the wrong operand count
        ["ctor", "Point", [u8, ["lit", True]]],  # an operand that is not its field's type
        ["field", ["var", "p"], 2],  # an index outside the record
        ["field", ["var", "p"], "x"],  # a malformed index
        ["field", u8, 0],  # a field of a non-record
    ):
        with pytest.raises(OracleError) as err:
            prog.compile(bad, {"p": point}, False)
        assert err.value.code in ("ir", "level"), bad
    with pytest.raises(OracleError) as err:
        prog.compile(["none", "List<U8>"], {}, False)
    assert err.value.level == "ir.list"


def test_the_gate_off_keeps_the_pre_s11c_behaviour_for_a_lane_that_does_not_declare_ir_data():
    # ``analyze_data=False`` is the program of a lane without ir.data (the conformance kit's ``levels``): every data
    # tag, a match on a non-Result and an Option outside the intermediate use is the level ir.data, as before S11a.
    off = Program({}, {}, analyze_data=False)
    for ir in (["some", ["lit", True]], ["none", "U8"], ["ctor", "Dot", []], ["field", ["lit", True], 0]):
        with pytest.raises(OracleError) as err:
            off.compile(ir, {})
        assert err.value.level == "ir.data"
    with pytest.raises(OracleError) as err:
        off.compile(["match", ["lit", True], [[["wild"], ["lit", True]]]], {})
    assert err.value.level == "ir.data"
    assert Program({}, {}).analyze_data is True  # open by default since S11c-1


# -- match with the data patterns -------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "body,params,ret,env,expected",
    [
        ("match o with Some(x) -> x | None -> 0u8", "o: Option<U8>", "U8", {"o": some(7)}, 7),
        ("match o with Some(x) -> x | None -> 0u8", "o: Option<U8>", "U8", {"o": NONE}, 0),
        ("match o with Some(Some(x)) -> 2 | Some(None) -> 1 | None -> 0", "o: Option<Option<Bool>>", "Nat", {"o": some(some(True))}, 2),
        ("match o with Some(Some(x)) -> 2 | Some(None) -> 1 | None -> 0", "o: Option<Option<Bool>>", "Nat", {"o": some(NONE)}, 1),
        ("match o with Some(Some(x)) -> 2 | Some(None) -> 1 | None -> 0", "o: Option<Option<Bool>>", "Nat", {"o": NONE}, 0),
        ("match o with Some((true, n)) -> n | Some((false, _)) -> 0 | None -> 0", "o: Option<(Bool,Nat)>", "Nat", {"o": some((True, 9))}, 9),
        ("match o with Some((true, n)) -> n | Some((false, _)) -> 0 | None -> 0", "o: Option<(Bool,Nat)>", "Nat", {"o": some((False, 9))}, 0),
        ("match b with true -> 1 | false -> 0", "b: Bool", "Nat", {"b": True}, 1),
        ("match b with true -> 1 | false -> 0", "b: Bool", "Nat", {"b": False}, 0),
        ("match b with true -> 1 | _ -> 0", "b: Bool", "Nat", {"b": False}, 0),
        ("match t with (true, x) -> x | (false, _) -> false", "t: (Bool,Bool)", "Bool", {"t": (True, True)}, True),
        ("match p with Point(x, y) -> x + y", "p: Point", "U8", {"p": (1, 2)}, 3),
        ("match p with Point(_, y) -> y", "p: Point", "U8", {"p": (1, 2)}, 2),
        ("match s with Circle(r) -> r | Rect(w, h) -> w + h | Dot -> 0u8", "s: Shape", "U8", {"s": ("Circle", (4,))}, 4),
        ("match s with Circle(r) -> r | Rect(w, h) -> w + h | Dot -> 0u8", "s: Shape", "U8", {"s": ("Rect", (4, 5))}, 9),
        ("match s with Circle(r) -> r | Rect(w, h) -> w + h | Dot -> 0u8", "s: Shape", "U8", {"s": ("Dot", ())}, 0),
        ("match s with Dot -> true | _ -> false", "s: Shape", "Bool", {"s": ("Circle", (1,))}, False),
        ("match o with Add -> 0 | Sub -> 1 | Neg -> 2", "o: Mode", "Nat", {"o": ("Sub", ())}, 1),
        (
            "match s with Slot(Some(_), (true, n)) -> n | Slot(Some(_), (false, _)) -> 1 | Slot(None, _) -> 0",
            "s: Slot",
            "Nat",
            {"s": (some(1), (True, 8))},
            8,
        ),
        (
            "match s with Slot(Some(_), (true, n)) -> n | Slot(Some(_), (false, _)) -> 1 | Slot(None, _) -> 0",
            "s: Slot",
            "Nat",
            {"s": (NONE, (True, 8))},
            0,
        ),
        ("match s with Circle(r) -> Rect(r, r) | other -> other", "s: Shape", "Shape", {"s": ("Circle", (2,))}, ("Rect", (2, 2))),
        ("match s with Circle(r) -> Rect(r, r) | other -> other", "s: Shape", "Shape", {"s": ("Dot", ())}, ("Dot", ())),
        # a Result with a data pattern: ir.tuple and ir.data together
        ("match r with Ok(Some(_)) -> 1 | Ok(None) -> 0 | Err(_) -> 2", "r: Result<Option<U8>,E>", "Nat", {"r": ("ok", some(1))}, 1),
        ("match r with Ok(Some(_)) -> 1 | Ok(None) -> 0 | Err(_) -> 2", "r: Result<Option<U8>,E>", "Nat", {"r": ("ok", NONE)}, 0),
        ("match r with Ok(Some(_)) -> 1 | Ok(None) -> 0 | Err(_) -> 2", "r: Result<Option<U8>,E>", "Nat", {"r": ("err", "A")}, 2),
        ("match r with Ok(Point(x, _)) -> x | Err(E.A) -> 0u8 | Err(_) -> 1u8", "r: Result<Point,E>", "U8", {"r": ("ok", (6, 7))}, 6),
        ("match r with Ok(Point(x, _)) -> x | Err(E.A) -> 0u8 | Err(_) -> 1u8", "r: Result<Point,E>", "U8", {"r": ("err", "B")}, 1),
        # nested matches and a match on a call
        ("match o with Some(_) -> (match b with true -> 1 | false -> 2) | None -> 0", "o: Option<U8>, b: Bool", "Nat", {"o": some(1), "b": False}, 2),
    ],
)
def test_match_with_the_data_patterns_evaluates_exactly_one_arm(body, params, ret, env, expected):
    assert evaluate(body, params, ret, env) == expected


def test_a_match_arm_is_evaluated_only_when_its_pattern_matches():
    # one arm, the first that matches; the default of an unmatched arm is never evaluated (no short circuit applies
    # to unwrap_or's default, but a match arm is lazy): a division in a dead arm cannot fail
    body = "match o with Some(x) -> x | None -> unwrap_or(div(1u8, 0u8), 7u8)"
    assert evaluate(body, "o: Option<U8>", "U8", {"o": some(3)}) == 3
    assert evaluate(body, "o: Option<U8>", "U8", {"o": NONE}) == 7


def test_a_ctor_pattern_needs_the_declared_types():
    with pytest.raises(OracleError) as err:
        P.compile_pattern(["ctor", "Point", [["wild"], ["wild"]]])
    assert err.value.code == "ir"
    with pytest.raises(OracleError):
        P.compile_pattern(["ctor", "Nowhere", []], P.Types({}, {"Point": (("Point", ()),)}))
    # a record: the tuple of fields; a variant: (constructor, fields)
    types = P.Types({}, {"Point": (("Point", ()),), "Shape": (("Circle", ()), ("Dot", ()))})
    record = P.compile_pattern(["ctor", "Point", [["bind", "a"], ["wild"]]], types)
    binds: dict = {}
    assert record((3, 4), binds) and binds == {"a": 3}
    variant = P.compile_pattern(["ctor", "Dot", []], types)
    assert variant(("Dot", ()), {}) and not variant(("Circle", (1,)), {})


# -- structural equality ------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "body,params,env,expected",
    [
        ("a == b", "a: Point, b: Point", {"a": (1, 2), "b": (1, 2)}, True),
        ("a == b", "a: Point, b: Point", {"a": (1, 2), "b": (1, 3)}, False),
        ("a != b", "a: Point, b: Point", {"a": (1, 2), "b": (1, 3)}, True),
        ("a == b", "a: Slot, b: Slot", {"a": (some(1), (True, 2)), "b": (some(1), (True, 2))}, True),
        ("a == b", "a: Slot, b: Slot", {"a": (some(1), (True, 2)), "b": (NONE, (True, 2))}, False),
        ("a == b", "a: Shape, b: Shape", {"a": ("Circle", (1,)), "b": ("Circle", (1,))}, True),
        ("a == b", "a: Shape, b: Shape", {"a": ("Circle", (1,)), "b": ("Rect", (1, 1))}, False),
        ("a == Dot", "a: Shape", {"a": ("Dot", ())}, True),
        ("a == b", "a: Option<U8>, b: Option<U8>", {"a": some(1), "b": some(1)}, True),
        ("a == b", "a: Option<U8>, b: Option<U8>", {"a": some(1), "b": NONE}, False),
        ("a != b", "a: Option<U8>, b: Option<U8>", {"a": NONE, "b": NONE}, False),
        ("a == None", "a: Option<U8>", {"a": NONE}, True),
        ("Some(1u8) == a", "a: Option<U8>", {"a": some(1)}, True),
        # D60: Option<Nat> and Option<Int> unify by their payloads; the values are the same integers
        ("a == b", "a: Option<Nat>, b: Option<Nat>", {"a": some(5), "b": some(5)}, True),
    ],
)
def test_equality_of_records_variants_and_options_is_structural(body, params, env, expected):
    assert evaluate(body, params, "Bool", env) is expected


def test_an_ordering_of_a_record_is_undefined():
    # ordering is defined for numbers only (R30: "equality is structural, ordering is undefined")
    _spec, prog = spec_and_program()
    with pytest.raises(OracleError) as err:
        prog.compile(["lt", ["var", "a"], ["var", "b"]], {"a": A.Ty("Point"), "b": A.Ty("Point")}, False)
    assert err.value.code == "ir"


def test_wraps_are_counted_inside_a_data_match_and_a_model_call_with_a_record_argument():
    spec, prog = spec_and_program(
        "  model sum(p: Point) -> U8 = match p with Point(x, y) -> x + y\n"
        "  model twice(p: Point) -> U8 = sum(p) + sum(p)\n"
    )
    del spec
    prog.stats.counting = True
    assert prog.model("sum").fn({"p": (200, 100)}) == 44 and prog.stats.wraps == 1
    assert prog.model("twice").fn({"p": (1, 2)}) == 6 and prog.stats.wraps == 1


# -- values: JSON, TFV, the stream, the example limit --------------------------------------------------------------


@pytest.fixture(scope="module")
def typed():
    spec = lower_document(parse_source(document()))
    return spec


def ty(text: str) -> A.Ty:
    from coding_adventures_warrant_oracle.typenames import parse_type_name

    return parse_type_name(text)


VALUES = [
    ("Option<U8>", NONE, {"none": True}, "L0", "40"),
    ("Option<U8>", some(7), {"some": "7"}, "L1 I7", "41" + "10" + "0700000000000000"),
    ("Option<Bool>", some(False), {"some": False}, "L1 F", "4100"),
    ("Option<Option<Bool>>", some(NONE), {"some": {"none": True}}, "L1 L0", "4140"),
    ("Point", (1, 2), ["1", "2"], "L2 I1 I2", "64" + "02000000" + "10" + "0100000000000000" + "10" + "0200000000000000"),
    ("Slot", (some(9), (True, 2)), [{"some": "9"}, [True, "2"]], "L2 L1 I9 L2 T I2", None),
    ("Shape", ("Circle", (3,)), {"ctor": "Circle", "fields": ["3"]}, "L2 S436972636c65 L1 I3", None),
    ("Shape", ("Dot", ()), {"ctor": "Dot", "fields": []}, "L2 S446f74 L0", "65" + "03000000" + "446f74" + "00000000"),
    ("Mode", ("Neg", ()), {"ctor": "Neg", "fields": []}, "L2 S4e6567 L0", None),
    ("Result<Point,E>", ("ok", (1, 2)), {"ok": ["1", "2"]}, "M1 S6f6b L2 I1 I2", None),
    ("Result<Option<U8>,E>", ("ok", NONE), {"ok": {"none": True}}, "M1 S6f6b L0", None),
    ("Result<Shape,E>", ("err", "B"), {"err": "B"}, "M1 S657272 S42", None),
]


@pytest.mark.parametrize("text,value,as_json,as_tfv,stream", VALUES)
def test_json_tfv_and_stream_of_data_values(typed, text, value, as_json, as_tfv, stream):
    t = ty(text) if not text.startswith("Result") else _result(text)
    assert value_json(value, t, typed) == as_json
    assert tfv.value(value, t, typed) == as_tfv
    if stream is not None:
        assert make_encoder(typed, t)(value).hex() == stream


def _result(text: str) -> A.Ty:
    inner, _, error = text[len("Result<") : -1].rpartition(",")
    return A.Ty("Result", (ty(inner), A.Ty(error)))


def test_an_option_and_a_record_use_the_tags_of_section_2_4_and_a_one_field_record_is_not_an_option(typed):
    # ``L1 x`` is an Option's some AND a one-field record: the declared type decides, never the text (section 2.5)
    assert tfv.value(some(3), ty("Option<U8>"), typed) == "L1 I3"
    one = lower_document(parse_source(b"warrant t v1\n  record One(a: U8)\n"))
    assert tfv.value((3,), A.Ty("One"), one) == "L1 I3"
    assert value_json((3,), A.Ty("One"), one) == ["3"]
    assert make_encoder(one, A.Ty("One"))((3,)).hex() == "64" + "01000000" + "10" + "0300000000000000"


def test_a_value_the_writers_cannot_name_is_refused_as_the_reserved_level_ir_list(typed):
    for writer in (lambda: value_json((), A.Ty("List", (A.Ty("U8"),)), typed), lambda: tfv.value((), A.Ty("List", (A.Ty("U8"),)), typed)):
        with pytest.raises(OracleError) as err:
            writer()
        assert err.value.code == "level" and err.value.level == "ir.list"
    with pytest.raises(OracleError):  # a record needs the document that declares it
        value_json((1, 2), A.Ty("Point"))
    with pytest.raises(OracleError):
        tfv.value((1, 2), A.Ty("Point"))


def _chain(n: int, kind: str):
    """A chain of ``n`` named types, each holding the next (no recursion, no wrapper in any type text): records
    ``R0(f: R1) .. R<n-1>(f: Bool)`` or variants ``V0 = C0(f: V1) | E0 .. V<n-1> = E | F``; and a value of ``R0`` / ``V0``."""
    lines = ["warrant t v1"]
    if kind == "record":
        for i in range(n):
            lines.append(f"  record R{i}(f: R{i + 1})" if i < n - 1 else f"  record R{i}(f: Bool)")
        value: object = (True,)
        for _ in range(n - 1):
            value = (value,)
    else:
        for i in range(n):
            lines.append(f"  variant V{i} = C{i}(f: V{i + 1}) | E{i}" if i < n - 1 else f"  variant V{i} = E{i} | F{i}")
        value = (f"E{n - 1}", ())
        for i in reversed(range(n - 1)):
            value = (f"C{i}", (value,))
    return lower_document(parse_source(("\n".join(lines) + "\n").encode())), value


def _line(spec, value, head: str) -> str:
    return tfv.record("x", "mk", [], [], None, value, A.Ty(head), spec)


def test_tfv_depth_is_64_containers_of_one_field_value_and_the_lines_r_does_not_count():
    # R99: a record is one container, a variant two (``L2`` and ``L<k>``), the outermost of a field is depth 1; the 65th is
    # refused with the code ``limit`` (VER04j 4.9, W0709).  A chain of *named* types adds nothing to a type expression's depth.
    for kind, head, per_link in (("record", "R0", 1), ("variant", "V0", 2)):
        fits = 64 // per_link  # 64 containers exactly
        spec, value = _chain(fits, kind)
        assert _line(spec, value, head).startswith("x R2 Kop S6d6b Kexpect L")
        spec, value = _chain(fits + 1, kind)
        with pytest.raises(OracleError) as err:
            _line(spec, value, head)
        assert err.value.code == "limit"
    # the review's probe: a 40-link variant chain (80 containers) in an example's result
    spec, value = _chain(40, "variant")
    with pytest.raises(OracleError) as err:
        _line(spec, value, "V0")
    assert err.value.code == "limit"
    # a Result's M1 (Ok and Err), an Option's L0 and L1 and a tuple's L<n> count as well
    res = A.Ty("Result", (A.Ty("()", (A.Ty("U8"),)), A.Ty("E")))
    assert tfv.value(("ok", (1,)), res, None, 62) == "M1 S6f6b L1 I1"
    with pytest.raises(OracleError) as err:
        tfv.value(("ok", (1,)), res, None, 63)  # M1 is container 64, the tuple's L1 is 65
    assert err.value.code == "limit"
    assert tfv.value(("err", "A"), res, None, 63) == "M1 S657272 S41" or tfv.value(("err", "A"), res, None, 63).startswith("M1")
    with pytest.raises(OracleError):
        tfv.value(("err", "A"), res, None, 64)
    assert tfv.value(some(1), ty("Option<U8>"), None, 63) == "L1 I1" and tfv.value(NONE, ty("Option<U8>"), None, 63) == "L0"
    for v in (some(1), NONE):
        with pytest.raises(OracleError) as err:
            tfv.value(v, ty("Option<U8>"), None, 64)
        assert err.value.code == "limit"


def test_the_fallback_refusal_names_the_level_of_the_missing_type():
    # a record or a variant given without its document is ir.data; only List is the reserved ir.list
    with pytest.raises(OracleError) as err:
        tfv.value((1, 2), A.Ty("Point"))
    assert err.value.level == "ir.data"
    with pytest.raises(OracleError) as err:
        value_json(("Dot", ()), A.Ty("Shape"))
    assert err.value.level == "ir.data"
    with pytest.raises(OracleError) as err:
        tfv.value([], A.Ty("List", (A.Ty("U8"),)))
    assert err.value.level == "ir.list"


def test_the_4096_byte_limit_of_an_example_sees_through_options_records_and_variants():
    spec = lower_document(
        parse_source(b"warrant t v1\n  record Blob(d: Bytes, n: Nat)\n  variant Msg = Short | Big(d: Bytes)\n")
    )
    big, small = b"\0" * 4097, b"\0" * 4096
    blob, msg = A.Ty("Blob"), A.Ty("Msg")
    opt = A.Ty("Option", (A.Ty("Bytes"),))
    assert _over_limit((big, 1), blob, spec) and not _over_limit((small, 1), blob, spec)
    assert _over_limit(("Big", (big,)), msg, spec) and not _over_limit(("Short", ()), msg, spec)
    assert _over_limit(some(big), opt, spec) and not _over_limit(NONE, opt, spec)
    assert not _over_limit(("Big", (small,)), msg, spec) and not _over_limit(5, A.Ty("Nat"))


# -- the faults that need ir.data: the exact JSON of section 8.2 -----------------------------------------------------


def adopter_faults(name: str):
    spec = lower_document(parse_source(adopter_bytes(name)))
    data = frozenset([*spec.records, *spec.variants])

    def row(op: str, fault: str) -> dict:
        return fault_row(spec.ops[op], fault, spec.errors, data, dict(spec.records))

    return spec, row


R = ["var", "result"]
U8_0 = ["lit", {"type": "U8", "value": "0"}]
FALSE = ["lit", False]


def ok_if(payload):
    return ["if", ["is_err", R], ["ok", payload], R]


def test_the_two_adopters_missing_error_rows_are_the_json_of_the_section_8_2_table():
    _spec, row = adopter_faults("arithmetic")
    ripple = ok_if(["ctor", "RippleCarryResult", [["var", "a"], U8_0, FALSE]])
    with_carry = ok_if(["ctor", "RippleCarryResult", [["var", "a"], ["var", "carry_in"], FALSE]])
    alu = ok_if(["ctor", "AluResult", [["var", "a"], FALSE, FALSE, FALSE, FALSE]])
    assert row("ripple_carry_adder", "missing-error") == {
        "fault": "missing-error", "op": "ripple_carry_adder", "kind": "replace", "requires": ["ir.data"], "expr": ripple,
    }
    assert row("ripple_carry_adder_with_carry", "missing-error")["expr"] == with_carry
    assert row("ripple_carry_adder_with_carry", "missing-error")["requires"] == ["ir.data"]
    assert row("alu", "missing-error")["expr"] == alu
    _lg, lg = adopter_faults("logic-gates")
    assert lg("tri_state", "missing-error") == {
        "fault": "missing-error", "op": "tri_state", "kind": "replace", "requires": ["ir.data"],
        "expr": ok_if(["none", "U8"]),
    }
    # the rows of the table that do not need ir.data are unchanged by the data-aware rule
    assert lg("and_gate", "missing-error")["expr"] == ok_if(["var", "a"])
    assert lg("and_n", "missing-error")["expr"] == ok_if(U8_0)
    assert lg("priority_encoder", "missing-error")["expr"] == ok_if(["tuple", [["var", "inputs"], U8_0]])
    assert lg("priority_encoder", "missing-error")["requires"] == ["ir.tuple"]


def test_field_zero_of_the_two_adders_is_the_json_of_section_8_2():
    _spec, row = adopter_faults("arithmetic")
    expected = [
        "match", R,
        [
            [["ok", ["bind", "ok_payload"]],
             ["ok", ["ctor", "RippleCarryResult", [["field", ["var", "ok_payload"], 0], U8_0, ["field", ["var", "ok_payload"], 2]]]]],
            [["err_any"], R],
        ],
    ]  # fmt: skip
    for op in ("ripple_carry_adder", "ripple_carry_adder_with_carry"):
        r = row(op, "field-zero")
        assert (r["kind"], r["requires"], r["expr"]) == ("post", ["ir.data", "ir.tuple"], expected)
    with pytest.raises(OracleError, match="numeric field"):  # AluResult has none: only missing-error binds the ALU
        row("alu", "field-zero")


def test_field_zero_picks_the_first_numeric_field_of_its_type_bare_and_in_an_ok():
    src = (
        b"warrant t v1\n  error E = A\n  record R(a: Bool, b: Bytes, c: I16, d: U8)\n  record N(a: Nat)\n"
        b"  op bare(x: U8) -> R\n  op wrapped(x: U8) -> Result<R,E>\n  op nat(x: U8) -> N\n"
    )
    spec = lower_document(parse_source(src))
    data, records = frozenset(spec.records), dict(spec.records)

    def row(op):
        return fault_row(spec.ops[op], "field-zero", spec.errors, data, records)

    bare = row("bare")
    assert bare["requires"] == ["ir.data"] and bare["kind"] == "post"
    assert bare["expr"] == ["ctor", "R", [["field", R, 0], ["field", R, 1], ["lit", {"type": "I16", "value": "0"}], ["field", R, 3]]]
    assert row("wrapped")["requires"] == ["ir.data", "ir.tuple"]
    assert row("nat")["expr"] == ["ctor", "N", [["lit", {"type": "Nat", "value": "0"}]]]


@pytest.mark.parametrize(
    "decl",
    [
        "op f(a: U8) -> U8",
        "op f(a: U8) -> Bytes",
        "op f(a: U8) -> (U8,Nat)",
        "op f(a: U8) -> Shape",
        "op f(a: U8) -> Option<Point>",
        "op f(a: U8) -> Result<U8,E>",
        "op f(a: U8) -> Result<Shape,E>",
        "op f(a: U8) -> Result<Option<Point>,E>",
        "op f(ok_payload: U8) -> Result<Point,E>",
    ],
)
def test_field_zero_refuses_what_has_no_template(decl):
    spec = lower_document(parse_source(f"warrant t v1\n  error E = A | B\n  record Point(x: U8, y: U8)\n  variant Shape = Circle(r: U8) | Dot\n  {decl}\n".encode()))
    with pytest.raises(OracleError) as err:
        fault_row(spec.ops["f"], "field-zero", spec.errors, frozenset(["Point", "Shape"]), dict(spec.records))
    assert err.value.code == "generation" and err.value.message.startswith("fault:")


def test_there_is_one_rule_and_no_data_blind_call():
    # D71: records=None is not a mode.  A caller that passes no tables is a document that declares no record and no
    # variant, so field-zero has no template there; and a document's own call writes the Option template that the
    # committed scalar-faults-v1 dump once pinned as refused (its row is gone: 52 rows).
    spec = lower_document(parse_source(b"warrant t v1\n  error E = A\n  record P(x: U8)\n  op o(a: U8) -> Result<Option<U8>,E>\n  op p(a: U8) -> P\n"))
    assert fault_row(spec.ops["o"], "missing-error", spec.errors)["expr"][2] == ["ok", ["none", "U8"]]
    with pytest.raises(OracleError, match="needs a record or Result<record, E> result"):
        fault_row(spec.ops["p"], "field-zero", spec.errors)
    assert fault_row(spec.ops["o"], "missing-error", spec.errors, frozenset(["P"]), dict(spec.records))["expr"][2] == ["ok", ["none", "U8"]]
    assert fault_row(spec.ops["p"], "field-zero", spec.errors, frozenset(["P"]), dict(spec.records))["requires"] == ["ir.data"]


def test_a_nested_tuple_component_is_always_built_aware_or_not():
    # 8.2, R31(a): the refusal "a component of this type has no zero literal" is unreachable for a tuple
    spec = lower_document(parse_source(b"warrant t v1\n  error E = A\n  op f(n: Nat) -> Result<((Bool,Bool),Nat),E>\n"))
    expr = fault_row(spec.ops["f"], "missing-error", spec.errors)["expr"]
    assert expr[2] == ["ok", ["tuple", [["tuple", [FALSE, FALSE]], ["var", "n"]]]]


def test_the_kit_instantiates_a_data_fault_with_the_cases_records():
    from coding_adventures_warrant_oracle.conformance import instantiate_fault

    types = {"P": {"kind": "record", "fields": [["x", "U8"], ["y", "Bool"]]}, "E": {"kind": "error", "cases": ["A"]}}
    op = {"params": [["a", "U8"]], "ret": "Result<P,E>"}
    row = instantiate_fault("field-zero", "f", op, types)
    assert row["requires"] == ["ir.data", "ir.tuple"] and row["expr"][0] == "match"
    assert instantiate_fault("missing-error", "f", op, types)["expr"][2] == [
        "ok",
        ["ctor", "P", [["var", "a"], FALSE]],
    ]


@pytest.mark.parametrize(
    "decl,payload,requires",
    [
        ("op f(a: U8) -> Result<Option<U8>,E>", ["none", "U8"], ["ir.data"]),
        ("op f(o: Option<U8>) -> Result<Option<U8>,E>", ["var", "o"], ["ir.core", "ir.data"]),
        ("op f(a: U8, o: Option<U8>) -> Result<Option<(Bool,Nat)>,E>", ["none", "(Bool,Nat)"], ["ir.data", "ir.tuple"]),
        ("op f(s: Shape) -> Result<Shape,E>", ["var", "s"], ["ir.core", "ir.data"]),
        ("op f(n: Nat) -> Result<Point,E>", ["ctor", "Point", [U8_0, U8_0]], ["ir.data"]),
        ("op f(a: U8, y: U8) -> Result<Point,E>", ["ctor", "Point", [["var", "a"], ["var", "a"]]], ["ir.data"]),
        ("op f(n: Nat) -> Result<Pair,E>", ["ctor", "Pair", [FALSE, ["var", "n"]]], ["ir.data"]),
        ("op f(o: Option<U8>) -> Result<Slot,E>", ["ctor", "Slot", [["var", "o"], ["tuple", [FALSE, ["lit", {"type": "Nat", "value": "0"}]]]]], ["ir.data", "ir.tuple"]),
        ("op f(a: U8) -> Result<Option<Option<Bool>>,E>", ["none", "Option<Bool>"], ["ir.data"]),
        # a tuple payload now builds a nested tuple (a nested tuple had no template before S11c-1)
        ("op f(n: Nat) -> Result<((Bool,Bool),Nat),E>", ["tuple", [["tuple", [FALSE, FALSE]], ["var", "n"]]], ["ir.tuple"]),
    ],
)
def test_missing_error_builds_records_and_takes_leaves_by_exact_type(decl, payload, requires):
    spec = lower_document(parse_source(f"warrant t v1\n{DECLS}  {decl}\n".encode()))
    r = fault_row(spec.ops["f"], "missing-error", spec.errors, frozenset([*spec.records, *spec.variants]), dict(spec.records))
    assert r["kind"] == "replace" and r["requires"] == requires
    assert r["expr"] == ["if", ["is_err", R], ["ok", payload], R]


@pytest.mark.parametrize(
    "decl",
    [
        "op f(a: U8) -> Result<Shape,E>",  # a variant has no zero and no parameter of its type
        "op f(a: U8) -> Result<Result<U8,E>,E>",
        "op f(a: U8) -> Point",
    ],
)
def test_missing_error_refuses_a_variant_with_no_parameter_and_what_is_not_a_result(decl):
    spec = lower_document(parse_source(f"warrant t v1\n{DECLS}  {decl}\n".encode()))
    with pytest.raises(OracleError) as err:
        fault_row(spec.ops["f"], "missing-error", spec.errors, frozenset([*spec.records, *spec.variants]), dict(spec.records))
    assert err.value.message.startswith("fault:")


def test_a_variant_field_of_a_record_and_a_named_width_inside_an_option_have_no_zero():
    spec = lower_document(parse_source(b"warrant t v1\n  error E = A\n  variant K = P | Q\n  record W(k: K)\n  op f(a: U8) -> Result<W,E>\n  op g<N: Nat>(a: U8) -> Result<Option<Bytes<N>>,E>\n  op h(k: K) -> Result<W,E>\n"))
    data, records = frozenset(["K", "W"]), dict(spec.records)
    with pytest.raises(OracleError, match="missing-error has no template for this payload type"):
        fault_row(spec.ops["f"], "missing-error", spec.errors, data, records)
    with pytest.raises(OracleError, match="named width"):
        fault_row(spec.ops["g"], "missing-error", spec.errors, data, records)
    assert fault_row(spec.ops["h"], "missing-error", spec.errors, data, records)["expr"][2] == ["ok", ["ctor", "W", [["var", "k"]]]]


def test_wrong_ctor_is_not_in_the_registry_and_field_zero_joins_it():
    # [D49]: wrong-ctor waits for its first adopter; the registry has 18 names with field-zero (R37)
    assert "wrong-ctor" not in REGISTRY and "field-zero" in REGISTRY and len(REGISTRY) == 18
    spec = lower_document(parse_source(b"warrant t v1\n  record P(x: U8)\n  op f(a: U8) -> P\n"))
    with pytest.raises(OracleError, match="unknown fault name"):
        fault_row(spec.ops["f"], "wrong-ctor", spec.errors, frozenset(["P"]), dict(spec.records))


# -- R67 and R94: the first comparison of a fault row's type with a declared type ---------------------------------


def _typed_row(decl: str, expr: list, fault: str = "missing-error"):
    spec = lower_document(parse_source(f"warrant t v1\n  error E = A\n  record Holder(d: Bytes, pair: (Bytes,Bool))\n  record Pair2(p: (Bytes<2>,Bool))\n  {decl}\n".encode()))
    prog = build_program(spec)
    row = {"fault": fault, "op": "f", "kind": "replace", "requires": [], "expr": expr}
    return _compile_fault(prog, spec.ops["f"], row, {})


def lit_bytes(hex_: str) -> list:
    return ["lit", {"type": "Bytes", "value": hex_}]


def test_r67_a_missing_error_on_a_plain_bytes_tuple_payload_with_no_bytes_parameter_compiles():
    # first test of R67: the zero literal of a component declared plain ``Bytes`` has the type ``Bytes``
    spec = lower_document(parse_source(b"warrant t v1\n  error E = A\n  op f(a: Nat) -> Result<(Bytes,Bool),E>\n"))
    row = fault_row(spec.ops["f"], "missing-error", spec.errors, frozenset(), {})
    assert row["expr"][2] == ["ok", ["tuple", [lit_bytes(""), FALSE]]]
    assert callable(_compile_fault(build_program(spec), spec.ops["f"], row, {}))


def test_r67_the_same_tree_with_a_one_byte_literal_in_a_bytes_2_component_is_refused():
    # second test: the literal is a ``Bytes<1>`` where the declared component is ``Bytes<2>`` and a tuple is invariant
    bad = ok_if(["tuple", [lit_bytes("01"), FALSE]])
    with pytest.raises(OracleError, match="no common type"):
        _typed_row("op f(a: Nat) -> Result<(Bytes<2>,Bool),E>", bad)
    assert callable(_typed_row("op f(a: Nat) -> Result<(Bytes<2>,Bool),E>", ok_if(["tuple", [lit_bytes("0102"), FALSE]])))


def test_r67_a_missing_error_on_a_record_with_a_bytes_bool_field_and_no_bytes_parameter_compiles():
    # third test: the ``ctor`` argument that is a tuple is checked against its field by the existing ctor typing
    spec = lower_document(parse_source(b"warrant t v1\n  error E = A\n  record R(d: Bytes, pair: (Bytes,Bool))\n  op f(a: Nat) -> Result<R,E>\n"))
    data, records = frozenset(["R"]), dict(spec.records)
    row = fault_row(spec.ops["f"], "missing-error", spec.errors, data, records)
    assert row["expr"][2] == ["ok", ["ctor", "R", [lit_bytes(""), ["tuple", [lit_bytes(""), FALSE]]]]]
    assert callable(_compile_fault(build_program(spec), spec.ops["f"], row, {}))
    # ... and a one-byte literal in a (Bytes<2>,Bool) field is refused by the ctor typing, whatever the ``if`` does
    with pytest.raises(OracleError):
        _typed_row("op f(a: Nat) -> Result<Pair2,E>", ok_if(["ctor", "Pair2", [["tuple", [lit_bytes("01"), FALSE]]]]))


def test_a_pre_rewrite_is_typed_against_its_parameter():
    spec = lower_document(parse_source(b"warrant t v1\n  op f(a: U8, d: Bytes<2>) -> U8\n"))
    prog = build_program(spec)
    ok = {"fault": "x", "op": "f", "kind": "pre", "requires": [], "rewrites": [{"param": "a", "expr": ["add", ["var", "a"], ["lit", {"type": "U8", "value": "1"}]]}]}
    assert _compile_fault(prog, spec.ops["f"], ok, {})
    bad = {**ok, "rewrites": [{"param": "d", "expr": ["lit", True]}]}  # a Bool for a Bytes<2> parameter
    with pytest.raises(OracleError, match="no common type"):
        _compile_fault(prog, spec.ops["f"], bad, {})
    nameless = {**ok, "rewrites": [{"param": "nowhere", "expr": ["lit", True]}]}  # nothing declared to compare with
    assert _compile_fault(prog, spec.ops["f"], nameless, {})
    wider = {**ok, "rewrites": [{"param": "d", "expr": ["take", ["var", "d"], ["lit", {"type": "Nat", "value": "1"}]]}]}
    assert _compile_fault(prog, spec.ops["f"], wider, {})  # plain Bytes against Bytes<2>: a common type, not "flows to"


def test_r94_compares_every_branch_of_an_if_with_the_declared_type_not_only_the_first():
    # a review probe: the else branch ``ok(true)`` is a Result<Bool,?> where Result<U8,E> is declared; ``_merge_result``
    # once kept the first branch (``result``) whole and the whole passed
    bad = ["if", ["is_err", R], R, ["ok", ["lit", True]]]
    with pytest.raises(OracleError, match="no common type"):
        _typed_row("op f(a: Nat) -> Result<U8,E>", bad)
    # the same shape with the right payload, in either branch, is accepted
    assert callable(_typed_row("op f(a: Nat) -> Result<U8,E>", ["if", ["is_err", R], R, ["ok", U8_0]]))
    assert callable(_typed_row("op f(a: Nat) -> Result<U8,E>", ["if", ["is_err", R], ["ok", U8_0], R]))
    with pytest.raises(OracleError, match="no common type"):
        _typed_row("op f(a: Nat) -> Result<U8,E>", ["if", ["is_err", R], ["ok", ["lit", True]], R])


def test_r94_compares_every_arm_of_a_match_with_the_declared_type_not_only_the_first():
    def arms(second):
        return ["match", R, [[["ok", ["bind", "p"]], ["ok", ["var", "p"]]], [["err_any"], second]]]

    with pytest.raises(OracleError, match="no common type"):
        _typed_row("op f(a: Nat) -> Result<U8,E>", arms(["ok", ["lit", True]]))
    assert callable(_typed_row("op f(a: Nat) -> Result<U8,E>", arms(R)))
    # a nested if inside an arm inherits the declared type too
    nested = arms(["if", ["is_err", R], R, ["ok", ["lit", True]]])
    with pytest.raises(OracleError, match="no common type"):
        _typed_row("op f(a: Nat) -> Result<U8,E>", nested)


def test_r98a_an_if_or_match_in_an_operand_or_condition_is_typed_by_r51_alone():
    # only the value spine (the branches of an if, the arms of a match, the payload of an ok, a tuple component, a ctor
    # argument) is compared with the declared type; the inner ``if`` below has Bool branches inside a Bytes result
    inner = ["if", ["lit", True], ["lit", True], ["lit", False]]
    assert callable(_typed_row("op f(a: Nat) -> Bytes", ["if", inner, R, R]))
    n1 = ["lit", {"type": "Nat", "value": "1"}]
    nat_if = ["if", ["lit", True], n1, n1]
    assert callable(_typed_row("op f(a: Nat) -> Bytes", ["take", R, nat_if]))
    # ... while the same inner if as a *branch* is compared
    with pytest.raises(OracleError, match="no common type"):
        _typed_row("op f(a: Nat) -> Bytes", ["if", ["lit", True], inner, R])


def test_the_merge_of_two_result_types_takes_each_part_from_whichever_knows_it():
    from coding_adventures_warrant_oracle.ir_eval import _merge_result

    q, e1 = A.Ty("?"), A.Ty("E1")
    res = lambda p, e: A.Ty("Result", (p, e))  # noqa: E731
    pair = ty("(Bytes,Bool)")
    # 8.2: "the common type takes the other arm's error type" (the typing dump read Result<(Bytes,Bool),?>)
    assert _merge_result(res(pair, q), res(pair, e1)) == res(pair, e1)
    assert _merge_result(res(pair, e1), res(pair, q)) == res(pair, e1)
    assert _merge_result(res(q, q), res(pair, e1)) == res(pair, e1)


@pytest.mark.parametrize(
    "a,b,expected",
    [
        ("Bytes", "Bytes<32>", "Bytes"),
        ("Nat", "Int", "Int"),
        ("Option<Nat>", "Option<Int>", "Option<Int>"),
        ("Bool", "Bytes", None),
        ("Bytes", "U8", None),
        ("(Bytes,Bool)", "(Bytes<2>,Bool)", None),  # a tuple is invariant (R51), R67's second test
        ("(U64,Int)", "(U64,Nat)", "(U64,Int)"),  # the one component widening 8.2 states for off-by-one-consumed
        ("Option<(Bytes,Bool)>", "Option<(Bytes<2>,Bool)>", None),
        ("Option<Nat>", "Bool", None),
        ("Option<Bytes<1>>", "Option<Bytes<3>>", "Option<Bytes>"),  # R94 at the top level ...
        ("((Bytes<1>,Bool),Nat)", "((Bytes<3>,Bool),Nat)", None),  # ... but nothing lifts below a tuple, at any depth (R96)
        ("(Option<Bytes<1>>,Bool)", "(Option<Bytes<3>>,Bool)", None),
    ],
)
def test_the_common_type_rule_of_r94(a, b, expected):
    got = _fault_unify(ty(a), ty(b))
    assert (None if got is None else str(got)) == expected


def test_the_common_type_rule_merges_the_unknown_error_type_of_a_bare_ok():
    q = A.Ty("?")
    res = lambda p, e: A.Ty("Result", (p, e))  # noqa: E731
    u8, e1, e2 = A.Ty("U8"), A.Ty("E1"), A.Ty("E2")
    assert _fault_unify(res(u8, q), res(u8, e1)) == res(u8, e1)
    assert _fault_unify(res(q, q), res(u8, e1)) == res(u8, e1)
    assert _fault_unify(res(u8, e1), res(u8, e2)) is None
    assert _fault_unify(res(ty("(U8,Nat)"), q), res(ty("(U8,Int)"), e1)) == res(ty("(U8,Int)"), e1)
    assert _fault_unify(res(ty("(U8,Bool)"), q), res(ty("(U8,Nat)"), e1)) is None
    assert _fault_unify(res(ty("(U8,Bool)"), q), res(ty("(U8,Bool,Bool)"), e1)) is None
    assert _fault_unify(ty("Option<(Bool,Nat)>"), ty("Option<(Bool,Int)>")) == ty("Option<(Bool,Int)>")


def test_the_tuple_component_rule_looks_inside_an_option_and_a_result_but_lifts_only_nat_to_int():
    q, e1, e2 = A.Ty("?"), A.Ty("E1"), A.Ty("E2")
    res = lambda p, e: A.Ty("Result", (p, e))  # noqa: E731
    pair = lambda *c: A.Ty("()", tuple(c))  # noqa: E731
    opt = lambda t: A.Ty("Option", (t,))  # noqa: E731
    nat, int_, u8, boolean = A.Ty("Nat"), A.Ty("Int"), A.Ty("U8"), A.Ty("Bool")
    # an Option component unifies by its payload under the same component rule
    assert _fault_unify(pair(opt(nat), boolean), pair(opt(int_), boolean)) == pair(opt(int_), boolean)
    assert _fault_unify(pair(opt(boolean), boolean), pair(opt(nat), boolean)) is None
    # a Result component merges the unknown error type and its unknown payload, and keeps one known error type
    assert _fault_unify(pair(res(u8, q), boolean), pair(res(u8, e1), boolean)) == pair(res(u8, e1), boolean)
    assert _fault_unify(pair(res(q, e1), boolean), pair(res(u8, e1), boolean)) == pair(res(u8, e1), boolean)
    assert _fault_unify(pair(res(u8, e1), boolean), pair(res(q, q), boolean)) == pair(res(u8, e1), boolean)
    assert _fault_unify(pair(res(u8, e1), boolean), pair(res(u8, e2), boolean)) is None
    assert _fault_unify(pair(res(boolean, q), boolean), pair(res(u8, e1), boolean)) is None
    # a nested tuple component, a component of unrelated types, and a pair of different arities
    assert _fault_unify(pair(pair(nat, nat), boolean), pair(pair(int_, nat), boolean)) == pair(pair(int_, nat), boolean)
    assert _fault_unify(pair(boolean, nat), pair(A.Ty("Bytes"), nat)) is None
    assert _fault_unify(pair(boolean, nat), pair(boolean, nat, nat)) is None


def test_every_template_of_the_registry_types_against_the_declared_result_it_is_written_for():
    # derive_controls types every row once, even when no claim can stand in for the subject: here the only claim has a
    # computed argument (unproven), so only the row-level check ever compiles the fault.
    src = (
        b"warrant t v1\n  error E = A | B\n  record Point(x: U8, y: U8)\n  op f(a: U8) -> Point\n"
        b"  claim c:\n    forall a: U8. f(a + 1u8) == Point(a, a)\n"
    )
    prof = b"fault f field-zero\n"
    corpus = derive(src, prof, spec_ref="t.warrant", generator=GEN).corpus
    assert corpus["controls"][0]["slim_witness"] == [{"claim": "warrant/t/exhaustive/c", "unproven": True}]
    assert corpus["faults"][0]["requires"] == ["ir.data"]


def test_an_untypable_row_is_refused_even_when_every_candidate_is_unproven(monkeypatch):
    # a template whose type has no common type with the declared one is a generation error at the row, not at a search
    from coding_adventures_warrant_oracle import faults

    monkeypatch.setitem(faults.REGISTRY, "invert", lambda op, errors: ("post", [faults.CORE], ["lit", True]))
    src = b"warrant t v1\n  op f(a: U8) -> Bytes\n  claim c:\n    forall a: U8. f(a + 1u8) == b\"\"\n"
    with pytest.raises(OracleError, match="no common type"):
        derive(src, b"fault f invert\n", spec_ref="t.warrant", generator=GEN)


# -- the gate: opened here, no flip ----------------------------------------------------------------------------------

DATA_DOC = b"""warrant d v1
  error E = A | B
  record Point(x: U8, y: U8)
  record Sum(sum: Bytes, carry: U8, over: Bool)
  variant Shape = Circle(r: U8) | Dot
  op mk(a: U8, b: U8) -> Point
  op tri(d: U8, e: U8) -> Result<Option<U8>,E>
  op add(a: Bytes, b: Bytes) -> Result<Sum,E>
  op shp(s: Shape) -> Shape
  model m_mk(a: U8, b: U8) -> Point = Point(a, b)
  model m_tri(d: U8, e: U8) -> Result<Option<U8>,E> = if e > 1u8 then Err(E.A) else Ok(if e == 1u8 then Some(d) else None)
  model m_add(a: Bytes, b: Bytes) -> Result<Sum,E> = if len(a) == len(b) then Ok(Sum(a, 3u8, true)) else Err(E.B)
  claim c_mk:
    forall a: U8, b: U8. mk(a, b) == m_mk(a, b)
  claim c_tri:
    forall d: U8, e: U8. tri(d, e) == m_tri(d, e)
  claim c_add:
    forall a: Bytes, b: Bytes. add(a, b) == m_add(a, b)
  claim c_shp:
    forall s: Shape. shp(s) == s
  example e_mk:
    mk(1u8, 2u8) == Point(1u8, 2u8)
  example e_tri:
    tri(7u8, 1u8) == Ok(Some(7u8))
  example e_add:
    add(b"01", b"02") == Ok(Sum(b"01", 3u8, true))
"""
DATA_PROFILE = (
    b"chunk_size 1024\nfault mk field-zero\nfault tri missing-error\nfault add missing-error\nfault add field-zero\n"
)


def write_corpus(tmp_path: pathlib.Path, source: bytes = DATA_DOC, profile: bytes = DATA_PROFILE, **kw):
    out = derive(source, profile, spec_ref="d.warrant", generator=GEN, **kw)
    root = tmp_path / "corpus"
    (root / "chunks").mkdir(parents=True)
    (root / "corpus.json").write_bytes(canonical(out.corpus))
    (root / "cases.tfv").write_bytes(out.tfv)
    for name, text in out.chunk_files.items():
        (root / "chunks" / name).write_bytes(text)
    warrant, prof = tmp_path / "d.warrant", tmp_path / "corpus.profile"
    warrant.write_bytes(source)
    write_lf(prof, profile.decode())
    return root, warrant, prof, out


def verify(tmp_path, root, warrant, prof):
    return verify_v2(root, warrant, prof, repo_root=tmp_path)[0]


def test_the_shipped_gate_opens_ir_data_and_nothing_else_does():
    assert IMPLEMENTED == ("ir.core", "ir.fixed", "ir.tuple", "ir.data") and DATA_LEVEL == "ir.data"
    default = derive(DATA_DOC, DATA_PROFILE, spec_ref="d.warrant", generator=GEN)
    opened = derive(DATA_DOC, DATA_PROFILE, spec_ref="d.warrant", generator=GEN, implemented=IMPLEMENTED)
    closed = derive(DATA_DOC, DATA_PROFILE, spec_ref="d.warrant", generator=GEN, implemented=CLOSED)
    assert default.corpus == opened.corpus
    for control in closed.corpus["controls"]:
        assert all(w.get("deferred") == "ir.data" or w.get("unproven") for w in control["slim_witness"]), control["id"]
    for control in opened.corpus["controls"]:
        assert all("chunk" in w for w in control["slim_witness"]), control["id"]
    # the gate moves the controls and the slim sets only
    for key in ("suites", "faults", "examples", "ops", "models", "levels", "types"):
        if key != "suites":
            assert closed.corpus[key] == opened.corpus[key], key
    assert closed.chunk_files == opened.chunk_files and closed.tfv == opened.tfv


def test_a_synthetic_data_document_derives_and_verify_v2_accepts_it(tmp_path):
    root, warrant, prof, out = write_corpus(tmp_path)
    corpus = out.corpus
    assert corpus["levels"] == ["ir.core", "ir.data", "ir.tuple"]
    rows = {s["claim"]: s for s in corpus["suites"]}
    assert rows["c_tri"]["stream"] == "warrant-stream-v1"  # U8, U8 and Result<Option<U8>,E>: the v1 type set
    assert rows["c_mk"]["stream"] == rows["c_add"]["stream"] == rows["c_shp"]["stream"] == "warrant-stream-v2"
    assert rows["c_shp"]["binders"][0][1]["domain"]["d"] == "variant"
    assert corpus["types"]["Sum"] == {"kind": "record", "fields": [["sum", "Bytes"], ["carry", "U8"], ["over", "Bool"]]}
    assert [e["expected"] for e in corpus["examples"]] == [["1", "2"], {"ok": {"some": "7"}}, {"ok": [ "01", "3", True]}]
    assert out.tfv.decode().splitlines()[1].endswith("Kexpect L2 I1 I2")
    report = verify(tmp_path, root, warrant, prof)
    assert report.ok, report.mismatches


@pytest.mark.parametrize(
    "name,change",
    [
        (
            "a stale deferred ir.data witness where the open gate says chunk",
            lambda c: c["controls"][0]["slim_witness"].__setitem__(
                0, {"claim": c["controls"][0]["slim_witness"][0]["claim"], "deferred": "ir.data"}
            ),
        ),
        (
            "every ir.data control deferred, as a corpus derived before the gate opened",
            lambda c: [
                w.update({"deferred": "ir.data"}) or w.pop("chunk")
                for ctl in c["controls"]
                for w in ctl["slim_witness"]
                if "chunk" in w
            ],
        ),
        ("an altered field-zero expression", lambda c: next(f for f in c["faults"] if f["fault"] == "field-zero").update({"expr": ["var", "result"]})),
        ("a wrong requires on a data fault", lambda c: c["faults"][0].update({"requires": ["ir.tuple"]})),
        ("a wrong suite requires", lambda c: c["suites"][0].update({"requires": ["ir.core"]})),
        ("a wrong record descriptor", lambda c: c["suites"][0]["binders"][0][1].update({"domain": {"d": "bool", "size": 2}})),
        ("a wrong types row", lambda c: c["types"]["Sum"]["fields"].reverse()),
        ("a wrong digest", lambda c: c["suites"][1].update({"inputs_sha256": "0" * 64})),
        ("a wrong example value", lambda c: c["examples"][1].update({"expected": {"ok": {"none": True}}})),
    ],
)
def test_a_mutated_data_corpus_fails_verify_v2(tmp_path, name, change):
    root, warrant, prof, _ = write_corpus(tmp_path)
    corpus = json.loads((root / "corpus.json").read_bytes())
    change(corpus)
    (root / "corpus.json").write_bytes(canonical(corpus))
    assert not verify(tmp_path, root, warrant, prof).ok, name


def test_a_closed_gate_data_corpus_fails_verify_v2_under_the_shipped_gate(tmp_path):
    # the S10c pattern: a corpus derived under the gate before this slice says ``deferred`` for an ir.data control,
    # which the shipped (open) gate refutes; the entry ``{"deferred": "ir.data"}`` is rejected
    root, warrant, prof, _ = write_corpus(tmp_path)
    closed = derive(DATA_DOC, DATA_PROFILE, spec_ref="d.warrant", generator=GEN, implemented=CLOSED)
    assert any(w.get("deferred") == "ir.data" for c in closed.corpus["controls"] for w in c["slim_witness"])
    (root / "corpus.json").write_bytes(canonical(closed.corpus))
    assert not verify(tmp_path, root, warrant, prof).ok


def test_the_cli_emits_and_verifies_a_data_document(tmp_path, capsys):
    doc, profile = tmp_path / "d.warrant", tmp_path / "corpus.profile"
    doc.write_bytes(DATA_DOC)
    write_lf(profile, DATA_PROFILE.decode())
    out = tmp_path / "o.json"
    assert run(["emit-v2", "--spec", str(doc), "--profile", str(profile), "--out", str(out), "--depth", "full"]) == 0
    tree = json.loads(out.read_text())
    assert tree["levels_implemented"] == ["ir.core", "ir.data", "ir.fixed", "ir.tuple"]
    assert tree["summary"] == {"ok": 4, "not-live": 0, "refused": 0, "not_live_by_level": {}}
    assert tree["suites"]["c_shp"]["descriptors"][0]["d"] == "variant"
    assert tree["suites"]["c_tri"]["stream_header"] == "warrant-stream-v1"
    (tmp_path / "w").mkdir()
    root, warrant, prof, _ = write_corpus(tmp_path / "w")
    capsys.readouterr()
    argv = ["verify-v2", str(root), "--warrant", str(warrant), "--profile", str(prof), "--repo-root", str(tmp_path / "w")]
    assert run(argv) == 0
    assert "verify-v2: ok" in capsys.readouterr().out


def test_a_document_that_needs_ir_data_is_no_longer_refused_at_evaluation_and_a_list_still_is():
    spec = lower_document(parse_source(DATA_DOC))
    assert all(m.unsupported is None for m in spec.models.values())
    assert all(o.unsupported is None for o in spec.ops.values())
    assert all(s.unsupported is None and s.lowered for s in spec.suites)
    assert all(x.unsupported is None for x in spec.examples)
    listy = b"warrant t v1\n  op f(a: U8) -> List<U8>\n  claim c:\n    forall a: U8. f(a) == f(a)\n"
    with pytest.raises(OracleError) as err:
        derive(listy, b"", spec_ref="t.warrant", generator=GEN)
    assert err.value.level == "ir.list"


def test_the_gate_reaches_derive_controls_only_through_a_library_parameter():
    spec = lower_document(parse_source(DATA_DOC))
    prog = build_program(spec)
    from coding_adventures_warrant_oracle.suites import SuitePlan

    plans = {s.claim: SuitePlan(spec, prog, s, chunk_size=1024) for s in spec.suites}
    faults = [("mk", "field-zero")]
    closed = derive_controls(spec, prog, plans, faults, 1024, implemented=CLOSED)[1]
    opened = derive_controls(spec, prog, plans, faults, 1024)[1]
    assert closed[0]["slim_witness"] == [{"claim": "warrant/d/exhaustive/c_mk", "deferred": "ir.data"}]
    assert opened[0]["slim_witness"] == [{"claim": "warrant/d/exhaustive/c_mk", "chunk": 0}]
    assert closed[0]["requires"] == opened[0]["requires"] == ["ir.data"]
