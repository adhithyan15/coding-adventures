"""Slice S10c-1: the domains of [D21], [D25] and [D26] and the shift-count rule R10 (VER04r 5.1).

* [D21] the signed ``fx_edge`` set (527 / 559 / 623 values, ascending) is checked in ``test_domains``;
* [D25] a 16-bit ``fx_edge`` exists by budget demotion only, an 8-bit one is illegal;
* [D26] ``nat_shift`` (73 values) is chosen for a ``Nat`` binder that **is** the count operand of
  ``shl shr sar rotl rotr bit`` in the claim body, and for nothing else (R10).
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.domains import count_binders, descriptor_size
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.suites import SuitePlan

V = lambda n: ["var", n]  # noqa: E731
N1 = ["lit", {"type": "Nat", "value": "1"}]


# -- count_binders: only the count operand itself selects ----------------------------------------


@pytest.mark.parametrize("tag", ["shl", "shr", "sar", "rotl", "rotr", "bit"])
def test_the_count_operand_of_every_shift_builtin_selects(tag):
    assert count_binders(["eq", [tag, V("a"), V("n")], V("b")]) == {"n"}
    # the *first* operand is the value, never a count
    assert count_binders([tag, V("n"), N1]) == set()


def test_only_a_bare_binder_selects_and_other_counts_do_not():
    expr = ["add", V("n"), N1]
    assert count_binders(["shl", V("a"), expr]) == set()  # an expression of the binder
    assert count_binders(["shl", V("a"), ["project", V("t"), 1]]) == set()  # a projection
    assert count_binders(["shl", V("a"), N1]) == set()  # a literal
    # a count passed through a model_call argument does not select
    assert count_binders(["model_call", "m", [V("a"), V("n")]]) == set()
    # ... but a shift *inside* an argument does
    assert count_binders(["model_call", "m", [["shl", V("a"), V("k")]]]) == {"k"}


def test_the_walk_follows_subject_args_tuple_operands_and_match_arms_but_never_patterns():
    sub = ["subject_op", "f", [["shr", V("a"), V("n")]]]
    assert count_binders(sub) == {"n"}
    assert count_binders(["tuple", [["shl", V("a"), V("n")], V("a")]]) == {"n"}
    arm = ["match", V("r"), [[["ok", ["bind", "x"]], ["rotl", V("x"), V("k")]], [["err_any"], ["lit", True]]]]
    assert count_binders(arm) == {"k"}
    # a pattern shaped like a shift node is not an expression
    fake = ["match", V("r"), [[["ok", ["bind", "shl"]], ["lit", True]], [["err_any"], ["lit", False]]]]
    assert count_binders(fake) == set()
    # nested and in the condition of an if
    deep = ["if", ["bit", V("a"), V("i")], ["shl", V("a"), V("j")], V("a")]
    assert count_binders(deep) == {"i", "j"}


# -- end to end through the lowerer and the suite plan -----------------------------------------------

HEAD = """warrant s v1
  error E = A
  op f(a: U8, n: Nat) -> U8
  op g(a: U8, n: Nat, k: Nat) -> U8
  op p(a: U8, n: Nat) -> (U8,U8)
  op r(n: Nat, x: Result<U8,E>) -> Bool
  op tt(t: (U8,Nat)) -> U8
  op s(a: I8, n: Nat) -> I8
  model m(a: U8, n: Nat) -> U8 = shl(a, n)
