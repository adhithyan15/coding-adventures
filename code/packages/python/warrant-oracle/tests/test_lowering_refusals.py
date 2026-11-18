"""Lowering-time ``W0701`` refusals of a claim (CV2-F1, oracle half; VER04r 4.3 and 4.4).

Two rules the generator's lowering has always applied and the oracle used to apply only (the first)
when it *evaluated* a suite, or (the second) never:

* **4.4, an impure guard.**  A subject call may sit under a guard (the condition of an ``if``, the
  left operand of ``and``/``or``/``implies``, the scrutinee of a ``match`` whose arms call the
  subject) only if the guard calls no subject.
* **4.3, a claim that calls no op** has nothing to test: its suite would pass for any
  implementation.

The oracle's lowering is per item (a level-unsupported unit is recorded, not fatal), so the refusal
is recorded in ``LoweredSuite.refusal`` and raised by every consumer: the corpus derivation, the
enumeration and the lowering dump.  The cases mirror the unit tests of the generator's
``analysis.rs`` (``check_pure_guards``).
"""

from __future__ import annotations

import pytest
from conftest import adopter_bytes

from coding_adventures_warrant_oracle.corpus_v2 import derive
from coding_adventures_warrant_oracle.emit import emit
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import check_pure_guards, lower_document
from coding_adventures_warrant_oracle.lowering_dump import view
from coding_adventures_warrant_oracle.parser import parse_source

HEAD = """warrant guards v1
  error E = A | B
  op h(a: Bool) -> Result<Bool,E>
  op f(a: U8) -> U8
  op g(a: U8) -> U8
  assume platform:
    none
  assume environment:
    none
"""
OPS = ("h", "f", "g")


def document(claim: str, binders: str = "a: U8") -> bytes:
    text = HEAD
    for op in OPS:
        text += f"  labels {op}:\n    a secret\n"
    for op in OPS:
        text += f"  input {op}.a:\n    hostile\n    ensures no_panic, terminates\n"
    for op in OPS:
        text += f"  bound {op}:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n"
    return (text + f"  claim c:\n    forall {binders}. {claim}\n").encode()


def lowered(claim: str, binders: str = "a: U8"):
    return lower_document(parse_source(document(claim, binders))).suites[0]


IMPURE = [
    # an impure `if` condition, with the subject call in either branch
    "if f(a) == 1u8 then g(a) == 1u8 else true",
    "if f(a) == 1u8 then true else g(a) == 1u8",
    # an impure left operand guarding a subject right operand, for all three connectives
    "f(a) == 1u8 and g(a) == 1u8",
    "f(a) == 1u8 or g(a) == 1u8",
    "f(a) == 1u8 => g(a) == 1u8",
    # the rule applies inside arguments and nested nodes
    "g(a) == 1u8 and (f(a) == 1u8 or g(a) == 1u8)",
    "not (f(a) == 1u8 and g(a) == 1u8)",
    "(f(a) == 1u8 and g(a) == 1u8) == true",
    "f(if f(a) == 1u8 then g(a) else a) == 1u8",
]

PURE = [
    # a pure guard over a subject call
    "if a < 10u8 then f(a) == 1u8 else true",
    "a < 10u8 and f(a) == 1u8",
    "a < 10u8 or f(a) == 1u8",
    "a < 10u8 => f(a) == 1u8",
    # an impure operand that guards nothing that calls
    "f(a) == 1u8 and a < 10u8",
    "f(a) == 1u8 or true",
    "if f(a) == 1u8 then true else false",
    # no guard at all
    "f(a) == g(a)",
    "f(a) <= 3u8",
    "not (f(a) == 1u8)",
]


@pytest.mark.parametrize("claim", IMPURE)
def test_a_subject_call_under_an_impure_guard_is_refused_at_lowering(claim):
    suite = lowered(claim)
    assert suite.unsupported is None  # an ir.core claim: the level is not the reason
    assert suite.refusal is not None and suite.refusal.code == "generation"
    assert suite.refusal.message.startswith("W0701") and "impure guard" in suite.refusal.message
    # the claim is still lowered: its IR is what a census and the dump show
    assert suite.body != ["lit", True]


@pytest.mark.parametrize("claim", PURE)
def test_a_pure_guard_or_no_guard_is_accepted(claim):
    suite = lowered(claim)
    assert suite.refusal is None and suite.unsupported is None


