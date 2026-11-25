"""Slice S10t-1: ``ir.tuple`` lowering, typing, cost and ``requires`` (VER04r A.1, 16.10 R2, R9, D35).

The tuple type ``(T1,..,Tn)``, the literal ``["tuple", [e1, .., en]]``, ``["project", e, i]`` and
``match`` on a ``Result`` with the six patterns ``ok err err_any tuple bind wild`` are lowered by the
source typer (``lower.py``), typed and costed again by the IR typer (``ir_eval.py``) and counted in
``requires`` (``levels.py``); the corpus errors of Appendix A.1 are refused by both.
"""

from __future__ import annotations

import pytest
from conftest import adopter_bytes

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle import levels as L
from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.ir_eval import ModelDef, Program
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source

BASE = (
    "warrant t v1\n"
    "  error E = A | B | C\n"
    "  op k(a: Bool) -> Result<(Bool,Nat),E>\n"
    "  op g(a: U8) -> Result<U8,E>\n"
    "  op h(a: Bool) -> Result<Bool,E>\n"
    "  op p(a: Bool, n: Nat) -> (Bool,Nat)\n"
)
NAT = A.Ty("Nat")
BOOL = A.Ty("Bool")


def lower(text: str | bytes):
    return lower_document(parse_source(text))


def model(body: str, params: str = "a: Bool", ret: str = "Bool"):
    """The lowered model ``m`` of ``BASE`` plus one ``model m(params) -> ret = body``."""
    return lower(BASE + f"  model m({params}) -> {ret} = {body}\n").models["m"]


def lit(ty: str, value: object) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


def var(name: str) -> list:
    return ["var", name]


def tuple_ty(*parts: str) -> A.Ty:
    return A.Ty("()", tuple(A.Ty(p) for p in parts))


RESULT_BOOL = A.Ty("Result", (BOOL, A.Ty("E")))
RESULT_PAIR = A.Ty("Result", (tuple_ty("Bool", "Bool"), A.Ty("E")))
ERRORS = {"E": ("A", "B", "C")}


def typed(ir, tenv=None, *, models=None, analyze=False, analyze_data=False, **kw):
    # ``analyze_data=False``: the program of a lane that does not declare ir.data, which these tests pin (the
    # tuple level's own rules and the level ir.data they refuse); since S11c-1 the default program evaluates ir.data
    return Program(
        models or {}, {}, analyze=analyze, errors=ERRORS, analyze_data=analyze_data, **kw
    ).compile(ir, tenv or {})


# -- the tuple literal ---------------------------------------------------------------------------


@pytest.mark.parametrize("n", range(2, 9))
def test_a_tuple_literal_of_every_arity_lowers_to_one_node(n):
    names = "abcdefgh"[:n]
    ty = "(" + ",".join(["Bool"] * n) + ")"
    m = model("(" + ", ".join(names) + ")", ", ".join(f"{c}: Bool" for c in names), ty)
    assert m.ir == ["tuple", [var(c) for c in names]]
    assert m.requires == ("ir.core", "ir.tuple")
    c = typed(m.ir, {c: BOOL for c in names})
    assert str(c.ty) == ty and c.cost == 1 + n  # 1 + the sum of the operands (rule 6)


def test_the_tuple_literal_evaluates_to_a_python_tuple():
    assert typed(["tuple", [lit("U8", 1), lit("Nat", 2)]]).fn({}) == (1, 2)
    assert typed(["tuple", [["tuple", [var("a"), var("b")]], var("a")]], {"a": BOOL, "b": BOOL}).fn(
        {"a": True, "b": False}
    ) == ((True, False), True)


def test_a_literal_component_takes_its_type_from_the_context():
    m = model("(1, 2)", "", "(Nat,Nat)")
    assert m.ir == ["tuple", [lit("Nat", 1), lit("Nat", 2)]]
    # no context: a bare literal is an Int (section 2.1)
    m = model("(1, 2).1 == 2", "")
    assert m.ir[1] == ["project", ["tuple", [lit("Int", 1), lit("Int", 2)]], 1]


def test_a_tuple_literal_in_a_result_context_types_its_components():
    m = model("if a then Ok((a, 1)) else Err(E.A)", "a: Bool", "Result<(Bool,Nat),E>")
    assert m.ir == [
        "if",
        var("a"),
        ["ok", ["tuple", [var("a"), lit("Nat", 1)]]],
        ["err", "A"],
    ]


def test_a_one_component_parenthesis_is_not_a_tuple():
    assert model("(a)", "a: Bool").ir == var("a")


def test_nesting_and_mixed_component_types():
    m = model('(((a, 1), not a), b"")', "a: Bool", "(((Bool,Nat),Bool),Bytes<0>)")
    assert m.ir == [
        "tuple",
        [
            ["tuple", [["tuple", [var("a"), lit("Nat", 1)]], ["not", var("a")]]],
            ["lit", {"type": "Bytes", "value": ""}],
        ],
    ]
    c = typed(m.ir, {"a": BOOL})
    assert str(c.ty) == "(((Bool,Nat),Bool),Bytes<0>)"
    assert c.cost == 1 + (1 + (1 + 1 + 1) + (1 + 1)) + 1