"""


def plan(claim: str, *, version_head: str = HEAD, budget: int = 1 << 24):
    spec = lower_document(parse_source((version_head + claim).encode()))
    prog = build_program(spec)
    return SuitePlan(spec, prog, spec.suites[0], budget=budget)


def descs(claim: str, **kw) -> dict:
    return {b.name: b.desc.to_json() for b in plan(claim, **kw).binders}


SHIFT = {"d": "nat_shift", "size": 73}
COUNT = {"d": "nat_0_64", "size": 65}


@pytest.mark.parametrize("tag", ["shl", "shr", "rotl", "rotr", "bit"])
def test_a_nat_binder_that_is_the_count_gets_nat_shift(tag):
    pure = "(if bit(a, n) then a else a)" if tag == "bit" else f"{tag}(a, n)"
    claim = f"  claim c:\n    forall a: U8, n: Nat. f(a, n) == {pure}\n"
    d = descs(claim)
    assert d["n"] == SHIFT and d["a"]["d"] == "u8_all"
    assert plan(claim).product == 256 * 73


def test_sar_selects_on_a_signed_operand():
    d = descs("  claim c:\n    forall a: I8, n: Nat. s(a, n) == sar(a, n)\n")
    assert d["n"] == SHIFT and d["a"] == {"d": "fx_full", "t": "I8", "size": 256}


def test_an_expression_of_the_binder_does_not_select():
    d = descs('  claim c:\n    forall a: U8, n: Nat. f(a, n) == shl(a, len(take(b"0102030405", n)))\n')
    assert d["n"] == COUNT


def test_a_model_call_argument_does_not_select_even_if_the_model_shifts_by_it():
    d = descs("  claim c:\n    forall a: U8, n: Nat. f(a, n) == m(a, n)\n")
    assert d["n"] == COUNT


def test_a_binder_that_is_a_count_only_in_the_admit_does_not_select():
    d = descs("  claim c:\n    forall a: U8, n: Nat | bit(a, n). f(a, n) == a\n")
    assert d["n"] == COUNT
    # ... and one that is a count in the body as well does
    d = descs("  claim c:\n    forall a: U8, n: Nat | bit(a, n). f(a, n) == shl(a, n)\n")
    assert d["n"] == SHIFT


def test_only_the_count_binder_of_several_nat_binders_selects():
    d = descs("  claim c:\n    forall a: U8, n: Nat, k: Nat. g(a, n, k) == shl(a, n)\n")
    assert (d["n"], d["k"]) == (SHIFT, COUNT)


def test_a_projection_of_a_tuple_binder_does_not_select():
    d = descs("  claim c:\n    forall t: (U8,Nat). tt(t) == shl(t.0, t.1)\n")
    assert d["t"]["d"] == "tuple" and d["t"]["of"][1] == COUNT


def test_a_shift_inside_a_tuple_literal_and_a_match_arm_selects():
    d = descs("  claim c:\n    forall a: U8, n: Nat. p(a, n) == (shl(a, n), a)\n")
    assert d["n"] == SHIFT
    d = descs(
        "  claim c:\n    forall n: Nat, x: Result<U8,E>. "
        "match x with Ok(v) -> f(v, n) == shr(v, n) | Err(_) -> true\n"
    )
    assert d["n"] == SHIFT


def test_a_subject_argument_that_shifts_selects():
    d = descs("  claim c:\n    forall a: U8, n: Nat. f(shl(a, n), 1) == a\n")
    assert d["n"] == SHIFT


def test_a_v0_document_never_refines():
    # no built-ins exist in a v0 document, so there is no count to find: the plain Nat domain stays
    spec = lower_document(
        parse_source(b"warrant s\n  op f(n: Nat) -> Nat\n  claim c:\n    forall n: Nat. f(n) == n\n")
    )
    p = SuitePlan(spec, build_program(spec), spec.suites[0])
    assert p.binders[0].desc.to_json() == COUNT


def test_the_nat_shift_domain_is_not_demoted_and_still_counts_toward_the_budget():
    claim = "  claim c:\n    forall a: U8, n: Nat. f(a, n) == shl(a, n)\n"
    assert plan(claim, budget=256 * 73).product == 256 * 73
    with pytest.raises(OracleError) as err:
        plan(claim, budget=256 * 73 - 1)  # nothing to demote: a U8 is never demoted
    assert "W0706" in err.value.message


def test_16_bit_binders_demote_to_fx_edge_by_budget_only_d25():
    head = HEAD + "  op w(a: U16, b: U16) -> U16\n  op v(a: I16, n: Nat) -> I16\n"
    full = plan("  claim c:\n    forall a: U16. w(a, a) == a\n", version_head=head)
    assert full.binders[0].desc.to_json() == {"d": "fx_full", "t": "U16", "size": 65536}
    assert full.demoted == []
    two = plan("  claim c:\n    forall a: U16, b: U16. w(a, b) == a\n", version_head=head, budget=1 << 20)
    assert [b.desc.to_json() for b in two.binders] == [
        {"d": "fx_edge", "t": "U16", "size": 281},
        {"d": "fx_edge", "t": "U16", "size": 281},
    ]
    assert two.demoted == ["0", "1"] and two.product == 281 * 281
    sh = plan("  claim c:\n    forall a: I16, n: Nat. v(a, n) == shl(a, n)\n", version_head=head, budget=1 << 20)
    assert [b.desc.to_json() for b in sh.binders] == [
        {"d": "fx_edge", "t": "I16", "size": 527},
        {"d": "nat_shift", "size": 73},
    ]


# -- the consumer-side descriptor check --------------------------------------------------------------


def test_descriptor_size_recomputes_and_accepts_every_legal_leaf():
    for d in (
        {"d": "bool", "size": 2},
        {"d": "u8_all", "size": 256},
        {"d": "u64_edge", "size": 377},
        {"d": "nat_0_64", "size": 65},
        {"d": "nat_count", "k": 5, "size": 6},
        {"d": "nat_shift", "size": 73},
        {"d": "bytes_d1", "size": 347},
        {"d": "bytes_n", "n": 2, "size": 16},
        {"d": "fx_full", "t": "I8", "size": 256},
        {"d": "fx_full", "t": "U16", "size": 65536},
        {"d": "fx_edge", "t": "U32", "size": 313},
        {"d": "fx_edge", "t": "I64", "size": 623},
    ):
        assert descriptor_size(d) == d["size"]


def test_descriptor_size_checks_composites():
    shift = {"d": "nat_shift", "size": 73}
    assert descriptor_size({"d": "tuple", "of": [shift, {"d": "bool", "size": 2}], "size": 146}) == 146
    assert descriptor_size({"d": "option", "of": shift, "size": 74}) == 74
    assert descriptor_size({"d": "result", "ok": shift, "errs": ["A", "B"], "size": 75}) == 75
    assert descriptor_size({"d": "list", "of": {"d": "bool", "size": 2}, "size": 1 + 2 + 4 + 8}) == 15
    variant = {"d": "variant", "ctors": [["A", []], ["B", [shift]]], "size": 74}
    assert descriptor_size(variant) == 74
    deep = {"d": "bool", "size": 2}
    for _ in range(15):
        deep = {"d": "option", "of": deep, "size": deep["size"] + 1}
    assert descriptor_size(deep) == 17  # 16 levels deep: allowed
    deep = {"d": "option", "of": deep, "size": deep["size"] + 1}
    with pytest.raises(OracleError) as err:
        descriptor_size(deep)
    assert err.value.code == "limit"


@pytest.mark.parametrize(
    "bad",
    [
        {"d": "fx_edge", "t": "U8", "size": 256},  # [D25]: 8-bit is illegal
        {"d": "fx_edge", "t": "I8", "size": 256},
        {"d": "fx_edge", "t": "U64", "size": 377},  # a U64 binder is u64_edge in every version
        {"d": "fx_edge", "t": "I64", "size": 377},  # the withdrawn pattern-order size
        {"d": "fx_edge", "t": "I16", "size": 281},
        {"d": "fx_full", "t": "U8", "size": 256},
        {"d": "fx_full", "t": "I32", "size": 1 << 32},
        {"d": "nat_shift", "size": 65},
        {"d": "nat_shift", "size": 73, "k": 64},
        {"d": "nat_count", "k": 5000, "size": 5001},
        {"d": "fx_edge", "t": "Q9", "size": 1},
        {"d": "fx_edge", "size": 1},
        {"d": "bytes_n", "n": 4097, "size": 1},
        {"d": "nope", "size": 1},
        {"d": "bool", "size": True},
        "bool",
        {"d": "tuple", "of": [], "size": 1},
        {"d": "tuple", "of": [{"d": "bool", "size": 2}], "size": 3},
        {"d": "result", "ok": {"d": "bool", "size": 2}, "errs": [1], "size": 3},
        {"d": "variant", "ctors": [], "size": 0},
        {"d": "variant", "ctors": [["A"]], "size": 1},
    ],
)
def test_a_consumer_rejects_an_illegal_descriptor(bad):
    with pytest.raises(OracleError):
        descriptor_size(bad)


def test_a_size_that_overflows_64_bits_is_refused_never_zero():
    big = {"d": "u64_edge", "size": 377}
    node = big
    for _ in range(8):  # 377 ** 8 > 2 ** 64
        node = {"d": "tuple", "of": [node, big], "size": node["size"] * 377}
    with pytest.raises(OracleError) as err:
        descriptor_size(node)
    assert "W0706" in err.value.message