@pytest.mark.parametrize("claim", IMPURE[:3])
def test_every_consumer_raises_the_refusal(claim):
    data = document(claim)
    with pytest.raises(OracleError) as err:
        derive(data, b"", spec_ref="g.warrant", generator={"name": "t", "version": "0"})
    assert err.value.code == "generation" and err.value.message.startswith("W0701")
    with pytest.raises(OracleError) as err:
        view(data)
    assert err.value.code == "generation" and "impure guard" in err.value.message
    row = emit(data)["suites"]["c"]
    assert row["status"] == "refused" and row["code"] == "generation"
    assert "impure guard" in row["reason"]


def test_match_scrutinee_is_a_guard_for_its_arms():
    base = "match {scrutinee} with Ok(_) -> {arm} | Err(_) -> true"
    bad = base.format(scrutinee="h(f(a) == 1u8)", arm="g(a) == 1u8")
    suite = lowered(bad)
    assert suite.refusal is not None and "impure guard" in suite.refusal.message
    # a pure scrutinee guards a subject arm
    pure = lowered(base.format(scrutinee="r", arm="g(a) == 1u8"), "a: U8, r: Result<Bool,E>")
    assert pure.refusal is None
    # an impure scrutinee with pure arms guards nothing that calls
    plain = lowered("match h(a < 10u8) with Ok(x) -> x | Err(_) -> a < 10u8", "a: U8")
    assert plain.refusal is None
    # the subject is only in the scrutinee: fine
    only = lowered("match h(f(a) == 1u8) with Ok(x) -> x | Err(_) -> true")
    assert only.refusal is None
    # the subject is in an arm and in the scrutinee through a nested node
    nested = lowered("a < 10u8 and match h(f(a) == 1u8) with Ok(_) -> g(a) == 1u8 | Err(_) -> true")
    assert nested.refusal is not None


def test_the_walk_is_depth_bounded_like_the_generators():
    ir = ["subject_op", "f", [["var", "a"]]]
    for _ in range(70):
        ir = ["not", ir]
    with pytest.raises(OracleError) as err:
        check_pure_guards(ir)
    assert err.value.code == "limit"


def test_an_ir_fixed_claim_that_is_impure_is_refused_by_both_the_evaluation_and_the_dump():
    # Until S10c-1 a claim that was both ir.fixed and impure showed the *level* first in evaluation
    # (the generator meets the level while typing) and the guard rule only in the dump, which
    # accepted ir.fixed.  ir.fixed is evaluated now, so there is no level to meet: both refuse the
    # guard (W0701, code ``generation``).
    claim = "f(a) == bit_not(a) or f(a) == a"
    data = document(claim)
    suite = lower_document(parse_source(data)).suites[0]
    assert suite.unsupported is None
    assert suite.refusal is not None and "impure guard" in suite.refusal.message
    with pytest.raises(OracleError) as err:
        derive(data, b"", spec_ref="g.warrant", generator={"name": "t", "version": "0"})
    assert err.value.code == "generation"
    with pytest.raises(OracleError) as err:
        view(data)
    assert err.value.code == "generation"


# -- 4.3: a claim that calls no operation --------------------------------------------------------

NO_OP = [
    ("a == a", "a: U8"),
    ("true", "a: U8"),
    ("a < 10u8 or a >= 10u8", "a: U8"),
    ("if a < 10u8 then true else a == a", "a: U8"),
]


@pytest.mark.parametrize(("claim", "binders"), NO_OP)
def test_a_claim_that_calls_no_op_is_refused_at_lowering(claim, binders):
    suite = lowered(claim, binders)
    assert suite.refusal is not None and suite.refusal.code == "generation"
    assert suite.refusal.message.startswith("W0701") and "at least one operation" in suite.refusal.message
    data = document(claim, binders)
    with pytest.raises(OracleError) as err:
        derive(data, b"", spec_ref="g.warrant", generator={"name": "t", "version": "0"})
    assert err.value.code == "generation" and "at least one operation" in err.value.message
    with pytest.raises(OracleError):
        view(data)


def test_a_claim_that_calls_an_op_only_inside_a_branch_still_calls_one():
    assert lowered("if a < 10u8 then f(a) == 1u8 else true").refusal is None


