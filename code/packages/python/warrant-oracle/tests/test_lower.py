"""Lowering: syntax-directed IR, typing, level census, and the v1 corpus IR."""

from __future__ import annotations

import json
from collections import Counter

import pytest
from conftest import ADOPTERS, TINY, V1_CORPUS, adopter_bytes

from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source


def lower(text: str | bytes):
    return lower_document(parse_source(text))


def model_ir(body: str, params: str = "a: Nat, b: Nat", ret: str = "Bool", prelude: str = ""):
    spec = lower(f"warrant p v1\n{prelude}  model m({params}) -> {ret} = {body}\n")
    return spec.models["m"]


def test_ct_compare_lowering_equals_the_v1_corpus_ir():
    corpus = json.loads(V1_CORPUS.read_text())
    spec = lower(adopter_bytes("ct-compare"))
    assert {
        n: {"params": [p for p, _ in m.params], "body": m.ir} for n, m in spec.models.items()
    } == corpus["models"]
    rows = {s["claim"]: s for s in corpus["suites"]}
    for suite in spec.suites:
        row = rows[suite.claim]
        assert (suite.admit, suite.subject, suite.expected) == (
            row["admit"],
            row["lhs"],
            row["rhs"],
        )
        assert suite.ops == row["ops"] and str(suite.result) == row["result_type"]
    assert {n: o.corpus_row() for n, o in spec.ops.items()} == corpus["ops"]


# -- unoptimised, syntax-directed desugaring ----------------------------------------


def test_one_node_per_construct_no_folding():
    m = model_ir("1 + 2 == 3", params="a: Bool", ret="Bool", prelude="")
    # no constant folding: the literals and the add node all survive.
    assert m.ir == [
        "eq",
        ["add", ["lit", {"type": "Int", "value": "1"}], ["lit", {"type": "Int", "value": "2"}]],
        ["lit", {"type": "Int", "value": "3"}],
    ]


def test_not_equal_implication_and_negation_are_their_own_nodes():
    assert model_ir("a != b").ir == ["ne", ["var", "a"], ["var", "b"]]
    assert model_ir("a == b => true", params="a: Bool, b: Bool").ir[0] == "implies"
    assert model_ir("- a", params="a: Int", ret="Int").ir == ["neg", ["var", "a"]]


def test_no_reassociation_and_left_associativity():
    m = model_ir("a - b - a", params="a: U8, b: U8", ret="U8")
    assert m.ir == ["sub", ["sub", ["var", "a"], ["var", "b"]], ["var", "a"]]


def test_operands_stay_in_source_order():
    assert model_ir("b < a").ir == ["lt", ["var", "b"], ["var", "a"]]
    assert model_ir("b >= a").ir == ["ge", ["var", "b"], ["var", "a"]]


def test_bare_literals_are_typed_by_context():
    nat = model_ir("a == 1").ir
    assert nat[2] == ["lit", {"type": "Nat", "value": "1"}]
    nat_left = model_ir("1 == a").ir
    assert nat_left[1] == ["lit", {"type": "Nat", "value": "1"}]
    both = model_ir("1 == 1", params="x: Bool").ir
    assert both[1][1]["type"] == "Int"


def test_suffixed_literals_and_bytes_literals():
    assert model_ir("a + 1u8", params="a: U8", ret="U8").ir[2] == [
        "lit",
        {"type": "U8", "value": "1"},
    ]
    assert model_ir('a == b"00"', params="a: Bytes<1>").ir[2] == [
        "lit",
        {"type": "Bytes", "value": "00"},
    ]


def test_nat_arithmetic_is_int_and_widens():
    m = model_ir("a - b", ret="Int")
    assert m.ir[0] == "sub"
    with pytest.raises(OracleError):
        model_ir("a - b", ret="Nat")  # Nat - Nat is an Int and never narrows implicitly


def test_if_branches_unify_and_literals_follow_the_other_branch():
    m = model_ir("if true then 0 else a", ret="Nat")
    assert m.ir[2] == ["lit", {"type": "Nat", "value": "0"}]
    with pytest.raises(OracleError):
        model_ir("if true then 1u8 else true", ret="U8")


def test_result_context_for_ok_and_err():
    spec = lower(
        "warrant p\n  error E = BAD\n  op f(a: U8) -> Result<U8, E>\n"
        "  claim c:\n    forall a: U8. f(a) == Ok(a)\n  claim d:\n    forall a: U8. Err(E.BAD) == f(a)\n"
    )
    assert spec.suites[0].expected == ["ok", ["var", "a"]]
    assert spec.suites[1].expected == ["err", "BAD"]
    assert spec.suites[1].subject[0] == "subject_op"