def test_a_nine_component_tuple_is_a_parse_error_and_lowering_checks_arity():
    with pytest.raises(OracleError) as err:
        lower(BASE + "  model m() -> Bool = (1, 2, 3, 4, 5, 6, 7, 8, 9) == (1, 2, 3, 4, 5, 6, 7, 8, 9)\n")
    assert err.value.code == "parse"
    assert typed(["tuple", [lit("Nat", 1)] * 8]).cost == 9
    for operands in ([lit("Nat", 1)], [lit("Nat", 1)] * 9, []):
        with pytest.raises(OracleError) as err:
            typed(["tuple", operands])
        assert err.value.code == "ir"
    with pytest.raises(OracleError) as err:
        typed(["tuple", lit("Nat", 1)])  # operands are one list
    assert err.value.code == "ir"


def test_a_tuple_binder_and_a_tuple_result_type_are_counted_in_requires():
    m = model("a", "a: (Bool,Bool)", "(Bool,Bool)")
    assert m.requires == ("ir.core", "ir.tuple")  # no tuple tag at all: the type does it (R9)


# -- structural equality --------------------------------------------------------------------------


@pytest.mark.parametrize("op,tag", [("==", "eq"), ("!=", "ne")])
def test_tuple_equality_lowers_and_is_structural(op, tag):
    m = model(f"a {op} b", "a: (Bool,Nat), b: (Bool,Nat)")
    assert m.ir == [tag, var("a"), var("b")]
    c = typed(m.ir, {"a": tuple_ty("Bool", "Nat"), "b": tuple_ty("Bool", "Nat")})
    assert c.ty == BOOL
    same, other = (True, 3), (True, 4)
    assert c.fn({"a": same, "b": same}) is (tag == "eq")
    assert c.fn({"a": same, "b": other}) is (tag == "ne")


def test_equality_of_tuples_is_invariant_in_every_component():
    """A tuple never lifts a component (VER04r 16.16 R51, the checker's ``common_type`` has no tuple rule)."""
    assert model("a == (true, 3)", "a: (Bool,Nat)").unsupported is None
    ir = ["eq", ["tuple", [lit("Bytes", "0001"), lit("Nat", 1)]], var("t")]
    ir[1][1][0] = ["lit", {"type": "Bytes", "value": "0001"}]
    assert typed(ir, {"t": A.Ty("()", (A.Ty("Bytes", (2,)), NAT))}).ty == BOOL
    # ... but Bytes<2> against Bytes, and Nat against Int, inside a tuple, are no longer unified
    with pytest.raises(OracleError) as err:
        typed(ir, {"t": A.Ty("()", (A.Ty("Bytes"), NAT))})
    assert err.value.code == "ir"
    with pytest.raises(OracleError) as err:
        model("a == b", "a: (Bool,Nat), b: (Bool,Int)")
    assert err.value.code == "type"
    with pytest.raises(OracleError) as err:
        model("a", "a: (Nat,Nat)", "(Int,Int)")
    assert err.value.code == "type"


def test_equality_of_tuples_of_different_shapes_is_a_type_error():
    with pytest.raises(OracleError) as err:
        model("a == b", "a: (Bool,Bool), b: (Bool,Bool,Bool)")
    assert err.value.code == "type"
    with pytest.raises(OracleError) as err:
        typed(["eq", var("a"), var("b")], {"a": tuple_ty("Bool", "Bool"), "b": tuple_ty("Bool", "Nat")})
    assert err.value.code == "ir"


# -- project ----------------------------------------------------------------------------------------


def test_project_by_name_and_chain():
    assert model("t.0", "t: (Bool,Nat)").ir == ["project", var("t"), 0]
    m = model("t.0.1", "t: ((Bool,Nat),Bool)", "Nat")
    assert m.ir == ["project", ["project", var("t"), 0], 1]
    assert m.requires == ("ir.core", "ir.tuple")


def test_project_after_a_call_and_on_a_literal():
    # a model body calls no subject (VER04r 4.2 rule 3), so the call is lowered in a claim
    suite = lower(BASE + "  claim c:\n    forall a: Bool. p(a, 3).1 == 3\n").suites[0]
    assert suite.body[1] == ["project", ["subject_op", "p", [var("a"), lit("Nat", 3)]], 1]
    assert model("(true, 2u8).1", "", "U8").ir == [
        "project",
        ["tuple", [["lit", True], lit("U8", 2)]],
        1,
    ]


def test_project_types_and_evaluates():
    t = tuple_ty("Bool", "Nat")
    c = typed(["project", var("t"), 1], {"t": t})
    assert c.ty == NAT and c.cost == 2 and c.fn({"t": (True, 9)}) == 9
    assert typed(["project", var("t"), 0], {"t": t}).ty == BOOL


@pytest.mark.parametrize(
    "text",
    [
        "t.2",  # out of range
        "t.x",  # a name, not an index
        "t.0.0",  # the component is not a tuple
    ],
)
def test_a_bad_projection_is_a_type_error(text):
    with pytest.raises(OracleError) as err:
        model(text, "t: (Bool,Nat)", "Bool")
    assert err.value.code == "type"


def test_a_non_canonical_index_is_refused_by_the_lexer():
    with pytest.raises(OracleError) as err:
        model("t.01", "t: (Bool,Nat)", "Bool")
    assert err.value.code == "lex"


def test_projection_of_a_non_tuple_is_a_type_error():
    # Slice S11a (VER04r 16.14 R39 (c)): `e.i` on a value that is not a tuple was the level `ir.data`
    # (records could not be lowered); it is a typing error now, a name or a postfix form alike.
    for body in ("a.0", "(a, a).0.0"):
        with pytest.raises(OracleError) as err:
            model(body, "a: Bool")
        assert err.value.code == "type", body


