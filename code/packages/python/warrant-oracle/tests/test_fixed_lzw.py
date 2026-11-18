"""Slice S10c-1 and the lzw adopter row of VER04r section 13: ``lzw.warrant`` evaluates.

``lzw.warrant`` has six decompression-bomb examples (lines 231 to 245).  With ``ir.tuple`` lowered
(S10t-1) and ``be_bytes`` evaluated (this slice) each classifies under an ``omit_example`` line:

* the two ``match decompress(..) with Ok(r) -> .. | Err(e) -> ..`` examples are ``not-op-eq-pure``;
* the four eq-shaped ones evaluate ``bomb1024(..)``, whose ``concat`` leaves ``MAX_IR_VALUE``
  (65,536 bytes; a bomb is 285,696 bytes) and so are ``value-over-limit`` (a ``SizeError``, VER04r 8.4).

The whole document is **not** derived here: its largest suite is 2**24 cases (256 chunks), which the lzw
adopter PR measures again (D33).  Everything that is
cheap is checked: all 21 suites plan, every non-omitted example evaluates, and the six classifications.
(Measured while writing this slice: a full-depth ``derive`` of the whole document under the profile
below took about 21 seconds in one process, so D33's "shard by chunk" is not needed for the oracle.)
"""

from __future__ import annotations

import pytest
from conftest import adopter_bytes

from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.corpus_v2 import _examples, omit_reason
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.profile import parse_profile
from coding_adventures_warrant_oracle.suites import SuitePlan, Work

BOMBS = {
    "bomb_512_units_is_under_the_cap_and_decodes": "not-op-eq-pure",
    "bomb_1024_units_is_rejected": "value-over-limit",
    "bomb_1024_units_declaring_one_byte_over_the_cap_is_rejected": "value-over-limit",
    "bomb_1024_units_declaring_one_gibibyte_is_rejected": "value-over-limit",
    "bomb_1024_units_declaring_exactly_the_cap_delivers_the_cap": "not-op-eq-pure",
    "bomb_1024_units_declaring_1024_bytes_delivers_them": "value-over-limit",
}
PROFILE = "max_ir_cost 16384\n" + "".join(f"omit_example warrant/lzw/example/{n}\n" for n in sorted(BOMBS))


@pytest.fixture(scope="module")
def lzw():
    spec = lower_document(parse_source(adopter_bytes("lzw")))
    profile = parse_profile(PROFILE.encode())
    return spec, profile, build_program(spec, max_cost=profile.max_ir_cost)


def test_the_document_lowers_completely_and_every_unit_is_evaluable(lzw):
    spec, _, _ = lzw
    assert len(spec.suites) == 21 and len(spec.examples) == 27
    for unit in [*spec.models.values(), *spec.ops.values(), *spec.suites, *spec.examples]:
        assert unit.unsupported is None
    assert all(s.refusal is None for s in spec.suites)
    assert {lv for s in spec.suites for lv in s.requires} == {"ir.core", "ir.fixed"}


def test_the_six_bomb_examples_classify_as_the_row_says(lzw):
    spec, _, prog = lzw
    by_name = {x.name: x for x in spec.examples}
    got = {name: omit_reason(spec, prog, by_name[name]) for name in BOMBS}
    assert got == BOMBS
    assert sum(v == "not-op-eq-pure" for v in got.values()) == 2
    assert sum(v == "value-over-limit" for v in got.values()) == 4


def test_the_example_stage_of_derive_omits_exactly_the_six_and_evaluates_the_rest(lzw):
    spec, profile, prog = lzw
    rows, tfv_text, omitted = _examples(spec, prog, profile.omit_examples)
    assert omitted == [
        {"id": f"warrant/lzw/example/{name}", "reason": BOMBS[name]} for name in sorted(BOMBS)
    ]
    assert len(rows) == 21 and tfv_text.startswith(b"tfv 1\n") and tfv_text.count(b"\n") == 22
    ids = {r["id"] for r in rows}
    assert not ids & {o["id"] for o in omitted}
    # an emitted example is at most 4,096 bytes everywhere (VER04r 2.3)
    for row in rows:
        for value in [*row["args"], row["expected"]]:
            text = value if isinstance(value, str) else str(value)
            assert len(text) <= 2 * 4096 + 16


def test_without_the_profile_lines_the_first_bomb_in_source_order_refuses_generation(lzw):
    spec, _, prog = lzw
    with pytest.raises(OracleError) as err:
        _examples(spec, prog, [])
    # ``bomb_512_units..`` is the first of the six: a ``match`` is not ``op(args) == pure`` (W0701)
    assert err.value.code == "generation" and "W0701" in err.value.message


def test_the_omit_lines_must_name_examples_that_are_really_inexpressible(lzw):
    spec, _, prog = lzw
    emittable = "warrant/lzw/example/decompress_empty_input"
    with pytest.raises(OracleError) as err:
        _examples(spec, prog, [emittable])
    assert err.value.code == "corpus" and "emittable" in err.value.message


def test_all_21_suites_plan_under_the_profile_and_the_default_cost_bound_is_too_small(lzw):
    spec, profile, prog = lzw
    planned = {}
    for suite in spec.suites:
        planned[suite.claim] = SuitePlan(
            spec, prog, suite, chunk_size=profile.chunk_size, budget=profile.budget, work=Work("full")
        )
    assert max(p.ir_cost for p in planned.values()) == 15_105  # round_trip_of_long_runs
    assert planned["decompress_three_bytes_rejected"].product == 1 << 24
    assert planned["decompress_three_bytes_rejected"].chunks_total == 256
    # D25: the two-U16 claims demote their leftmost 16-bit leaf, so lzw needs the 16-bit fx_edge
    demoted = {c for c, p in planned.items() if p.demoted}
    assert demoted == {"decompress_two_byte_payload_model", "decompress_missing_clear_rejected"}
    for claim in demoted:
        kinds = [b.desc.to_json() for b in planned[claim].binders]
        assert {"d": "fx_edge", "t": "U16", "size": 281} in kinds
    # at the default bound of 2,000 the long-run claim is refused, so the profile line is needed
    default = build_program(spec)
    long_run = next(s for s in spec.suites if s.claim == "round_trip_of_long_runs")
    with pytest.raises(OracleError) as err:
        SuitePlan(spec, default, long_run)
    assert err.value.code == "limit"


def test_stream_headers_follow_the_types_of_the_suite(lzw):
    spec, profile, prog = lzw
    by_claim = {s.claim: s for s in spec.suites}
    for claim, header in (
        ("compress_one_byte_matches_the_model", b"warrant-stream-v1\0"),  # U8 binder, Bytes result
        ("declared_length_cuts_the_output", b"warrant-stream-v2\0"),  # a U32 binder
        ("round_trip_of_repeated_byte_pairs", b"warrant-stream-v2\0"),  # a U16 binder
    ):
        plan = SuitePlan(spec, prog, by_claim[claim], chunk_size=profile.chunk_size, work=Work("full"))
        assert plan.header == header, claim


def test_the_cheap_suites_run_to_a_digest(lzw):
    spec, profile, prog = lzw
    for suite in spec.suites:
        if suite.claim in ("compress_one_byte_matches_the_model", "decompress_invalid_first_code_every_value"):
            plan = SuitePlan(spec, prog, suite, chunk_size=profile.chunk_size, work=Work("full"))
            result = plan.run("full")
            assert result.cases == 256 and result.inputs_sha256 and result.wraps == 0
