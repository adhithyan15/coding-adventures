"""Owner decisions of 2026-11-17, second batch (VER04r 16.9), in the oracle.

[D18] v0 rule 2 applies to the evaluation of an example: ``f(255u8 + 2u8) == 1u8`` fails
generation in a v0 document and is accepted in a v1 document.  [D19] a subject call whose
arguments are outside the callee's ``admit`` is a generation error, in a claim (naming the
claim, the op and the first admitted-case ordinal) and in an example.  The same documents are
in the generator's ``tests/admit_calls.rs``; both must say the same thing.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle.corpus_v2 import derive
from coding_adventures_warrant_oracle.errors import OracleError

GEN = {"name": "test", "version": "0"}


def doc(version: str, extra: str) -> bytes:
    s = f"warrant synth{version}\n"
    s += "  model add8(a: U8, b: U8) -> U8 = a + b\n"
    s += "  model first(a: U8, b: U8) -> U8 = a\n"
    s += "  op op_add(a: U8, b: U8) -> U8\n"
    s += "  op op_lo(a: U8 | a < 10u8, b: U8) -> U8\n"
    s += "  assume platform:\n    none\n  assume environment:\n    none\n"
    for op in ("op_add", "op_lo"):
        s += f"  labels {op}:\n    a secret\n    b secret\n"
    for op in ("op_add", "op_lo"):
        for p in "ab":
            s += f"  input {op}.{p}:\n    hostile\n    ensures no_panic, terminates\n"
    for op in ("op_add", "op_lo"):
        s += f"  bound {op}:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n"
    return (s + extra).encode()


def gen(version: str, extra: str):
    return derive(doc(version, extra), b"", spec_ref="s.warrant", generator=GEN)


WRAPPING_EXAMPLE = "  example wrap:\n    op_add(255u8 + 2u8, 0u8) == 1u8\n"
PLAIN_CLAIM = (
    "  claim c:\n    forall a: U8 | a < 2u8, b: U8 | b < 2u8. op_add(a, b) == add8(a, b)\n"
)


def refused(version: str, extra: str) -> str:
    with pytest.raises(OracleError) as err:
        gen(version, extra)
    assert err.value.code == "generation"
    assert err.value.message.startswith("W0701"), err.value.message
    return err.value.message


def test_d18_an_example_that_wraps_fails_in_a_v0_document():
    msg = refused("", PLAIN_CLAIM + WRAPPING_EXAMPLE)
    assert "`wrap`" in msg and "wraps" in msg


def test_d18_the_same_example_is_accepted_in_a_v1_document():
    out = gen(" v1", PLAIN_CLAIM + WRAPPING_EXAMPLE)
    assert b"Ka I1 Kb I0 Kexpect I1" in out.tfv


def test_d18_a_v0_example_that_does_not_wrap_is_accepted():
    gen("", PLAIN_CLAIM + "  example fine:\n    op_add(1u8 + 2u8, 0u8) == 3u8\n")


def test_d19_a_claim_weaker_than_the_ops_admit_is_refused():
    weak = "  claim weak:\n    forall a: U8 | a < 20u8, b: U8 | b < 2u8. op_lo(a, b) == first(a, b)\n"
    msg = refused(" v1", weak)
    assert "`weak`" in msg and "`op_lo`" in msg and "admitted case 20" in msg


def test_d19_a_matching_refinement_is_accepted():
    ok = "  claim ok:\n    forall a: U8 | a < 10u8, b: U8 | b < 2u8. op_lo(a, b) == first(a, b)\n"
    out = gen(" v1", ok)
    assert out.corpus["suites"][0]["cases"] == 20


def test_d19_an_unrefined_binder_names_the_first_offending_case():
    open_ = "  claim open:\n    forall a: U8, b: U8. op_lo(a, b) == first(a, b)\n"
    assert "admitted case 2560" in refused(" v1", open_)


def test_d19_an_example_outside_the_ops_admit_is_refused():
    msg = refused(" v1", "  example out:\n    op_lo(10u8, 0u8) == 10u8\n")
    assert "`out`" in msg and "`op_lo`" in msg
    gen(" v1", "  example in_admit:\n    op_lo(9u8, 0u8) == 9u8\n")


def test_d19_only_the_taken_branch_of_a_pure_guard_is_checked():
    guarded = (
        "  claim g:\n    forall a: U8, b: U8. (if a < 10u8 then op_lo(a, b) else op_add(a, b))"
        " == (if a < 10u8 then first(a, b) else add8(a, b))\n"
    )
    gen(" v1", guarded)
    msg = refused(" v1", guarded.replace("a < 10u8 then op_lo", "a < 11u8 then op_lo"))
    assert "outside the op's admit" in msg


def test_d19_an_admit_that_reads_a_subject_dependent_argument_is_refused_statically():
    nested = (
        "  claim n:\n    forall a: U8 | a < 10u8, b: U8 | b < 2u8."
        " op_lo(op_add(a, b), b) == first(add8(a, b), b)\n"
    )
    assert "depends on a subject result" in refused(" v1", nested)


def test_d19_a_subject_dependent_argument_the_admit_does_not_read_is_accepted():
    # op_lo's admit reads `a` only: `b` may be a subject result (the gf256 shape
    # ``power(add(a, b), 2u64)``), and `a` is still checked on every admitted case.
    ok = (
        "  claim n:\n    forall a: U8 | a < 10u8, b: U8 | b < 2u8."
        " op_lo(a, op_add(a, b)) == first(a, add8(a, b))\n"
    )
    gen(" v1", ok)
    assert "admitted case 20" in refused(" v1", ok.replace("a < 10u8", "a < 12u8"))