def test_project_ir_errors():
    t = tuple_ty("Bool", "Nat")
    for node in (
        ["project", var("t")],  # no index
        ["project", var("t"), True],  # a JSON bool is not an integer
        ["project", var("t"), "0"],  # an index is an integer, not text
        ["project", var("t"), 2],  # out of range
        ["project", var("t"), -1],
        ["project", lit("Nat", 1), 0],  # not a tuple
    ):
        with pytest.raises(OracleError) as err:
            typed(node, {"t": t})
        assert err.value.code == "ir"


# -- match: the six patterns -------------------------------------------------------------------------


def match_ir(body: str, params: str = "r: Result<Bool,E>", ret: str = "Bool"):
    return model(body, params, ret).ir


def test_the_six_patterns_have_their_appendix_a1_shapes():
    ok, err_any, wild = (
        ["ok", ["bind", "x"]],
        ["err_any"],
        ["wild"],
    )
    assert match_ir("match r with Ok(x) -> x | Err(_) -> false") == [
        "match",
        var("r"),
        [[ok, var("x")], [err_any, ["lit", False]]],
    ]
    # err: one error case; err_any: Err(e) as well as Err(_); bind: a name; wild: _
    assert match_ir(
        "match r with Ok(_) -> 0 | Err(E.A) -> 1 | Err(e) -> 2", ret="Nat"
    ) == [
        "match",
        var("r"),
        [
            [["ok", wild], lit("Nat", 0)],
            [["err", "A"], lit("Nat", 1)],
            [err_any, lit("Nat", 2)],
        ],
    ]
    assert match_ir("match r with Ok(_) -> true | whole -> whole == r")[2][1][0] == ["bind", "whole"]
    assert match_ir("match r with Ok(_) -> 1 | _ -> 0", ret="Nat")[2][1][0] == wild


def test_a_tuple_pattern_nests_and_may_be_wild_and_bind_mixed():
    ir = match_ir(
        "match r with Ok(((a, _), c)) -> a and c | Err(_) -> false",
        "r: Result<((Bool,Bool),Bool),E>",
    )
    pattern = ir[2][0][0]
    assert pattern == [
        "ok",
        ["tuple", [["tuple", [["bind", "a"], ["wild"]]], ["bind", "c"]]],
    ]


def test_match_in_a_claim_an_example_and_an_op_admit_lowers():
    spec = lower(
        BASE
        + "  op q(r: Result<Bool,E> | match r with Ok(x) -> x | Err(_) -> true) -> Bool\n"
        + "  claim c:\n    forall a: Bool. match h(a) with Ok(x) -> x == a | Err(_) -> true\n"
        + "  example e:\n    match h(true) with Ok(x) -> x | Err(_) -> false\n"
    )
    assert spec.suites[0].unsupported is None and spec.suites[0].requires == ("ir.core", "ir.tuple")
    assert spec.suites[0].kind == "predicate"
    assert spec.examples[0].unsupported is None
    assert spec.examples[0].body[0] == "match" and spec.examples[0].subject is None
    assert spec.ops["q"].requires == ("ir.core", "ir.tuple")
    assert spec.ops["q"].admit[0] == "match"


def test_match_with_a_subject_call_as_scrutinee_keeps_the_op_listed():
    spec = lower(
        BASE + "  claim c:\n    forall a: Bool. match h(a) with Ok(x) -> x | Err(_) -> true\n"
    )
    assert spec.suites[0].ops == ["h"]


def test_match_arms_unify_and_an_arm_that_needs_a_context_takes_the_first_typed_arm():
    # the Ok arm needs a context (Ok(..) has no type of its own); the Err arm does too: the
    # model's return type is the context
    ir = match_ir(
        "match r with Ok(x) -> Ok(x) | Err(_) -> Err(E.B)", ret="Result<Bool,E>"
    )
    assert ir[2][0][1] == ["ok", var("x")] and ir[2][1][1] == ["err", "B"]
    # an arm that needs a context after one that does not: the typed arm is lowered first
    assert match_ir("match r with Ok(_) -> 1 | Err(_) -> 2 + 3", ret="Int")[0] == "match"
    # a bare literal arm takes the type of the first self-typed arm (Nat here)
    ir = match_ir("match r with Ok(_) -> 0 | Err(_) -> len(b\"00\")", ret="Nat")
    assert ir[2][0][1] == lit("Nat", 0)


def test_match_arms_with_no_common_type_are_a_type_error():
    with pytest.raises(OracleError) as err:
        match_ir("match r with Ok(_) -> true | Err(_) -> 1")
    assert err.value.code == "type"


def test_match_is_ir_tuple_only_on_a_result():
    for params, scrutinee in (
        ("a: Bool", "a"),  # a Bool
        ("a: Bytes", "a"),
        ("t: (Bool,Bool)", "t"),  # a bare tuple is ir.data
        ("t: (Bool,Bool)", "(t.0, t.1)"),
    ):
        m = model(f"match {scrutinee} with x -> true", params)
        # S11a: the evaluation refusal ``level ir.data``; S11c-1 evaluates it (the level is still in ``requires``)
        assert m.unsupported is None and "ir.data" in m.requires


