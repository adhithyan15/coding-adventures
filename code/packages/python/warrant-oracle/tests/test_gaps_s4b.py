"""Oracle gaps closed by slice S4 part 2 (VER04r 16.4 item 7, 16.5 items 7 and 12, 3.3 rule 3)."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.ir_eval import MAX_IR_DEPTH, Program
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.profile import parse_profile
from coding_adventures_warrant_oracle.stream import slim_base_set
from coding_adventures_warrant_oracle.suites import SuitePlan

# -- profile strictness (16.4 item 7) -------------------------------------------------


@pytest.mark.parametrize(
    "text",
    [
        "fault eq invert\nfault eq invert\n",  # repeated fault line
        "slim_chunk c 3\nslim_chunk c 3\n",  # repeated slim_chunk line
        "fault eq \n",  # empty token (trailing space)
        "fault  eq invert\n",  # empty token (double space)
        " fault eq invert\n",  # empty leading token
        "   \n",  # a whitespace-only line is not an empty line
        "budget 0\n",
        "slim_stride 0\n",
        "slim_stride -1\n",
        "slim_stride 01\n",
        "budget 1\nbudget 2\n",
        "chunk_size 1000\n",
        "max_ir_cost 1999\n",
        "max_ir_cost 65537\n",
        "bogus 1\n",
        "fault eq\n",
        "slim_chunk c\n",
        "budget\n",
        "fault eq invert\r\n",
    ],
)
def test_profile_strictness(text):
    with pytest.raises(OracleError) as err:
        parse_profile(text.encode())
    assert err.value.code == "corpus"


def test_profile_accepts_distinct_lines_and_blank_lines():
    prof = parse_profile(
        b"budget 16777216\n\nchunk_size 1024\nslim_stride 3\nmax_ir_cost 2000\n"
        b"fault eq invert\nfault eq ignore-last-byte\nslim_chunk c 1\nslim_chunk c 2\n"
    )
    assert prof.slim_stride == 3 and prof.chunk_size == 1024
    assert prof.faults == [("eq", "invert"), ("eq", "ignore-last-byte")]
    assert prof.slim_chunks == [("c", 1), ("c", 2)]
    assert parse_profile(b"slim_stride auto\n").slim_stride == "auto"
    assert parse_profile(b"").budget == 1 << 24


# -- the slim clamp (16.5 item 7) --------------------------------------------------------


def test_slim_stride_is_clamped_per_suite_and_never_sparser():
    sid = "warrant/x/exhaustive/c"
    auto = slim_base_set(sid, 64)  # k = ceil(64 / 14) = 5
    assert slim_base_set(sid, 64, "auto") == auto
    assert slim_base_set(sid, 64, 100) == auto  # clamped to the automatic stride
    assert slim_base_set(sid, 64, 5) == auto
    dense = slim_base_set(sid, 64, 2)  # denser: k = 2
    assert len(dense) > len(auto) and {0, 63} <= set(dense)
    assert slim_base_set(sid, 10, 1) == list(range(10))  # m <= 16 is every chunk, stride or not
    assert slim_base_set(sid, 16, 1000) == list(range(16))
    for bad in (0, -3, "x", 1.5, True):
        with pytest.raises(OracleError):
            slim_base_set(sid, 64, bad)


def test_the_clamped_stride_picks_the_offset_from_the_effective_stride():
    sid = "warrant/x/exhaustive/c"
    got = slim_base_set(sid, 64, 2)
    assert got[0] == 0 and got[-1] == 63
    steps = [b - a for a, b in zip(got[1:-1], got[2:-1])]
    assert steps and all(s == 2 for s in steps)


# -- IR depth: 64 levels, not 65 (16.5 item 12) ------------------------------------------


def _nested_not(levels: int) -> list:
    node: list = ["lit", True]
    for _ in range(levels - 1):
        node = ["not", node]
    return node


def test_ir_depth_is_sixty_four_levels():
    assert MAX_IR_DEPTH == 64
    prog = Program({}, {})
    assert prog.compile(_nested_not(64), {}).ty.head == "Bool"
    with pytest.raises(OracleError) as err:
        prog.compile(_nested_not(65), {})
    assert err.value.code == "limit"


def test_depth_bound_also_holds_for_model_bodies_and_call_counting():
    from coding_adventures_warrant_oracle.ast import Ty
    from coding_adventures_warrant_oracle.ir_eval import ModelDef, compile_calls

    ok = ModelDef("m", [], Ty("Bool"), _nested_not(64))
    bad = ModelDef("m", [], Ty("Bool"), _nested_not(65))
    Program({"m": ok}, {}).model("m")
    with pytest.raises(OracleError):
        Program({"m": bad}, {}).model("m")
    with pytest.raises(OracleError):
        compile_calls(Program({}, {}), _wrap_subject(65), {})


def _wrap_subject(levels: int) -> list:
    node: list = ["subject_op", "f", []]
    for _ in range(levels):
        node = ["not", node]
    return node


# -- concat is evaluated on the pure trace (16.5 item 12) ---------------------------------

CONCAT = b"""warrant cc v1
  op f(a: Bytes) -> Bytes
  model d1(x: Bytes) -> Bytes = concat(x, x)
  model d2(x: Bytes) -> Bytes = d1(d1(x))
  model d3(x: Bytes) -> Bytes = d2(d2(x))
  model d4(x: Bytes) -> Bytes = d3(d3(x))
  model d5(x: Bytes) -> Bytes = d4(d4(x))
  claim one:
    forall a: Bytes | len(a) == 1. f(d5(a)) == a
  claim two:
    forall a: Bytes | len(a) == 2. f(d5(a)) == a