def test_signed_minimum_literal_only_under_negation():
    m = model_ir("-128i8", params="x: Bool", ret="I8")
    assert m.ir == ["neg", ["lit", {"type": "I8", "value": "-128"}]]
    with pytest.raises(OracleError):
        model_ir("128i8", params="x: Bool", ret="I8")


def test_bytes_width_adds_through_concat():
    m = model_ir("concat(a, b)", params="a: Bytes<1>, b: Bytes<30>", ret="Bytes<31>")
    assert m.ir == ["concat", ["var", "a"], ["var", "b"]]
    with pytest.raises(OracleError):
        model_ir("concat(a, b)", params="a: Bytes, b: Bytes<30>", ret="Bytes<31>")


def test_take_and_drop_on_bytes_are_core():
    assert model_ir("take(a, 2)", params="a: Bytes", ret="Bytes").ir == [
        "take",
        ["var", "a"],
        ["lit", {"type": "Nat", "value": "2"}],
    ]
    assert model_ir("drop(a, 2)", params="a: Bytes", ret="Bytes").ir[0] == "drop"


# -- claims ---------------------------------------------------------------------------


def test_claim_classification_and_admit_folding():
    spec = lower(TINY)
    by = {s.claim: s for s in spec.suites}
    assert by["bump_is_inc"].kind == "reference" and by["bump_is_inc"].admit == ["lit", True]
    assert by["bump_commutes_with_itself"].kind == "predicate"
    assert by["bump_commutes_with_itself"].admit == ["ne", ["var", "a"], ["var", "b"]]
    assert by["pick_total"].kind == "reference"
    assert by["bump_is_inc"].id == "warrant/tiny/exhaustive/bump_is_inc"


def test_several_refinements_fold_left_into_binary_and():
    spec = lower(
        "warrant p\n  op f(a: U8) -> U8\n  claim c:\n"
        "    forall a: U8 | a != 0u8, b: U8 | b != 0u8, c: U8 | c != 1u8. f(a) == f(b)\n"
    )
    admit = spec.suites[0].admit
    assert (
        admit[0] == "and"
        and admit[1][0] == "and"
        and admit[2] == ["ne", ["var", "c"], ["lit", {"type": "U8", "value": "1"}]]
    )


def test_claim_without_a_quantifier_has_no_binders():
    spec = lower("warrant p\n  op f() -> Bool\n  claim c:\n    f() == true\n")
    assert spec.suites[0].binders == []


# -- level census (S10a: a unit is named by the first unlowerable construct in source order) -------------------------------

CENSUS = {
    "ct-compare": {"ir.core": 9},
    "gf256": {"ir.core": 38},
    # S10t-1: ``ir.tuple`` is lowered and evaluated, so no suite is *blocked* on it any more (was
    # wasm-leb128 10/24/8, logic-gates 28/3/2/55, arithmetic 5/45/4/9 for core/tuple-or-data/fixed).
    # S10c-1: ``ir.fixed`` is evaluated too, so a suite that needs it is live: the count under
    # ``ir.core`` is every suite the oracle evaluates.  Only ``ir.data`` still blocks one (was
    # wasm-leb128 18/24, logic-gates 28/3/57 core/data/fixed, lzw 4/17, arithmetic 7/45/11,
    # x25519 6/2).
    "wasm-leb128": {"ir.core": 44},
    # S11c-1: ``ir.data`` is evaluated too, so nothing blocks a suite of logic-gates (was 85/3) or arithmetic (was 8/55).
    "logic-gates": {"ir.core": 88},
    "lzw": {"ir.core": 21},
    "arithmetic": {"ir.core": 64},  # was 63 before VER04r D76 split the 9-binder claim in two
    "x25519": {"ir.core": 8},
}


@pytest.mark.parametrize("name", ADOPTERS)
def test_level_census_per_adopter(name):
    spec = lower(adopter_bytes(name))
    census = Counter(s.unsupported.level if s.unsupported else "ir.core" for s in spec.suites)
    assert dict(census) == CENSUS[name]
    for s in spec.suites:
        if s.unsupported:
            assert s.unsupported.code == "level" and s.unsupported.level.startswith("ir.")


def test_unsupported_models_are_recorded_not_fatal():
    spec = lower(adopter_bytes("arithmetic"))
    # S11c-1: the 19 models that needed ``ir.data`` (S11a gave them the evaluation refusal) are evaluated now
    assert [m for m in spec.models.values() if m.unsupported is not None] == []
    assert sum(1 for m in spec.models.values() if "ir.data" in m.requires) == 19
    # a reserved level is still recorded, not fatal
    listy = lower("warrant p v1\n  model g(a: U8) -> Bool = [a] == [a]\n")
    assert listy.models["g"].unsupported.level == "ir.list" and not listy.models["g"].lowered
    wasm = lower(adopter_bytes("wasm-leb128"))
    assert wasm.models["low7"].unsupported is None and wasm.models["low7"].requires == (
        "ir.core",
        "ir.fixed",
    )