def test_match_patterns_outside_the_six_are_refused_by_level_or_type():
    # Slice S11a: the literal pattern `bool` is a data pattern; `Ok(true)` on a Result needs BOTH levels
    # (a `match` on a Result is ir.tuple, and a data pattern inside it adds ir.data, Appendix A.2)
    m = model("match r with Ok(true) -> true | Ok(false) -> false | Err(_) -> false", "r: Result<Bool,E>")
    assert m.requires == ("ir.core", "ir.data", "ir.tuple") and m.lowered
    assert m.unsupported is None  # lowered and evaluated (S11c-1; S11a: ``level ir.data``)
    for body in (
        # an Option constructor on a Result is a type error (it was the level ir.data)
        "match h(a) with Some(x) -> true | _ -> false",
        "match h(a) with None -> true | _ -> false",
        "match h(a) with Ok -> true | _ -> false",  # a constructor name used as a binder
        "match h(a) with Ok(x, y) -> true | _ -> false",  # arity
        "match h(a) with Err(E.A, E.B) -> true | _ -> false",
        "match h(a) with Err(Ok(x)) -> true | _ -> false",  # an Err holds an error case
        "match h(a) with Err(E.Z) -> true | _ -> false",  # unknown case
        "match h(a) with Ok(_) -> true | Err(F.A) -> true | _ -> false",  # wrong error type
        "match h(a) with (x, y) -> true | _ -> false",  # a tuple pattern on a Result
        "match h(a) with Foo(x) -> true | _ -> false",  # an unknown constructor
        "match h(a) with Ok(x) -> x | E.A -> false | _ -> false",  # a bare case is not a pattern here
    ):
        with pytest.raises(OracleError) as err:
            model(body)
        assert err.value.code == "type", body


def test_an_error_value_bound_by_a_pattern_has_no_ir_form_so_it_may_not_be_used():
    r = "r: Result<Bool,E>"
    assert model("match r with Ok(x) -> x | Err(e) -> false", r).unsupported is None
    used = model("match r with Ok(x) -> x | Err(e) -> e == e", r)
    # outside every level: the feature name, not a level (VER04r 4.1, 16.14 R28); the model did not lower
    assert used.unsupported.code == "level" and used.unsupported.level == "err-value"
    assert not used.lowered


def test_the_bool_scrutinee_of_a_tuple_projection_in_match_arm_scope():
    # a pattern binder of tuple type can be projected inside its arm
    ir = match_ir(
        "match r with Ok(t) -> t.0 | Err(_) -> false", "r: Result<(Bool,Nat),E>"
    )
    assert ir[2][0] == [["ok", ["bind", "t"]], ["project", var("t"), 0]]


# -- the corpus errors of A.1 -------------------------------------------------------------------------


def corpus_error(body: str, params: str = "r: Result<Bool,E>", ret: str = "Bool") -> str:
    with pytest.raises(OracleError) as err:
        model(body, params, ret)
    assert err.value.code == "corpus", body
    return err.value.message


@pytest.mark.parametrize(
    "body,params",
    [
        ("match r with Ok(_) -> true", "r: Result<Bool,E>"),  # no Err arm
        ("match r with Err(_) -> true", "r: Result<Bool,E>"),  # no Ok arm
        ("match r with Ok(_) -> true | Err(E.A) -> true | Err(E.B) -> true", "r: Result<Bool,E>"),
        ("match r with Ok((a, b)) -> a | Err(_) -> false", "r: Result<(Bool,Bool),E>"),
    ],
)
def test_a_match_that_can_fall_through_is_a_corpus_error_unless_it_is_exhaustive(body, params):
    if "Ok((a, b))" in body:  # a tuple pattern of a Bool-pair is exhaustive: the check is by type
        assert model(body, params).unsupported is None
        return
    assert "fall through" in corpus_error(body, params)


def test_fall_through_through_a_partial_tuple_and_nested_result():
    body = "match r with Ok((true, b)) -> b | Err(_) -> false"
    # `Ok((true, b))` is a data pattern (bool) and leaves `Ok((false, _))` uncovered
    assert "fall through" in corpus_error(body, "r: Result<(Bool,Bool),E>")
    # Ok(Ok(_)) and Err(_) leave Ok(Err(_)) uncovered
    msg = corpus_error(
        "match r with Ok(Ok(_)) -> true | Err(_) -> false", "r: Result<Result<Bool,E>,E>"
    )
    assert "fall through" in msg


@pytest.mark.parametrize(
    "body",
    [
        "match r with _ -> true | Ok(_) -> false",
        "match r with x -> true | Err(_) -> false",
        "match r with Ok(_) -> true | Err(_) -> false | Err(E.A) -> true",
        "match r with Ok(_) -> true | Err(E.A) -> false | Err(E.A) -> true | Err(_) -> false",
        "match r with Ok(_) -> true | Err(E.A) -> true | Err(E.B) -> true | Err(E.C) -> true | Err(_) -> true",
        "match r with Ok(_) -> true | Ok(x) -> x | Err(_) -> false",
        "match r with Err(_) -> false | Err(e) -> true | Ok(_) -> true",
    ],
)
def test_an_arm_no_value_can_reach_is_a_corpus_error(body):
    assert "no value can reach" in corpus_error(body)


def test_an_unreachable_tuple_arm_after_a_bind_of_the_payload():
    msg = corpus_error(
        "match r with Ok(t) -> true | Ok((a, b)) -> a | Err(_) -> false", "r: Result<(Bool,Bool),E>"
    )
    assert "no value can reach" in msg


@pytest.mark.parametrize(
    "body,params",
    [
        ("match r with Ok((a, a)) -> a | Err(_) -> false", "r: Result<(Bool,Bool),E>"),
        (
            "match r with Ok(((a, b), a)) -> a | Err(_) -> false",
            "r: Result<((Bool,Bool),Bool),E>",
        ),
    ],
)
def test_the_same_bind_twice_in_one_pattern_is_a_corpus_error(body, params):
    assert "twice" in corpus_error(body, params)