def test_the_adopters_are_unchanged_by_the_two_rules():
    # the committed adopter documents lower without a refusal.  wasm-leb128 used to carry two
    # impure-guard claims (`len(op(v)) >= 1 and len(op(v)) <= 10`, 42 claims, 2 refused); VER04r
    # [D28] split each into `len(op(v)) >= 1` and `len(op(v)) <= 10` (44 claims), so no committed
    # adopter has one now.  The claims it lost are asserted accepted, by name, and the rule itself
    # is still tested below with the old shape written as a synthetic claim.
    spec = lower_document(parse_source(adopter_bytes("wasm-leb128")))
    assert len(spec.suites) == 44
    assert [s.claim for s in spec.suites if s.refusal is not None] == []
    by_name = {s.claim: s for s in spec.suites}
    for op in ("unsigned", "signed"):
        names = [f"encode_{op}_at_least_one_byte", f"encode_{op}_at_most_ten_bytes"]
        assert all(by_name[n].refusal is None and by_name[n].unsupported is None for n in names)
        # the split claims sit side by side, `at_least_one_byte` first (the new one is inserted before)
        order = [s.claim for s in spec.suites]
        assert order.index(names[0]) + 1 == order.index(names[1])
    for name in ("ct-compare", "gf256"):
        assert all(s.refusal is None for s in lower_document(parse_source(adopter_bytes(name))).suites)


def test_the_old_wasm_leb128_shape_is_recorded_not_fatal():
    # the shape [D28] removed from wasm-leb128, as a synthetic claim: a refusal recorded on the
    # suite (the generator's per-item census records it the same way), never an error for the document
    suite = lowered("f(a) >= 1u8 and f(a) <= 10u8")
    assert suite.refusal is not None and suite.refusal.code == "generation"
    assert suite.refusal.message.startswith("W0701") and "impure guard" in suite.refusal.message
    # its two halves, as claims of their own, are accepted
    assert lowered("f(a) >= 1u8").refusal is None
    assert lowered("f(a) <= 10u8").refusal is None


# -- VER04r 4.2 rule 3: a model body calls no subject --------------------------------------------

MODEL_CASES = {
    "uncalled": "  model bad(a: U8) -> U8 = g(a)\n  claim c:\n    forall a: U8. f(a) == a\n",
    "called from a model": (
        "  model bad(a: U8) -> U8 = g(a)\n  model outer(a: U8) -> U8 = bad(a)\n"
        "  claim c:\n    forall a: U8. f(a) == a\n"
    ),
    "called from a claim": "  model bad(a: U8) -> U8 = g(a)\n  claim c:\n    forall a: U8. f(a) == bad(a)\n",
}


@pytest.mark.parametrize("case", MODEL_CASES)
def test_a_model_body_with_a_subject_call_is_refused_at_lowering(case):
    head = "warrant m v1\n  op f(a: U8) -> U8\n  op g(a: U8) -> U8\n"
    with pytest.raises(OracleError) as err:
        lower_document(parse_source((head + MODEL_CASES[case]).encode()))
    assert err.value.code == "generation"
    assert err.value.message.startswith("W0701") and "model `bad`" in err.value.message


# -- VER04r 3.3 rule 3, the v0 guard, at lowering -----------------------------------------------

V0 = "warrant v0doc\n  op f(a: U8) -> U8\n  model inc(a: U8) -> U8 = a + 1u8\n  model wrap2(a: U8) -> U8 = inc(a)\n"


def v0_suite(claim: str, version: str = "v0"):
    head = V0 if version == "v0" else V0.replace("v0doc", "v0doc v1")
    return lower_document(parse_source((head + f"  claim c:\n    forall a: U8. {claim}\n").encode())).suites[0]


@pytest.mark.parametrize(
    "claim",
    [
        "f(a) + 1u8 == a",
        "a == f(a) * 2u8",
        "(0u8 - f(a)) == a",
        "inc(f(a)) == a",  # a model whose body wraps
        "wrap2(f(a)) == a",  # ... through another model's call closure
    ],
)
def test_v0_rule_3_is_refused_at_lowering_in_a_v0_document_only(claim):
    suite = v0_suite(claim)
    assert suite.refusal is not None and suite.refusal.code == "generation"
    assert suite.refusal.message.startswith("W0701") and "v0 document" in suite.refusal.message
    assert v0_suite(claim, "v1").refusal is None  # a v1 document wraps everywhere (rule 5)


@pytest.mark.parametrize(
    "claim",
    [
        "f(a) == a + 1u8",  # the arithmetic is on the pure side
        "f(a + 1u8) == a",  # ... or in an argument, beneath no subject result
        "inc(a) == f(a)",  # a wrapping model over a pure argument
        "f(a) <= a",
    ],
)
def test_v0_rule_3_does_not_touch_pure_arithmetic(claim):
    assert v0_suite(claim).refusal is None


def test_the_dump_refuses_a_v0_rule_3_claim():
    data = (V0 + "  claim c:\n    forall a: U8. f(a) + 1u8 == a\n").encode()
    with pytest.raises(OracleError) as err:
        view(data)
    assert err.value.code == "generation" and "v0 document" in err.value.message