@pytest.mark.parametrize(
    "body,level",
    [
        # S11a: lists are the reserved ``ir.list``; ``Option`` and ``match`` lower and need ``ir.data``
        # (the unit was then refused only at *evaluation*, with the same level name).  S11c-1: they are
        # evaluated, so they are no longer refusals at all (``test_data_is_lowered_and_evaluated_not_refused``).
        ("[a] == [a]", "ir.list"),
        ("all(a, a)", "ir.seq"),
        ("range(a)", "ir.seq"),
        ("len([a]) == 1", "ir.list"),
        ("to_list(a)", "ir.list"),
        ("replicate(a, 2)", "ir.list"),
        ("fixed<4>(a)", "fixed-cast"),
    ],
)
def test_level_errors_name_the_level(body, level):
    m = model_ir(body, params="a: U8", ret="Bool")
    assert m.unsupported is not None and m.unsupported.level == level


@pytest.mark.parametrize("body", ["div(a, a) == div(a, a)", "match a with x -> true"])
def test_data_is_lowered_and_evaluated_not_refused(body):
    m = model_ir(body, params="a: U8", ret="Bool")
    assert m.lowered and m.unsupported is None and "ir.data" in m.requires


def test_suite_inherits_the_level_of_a_model_it_calls():
    spec = lower(
        "warrant p v1\n  op f(a: U8) -> U8\n  model g(a: U8) -> Bool = [a] == [a]\n"
        "  claim c:\n    forall a: U8. g(a) == (f(a) == a)\n"
    )
    assert spec.suites[0].unsupported.level == "ir.list" and not spec.suites[0].lowered


def test_a_suite_that_calls_an_ir_fixed_model_is_live_and_requires_ir_fixed():
    spec = lower(
        "warrant p v1\n  op f(a: U8) -> U8\n  model g(a: U8) -> U8 = bit_not(a)\n"
        "  claim c:\n    forall a: U8. f(a) == g(a)\n"
    )
    assert spec.suites[0].unsupported is None
    assert spec.suites[0].requires == ("ir.core", "ir.fixed")


# -- type errors ------------------------------------------------------------------------


@pytest.mark.parametrize(
    "text",
    [
        "warrant p\n  error E = A\n  error E = B\n",  # duplicate name
        "warrant p\n  error E = A | A\n",  # duplicate case
        "warrant p\n  op f(a: Nat, a: Nat) -> Bool\n",  # duplicate parameter
        "warrant p\n  op f(a: Frob) -> Bool\n",  # unknown type
        "warrant p\n  op f(a: Bytes<N>) -> Bool\n",  # unknown static width
        "warrant p\n  op f(a: Bytes<5000>) -> Bool\n",
        "warrant p\n  op f(a: Result<Nat, Nat>) -> Bool\n",  # error type must be declared
        "warrant p\n  op f(a: Nat<1>) -> Bool\n",  # primitive with arguments
        "warrant p\n  model A = Nat\n  model A = Bool\n",
        "warrant p\n  model A = Nat\n  op f(a: A<1>) -> Bool\n",
        "warrant p\n  error E = X\n  op f(a: E<1>) -> Bool\n",
        "warrant p\n  model m(a: Nat) -> Bool = b == 1\n",  # unbound name
        "warrant p\n  model m(a: Nat) -> Bool = E.x\n",  # qualified outside Err
        "warrant p\n  model m(a: Nat) -> Bool = a\n",  # body type mismatch
        "warrant p\n  model m(a: Nat, a: Nat) -> Bool = true\n",
        "warrant p\n  model m(a: Nat | a > 0) -> Bool = true\n",  # models have no refinements
        "warrant p\n  model m(a: Nat) -> Bool = len(a) == 0\n",
        "warrant p\n  model m(a: Nat) -> Bool = nope(a)\n",
        "warrant p\n  model m(a: U8) -> U8 = a + 1\n",  # fixed never mixes with Int
        "warrant p\n  model m(a: U8, b: Bool) -> Bool = a == b\n",
        "warrant p\n  model m(a: Bool) -> Bool = a < a\n",
        "warrant p\n  model m(a: U8) -> U8 = - a\n",  # unary minus on unsigned
        "warrant p\n  model m(a: Bool) -> Bool = - a\n",
        "warrant p\n  model m(a: Nat) -> Bool = not a\n",
        "warrant p\n  model m(a: Nat) -> Bool = a and a\n",
        "warrant p\n  model m(a: Nat) -> Bool = if a then true else true\n",
        'warrant p\n  model m(a: Nat) -> Bool = len(b"00", b"00") == 0\n',
        "warrant p\n  model m(a: Nat) -> Bool = 256u8 == 1u8\n",  # out of range
        'warrant p\n  model m(a: Nat) -> Bool = take(b"00", true) == b""\n',
        "warrant p\n  error E = A\n  op f() -> Result<Nat, E>\n  claim c:\n    f() == Ok(true)\n",
        "warrant p\n  error E = A\n  op f() -> Result<Nat, E>\n  claim c:\n    f() == Err(E.NOPE)\n",
        "warrant p\n  error E = A\n  error F = B\n  op f() -> Result<Nat, E>\n  claim c:\n    f() == Err(F.B)\n",
        "warrant p\n  error E = A\n  op f() -> Result<Nat, E>\n  claim c:\n    f() == Err(3)\n",
        "warrant p\n  op f() -> Nat\n  claim c:\n    f() == Ok(1)\n",
        "warrant p\n  op f() -> Nat\n  claim c:\n    f() == Err(E.A)\n",
        "warrant p\n  op f(a: Nat) -> Nat\n  claim c:\n    f() == 1\n",  # wrong arity
        "warrant p\n  op f(a: Nat) -> Nat\n  claim c:\n    f(true) == 1\n",
        'warrant p\n  op f<N: Nat>(a: Bytes<N>) -> Bool\n  claim c:\n    f(b"") == true\n',
        "warrant p\n  op f(a: Nat) -> Nat\n  claim c:\n    exists a: Nat. f(a) == 1\n",
        "warrant p\n  op f(a: Nat) -> Nat\n  claim c:\n    forall a: Nat, a: Nat. f(a) == 1\n",
        "warrant p\n  op f(a: Nat) -> Nat\n  claim c:\n    forall a: Nat | a. f(a) == 1\n",
        "warrant p\n  op f(a: Nat) -> Nat\n  claim c:\n    forall a: Nat. f(a)\n",
        "warrant p\n  op f(a: Nat) -> Bool\n  claim c:\n    forall a: Nat. forall b: Nat. f(a)\n",
        "warrant p v1\n  op f(a: Nat) -> Bool\n  claim c:\n    forall a: inout Nat. f(a)\n",
        "warrant p\n  claim c:\n    true\n  claim c:\n    true\n",  # duplicate test name
        "warrant p\n  model m() -> Bool = 1\n",
        "warrant p\n  example e:\n    1\n",
    ],
)
def test_ill_typed_documents_are_type_errors(text):
    with pytest.raises(OracleError) as err:
        lower(text)
    assert err.value.code in ("type", "limit", "generation")