@pytest.mark.parametrize(
    "body,params",
    [
        ("match r with Ok(a) -> a | Err(_) -> false", "a: Bool, r: Result<Bool,E>"),  # a parameter
        ("match r with Ok(r) -> r | Err(_) -> false", "r: Result<Bool,E>"),  # the scrutinee name
        ("match r with Ok(result) -> result | Err(_) -> false", "r: Result<Bool,E>"),  # result
        (
            "match r with Ok(x) -> (match x with Ok(x) -> x | Err(_) -> false) | Err(_) -> false",
            "r: Result<Result<Bool,E>,E>",
        ),  # an outer bind
        ("match r with Ok(x) -> x | Err(a) -> false", "a: Bool, r: Result<Bool,E>"),  # Err(name)
        ("match r with a -> true", "a: Bool, r: Result<Bool,E>"),  # a whole-value bind
    ],
)
def test_a_bind_that_shadows_is_a_corpus_error(body, params):
    assert "shadows" in corpus_error(body, params)


def test_a_bind_shadowing_a_claim_binder_is_a_corpus_error():
    with pytest.raises(OracleError) as err:
        lower(
            BASE
            + "  claim c:\n    forall a: Bool. h(a) == (match h(a) with Ok(a) -> Ok(a) | Err(_) -> Err(E.A))\n"
        )
    assert err.value.code == "corpus"


def test_names_may_be_reused_by_sibling_arms_and_after_the_match():
    m = model(
        "match r with Ok(x) -> x | Err(x) -> false", "r: Result<Bool,E>"
    )  # Err(x) is err_any with an unused name: legal beside Ok(x)
    assert m.unsupported is None
    m = model(
        "(match r with Ok(x) -> x | Err(_) -> false) and (match s with Ok(x) -> x | Err(_) -> false)",
        "r: Result<Bool,E>, s: Result<Bool,E>",
    )
    assert m.unsupported is None


def test_corpus_errors_are_raised_even_when_an_arm_body_is_a_level_error():
    # the patterns are checked before the bodies are lowered, so a body that would only be recorded
    # as a level error cannot hide a corpus error
    with pytest.raises(OracleError) as err:
        model("match r with Ok(_) -> [a] == [a]", "a: Bool, r: Result<Bool,E>")
    assert err.value.code == "corpus"


def test_exhaustiveness_by_error_cases_needs_the_declared_cases():
    ok = "match r with Ok(_) -> 0 | Err(E.A) -> 1 | Err(E.B) -> 2 | Err(E.C) -> 3"
    assert match_ir(ok, ret="Nat")[0] == "match"
    # a one-case error type is exhausted by that case
    one = lower(
        "warrant t v1\n  error O = Only\n"
        "  model m(r: Result<Bool,O>) -> Bool = match r with Ok(x) -> x | Err(O.Only) -> false\n"
    )
    assert one.models["m"].unsupported is None


# -- the IR typer: match ----------------------------------------------------------------------------------


def arm(pattern, body):
    return [pattern, body]


ENV = {"r": RESULT_BOOL}


def test_the_ir_typer_types_costs_and_runs_a_match():
    ir = ["match", var("r"), [arm(["ok", ["bind", "x"]], var("x")), arm(["err_any"], ["lit", False])]]
    c = typed(ir, ENV)
    assert c.ty == BOOL
    assert c.cost == 1 + 1 + max(1, 1)  # 1 + cost(scrutinee) + the maximum over the arms
    assert c.fn({"r": ("ok", True)}) is True
    assert c.fn({"r": ("ok", False)}) is False
    assert c.fn({"r": ("err", "B")}) is False


def test_match_cost_is_one_plus_the_scrutinee_plus_the_most_expensive_arm():
    big = ["add", lit("Nat", 1), ["add", lit("Nat", 2), lit("Nat", 3)]]  # cost 5
    ir = [
        "match",
        ["tuple", [var("a"), var("a")]],  # not a Result: the level ir.data
        [arm(["wild"], big)],
    ]
    with pytest.raises(OracleError) as err:
        typed(ir, {"a": BOOL})
    assert err.value.level == "ir.data"
    ir = [
        "match",
        ["if", var("c"), var("r"), var("r")],  # cost 1 + 1 + max(1, 1) = 3
        [
            arm(["ok", ["bind", "x"]], ["add", var("x"), ["add", lit("Nat", 2), lit("Nat", 3)]]),
            arm(["err", "A"], lit("Int", 0)),
            arm(["err_any"], lit("Int", 1)),
        ],
    ]
    env = {"c": BOOL, "r": A.Ty("Result", (NAT, A.Ty("E")))}
    c = typed(ir, env)
    assert c.cost == 1 + 3 + (1 + 1 + (1 + 1 + 1))  # the Ok arm is the worst one: 5


def test_exactly_one_arm_is_evaluated():
    boom = ["add", lit("Int", (1 << 127) - 1), lit("Int", (1 << 127) - 1)]  # ir-overflow if run
    ir = [
        "match",
        var("r"),
        [arm(["ok", ["wild"]], lit("Nat", 1)), arm(["err_any"], ["eq", boom, lit("Int", 0)])],
    ]
    # the type of the second arm is Bool, the first Nat: no common type
    with pytest.raises(OracleError):
        typed(ir, ENV)
    ir[2][0][1] = ["lit", True]
    c = typed(ir, ENV)
    assert c.fn({"r": ("ok", True)}) is True  # the err arm (which would overflow) never runs
    with pytest.raises(OracleError) as err:
        c.fn({"r": ("err", "A")})
    assert err.value.code == "ir-overflow"


