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


def test_a_level_refusal_wins_in_evaluation_but_the_dump_shows_the_refusal():
    # a claim that is both ir.fixed and impure: the generator meets the level first (while typing),
    # and only its dump mode, which accepts ir.fixed, reaches the guard rule.
    claim = "f(a) == bit_not(a) or f(a) == a"
    data = document(claim)
    suite = lower_document(parse_source(data)).suites[0]
    assert suite.unsupported is not None and suite.unsupported.code == "level"
    assert suite.refusal is not None and "impure guard" in suite.refusal.message
    with pytest.raises(OracleError) as err:
        derive(data, b"", spec_ref="g.warrant", generator={"name": "t", "version": "0"})
    assert err.value.code == "level"
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
    # the committed adopter documents lower as before; their impure-guard claims (wasm-leb128 has
    # `len(op(v)) >= 1 and len(op(v)) <= 10`) are recorded, not fatal, as the generator's
    # per-item census records them
    spec = lower_document(parse_source(adopter_bytes("wasm-leb128")))
    refused = [s.claim for s in spec.suites if s.refusal is not None]
    assert "encode_unsigned_at_most_ten_bytes" in refused
    for name in ("ct-compare", "gf256"):
        assert all(s.refusal is None for s in lower_document(parse_source(adopter_bytes(name))).suites)