def test_forward_and_self_model_references_are_generation_errors():
    with pytest.raises(OracleError) as err:
        lower("warrant p\n  model a(x: Bool) -> Bool = b(x)\n  model b(x: Bool) -> Bool = x\n")
    assert err.value.code == "generation"
    with pytest.raises(OracleError) as err:
        lower("warrant p\n  model a(x: Bool) -> Bool = a(x)\n")
    assert err.value.code == "generation"


def test_named_static_width_cannot_enter_the_ir():
    # only a literal width can be a static argument of a subject call in a claim
    with pytest.raises(OracleError):
        lower(
            "warrant p\n  op f<N: Nat>(a: Bytes<N>) -> Bool\n  claim c:\n    forall a: Bytes. f<M>(a) == true\n"
        )


def test_alias_resolution_and_depth():
    spec = lower("warrant p\n  model W = Bytes\n  op f(a: W) -> W\n")
    assert str(spec.ops["f"].ret) == "Bytes"
    chain = (
        "".join(f"  model A{i} = A{i + 1}\n" for i in range(20))
        + "  model A20 = Nat\n  op f(a: A0) -> Nat\n"
    )
    with pytest.raises(OracleError) as err:
        lower("warrant p\n" + chain)
    assert err.value.code == "limit"


def test_static_op_substitution_in_result_types():
    spec = lower(
        "warrant p\n  op f<N: Nat>(a: Bytes<N>) -> Bytes<N>\n"
        "  claim c:\n    forall a: Bytes<4>. f<4>(a) == a\n"
    )
    assert spec.suites[0].subject[3] == [4]
    assert str(spec.suites[0].result) == "Bytes<4>"


def test_static_arguments_on_a_plain_op_are_a_parse_level_error():
    # ``f<1>(1)`` on an op with no static parameters is a chained comparison.
    with pytest.raises(OracleError) as err:
        lower("warrant p\n  op f(a: Nat) -> Nat\n  claim c:\n    f<1>(1) == 1\n")
    assert err.value.code == "parse"


def test_projection_of_a_tuple_binder_lowers_since_s10t1():
    m = model_ir("a.0 == 1", params="a: (Nat, Nat)", ret="Bool")
    assert m.unsupported is None
    assert m.ir == ["eq", ["project", ["var", "a"], 0], ["lit", {"type": "Nat", "value": "1"}]]
    assert m.requires == ("ir.core", "ir.tuple")