def test_the_first_matching_arm_wins_and_binders_are_in_scope_only_in_their_arm():
    ir = [
        "match",
        var("r"),
        [
            arm(["ok", ["tuple", [["bind", "a"], ["bind", "b"]]]], ["tuple", [var("b"), var("a")]]),
            arm(["err", "A"], ["tuple", [["lit", False], ["lit", False]]]),
            arm(["err_any"], ["tuple", [["lit", True], ["lit", True]]]),
        ],
    ]
    c = typed(ir, {"r": RESULT_PAIR})
    assert str(c.ty) == "(Bool,Bool)"
    assert c.fn({"r": ("ok", (True, False))}) == (False, True)
    assert c.fn({"r": ("err", "A")}) == (False, False)
    assert c.fn({"r": ("err", "C")}) == (True, True)
    leak = ["and", ir, var("a")]
    with pytest.raises(OracleError):
        typed(["tuple", [ir, var("a")]], {"r": RESULT_PAIR})  # `a` is not in scope after the match
    del leak


def test_the_arms_of_a_match_unify_results_with_unknown_error_types():
    ir = [
        "match",
        var("r"),
        [
            arm(["ok", ["bind", "x"]], ["ok", ["tuple", [var("x"), var("x")]]]),
            arm(["err_any"], ["err", "B"]),
        ],
    ]
    c = typed(ir, {"r": RESULT_BOOL})
    assert c.ty.head == "Result" and str(c.ty.args[0]) == "(Bool,Bool)"
    assert c.fn({"r": ("ok", True)}) == ("ok", (True, True))
    assert c.fn({"r": ("err", "A")}) == ("err", "B")


def test_ir_match_errors():
    def run(ir, env=None):
        with pytest.raises(OracleError) as err:
            typed(ir, env or ENV)
        return err.value

    wild = arm(["wild"], ["lit", True])
    assert run(["match", var("r")]).code == "ir"  # no arms
    assert run(["match", var("r"), []]).code == "ir"
    assert run(["match", var("r"), [["wild"]]]).code == "ir"  # an arm is a pair
    assert run(["match", var("r"), "arms"]).code == "ir"
    # the scrutinee is a Bytes: another level
    e = run(["match", var("b"), [wild]], {"b": A.Ty("Bytes")})
    assert e.code == "level" and e.level == "ir.data"
    # malformed patterns
    for pattern in (
        "wild",
        ["frob"],
        ["wild", 1],
        ["bind"],
        ["bind", 3],
        ["bind", "_"],
        ["bind", "1x"],
        ["ok"],
        ["err"],
        ["err", 3],
        ["err", ""],
        ["err_any", 1],
        ["tuple"],
        ["tuple", [["wild"]]],
        ["tuple", "ab"],
        ["tuple", [["wild"], ["wild"]]],  # a tuple pattern on a Result
    ):
        assert run(["match", var("r"), [arm(pattern, ["lit", True]), wild]]).code == "ir", pattern
    # an ok pattern on a non-Result payload and an err case the error type does not have
    assert run(["match", var("r"), [arm(["ok", ["ok", ["wild"]]], ["lit", True]), wild]]).code == "ir"
    assert run(["match", var("r"), [arm(["err", "Z"], ["lit", True]), wild]]).code == "ir"


def test_ir_match_corpus_errors():
    def corpus(arms, env=None):
        with pytest.raises(OracleError) as err:
            typed(["match", var("r"), arms], env or ENV)
        assert err.value.code == "corpus"
        return err.value.message

    true = ["lit", True]
    assert "fall through" in corpus([arm(["ok", ["wild"]], true)])
    assert "fall through" in corpus([arm(["err", "A"], true), arm(["ok", ["wild"]], true)])
    assert "no value can reach" in corpus([arm(["wild"], true), arm(["ok", ["wild"]], true)])
    assert "no value can reach" in corpus(
        [arm(["ok", ["wild"]], true), arm(["err_any"], true), arm(["err", "A"], true)]
    )
    assert "twice" in corpus(
        [arm(["ok", ["tuple", [["bind", "a"], ["bind", "a"]]]], true), arm(["err_any"], true)],
        {"r": RESULT_PAIR},
    )
    for name, env in (("r", ENV), ("result", ENV), ("a", {**ENV, "a": BOOL})):
        assert "shadows" in corpus([arm(["ok", ["bind", name]], true), arm(["err_any"], true)], env)
    nested = [
        "match",
        var("r"),
        [
            arm(
                ["ok", ["bind", "x"]],
                ["match", var("x"), [arm(["ok", ["bind", "x"]], true), arm(["err_any"], true)]],
            ),
            arm(["err_any"], true),
        ],
    ]
    with pytest.raises(OracleError) as err:
        typed(nested, {"r": A.Ty("Result", (RESULT_BOOL, A.Ty("E")))})
    assert err.value.code == "corpus"


def test_an_unknown_error_type_is_an_open_set_so_a_case_never_completes_it():
    unknown = A.Ty("Result", (BOOL, A.Ty("?")))
    ir = ["match", ["err", "A"], [arm(["ok", ["wild"]], ["lit", True]), arm(["err", "A"], ["lit", True])]]
    with pytest.raises(OracleError) as err:
        typed(ir)
    assert err.value.code == "corpus" and "fall through" in err.value.message
    ir[2].append(arm(["err_any"], ["lit", False]))
    assert typed(ir).ty == BOOL
    del unknown


