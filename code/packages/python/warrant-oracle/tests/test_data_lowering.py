"""Slice S11a: lowering, typing, ``requires`` and cost of ``ir.data`` (VER04r 16.14, Appendix A.2).

What is pinned here, beside the committed ``data-forms-v1`` dumps (which are the parity contract with the Rust
generator's slice S11b and are checked byte for byte by ``test_data_forms_dump.py``):

* the four tags ``some none ctor field`` and their exact JSON, including the **typed** ``["none", "T"]``;
* the typing rules of R39 (``Some`` is typed exactly, a context-less ``None`` is a type error, ``e.i`` on a
  non-tuple and ``e.name`` on a non-record are type errors);
* ``match`` on any scrutinee with the data patterns, the level of each ``match`` and the usefulness check over
  ``Bool``, ``Option``, records and variants;
* the intermediate use of ``unwrap_or`` that adds no ``ir.data`` (R29);
* the refusals ``ir.list``, ``ir.seq``, ``err-value``, ``fixed-cast`` (R27, R28), the corpus limits (R34) and the
  refusal order (R35);
* the typed IR compiler's side (``Program(analyze_data=True)``), which re-types and costs every data tag, and
  its refusal to *evaluate* any of them.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle import irtree
from coding_adventures_warrant_oracle import levels as L
from coding_adventures_warrant_oracle import patterns as P
from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.emit import census, emit
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.ir_eval import Program
from coding_adventures_warrant_oracle.lower import lower_document, lowering_refusal
from coding_adventures_warrant_oracle.lowering_dump import dump, view
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.suites import SuitePlan
from coding_adventures_warrant_oracle.typenames import parse_type_name

PRELUDE = (
    "warrant t v1\n"
    "  error E = A | B | C\n"
    "  record Point(x: U8, y: U8)\n"
    "  record Slot(o: Option<U8>, t: (Bool,Nat))\n"
    "  variant Shape = Circle(r: U8) | Rect(w: U8, h: U8) | Dot\n"
    "  variant Mode = Add | Sub | Neg\n"
    "  op f(a: U8) -> U8\n"
)
U8 = A.Ty("U8")
OPT_U8 = A.Ty("Option", (U8,))


def lower(text: str):
    return lower_document(parse_source(text))


def model(body: str, params: str = "a: U8", ret: str = "Bool", extra: str = ""):
    """The lowered model ``m`` of the prelude plus ``model m(params) -> ret = body``."""
    return lower(PRELUDE + extra + f"  model m({params}) -> {ret} = {body}\n").models["m"]


def ir_of(body: str, params: str = "a: U8", ret: str = "Bool", extra: str = ""):
    m = model(body, params, ret, extra)
    assert m.lowered and m.ir is not None, m.unsupported
    return m.ir


def refused(body: str, params: str = "a: U8", ret: str = "Bool", extra: str = "") -> OracleError:
    with pytest.raises(OracleError) as err:
        model(body, params, ret, extra)
    return err.value


def lit(ty: str, value: object) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


# -- the four tags and their exact JSON ----------------------------------------------------------------


def test_some_none_ctor_and_field_have_the_shapes_of_appendix_a2():
    assert ir_of("Some(a)", ret="Option<U8>") == ["some", ["var", "a"]]
    assert ir_of("None", ret="Option<U8>") == ["none", "U8"]
    assert ir_of("Point(a, 1u8)", ret="Point") == ["ctor", "Point", [["var", "a"], lit("U8", 1)]]
    assert ir_of("Circle(a)", ret="Shape") == ["ctor", "Circle", [["var", "a"]]]
    assert ir_of("Dot", ret="Shape") == ["ctor", "Dot", []]  # the bare name of a payload-free constructor
    assert ir_of("p.y", "p: Point", "U8") == ["field", ["var", "p"], 1]  # the index, never the name
    assert ir_of("s.t.0", "s: Slot", "Bool") == ["project", ["field", ["var", "s"], 1], 0]


@pytest.mark.parametrize(
    "ret,name",
    [
        ("Option<Bytes<2>>", "Bytes<2>"),
        ("Option<Bytes>", "Bytes"),
        ("Option<(U8,Bool)>", "(U8,Bool)"),
        ("Option<Option<U8>>", "Option<U8>"),
        ("Option<Point>", "Point"),
        ("Option<Mode>", "Mode"),
        ("Option<Nat>", "Nat"),
        ("Option<Result<U8,E>>", "Result<U8,E>"),
    ],
)
def test_a_none_carries_the_canonical_name_of_the_payload_type_its_context_fixes(ret, name):
    assert ir_of("None", ret=ret) == ["none", name]


def test_a_none_takes_its_type_from_every_kind_of_context():
    # an if branch either way round, an Ok payload, a constructor argument, a model argument, a comparison
    assert ir_of("if a == 0u8 then Some(a) else None", ret="Option<U8>")[3] == ["none", "U8"]
    assert ir_of("if a == 0u8 then None else Some(a)", ret="Option<U8>")[2] == ["none", "U8"]
    assert ir_of("Ok(None)", ret="Result<Option<U8>,E>") == ["ok", ["none", "U8"]]
    assert ir_of("Slot(None, (true, 1))", ret="Slot")[2][0] == ["none", "U8"]
    assert ir_of("o == None", "o: Option<U8>")[2] == ["none", "U8"]
    assert ir_of("Some(None)", ret="Option<Option<U8>>") == ["some", ["none", "U8"]]
    ctx = "  model g(o: Option<Bytes<2>>) -> Bool = true\n"
    assert lower(PRELUDE + ctx + "  model m(a: U8) -> Bool = g(None)\n").models["m"].ir == [
        "model_call",
        "g",
        [["none", "Bytes<2>"]],
    ]


def test_a_some_literal_takes_its_type_from_the_context_and_a_none_does_not_guess():
    assert ir_of("Some(7)", ret="Option<Nat>") == ["some", lit("Nat", 7)]
    assert ir_of("Some(7u8)", ret="Option<U8>") == ["some", lit("U8", 7)]
    e = refused("None == None")
    assert e.code == "type" and "context" in e.message  # R39 (b)
    assert refused("match None with None -> true | _ -> false").code == "type"
    assert refused("None").code == "type"


def test_option_flows_and_unifies_by_its_payload_like_result_d60_r51():
    # `Some(d)` is Option<Bytes<2>> and *flows* to Option<Bytes>; Option<Nat> flows to Option<Int>
    assert ir_of("Some(d)", "d: Bytes<2>", "Option<Bytes>") == ["some", ["var", "d"]]
    assert ir_of("Some(d)", "d: Bytes<2>", "Option<Bytes<2>>") == ["some", ["var", "d"]]
    assert ir_of("Some(n)", "n: Nat", "Option<Int>") == ["some", ["var", "n"]]
    assert ir_of("o", "o: Option<Nat>", "Option<Int>") == ["var", "o"]
    assert ir_of("Some(Some(n))", "n: Nat", "Option<Option<Int>>")[0] == "some"  # recursively
    # the converse never flows
    assert refused("Some(n)", "n: Int", "Option<Nat>").code == "type"
    assert refused("o", "o: Option<Bytes>", "Option<Bytes<2>>").code == "type"
    # the operands of ==, the branches of an if and the arms of a match unify by payload
    assert ir_of("a == b", "a: Option<Nat>, b: Option<Int>")[0] == "eq"
    assert ir_of("Some(d) == Some(e)", "d: Bytes<2>, e: Bytes")[0] == "eq"
    assert ir_of("if c then a else b", "c: Bool, a: Option<Nat>, b: Option<Int>", "Option<Int>")[0] == "if"
    assert ir_of("match c with true -> a | false -> b", "c: Bool, a: Option<Nat>, b: Option<Int>", "Option<Int>")[0] == "match"
    # the default of unwrap_or flows to the payload, and a payload that is not an Option does not
    assert ir_of("unwrap_or(o, n)", "o: Option<Int>, n: Nat", "Int")[0] == "unwrap_or"
    assert refused("o", "o: Option<(Nat,Nat)>", "Option<(Int,Int)>").code == "type"  # a tuple never lifts
    # a constructor argument flows per argument: Bytes<N> to Bytes and through an Option, nothing through a tuple
    blob = "  record Blob(data: Bytes, n: Nat, o: Option<Bytes>)\n"
    ok = lower(PRELUDE + blob + "  model m(d: Bytes<2>) -> Blob = Blob(d, 1, Some(d))\n").models["m"]
    assert ok.ir[0] == "ctor"
    rt = "  record RT(p: (Int,Int))\n"
    with pytest.raises(OracleError) as err:
        lower(PRELUDE + rt + "  model m(n: Nat) -> RT = RT((n, n))\n")
    assert err.value.code == "type"
    assert ir_of("Ok(b\"6162\")", "u: Bool", "Result<Bytes<2>,E>")[0] == "ok"


def test_the_width_is_carried_by_the_type_through_some_ok_if_and_match_r51():
    """VER04r 16.16 R51: a bytes literal is ``Bytes<len>``; ``Some``/``Ok`` wrap it; the IR is unchanged."""
    lit2 = ["lit", {"type": "Bytes", "value": "6162"}]  # the IR keeps the written type `Bytes`
    assert ir_of('Some(b"6162")', "u: Bool", "Option<Bytes<2>>") == ["some", lit2]
    assert ir_of('Some(b"6162")', "u: Bool", "Option<Bytes>") == ["some", lit2]
    assert refused('Some(b"6162")', "u: Bool", "Option<Bytes<3>>").code == "type"
    assert ir_of('Ok(b"6162")', "u: Bool", "Result<Bytes<2>,E>") == ["ok", lit2]
    assert refused('Ok(b"6162")', "u: Bool", "Result<Bytes<3>,E>").code == "type"
    # concat of two literal widths is Bytes<sum>; a plain operand gives Bytes
    assert ir_of('Ok(concat(a, b"0102"))', "a: Bytes<1>", "Result<Bytes<3>,E>")[0] == "ok"
    assert refused('Ok(concat(a, b"0102"))', "a: Bytes<1>", "Result<Bytes<4>,E>").code == "type"
    assert refused('Ok(concat(a, b"0102"))', "a: Bytes", "Result<Bytes<3>,E>").code == "type"
    assert ir_of('Some(concat(b"61", b"62"))', "u: Bool", "Option<Bytes<2>>")[0] == "some"
    # take, drop give a plain Bytes
    assert refused("Ok(take(a, 1))", "a: Bytes<1>", "Result<Bytes<1>,E>").code == "type"
    assert ir_of("Ok(take(a, 1))", "a: Bytes<1>", "Result<Bytes,E>")[0] == "ok"
    assert refused("Some(drop(a, 1))", "a: Bytes<2>", "Option<Bytes<1>>").code == "type"
    # an if or a match keeps a width its branches share, and loses it otherwise
    keep = 'Some(if c then b"6162" else b"6364")'
    assert ir_of(keep, "c: Bool", "Option<Bytes<2>>")[0] == "some"
    assert refused(keep, "c: Bool", "Option<Bytes<3>>").code == "type"
    lose = 'Some(if c then b"6162" else b"616263")'
    assert ir_of(lose, "c: Bool", "Option<Bytes>")[0] == "some"
    assert refused(lose, "c: Bool", "Option<Bytes<2>>").code == "type"
    arms = 'Some(match c with true -> b"6162" | false -> b"6364")'
    assert ir_of(arms, "c: Bool", "Option<Bytes<2>>")[0] == "some"
    assert refused('Some(match c with true -> b"6162" | false -> b"61")', "c: Bool", "Option<Bytes<2>>").code == "type"
    # Option and Result are covariant in the width, a tuple is invariant
    assert ir_of('Some(Some(b"6162"))', "u: Bool", "Option<Option<Bytes>>")[0] == "some"
    assert refused('(b"6162", n)', "n: Nat", "(Bytes,Nat)").code == "type"
    assert ir_of('(b"6162", n)', "n: Nat", "(Bytes<2>,Nat)")[0] == "tuple"
    assert refused('Some((b"6162", n))', "n: Nat", "Option<(Bytes,Nat)>").code == "type"
    # a plain Bytes does not flow to Bytes<N>
    assert refused("Some(d)", "d: Bytes", "Option<Bytes<2>>").code == "type"
    # a width the operand type carries makes an equality of Bytes<2> against Bytes legal (they unify to Bytes)
    assert ir_of('d == b"6162"', "d: Bytes")[0] == "eq"


def test_none_takes_its_type_from_its_expected_type_only_r52():
    # right operand of == / != is typed from the left; the left operand has no expected type
    assert ir_of("o == None", "o: Option<U8>")[2] == ["none", "U8"]
    assert ir_of("o != None", "o: Option<U8>")[2] == ["none", "U8"]
    for body in ("None == o", "None != o"):
        e = refused(body, "o: Option<U8>")
        assert e.code == "type" and "context" in e.message
    # a branch or an arm never takes its type from its sibling
    assert refused("unwrap_or(if c then at(d, 0) else None, 0u8)", "c: Bool, d: Bytes", "U8").code == "type"
    assert refused("unwrap_or(if c then None else at(d, 0), 0u8)", "c: Bool, d: Bytes", "U8").code == "type"
    assert refused("unwrap_or(match b with true -> None | false -> at(d, 0), 0u8)", "b: Bool, d: Bytes", "U8").code == "type"
    assert refused("unwrap_or(match b with true -> at(d, 0) | false -> None, 0u8)", "b: Bool, d: Bytes", "U8").code == "type"
    assert refused("unwrap_or(Some(None), None)", "u: Bool", "U8").code == "type"
    # ... but they do from the expected type, which is the written one (under Option<Int>, Some(n) is lifted)
    ir = ir_of("if c then Some(n) else None", "c: Bool, n: Nat", "Option<Int>")
    assert ir[3] == ["none", "Int"]
    assert ir_of("match b with true -> at(d, 0) | false -> None", "b: Bool, d: Bytes", "Option<U8>")[2][1][1] == ["none", "U8"]


def test_projection_typing_r39_c():
    for body, params in (("a.0", "a: Bool"), ("a.x", "a: Bool"), ("t.x", "t: (Bool,Bool)"), ("s.r", "s: Shape")):
        assert refused(body, params, "U8").code == "type", body
    assert refused("p.0", "p: Point", "U8").code == "type"  # an index on a record
    assert refused("p.z", "p: Point", "U8").code == "type"  # an unknown field
    assert refused("t.2", "t: (Bool,Bool)").code == "type"  # out of range
    # a postfix field after a call and a chain through a record and a tuple
    pre = "  model mk(a: U8) -> Point = Point(a, a)\n"
    assert ir_of("mk(a).x", ret="U8", extra=pre) == ["field", ["model_call", "mk", [["var", "a"]]], 0]


# -- structural equality, ordering, unwrap_or -----------------------------------------------------------


def test_equality_is_structural_on_every_data_type_and_ordering_is_a_type_error():
    for ty in ("Option<U8>", "Point", "Shape", "Slot", "Mode"):
        assert ir_of("a == b", f"a: {ty}, b: {ty}")[0] == "eq"
        assert ir_of("a != b", f"a: {ty}, b: {ty}")[0] == "ne"
        assert refused("a < b", f"a: {ty}, b: {ty}").code == "type"
    assert refused("a == b", "a: Point, b: Shape").code == "type"
    assert ir_of("a == Dot", "a: Shape")[2] == ["ctor", "Dot", []]


def test_unwrap_or_takes_any_option_but_only_a_direct_producer_is_the_intermediate_use():
    # R29 (e): any Option-typed first operand
    assert ir_of("unwrap_or(o, 5u8)", "o: Option<U8>", "U8") == ["unwrap_or", ["var", "o"], lit("U8", 5)]
    assert refused("unwrap_or(a, 5u8)").code == "type"
    assert refused("unwrap_or(None, 5u8)", ret="U8").code == "type"  # no context for the None


# -- levels: R9 as extended, R29 -------------------------------------------------------------------------


@pytest.mark.parametrize(
    "body,params,ret,levels",
    [
        # the intermediate use: ir.fixed only, exactly as before slice S11a (R29 (f))
        ("unwrap_or(checked_add(a, b), 0u8)", "a: U8, b: U8", "U8", ("ir.core", "ir.fixed")),
        ("unwrap_or(try<U8>(n), 0u8)", "n: Nat", "U8", ("ir.core", "ir.fixed")),
        ("unwrap_or(at(d, 1), 0u8)", "d: Bytes", "U8", ("ir.core", "ir.fixed")),
        ("unwrap_or(div(a, b), 0u8)", "a: U8, b: U8", "U8", ("ir.core", "ir.fixed")),
        ("unwrap_or(rem_trunc(a, b), 0i8)", "a: I8, b: I8", "I8", ("ir.core", "ir.fixed")),
        # R29 (d): every other Option brings ir.data
        ("unwrap_or(o, 5u8)", "o: Option<U8>", "U8", ("ir.core", "ir.data", "ir.fixed")),
        ("checked_add(a, b) == Some(a)", "a: U8, b: U8", "Bool", ("ir.core", "ir.data", "ir.fixed")),
        ("Ok(rem(a, b))", "a: U8, b: U8", "Result<Option<U8>,E>", ("ir.core", "ir.data", "ir.fixed")),
        (
            "unwrap_or(if c then at(d, 0) else at(d, 1), 0u8)",
            "c: Bool, d: Bytes",
            "U8",
            ("ir.core", "ir.data", "ir.fixed"),
        ),
        ("try<U8>(n)", "n: Nat", "Option<U8>", ("ir.core", "ir.data", "ir.fixed")),
        # types alone: a record, a variant, an Option in a signature
        ("true", "p: Point", "Bool", ("ir.core", "ir.data")),
        ("true", "s: Shape", "Bool", ("ir.core", "ir.data")),
        ("true", "o: Option<(Bool,Bool)>", "Bool", ("ir.core", "ir.data", "ir.tuple")),
        # a match: ir.tuple on a Result, ir.data on anything else, both for a data pattern in a Result
        ("match r with Ok(x) -> x | Err(_) -> false", "r: Result<Bool,E>", "Bool", ("ir.core", "ir.tuple")),
        ("match r with Ok(true) -> 1 | Ok(false) -> 0 | Err(_) -> 2", "r: Result<Bool,E>", "Nat",
         ("ir.core", "ir.data", "ir.tuple")),
        ("match b with true -> 1 | false -> 0", "b: Bool", "Nat", ("ir.core", "ir.data")),
        ("match t with (a, b) -> a", "t: (Bool,Bool)", "Bool", ("ir.core", "ir.data", "ir.tuple")),
        ("match s with Dot -> 1 | _ -> 0", "s: Shape", "Nat", ("ir.core", "ir.data")),
    ],
)
def test_requires_by_tag_pattern_and_type(body, params, ret, levels):
    m = model(body, params, ret)
    assert m.lowered and m.requires == levels, (body, m.requires)
    # S11c-1: a unit that needs ir.data is evaluated, so it carries no evaluation refusal (S11a gave it
    # ``level ir.data`` exactly when ``ir.data`` was required)
    assert m.unsupported is None


def test_a_suite_and_an_example_inherit_data_through_the_call_closure_and_are_evaluated():
    pre = "  model g(o: Option<U8>) -> Bool = match o with Some(_) -> true | None -> false\n"
    spec = lower(
        PRELUDE
        + pre
        + "  claim c:\n    forall a: U8. g(None) == (f(a) == a)\n"
        + "  example e:\n    g(Some(1u8)) == (f(1u8) == 1u8)\n"
    )
    assert spec.suites[0].requires == ("ir.core", "ir.data") and spec.suites[0].lowered
    assert spec.suites[0].unsupported is None  # S11a: level ir.data at evaluation; S11c-1 evaluates
    assert spec.examples[0].requires == ("ir.core", "ir.data") and spec.examples[0].lowered
    assert spec.examples[0].unsupported is None
    assert lowering_refusal(spec) is None  # nothing was refused at lowering


def test_an_operation_with_a_data_signature_and_no_refinement_needs_ir_data_and_is_evaluated():
    spec = lower(PRELUDE + "  op g(p: Point) -> Option<U8>\n  op h(a: U8) -> U8\n")
    assert spec.ops["g"].requires == ("ir.core", "ir.data") and spec.ops["g"].unsupported is None
    assert spec.ops["h"].requires == ("ir.core",) and spec.ops["h"].unsupported is None
    assert spec.ops["g"].lowered


def test_an_operation_refinement_with_a_data_match_is_lowered():
    spec = lower(PRELUDE + "  op g(o: Option<U8> | match o with Some(_) -> true | None -> false) -> U8\n")
    op = spec.ops["g"]
    assert op.admit == ["match", ["var", "o"], [[["some", ["wild"]], ["lit", True]], [["none"], ["lit", False]]]]
    assert op.requires == ("ir.core", "ir.data") and op.lowered


# -- match: patterns ---------------------------------------------------------------------------------------


def test_every_data_pattern_lowers_to_its_ir_form():
    arms = irtree.match_arms(
        ir_of(
            "match s with Circle(r) -> r | Rect(w, _) -> w | Dot -> 0u8",
            "s: Shape",
            "U8",
        )
    )
    assert [p for p, _ in arms] == [
        ["ctor", "Circle", [["bind", "r"]]],
        ["ctor", "Rect", [["bind", "w"], ["wild"]]],
        ["ctor", "Dot", []],
    ]
    arms = irtree.match_arms(ir_of("match o with Some(Some(x)) -> 1 | Some(None) -> 2 | None -> 3", "o: Option<Option<U8>>", "Nat"))
    assert [p for p, _ in arms] == [["some", ["some", ["bind", "x"]]], ["some", ["none"]], ["none"]]
    arms = irtree.match_arms(ir_of("match b with true -> 1 | false -> 0", "b: Bool", "Nat"))
    assert [p for p, _ in arms] == [["bool", True], ["bool", False]]
    arms = irtree.match_arms(ir_of("match p with Point(_, y) -> y", "p: Point", "U8"))
    assert [p for p, _ in arms] == [["ctor", "Point", [["wild"], ["bind", "y"]]]]


def test_a_name_that_is_a_constructor_is_always_a_pattern_never_a_binder():
    # `Dot` is a constructor pattern (a payload-free one), `other` is a binder
    m = ir_of("match s with Dot -> true | other -> false", "s: Shape")
    assert irtree.match_arms(m)[0][0] == ["ctor", "Dot", []]
    assert refused("match s with Circle -> true | _ -> false", "s: Shape").code == "type"  # wrong field count
    assert refused("match o with Some -> true | _ -> false", "o: Option<U8>").code == "corpus"  # a reserved name


def test_a_match_arm_without_context_takes_the_first_self_typed_arms_type():
    m = ir_of("match o with Some(x) -> Some(x) | None -> None", "o: Option<U8>", "Option<U8>")
    assert irtree.match_arms(m)[1][1] == ["none", "U8"]
    m = ir_of("match o with None -> None | Some(x) -> Some(x)", "o: Option<U8>", "Option<U8>")
    assert irtree.match_arms(m)[0][1] == ["none", "U8"]


def test_the_scrutinee_of_a_match_may_be_any_expression():
    pre = "  model mk(a: U8) -> Shape = Circle(a)\n"
    assert ir_of("match mk(a) with Circle(r) -> r | _ -> 0u8", ret="U8", extra=pre)[0] == "match"
    assert ir_of("match Some(1u8) with Some(_) -> 1 | None -> 0", ret="Nat")[1] == ["some", lit("U8", 1)]
    assert ir_of("match a == 0u8 with true -> 1 | false -> 0", ret="Nat")[1][0] == "eq"


@pytest.mark.parametrize(
    "body,params,message",
    [
        ("match a with true -> true | _ -> false", "a: U8", "Bool pattern needs a Bool"),
        ("match a with Some(x) -> x | _ -> false", "a: Bool", "Some takes one pattern"),
        ("match a with None -> true | _ -> false", "a: Bool", "None needs an Option"),
        ("match o with (a, b) -> true | _ -> false", "o: Option<U8>", "tuple pattern"),
        ("match o with Ok(_) -> true | _ -> false", "o: Option<U8>", "needs a Result"),
        ("match s with Foo(x) -> true | _ -> false", "s: Shape", "unknown constructor"),
        ("match s with Add -> true | _ -> false", "s: Shape", "does not match the type"),
        ("match s with Rect(w) -> true | _ -> false", "s: Shape", "wrong number of fields"),
        ("match p with Point(_, _, _) -> true", "p: Point", "wrong number of fields"),
    ],
)
def test_pattern_structure_errors_are_type_errors(body, params, message):
    e = refused(body, params)
    assert e.code == "type" and message in e.message, (body, e.message)


@pytest.mark.parametrize(
    "body,params,message",
    [
        ("match b with true -> true", "b: Bool", "fall through"),
        ("match o with Some(_) -> true", "o: Option<U8>", "fall through"),
        ("match o with Some(Some(_)) -> true | None -> false", "o: Option<Option<U8>>", "fall through"),
        ("match s with Circle(_) -> true | Rect(_, _) -> false", "s: Shape", "fall through"),
        ("match b with true -> true | true -> false | false -> true", "b: Bool", "no value can reach"),
        ("match o with _ -> true | Some(_) -> false", "o: Option<U8>", "no value can reach"),
        ("match s with x -> true | Dot -> false", "s: Shape", "no value can reach"),
        ("match b with true -> 1 | true -> 2", "b: Bool", "no value can reach"),  # R12 order
        ("match s with Rect(a, a) -> true | _ -> false", "s: Shape", "twice"),
        ("match s with Rect(w, _) -> true | _ -> false", "w: U8, s: Shape", "shadows"),
    ],
)
def test_the_corpus_errors_of_appendix_a2(body, params, message):
    ret = "Nat" if "-> 1" in body else "Bool"
    e = refused(body, params, ret)
    assert e.code == "corpus" and message in e.message, (body, e.message)


def test_the_usefulness_check_over_tuples_records_and_nested_options():
    # exhaustive and irredundant: accepted
    assert ir_of("match t with (true, x) -> x | (false, _) -> false", "t: (Bool,Bool)")[0] == "match"
    assert ir_of(
        "match s with Slot(Some(_), (true, n)) -> n | Slot(Some(_), (false, _)) -> 1 | Slot(None, _) -> 0",
        "s: Slot",
        "Nat",
    )[0] == "match"
    # a Bool component left uncovered inside a tuple
    assert refused("match t with (true, _) -> true", "t: (Bool,Bool)").code == "corpus"
    # a record pattern after an irrefutable record pattern is unreachable
    e = refused("match p with Point(_, _) -> true | Point(x, _) -> false", "p: Point")
    assert "no value can reach" in e.message


def test_a_complete_truth_table_at_the_arm_limit_is_accepted():
    # 256 arms over eight Bool components: exhaustive, irredundant, and exactly at the 256-arm limit
    arms = " | ".join(
        f"({', '.join('true' if (i >> k) & 1 else 'false' for k in range(8))}) -> {str(i % 2 == 0).lower()}"
        for i in range(256)
    )
    m = model(f"match t with {arms}", "t: (Bool,Bool,Bool,Bool,Bool,Bool,Bool,Bool)")
    assert m.lowered and len(irtree.match_arms(m.ir)) == 256


# -- the refusals: reserved levels and feature names (R27, R28) ---------------------------------------------


@pytest.mark.parametrize(
    "body,level",
    [
        ("[a] == [a]", "ir.list"),
        ("slice(d, 0, 1) == d", "ir.list"),
        ("to_list(d) == to_list(d)", "ir.list"),
        ("to_bytes(d) == d", "ir.list"),
        ("replicate(a, 2) == replicate(a, 2)", "ir.list"),
        ("range(n) == range(n)", "ir.seq"),
        ("fold(n) == n", "ir.seq"),
        ("map(n) == n", "ir.seq"),
        ("all(n)", "ir.seq"),
        ("any(n)", "ir.seq"),
        ("count(n) == n", "ir.seq"),
        ("fixed<4>(d) == None", "fixed-cast"),
    ],
)
def test_a_reserved_level_or_feature_is_refused_by_its_name_and_never_ir_data(body, level):
    m = model(body, "a: U8, d: Bytes, n: Nat")
    assert not m.lowered and m.ir is None
    assert m.unsupported.code == "level" and m.unsupported.level == level
    assert m.unsupported.level != "ir.data"


def test_an_error_value_used_in_an_arm_is_err_value_and_an_unused_one_lowers_to_err_any():
    r = "r: Result<Bool,E>"
    assert ir_of("match r with Ok(x) -> x | Err(e) -> false", r)[2][1][0] == ["err_any"]
    for body in (
        "match r with Ok(x) -> x | Err(e) -> e == e",
        "match r with Ok(x) -> x | Err(E.A) -> true | Err(e) -> g(e)",
        "match r with Ok(x) -> x | Err(e) -> (match r with Ok(y) -> y | Err(_) -> e == e)",
    ):
        m = model(body, r, extra="  model g(a: Bool) -> Bool = a\n")
        assert not m.lowered and m.unsupported.level == "err-value", body


def test_a_list_type_in_a_declaration_or_signature_refuses_the_whole_document():
    for text in (
        "  record Q(xs: List<U8>)\n",
        "  variant W = One(xs: List<U8>) | Two\n",
        "  model Word = List<U8>\n",
        "  op h(xs: List<U8>) -> Bool\n",
        "  op h(a: U8) -> Result<List<U8>,E>\n",
    ):
        with pytest.raises(OracleError) as err:
            lower(PRELUDE + text)
        assert err.value.code == "level" and err.value.level == "ir.list", text


def test_a_list_type_in_a_model_or_a_binder_is_recorded_on_that_unit():
    spec = lower(
        PRELUDE
        + "  model m(xs: List<U8>) -> Bool = true\n  model ok(a: U8) -> Bool = true\n"
        + "  claim c:\n    forall xs: List<U8>. f(0u8) == 0u8\n  claim d:\n    forall a: U8. f(a) == a\n"
    )
    assert not spec.models["m"].lowered and spec.models["ok"].lowered
    assert not spec.suites[0].lowered and spec.suites[1].lowered
    c = census(spec)
    assert c["suites_refused_at_lowering"] == {"ir.list": 1} and c["suites_lowered"] == 1
    assert c["models_refused_at_lowering"] == 1


# -- limits (R34) and declarations --------------------------------------------------------------------------


@pytest.mark.parametrize(
    "decl",
    [
        "  record Big(" + ", ".join(f"f{i}: U8" for i in range(33)) + ")\n",
        "  variant Many = " + " | ".join(f"C{i}" for i in range(65)) + "\n",
        "  variant Wide = Fat(" + ", ".join(f"f{i}: U8" for i in range(9)) + ") | Thin\n",
        "  error Huge = " + " | ".join(f"Z{i}" for i in range(65)) + "\n",
    ],
)
def test_the_corpus_limits_refuse_the_document_whole_before_any_expression(decl):
    with pytest.raises(OracleError) as err:
        lower(PRELUDE + "  model m(a: U8) -> Bool = [a] == [a]\n" + decl)
    assert err.value.code == "limit"


def test_the_corpus_limits_are_inclusive():
    big = "  record Big(" + ", ".join(f"f{i}: U8" for i in range(32)) + ")\n"
    many = "  variant Many = " + " | ".join(f"C{i}" for i in range(64)) + "\n"
    wide = "  variant Wide = Fat(" + ", ".join(f"f{i}: U8" for i in range(8)) + ") | Thin\n"
    cases = "  error Huge = " + " | ".join(f"Z{i}" for i in range(64)) + "\n"
    spec = lower(PRELUDE + big + many + wide + cases)
    assert len(spec.records["Big"]) == 32 and len(spec.variants["Many"]) == 64


@pytest.mark.parametrize(
    "decl",
    [
        "  record P2(a: U8, a: U8)\n",
        "  variant W = One(a: U8, a: U8) | Two\n",
        "  variant W = Only(a: U8)\n",
        "  record Point(z: U8)\n",
        "  variant W = Point | Other\n",
        "  record Loop(next: Loop)\n",
        "  record A1(v: B1)\n  variant B1 = X(a: A1) | Y\n",
        "  record Bad(x: Nope)\n",
    ],
)
def test_declaration_errors_are_type_errors(decl):
    with pytest.raises(OracleError) as err:
        lower(PRELUDE + decl)
    assert err.value.code == "type"


def test_records_and_variants_may_nest_through_option_and_tuple_without_recursion():
    spec = lower(
        PRELUDE
        + "  record Deep(a: Slot, b: Option<Point>, c: (Shape,Mode), d: Result<Point,E>)\n"
        + "  model m(d: Deep) -> Bool = d.c.1 == Add\n"
    )
    assert spec.models["m"].ir == [
        "eq",
        ["project", ["field", ["var", "d"], 2], 1],
        ["ctor", "Add", []],
    ]
    assert spec.adts["Deep"][0][0] == "Deep"
    assert spec.pattern_types().ctors["Rect"] == ("Shape", 1)


# -- the refusal order (R35) ------------------------------------------------------------------------------


def _refusal(body: str) -> OracleError:
    try:
        spec = lower(PRELUDE + body)
    except OracleError as exc:
        return exc
    first = lowering_refusal(spec)
    assert first is not None
    return first


def test_a_level_or_feature_is_named_over_a_type_error_of_the_same_unit_and_the_first_in_source_order():
    assert _refusal("  model m(a: U8) -> Bool = None == None and fixed<4>(b\"00\") == None\n").level == "fixed-cast"
    assert _refusal("  model m(a: U8) -> Bool = to_list(a) == range(a)\n").level == "ir.list"
    assert _refusal("  model m(a: U8) -> Bool = range(a) == to_list(a)\n").level == "ir.seq"
    # a claim, an example and an operation refinement are units too
    assert _refusal("  claim c:\n    forall a: U8. f(a) == a and (None == None and range(1) == 2)\n").level == "ir.seq"
    assert _refusal("  example e:\n    f(1u8) == 1u8 and (None == None and range(1) == 2)\n").level == "ir.seq"
    op = "  op h(a: U8 | None == None and range(1) == 2) -> U8\n"
    assert _refusal(op).level == "ir.seq"


def test_units_are_reported_in_declaration_order():
    first_level = "  model a1(a: U8) -> Bool = [a] == [a]\n  model a2(a: U8) -> Bool = a\n"
    assert _refusal(first_level).level == "ir.list"
    first_type = "  model a1(a: U8) -> Bool = a\n  model a2(a: U8) -> Bool = [a] == [a]\n"
    e = _refusal(first_type)
    assert e.code == "type"
    # a type declaration (raised at the end) yields to a model declared before it, and wins otherwise
    assert _refusal("  model m(a: Bytes) -> Bool = fixed<4>(a) == None\n  record Q(xs: List<U8>)\n").level == "fixed-cast"
    assert _refusal("  record Q(xs: List<U8>)\n  model m(a: Bytes) -> Bool = fixed<4>(a) == None\n").level == "ir.list"
    # ... also over a type error of a later unit
    assert _refusal("  record Q(xs: List<U8>)\n  model m(a: U8) -> Bool = a\n").level == "ir.list"
    assert _refusal("  op h(xs: List<U8>) -> Bool\n  claim c:\n    forall a: U8. h(a) == true\n").level == "ir.list"
    assert _refusal("  model m(a: Nat) -> Bool = range(a) == a\n  op h(xs: List<U8>) -> Bool\n").level == "ir.seq"


def test_the_limits_come_first_and_patterns_before_arm_bodies():
    big = "  record Big(" + ", ".join(f"f{i}: U8" for i in range(33)) + ")\n"
    assert _refusal("  model m(a: U8) -> Bool = [a] == [a]\n" + big).code == "limit"
    e = _refusal("  model m(s: Shape) -> Bool = match s with Dot -> 1 | Foo(x) -> true | _ -> false\n")
    assert e.code == "type" and "unknown constructor" in e.message
    e = _refusal("  model m(t: (Bool,Bool,Bool)) -> Bool = match t with (a, a, Foo(x)) -> true | _ -> false\n")
    assert e.code == "type" and "unknown constructor" in e.message  # structure before the duplicate bind
    e = _refusal("  model m(s: Shape) -> Bool = match s with Rect(a, a) -> true | _ -> false\n")
    assert e.code == "corpus"


# -- the typed IR compiler: types, costs and the refusal to evaluate ------------------------------------------


def program(spec, **kw):
    return build_program(spec, analyze=True, analyze_data=True, max_cost=65_536, **kw)


def test_the_typed_compiler_retypes_and_costs_every_tag():
    spec = lower(PRELUDE + "  record Wide(a: Point, b: Bool)\n")
    prog = program(spec)
    tenv = {"a": U8, "w": A.Ty("Wide"), "o": OPT_U8}

    def comp(ir):
        return prog.compile(ir, tenv)

    c = comp(["some", ["var", "a"]])
    assert str(c.ty) == "Option<U8>" and c.cost == 2
    c = comp(["none", "(U8,Bool)"])
    assert str(c.ty) == "Option<(U8,Bool)>" and c.cost == 1
    c = comp(["ctor", "Point", [["var", "a"], ["var", "a"]]])
    assert str(c.ty) == "Point" and c.cost == 3
    c = comp(["ctor", "Dot", []])
    assert str(c.ty) == "Shape" and c.cost == 1
    c = comp(["field", ["field", ["var", "w"], 0], 1])
    assert str(c.ty) == "U8" and c.cost == 3
    m = comp(["match", ["var", "o"], [[["some", ["bind", "x"]], ["var", "x"]], [["none"], ["var", "a"]]]])
    assert str(m.ty) == "U8" and m.cost == 1 + 1 + 1  # 1 + scrutinee + the worst arm


def test_the_typed_compiler_evaluates_every_data_tag_and_pattern_since_s11c_1():
    # S11a: the closures refused to run (``level ir.data``); S11c-1 evaluates them (the values are in test_data_eval.py)
    prog = program(lower(PRELUDE))
    tenv = {"a": U8, "o": OPT_U8}
    assert prog.compile(["some", ["var", "a"]], tenv).fn({"a": 1}) == ("some", 1)
    assert prog.compile(["none", "U8"], tenv).fn({}) == ("none",)
    assert prog.compile(["ctor", "Dot", []], tenv).fn({}) == ("Dot", ())
    m = prog.compile(["match", ["var", "o"], [[["some", ["wild"]], ["lit", True]], [["none"], ["lit", False]]]], tenv)
    assert m.fn({"o": ("none",)}) is False and m.fn({"o": ("some", 3)}) is True
    assert P.compile_pattern(["some", ["wild"]])(("some", 1), {})


def test_without_analyze_data_every_data_tag_and_option_is_the_level_ir_data():
    # ``analyze_data=False`` is the program of a lane that does not declare ir.data (the kit, ``conformance``)
    prog = Program({}, {}, errors={"E": ("A",)}, adts={"Point": (("Point", (U8, U8)),)}, analyze_data=False)
    tenv = {"a": U8, "o": OPT_U8, "r": A.Ty("Result", (A.Ty("Bool"), A.Ty("E")))}
    for ir in (
        ["some", ["var", "a"]],
        ["none", "U8"],
        ["ctor", "Point", [["var", "a"], ["var", "a"]]],
        ["field", ["var", "a"], 0],
        ["var", "o"],
        ["match", ["var", "r"], [[["ok", ["bool", True]], ["lit", True]], [["ok", ["wild"]], ["lit", False]], [["err_any"], ["lit", False]]]],
    ):
        with pytest.raises(OracleError) as err:
            prog.compile(ir, tenv)
        assert err.value.code == "level" and err.value.level == "ir.data", ir


def test_the_tag_list_is_the_reserved_level_ir_list_and_never_counted():
    for analyze in (False, True):
        with pytest.raises(OracleError) as err:
            Program({}, {}, analyze_data=analyze).compile(["list", []], {})
        assert err.value.code == "level" and err.value.level == "ir.list"
    with pytest.raises(OracleError) as err:
        L.ir_levels(["list", []])
    assert err.value.level == "ir.list"
    with pytest.raises(OracleError) as err:
        Program({}, {}, analyze_data=True).compile(["var", "xs"], {"xs": A.Ty("List", (U8,))})
    assert err.value.level == "ir.list"


@pytest.mark.parametrize(
    "ir,tenv",
    [
        (["some"], {}),
        (["some", ["lit", True], ["lit", True]], {}),
        (["none"], {}),
        (["none", "Nope"], {}),  # an undeclared type name
        (["none", "Result<U8,Nope>"], {}),  # an undeclared error type
        (["none", "Bytes<5000>"], {}),
        (["none", "U8 "], {}),  # not a canonical name
        (["none", 7], {}),
        (["ctor", "Nope", []], {}),
        (["ctor", "Point", [["lit", True], ["lit", True]]], {}),  # operands of the wrong type
        (["ctor", "Point", [["lit", {"type": "U8", "value": "1"}]]], {}),  # a wrong operand count
        (["ctor", "Point"], {}),
        (["ctor", 3, []], {}),
        (["field", ["var", "a"], 0], {"a": U8}),  # not a record
        (["field", ["var", "p"], 5], {"p": A.Ty("Point")}),  # out of range
        (["field", ["var", "p"], "0"], {"p": A.Ty("Point")}),
        (["field", ["var", "p"]], {"p": A.Ty("Point")}),
        (["field", ["var", "s"], 0], {"s": A.Ty("Shape")}),  # a variant has no field
    ],
)
def test_malformed_data_ir_is_an_ir_error(ir, tenv):
    prog = Program(
        {},
        {},
        analyze_data=True,
        errors={"E": ("A",)},
        adts={
            "Point": (("Point", (U8, U8)),),
            "Shape": (("Circle", (U8,)), ("Dot", ())),
        },
    )
    with pytest.raises(OracleError) as err:
        prog.compile(ir, tenv)
    assert err.value.code == "ir", ir


def test_a_none_may_name_every_kind_of_declared_type():
    prog = Program(
        {},
        {},
        analyze_data=True,
        errors={"E": ("A",)},
        adts={"Point": (("Point", (U8, U8)),)},
    )
    for name in ("Bool", "Int", "Nat", "U8", "I64", "Bytes", "Bytes<4096>", "(U8,Bool)", "Option<Point>", "Result<U8,E>", "Point"):
        assert str(prog.compile(["none", name], {}).ty) == f"Option<{name}>"
    with pytest.raises(OracleError) as err:
        prog.compile(["none", "List<U8>"], {})
    assert err.value.level == "ir.list"


def test_data_patterns_are_typed_against_the_value_they_meet_in_ir():
    errors = {"E": ("A",)}
    types = P.Types(errors, {"Shape": (("Circle", (U8,)), ("Dot", ()))})
    binds: list = []
    P.type_pattern(["ctor", "Circle", [["bind", "r"]]], A.Ty("Shape"), types, binds)
    assert binds == [("r", U8)]
    for pattern, ty in (
        (["ctor", "Nope", []], A.Ty("Shape")),
        (["ctor", "Dot", []], A.Ty("Option", (U8,))),
        (["ctor", "Circle", []], A.Ty("Shape")),
        (["some", ["wild"]], A.Ty("Bool")),
        (["none"], A.Ty("Bool")),
        (["bool", True], U8),
        (["bool", 1], A.Ty("Bool")),
        (["ctor", "Circle"], A.Ty("Shape")),
    ):
        with pytest.raises(OracleError):
            P.type_pattern(pattern, ty, types, [])
    # the arms: complete and incomplete
    shape = A.Ty("Shape")
    P.check_arms([["ctor", "Circle", [["wild"]]], ["ctor", "Dot", []]], shape, types)
    with pytest.raises(OracleError) as err:
        P.check_arms([["ctor", "Circle", [["wild"]]]], shape, types)
    assert "fall through" in err.value.message
    with pytest.raises(OracleError) as err:
        P.check_arms([["wild"], ["ctor", "Dot", []]], shape, types)
    assert "no value can reach" in err.value.message


# -- levels, trees and type names ---------------------------------------------------------------------------------


def test_ir_levels_by_tag_and_the_levels_a_caller_gives_a_match():
    assert L.ir_levels(["some", ["lit", True]]) == ["ir.core", "ir.data"]
    assert L.ir_levels(["ctor", "Dot", []]) == ["ir.data"]
    assert L.ir_levels(["none", "U8"]) == ["ir.data"]
    assert L.ir_levels(["field", ["var", "p"], 0]) == ["ir.core", "ir.data"]
    node = ["match", ["var", "x"], [[["wild"], ["lit", True]]]]
    assert L.ir_levels(node) == ["ir.core", "ir.tuple"]  # no scrutinee type: the Result case
    node = ["match", ["var", "x"], [[["ok", ["bool", True]], ["lit", True]], [["wild"], ["lit", False]]]]
    assert L.ir_levels(node) == ["ir.core", "ir.data", "ir.tuple"]  # a data pattern is visible
    assert L.ir_levels(node, match_levels={id(node): frozenset(["ir.data"])}) == ["ir.core", "ir.data"]
    assert not L.has_data_pattern(["tuple", [["wild"], ["bind", "x"]]])
    assert L.has_data_pattern(["tuple", [["wild"], ["some", ["wild"]]]])
    assert L.has_data_pattern(["ok", ["ctor", "Dot", []]])
    assert L.type_levels(A.Ty("Result", (A.Ty("Option", (U8,)), A.Ty("E")))) == {"ir.data"}


def test_the_tree_shape_of_the_data_nodes():
    ctor = ["ctor", "Point", [["var", "a"], ["subject_op", "f", [["var", "a"]]]]]
    assert irtree.children(ctor) == [["var", "a"], ["subject_op", "f", [["var", "a"]]]]
    assert irtree.children(["none", "U8"]) == []
    assert irtree.children(["field", ["var", "p"], 0]) == [["var", "p"]]
    assert irtree.children(["some", ["var", "x"]]) == [["var", "x"]]
    assert irtree.pattern_binds(["some", ["ctor", "Rect", [["bind", "w"], ["tuple", [["bind", "h"], ["wild"]]]]]]) == [
        "w",
        "h",
    ]
    assert irtree.free_vars(["match", ["var", "o"], [[["some", ["bind", "x"]], ["var", "x"]]]]) == {"o"}


@pytest.mark.parametrize(
    "text",
    [
        "Bool",
        "U8",
        "Int",
        "Bytes",
        "Bytes<0>",
        "Bytes<4096>",
        "(U8,Bool)",
        "(U8,(Bool,Nat),Bytes<2>)",
        "Option<U8>",
        "Option<Option<(U8,Bool)>>",
        "Result<U8,E>",
        "List<U8>",
        "Point",
    ],
)
def test_canonical_type_names_round_trip(text):
    assert str(parse_type_name(text)) == text


@pytest.mark.parametrize(
    "text",
    [
        "",
        "U8 ",
        " U8",
        "(U8)",
        "(U8,)",
        "(U8,Bool",
        "Option<>",
        "Option<U8,U8>",
        "Option<U8",
        "Result<U8>",
        "Result<U8,Option<U8>>",
        "Bytes<N>",
        "Bytes<04>",
        "Bytes<1,2>",
        "Bytes<5",
        "List<U8,U8>",
        "9",
        "U8<U8>",
        "é",
        "(" * 20 + "U8" + ")" * 20,
        "Option<" * 20 + "U8" + ">" * 20,
        "(U8,Bool,Bool,Bool,Bool,Bool,Bool,Bool,Bool)",
        7,
        None,
        "A" * 600,
    ],
)
def test_malformed_type_names_are_refused(text):
    with pytest.raises(ValueError):
        parse_type_name(text)


# -- the dump, the census and evaluation -----------------------------------------------------------------------


def test_the_dump_carries_types_only_for_a_document_that_declares_one():
    with_types = view(
        (PRELUDE + "  model m(p: Point) -> U8 = p.x\n").encode()
    )
    assert set(with_types["types"]) == {"Point", "Slot", "Shape", "Mode"}
    assert with_types["types"]["Shape"]["ctors"] == [["Circle", ["U8"]], ["Rect", ["U8", "U8"]], ["Dot", []]]
    assert with_types["types"]["Slot"] == {"kind": "record", "fields": [["o", "Option<U8>"], ["t", "(Bool,Nat)"]]}
    assert with_types["models"]["m"]["ir_cost"] == 2
    plain = view(b"warrant c v1\n  op f(a: U8) -> U8\n  claim c:\n    forall a: U8. f(a) == a\n")
    assert "types" not in plain  # a document that does not declare one dumps as it did before the level


def test_the_dump_refuses_what_the_lowerer_refused_and_shows_what_it_lowered():
    assert dump((PRELUDE + "  model m(o: Option<U8>) -> Bool = o == None\n").encode())
    with pytest.raises(OracleError) as err:
        dump((PRELUDE + "  model m(a: U8) -> Bool = [a] == [a]\n").encode())
    assert err.value.level == "ir.list"
    with pytest.raises(OracleError) as err:  # the op's own refusal (a refinement with a reserved level)
        dump((PRELUDE + "  op h(a: U8 | all(a)) -> U8\n").encode())
    assert err.value.level == "ir.seq"


def test_emit_enumerates_a_data_suite_since_s11c_1_and_the_census_counts_it_under_ir_core():
    doc = (
        PRELUDE
        + "  op g(p: Point) -> U8\n"
        + "  claim data:\n    forall p: Point. g(p) == p.x\n"
        + "  claim plain:\n    forall a: U8. f(a) == a\n"
    )
    tree = emit(doc.encode())
    # S11a: ``{"status": "not-live", "level": "ir.data", "reason": "ir.data is lowered, typed and costed but not
    # evaluated yet (slice S11c-1)"}``
    assert tree["suites"]["data"]["status"] == "ok" and tree["suites"]["data"]["cases"] == 65536
    assert tree["suites"]["data"]["descriptors"] == [{"d": "tuple", "of": [{"d": "u8_all", "size": 256}] * 2, "size": 65536}]
    assert tree["suites"]["plain"]["status"] == "ok" and tree["summary"]["not-live"] == 0
    spec = lower(doc)
    assert SuitePlan(spec, build_program(spec), spec.suites[0]).product == 65536
    c = census(spec)
    assert c["suites_by_level"] == {"ir.core": 2} and c["suites_refused_at_lowering"] == {}


def test_the_model_declared_before_a_refinement_that_calls_it_still_lowers():
    spec = lower(
        PRELUDE
        + "  model small(a: U8) -> Bool = a < 100u8\n"
        + "  op g(a: U8 | small(a)) -> U8\n"
        + "  claim c:\n    forall a: U8 | small(a). g(a) == a\n"
    )
    assert spec.ops["g"].admit == ["model_call", "small", [["var", "a"]]]


# -- the pre-order scan that makes a reserved level win over a type error of the same unit (R35 (2)) ------------


def test_the_scan_finds_every_kind_of_reserved_construct_after_a_type_error():
    r = "r: Result<Bool,E>"
    cases = [
        # a list literal, a list built-in, a sequence built-in and the cast, each AFTER a type error
        ("None == None and [a] == [a]", "a: U8", "ir.list"),
        ("None == None and slice(d, 0, 1) == d", "a: U8, d: Bytes", "ir.list"),
        ("None == None and fold(a) == a", "a: U8", "ir.seq"),
        ("None == None and fixed<4>(d) == None", "a: U8, d: Bytes", "fixed-cast"),
        # a reserved construct inside a match scrutinee, an arm of the match, and an error value used by name
        ("match fixed<4>(d) with None -> None == None | _ -> true", "d: Bytes", "fixed-cast"),
        ("match r with Ok(x) -> None == None | Err(e) -> e == e", r, "err-value"),
        ("match r with Ok(x) -> None == None | Err(e) -> g(e)", r, "err-value"),
        ("match r with Ok(x) -> None == None | Err(e) -> e(true)", r, "err-value"),
        ("match r with Ok(x) -> None == None | Err(E.A) -> range(1) == 1 | Err(_) -> true", r, "ir.seq"),
    ]
    for body, params, level in cases:
        m = model(body, params, extra="  model g(a: Bool) -> Bool = a\n")
        assert not m.lowered and m.unsupported.level == level, body


def test_a_user_declared_name_is_never_mistaken_for_a_reserved_built_in():
    # a model called `count` or `slice` is the user's (the checker reserves neither), in the scan and in the lowering alike
    spec = lower(
        PRELUDE
        + "  model count(a: U8) -> U8 = a\n  model slice(a: U8) -> U8 = a\n"
        + "  model m(a: U8) -> Bool = count(a) == slice(a)\n"
    )
    assert spec.models["m"].lowered and spec.models["m"].ir[0] == "eq"


def test_ill_typed_uses_of_the_core_built_ins_stay_type_errors():
    for body in ("len(a) == 1", "concat(a, a) == a", "take(a, 1) == a", "concat(d, a) == d"):
        assert refused(body, "a: U8, d: Bytes").code == "type", body
    assert refused("match r with Ok(_) -> true | Err(Ok) -> false", "r: Result<Bool,E>").code == "type"
    assert refused("match r with Ok(_) -> true | Err(None) -> false", "r: Result<Bool,E>").code == "type"
    assert refused("match r with Ok(_) -> true | Err(Dot) -> false", "r: Result<Bool,E>").code == "type"


def test_the_model_error_of_a_unit_is_raised_when_no_earlier_unit_holds_a_refusal():
    # a type error in the first unit, none earlier: raised as is (the `_in_order` fall-through)
    with pytest.raises(OracleError) as err:
        lower(PRELUDE + "  model a1(a: U8) -> Bool = a\n")
    assert err.value.code == "type" and "declared type" in err.value.message


# -- review fixes of S11a (VER04r 16.16 R53, R57, R58) ------------------------------------------------------


def _doc_refusal(text: str) -> OracleError:
    """The refusal of a whole document: the error it raises, or the first one a unit recorded."""
    try:
        err = lowering_refusal(lower("warrant t v1\n  error E = A | B\n" + text))
    except OracleError as exc:
        return exc
    assert err is not None, "not refused"
    return err


@pytest.mark.parametrize(
    "unit",
    [
        "  model m(r: Q) -> Nat = len(r.xs)\n",
        "  model m(r: Q) -> Bytes = concat(r.xs, r.xs)\n",
        "  model m(r: Q) -> Bytes = take(r.xs, 1)\n",
        "  model m(r: Q) -> Bytes = drop(r.xs, 1)\n",
        "  model m(r: Q) -> Option<U8> = at(r.xs, 1)\n",
        "  model m(a: Bytes) -> Nat = len(f(a))\n",
        "  model m(a: Bytes) -> Bool = f(a) == f(a)\n",
        "  model m(a: Bytes) -> Bytes = take(f(a), 1)\n",
        "  model m(a: Bytes) -> Option<U8> = at(f(a), 1)\n",
    ],
)
def test_a_list_operand_or_a_call_to_a_list_op_is_ir_list_before_or_after_its_declaration_r57(unit):
    refused_decls = "  record Q(xs: List<U8>)\n  op f(a: Bytes) -> List<U8>\n"
    for text in (unit + refused_decls, refused_decls + unit):
        err = _doc_refusal(text)
        assert (err.code, err.level) == ("level", "ir.list"), text


def test_a_unit_before_a_refused_declaration_has_no_spurious_type_error_r57():
    # the typing error of a unit that does not depend on the refused declaration stays a typing error
    err = _doc_refusal("  model m(a: U8) -> Bool = a\n  op f(a: Bytes) -> List<U8>\n")
    assert err.code == "type"


@pytest.mark.parametrize("name", ["Some", "None", "Ok", "Err", "Int", "len", "rotl", "unwrap_or", "U16"])
def test_a_reserved_built_in_is_refused_as_a_declared_type_or_constructor_r58(name):
    for decl in (
        f"  variant W = {name} | Other\n",
        f"  variant W = Other | {name}(a: U8)\n",
        f"  variant {name} = A1 | B1\n",
        f"  record {name}(a: U8)\n",
    ):
        err = _doc_refusal(decl)
        assert err.code == "type" and "reserved built-in" in err.message, decl
    assert lower("warrant t v1\n  variant W = Other | Fine\n").variants["W"]


def test_an_op_signature_error_yields_to_an_earlier_level_refusal_r53():
    earlier = "  record Q(xs: List<U8>)\n"
    for sig in ("  op g(a: U8, a: U8) -> U8\n", "  op g(a: Nope) -> U8\n", "  op g(a: U8) -> Nope\n"):
        err = _doc_refusal(earlier + sig)
        assert (err.code, err.level) == ("level", "ir.list"), sig  # the earlier refusal is named
        err = _doc_refusal(sig + earlier)
        assert err.code == "type", sig  # declared first: its own typing error
    # an earlier op signature that is refused holds the same place
    err = _doc_refusal("  op h(xs: List<U8>) -> Bool\n  op g(a: U8, a: U8) -> U8\n")
    assert (err.code, err.level) == ("level", "ir.list")
    # the name pass and the recursion check yield to nothing
    err = _doc_refusal("  record Q(xs: List<U8>)\n  record Loop(next: Loop)\n")
    assert err.code == "type"
    err = _doc_refusal("  record Q(xs: List<U8>)\n  record Q(a: U8)\n")
    assert err.code == "type"


def test_the_dump_raises_a_claims_own_lowering_refusal():
    # a claim that calls no operation of the subject is W0701 (VER04r 4.3): the dump raises it, as the generator does
    text = PRELUDE + "  claim c:\n    forall a: U8. a == a\n"
    with pytest.raises(OracleError) as err:
        view(text.encode())
    assert err.value.code == "generation" and "W0701" in err.value.message


def test_every_declared_module_name_is_checked_against_the_reserved_built_ins_r58():
    head = "warrant t v1\n"
    for decl in (
        "  model len(a: U8) -> U8 = a\n",
        "  model rotl(a: U8) -> U8 = a\n",
        "  op Some(a: U8) -> U8\n",
        "  op rem(a: U8) -> U8\n",
        "  error rotl = A | B\n",
        "  error Int = A | B\n",
        "  model Word = Bytes\n  model U16 = Bytes\n",
        "  model U16 = Bytes\n",
    ):
        with pytest.raises(OracleError) as err:
            lower(head + decl)
        assert err.value.code == "type" and "reserved built-in" in err.value.message, decl
    # the checker reserves exactly V0_BUILTINS, V1_TYPES and V1_FUNCTIONS: `take`, `count`, `slice` are free names
    ok = lower(head + "  variant W = take | Other\n  model count(a: U8) -> U8 = a\n  op slice(a: U8) -> U8\n")
    assert "take" in ok.ctors


def test_ok_takes_its_expected_type_as_a_hint_only_like_some_r51():
    # the position decides: a model body is checked against its declared type ...
    assert ir_of("Ok(n)", "n: Nat", "Result<Int,E>") == ["ok", ["var", "n"]]
    assert refused("Ok(n)", "n: Int", "Result<Nat,E>").code == "type"
    assert refused('Ok(b"6162")', "u: Bool", "Result<Bytes<3>,E>").code == "type"
    # ... and an equality in a claim decides by common type (the sim-discrete-arcade-logic shape): the model
    # returns Int, the op Result<Nat,E>: Ok(Int) and Result<Nat,E> unify to Result<Int,E>
    doc = (
        PRELUDE
        + "  op g(v: Nat) -> Result<Nat,E>\n  model h(v: Nat) -> Nat = v\n"
        + "  claim c:\n    forall v: Nat. g(v) == (if v <= 9 then Ok(h(v)) else Err(E.A))\n"
    )
    suite = lower(doc).suites[0]
    assert suite.unsupported is None and suite.body[0] == "eq"
    wide = doc.replace("model h(v: Nat) -> Nat = v", "model h(v: Nat) -> Int = v")
    assert lower(wide).suites[0].unsupported is None  # Ok(Int) vs Result<Nat,E>: the claim lowers, as in the spec