"""


def _plan(src: bytes, claim: str) -> SuitePlan:
    spec = lower_document(parse_source(src))
    return SuitePlan(
        spec, build_program(spec), next(s for s in spec.suites if s.claim == claim), budget=1 << 20
    )


def test_a_pure_subject_argument_with_concat_past_the_value_bound_fails_generation():
    assert _plan(CONCAT, "one").run("full").cases == 256  # exactly MAX_IR_VALUE bytes: allowed
    with pytest.raises(OracleError) as err:
        _plan(CONCAT, "two").run("full")
    assert err.value.code == "ir-overflow"


def test_a_concat_on_the_expected_side_is_evaluated_too():
    src = CONCAT.replace(b"f(d5(a)) == a", b"f(a) == d5(a)")
    assert _plan(src, "one").run("full").cases == 256
    with pytest.raises(OracleError):
        _plan(src, "two").run("full")


# -- v0 rules 2 and 3 (section 3.3) ----------------------------------------------------------

V0 = b"""warrant r0
  op f(a: U8) -> U8
  op g(a: U8) -> U8
  model inc(x: U8) -> U8 = x + 1u8
  model inc2(x: U8) -> U8 = inc(x)
  model idn(x: U8) -> U8 = x
  op h(a: U8) -> Bytes
  claim direct:
    forall a: U8. f(a) + 1u8 == a
  claim negated:
    forall a: U8. 0u8 - f(a) == a
  claim via_model:
    forall a: U8. inc(f(a)) == a
  claim via_model2:
    forall a: U8. inc2(f(a)) == a
  claim via_model_arg_nested:
    forall a: U8. g(inc(f(a))) == a
  claim fine_model:
    forall a: U8. idn(f(a)) == a
  claim fine_nat:
    forall a: U8. len(h(a)) + 1 > 0
  claim pure_model_only:
    forall a: U8 | a < 200u8. f(a) == inc(a)
  claim wrapping_arg:
    forall a: U8. f(a + 1u8) == a
"""


@pytest.mark.parametrize(
    "claim", ["direct", "negated", "via_model", "via_model2", "via_model_arg_nested"]
)
def test_v0_rule_3_refuses_fixed_width_arithmetic_over_a_subject(claim):
    with pytest.raises(OracleError) as err:
        _plan(V0, claim)
    assert err.value.code == "generation" and "W0701" in err.value.message


def test_v0_rule_3_does_not_over_refuse():
    for claim in ("fine_model", "pure_model_only"):
        _plan(V0, claim)  # no arithmetic above a subject, or no subject beneath the arithmetic
    # ``Nat`` arithmetic over a subject-derived length is exact in both evaluators.
    _plan(V0, "fine_nat")


def test_v0_rule_3_is_a_v0_rule():
    v1 = V0.replace(b"warrant r0", b"warrant r1 v1")
    for claim in ("direct", "negated", "via_model", "via_model2", "via_model_arg_nested"):
        _plan(v1, claim)


def test_v0_rule_2_wraps_in_a_subject_argument_are_refused():
    with pytest.raises(OracleError) as err:
        _plan(V0, "wrapping_arg").run("full")
    assert "W0701" in err.value.message and "wraps" in err.value.message
    v1 = V0.replace(b"warrant r0", b"warrant r1 v1")
    assert _plan(v1, "wrapping_arg").run("full").wraps == 1  # informational in v1