def test_a_program_without_the_error_table_cannot_check_cases_but_still_checks_shape():
    ir = [
        "match",
        var("r"),
        [arm(["ok", ["wild"]], ["lit", True]), arm(["err", "Q"], ["lit", True]), arm(["err_any"], ["lit", True])],
    ]
    assert Program({}, {}).compile(ir, ENV).ty == BOOL  # E unknown to this program: any case


def test_a_pattern_binder_of_type_option_is_not_possible_and_a_match_arm_may_not_be_an_option():
    ir = [
        "match",
        var("r"),
        [
            arm(["ok", ["wild"]], ["lit", True]),
            arm(["err_any"], ["eq", ["lit", True], ["lit", True]]),
        ],
    ]
    assert typed(ir, ENV).ty == BOOL


def test_match_nesting_counts_against_the_depth_limit():
    ir = ["lit", True]
    for _ in range(70):
        ir = ["match", var("r"), [arm(["ok", ["wild"]], ir), arm(["err_any"], ["lit", True])]]
    with pytest.raises(OracleError) as err:
        typed(ir, ENV)
    assert err.value.code == "limit"


# -- requires (R9): tags and types ----------------------------------------------------------------------------


def test_requires_of_a_match_is_ir_tuple():
    assert L.ir_levels(
        ["match", var("r"), [arm(["ok", ["bind", "x"]], var("x")), arm(["wild"], ["lit", True])]]
    ) == ["ir.core", "ir.tuple"]
    # a pattern is never walked as an expression: ``["tuple", [..]]`` and ``["ok", ..]`` inside it
    assert L.ir_levels(["match", var("r"), [arm(["ok", ["tuple", [["wild"], ["wild"]]]], ["lit", True])]]) == [
        "ir.core",
        "ir.tuple",
    ]
    with pytest.raises(OracleError) as err:
        L.ir_levels(["match", var("r")])
    assert err.value.code == "ir"


def test_requires_counts_a_tuple_type_without_any_tuple_tag():
    # an op that returns a tuple, compared with eq against a model call: no tuple tag at all
    spec = lower(
        "warrant t v1\n  op f(a: U8) -> (U8,U8)\n  model m(a: U8) -> (U8,U8) = (a, a)\n"
        "  claim c:\n    forall a: U8. f(a) == m(a)\n"
    )
    assert spec.suites[0].requires == ("ir.core", "ir.tuple")
    assert spec.ops["f"].requires == ("ir.core", "ir.tuple")  # the result type, no admit
    # ... or Result<(U64,Nat),E> compared with eq
    spec = lower(
        "warrant t v1\n  error E = A\n  op f(a: U8) -> Result<(U64,Nat),E>\n"
        "  claim c:\n    forall a: U8. f(a) == f(a)\n"
    )
    assert spec.suites[0].requires == ("ir.core", "ir.tuple")


def test_requires_counts_each_place_a_tuple_type_can_hide():
    base = "warrant t v1\n  error E = A\n"
    cases = {
        "a binder": "  op f(a: U8) -> U8\n  claim c:\n    forall t: (U8,U8). f(t.0) == t.0\n",
        "a model parameter": "  op f(a: U8) -> U8\n  model m(t: (U8,U8)) -> U8 = t.0\n  claim c:\n    forall a: U8. f(a) == m((a, a))\n",
        "a model result": "  op f(a: U8) -> U8\n  model m(a: U8) -> (U8,U8) = (a, a)\n  claim c:\n    forall a: U8. f(a) == m(a).0\n",
        "an op parameter": "  op f(t: (U8,U8)) -> U8\n  claim c:\n    forall a: U8. f((a, a)) == a\n",
        "an example value": "  op f(a: U8) -> U8\n  example e:\n    f(1u8) == (1u8, 2u8).0\n",
        "a pattern binder": "  op f(r: Result<(U8,U8),E>) -> U8\n  claim c:\n    forall r: Result<(U8,U8),E>. f(r) == (match r with Ok(t) -> t.0 | Err(_) -> 0u8)\n",
    }
    for name, text in cases.items():
        spec = lower(base + text)
        unit = spec.suites[0] if spec.suites else spec.examples[0]
        assert "ir.tuple" in unit.requires, name
    # no tuple type anywhere: no ir.tuple
    spec = lower(base + "  op f(a: U8) -> U8\n  claim c:\n    forall a: U8. f(a) == a\n")
    assert spec.suites[0].requires == ("ir.core",)


def test_requires_takes_the_call_closure_through_a_model_that_uses_a_tuple():
    spec = lower(
        "warrant t v1\n  op f(a: Bool) -> Bool\n"
        "  model inner(a: Bool) -> Bool = (a, a).0\n  model outer(a: Bool) -> Bool = inner(a)\n"
        "  claim c:\n    forall a: Bool. f(a) == outer(a)\n"
    )
    assert spec.models["outer"].requires == ("ir.core", "ir.tuple")
    assert spec.suites[0].requires == ("ir.core", "ir.tuple")


def test_requires_unions_ir_fixed_and_ir_tuple_and_the_suite_is_evaluable():
    spec = lower(
        "warrant t v1\n  op f(a: U8) -> (U8,U8)\n"
        "  claim c:\n    forall a: U8. f(a) == (bit_not(a), a)\n"
    )
    suite = spec.suites[0]
    assert suite.requires == ("ir.core", "ir.fixed", "ir.tuple")
    assert suite.unsupported is None  # ir.fixed is evaluated since S10c-1


# -- the lzw acceptance conditions (VER04r 13, the lzw adopter row) ----------------------------------------------


LZW_MATCH = (
    "bomb_512_units_is_under_the_cap_and_decodes",
    "bomb_1024_units_declaring_exactly_the_cap_delivers_the_cap",
)


def test_lzw_match_examples_lower_and_are_not_op_eq_pure():
    """The two ``match decompress(..) with Ok/Err`` bomb examples lower (S10t-1) and classify as
    ``not-op-eq-pure`` under an ``omit_example`` line.  Since S10c-1 they are evaluable at every
    level they use (``be_bytes`` in ``bomb512``), and the four eq-shaped ones classify as
    ``value-over-limit`` (``tests/test_fixed_lzw.py``)."""
    from coding_adventures_warrant_oracle.corpus_v2 import example_id, omit_reason

    spec = lower(adopter_bytes("lzw"))
    prog = build_program(spec, max_cost=65_536)
    by_name = {x.name: x for x in spec.examples}
    for name in LZW_MATCH:
        ex = by_name[name]
        # it lowered: a ``match`` with the scrutinee ``decompress(..)`` and the arms Ok(r) / Err(e)
        assert ex.body[0] == "match" and ex.body[1][0] == "subject_op"
        assert ex.body[2][0][0] == ["ok", ["bind", "r"]] and ex.body[2][1][0] == ["err_any"]
        assert ex.unsupported is None and set(ex.requires) == {"ir.core", "ir.fixed", "ir.tuple"}
        assert omit_reason(spec, prog, ex) == "not-op-eq-pure"
        assert example_id(spec, ex) == f"warrant/lzw/example/{name}"


def test_lzw_match_examples_are_omitted_by_the_profile_line_machinery():
    """The omit path on the two lzw match examples alone (the rest of the document needs S10c)."""
    from coding_adventures_warrant_oracle.corpus_v2 import _examples, example_id

    spec = lower(adopter_bytes("lzw"))
    prog = build_program(spec, max_cost=65_536)
    spec.examples = [x for x in spec.examples if x.name in LZW_MATCH]
    ids = [example_id(spec, x) for x in spec.examples]
    rows, tfv_text, omitted = _examples(spec, prog, ids)
    assert rows == [] and tfv_text == b"tfv 1\n"
    assert omitted == [{"id": i, "reason": "not-op-eq-pure"} for i in sorted(ids)]


def test_the_adopters_with_tuples_lower_without_a_tuple_level_refusal():
    for name in ("wasm-leb128", "logic-gates", "arithmetic"):
        spec = lower(adopter_bytes(name))
        for unit in [*spec.suites, *spec.examples]:
            if unit.unsupported is not None:
                assert unit.unsupported.level != "ir.tuple", name


def test_model_def_programs_accept_a_match_in_a_model_body():
    body = ["match", var("r"), [arm(["ok", ["bind", "x"]], var("x")), arm(["err_any"], ["lit", False])]]
    prog = Program({"m": ModelDef("m", [("r", RESULT_BOOL)], BOOL, body)}, {}, errors=ERRORS)
    c = prog.compile(["model_call", "m", [var("r")]], {"r": RESULT_BOOL})
    assert c.cost == 1 + 1 + (1 + 1 + 1)
    assert c.fn({"r": ("ok", True)}) is True


# -- a few more edges ---------------------------------------------------------------------------------------------


def test_projection_of_a_record_lowers_to_field_and_needs_ir_data():
    spec = lower(
        "warrant r v1\n  record Point(x: U8, y: U8)\n  op f(a: U8) -> Point\n"
        "  model m(p: Point) -> U8 = p.y\n"
    )
    m = spec.models["m"]
    assert m.ir == ["field", ["var", "p"], 1] and m.requires == ("ir.core", "ir.data")
    assert m.lowered and m.unsupported is None  # evaluated since S11c-1 (S11a: the evaluation refusal)
    spec = lower(
        "warrant r v1\n  record Point(x: U8, y: U8)\n  op f(a: U8) -> Point\n"
        "  claim c:\n    forall a: U8. f(a).x == a\n"
    )
    s = spec.suites[0]
    assert s.lowered and s.requires == ("ir.core", "ir.data")
    assert s.subject == ["field", ["subject_op", "f", [["var", "a"]]], 0]


def test_a_match_with_no_context_types_its_arms_from_the_first_typed_arm():
    # a comparison operand has no expected type, so the first arm's type is the hint for the next
    m = model("(match r with Ok(_) -> 1 | Err(_) -> 2) == 1", "r: Result<Bool,E>")
    assert m.unsupported is None
    assert m.ir[1][2][0][1] == lit("Int", 1)


def test_ir_tuple_and_match_shape_limits():
    for node in (["tuple"], ["tuple", [lit("Nat", 1), lit("Nat", 2)], 3]):
        with pytest.raises(OracleError) as err:
            typed(node)
        assert err.value.code == "ir"
    arms = [[["err", "A"], ["lit", True]]] + [[["wild"], ["lit", True]]] * 256
    with pytest.raises(OracleError) as err:
        typed(["match", var("r"), [[["ok", ["wild"]], ["lit", True]], *arms]], ENV)
    assert err.value.code == "limit"
