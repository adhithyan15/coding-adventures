"""Enumeration order, admission arithmetic, chunk independence and digests.

The reference for every digest here is a deliberately naive enumerator written in
this file (``itertools.product`` and hand-built streams), so the production code is
compared against a second, simpler implementation of the same rules.
"""

from __future__ import annotations

import hashlib
import itertools

import pytest
from conftest import TINY

from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.stream import (
    chunk_digest_framing,
    slim_base_set,
)
from coding_adventures_warrant_oracle.suites import MAX_CHUNKS, SuitePlan


def le32(n: int) -> bytes:
    return n.to_bytes(4, "little")


def name(text: str) -> bytes:
    return le32(len(text.encode())) + text.encode()


def u8(v: int) -> bytes:
    return b"\x10" + v.to_bytes(8, "little")


def plan_for(source: bytes | str, claim: str, **kw):
    spec = lower_document(parse_source(source))
    prog = build_program(spec)
    suite = next(s for s in spec.suites if s.claim == claim)
    return SuitePlan(spec, prog, suite, **kw), spec


# -- a naive U8 x U8 reference --------------------------------------------------------

TWO = b"""warrant two v1
  op f(a: U8, b: U8) -> U8
  model keep(a: U8, b: U8) -> U8 = a + b
  claim all_pairs:
    forall a: U8, b: U8. f(a, b) == keep(a, b)
  claim lead_refined:
    forall a: U8 | a >= 100u8 and a < 130u8, b: U8. f(a, b) == keep(a, b)
  claim tail_refined:
    forall a: U8, b: U8 | b >= 250u8. f(a, b) == keep(a, b)
  claim both_refined:
    forall a: U8, b: U8 | a != b. f(a, b) == keep(a, b)
  claim nothing_until_the_end:
    forall a: U8 | a == 255u8, b: U8. f(a, b) == keep(a, b)
  claim relational:
    forall a: U8, b: U8. f(a, b) == f(b, a)
"""


def naive_chunks(plan, admit, expected=None):
    """Hash every chunk by brute force: product order, admit, encode, join."""
    cs = plan.chunk_size
    prefix = plan.prefix
    out = []
    first = 0
    rows = list(itertools.product(range(256), range(256)))
    for c in range((len(rows) + cs - 1) // cs):
        h_in = hashlib.sha256(prefix + le32(c))
        h_ex = hashlib.sha256(prefix + le32(c)) if expected else None
        cases = 0
        for a, b in rows[c * cs : (c + 1) * cs]:
            if admit(a, b):
                h_in.update(u8(a) + u8(b))
                if expected:
                    h_ex.update(u8(expected(a, b)))
                cases += 1
        out.append((c, cases, first, h_in.digest(), h_ex.digest() if h_ex else None))
        first += cases
    return out


@pytest.mark.parametrize(
    "claim,admit",
    [
        ("all_pairs", lambda a, b: True),
        ("lead_refined", lambda a, b: 100 <= a < 130),
        ("tail_refined", lambda a, b: b >= 250),
        ("both_refined", lambda a, b: a != b),
        ("nothing_until_the_end", lambda a, b: a == 255),
    ],
)
@pytest.mark.parametrize("chunk_size", [1 << 10, 1 << 12, 1 << 16])
def test_every_chunk_matches_a_naive_enumeration(claim, admit, chunk_size):
    plan, _ = plan_for(TWO, claim, chunk_size=chunk_size)
    naive = naive_chunks(plan, admit, expected=lambda a, b: (a + b) & 255)
    assert plan.chunks_total == len(naive) == 65536 // chunk_size
    for c, cases, first, d_in, d_ex in naive:
        r = plan.run_chunk(c)
        assert (r.index, r.cases, r.first_admitted) == (c, cases, first)
        assert r.inputs == d_in and r.expected == d_ex


def test_a_chunk_enumerated_alone_equals_the_same_chunk_after_its_predecessors():
    plan, _ = plan_for(TWO, "both_refined", chunk_size=1 << 10)
    alone = plan.run_chunk(37)
    for c in range(37):
        plan.run_chunk(c)
    again = plan.run_chunk(37)
    fresh, _ = plan_for(TWO, "both_refined", chunk_size=1 << 10)
    assert alone == again == fresh.run_chunk(37)


def test_a_chunk_with_zero_admitted_cases():
    plan, _ = plan_for(TWO, "nothing_until_the_end", chunk_size=1 << 10)
    empty = plan.run_chunk(0)
    assert empty.cases == 0 and empty.first_admitted == 0 and empty.calls == 0
    assert empty.inputs == hashlib.sha256(plan.prefix + le32(0)).digest()
    last = plan.run_chunk(plan.chunks_total - 1)
    assert last.cases == 256 and last.first_admitted == 0  # a == 255 is the last 256 ordinals
    assert plan.admitted_before(65536) == 256


def test_a_chunk_boundary_inside_a_refinement_discarded_run():
    # a in 100..129 admits product ordinals 25600..33279; boundary 32768 is inside the run.
    plan, _ = plan_for(TWO, "lead_refined", chunk_size=1 << 15)
    assert plan.chunks_total == 2
    c0, c1 = plan.run_chunk(0), plan.run_chunk(1)
    assert c0.cases == 32768 - 25600 and c1.cases == 33280 - 32768
    assert c0.first_admitted == 0 and c1.first_admitted == c0.cases
    assert plan.total_cases() == 30 * 256


def test_admitted_before_is_monotone_and_exact():
    plan, _ = plan_for(TWO, "both_refined", chunk_size=1 << 10)
    for ordinal in (0, 1, 255, 256, 257, 1024, 30000, 65535, 65536):
        naive = sum(1 for p in range(ordinal) if p // 256 != p % 256)
        assert plan.admitted_before(ordinal) == naive


def test_case_counts_and_discards():
    plan, _ = plan_for(TWO, "both_refined")
    r = plan.run("full")
    assert (r.cases, r.discarded, r.product) == (65536 - 256, 256, 65536)
    assert r.expected_calls == r.cases  # one subject call per admitted case
    assert r.header == "warrant-stream-v1" and r.label == "full" and r.kind == "reference"


def test_suite_digest_is_the_framing_over_all_chunk_digests():
    plan, _ = plan_for(TWO, "all_pairs", chunk_size=1 << 12)
    r = plan.run("full")
    assert len(r.chunks) == 16
    assert r.inputs_sha256 == chunk_digest_framing([c.inputs for c in r.chunks])
    assert r.expected_sha256 == chunk_digest_framing([c.expected for c in r.chunks])
    # the v1 whole-suite digest is over the unchunked stream
    naive = hashlib.sha256(plan.prefix)
    for a, b in itertools.product(range(256), range(256)):
        naive.update(u8(a) + u8(b))
    assert r.v1_inputs_sha256 == naive.hexdigest()
    naive_ex = hashlib.sha256(plan.prefix)
    for a, b in itertools.product(range(256), range(256)):
        naive_ex.update(u8((a + b) & 255))
    assert r.v1_expected_sha256 == naive_ex.hexdigest()


def test_chunk_digests_differ_by_chunk_index_even_for_identical_bodies():
    plan, _ = plan_for(TWO, "relational", chunk_size=1 << 10)
    # chunks 0 and 1 hold different tuples, but the index alone must also separate them
    h = hashlib.sha256(plan.prefix + le32(0)).digest()
    assert plan.run_chunk(0).inputs != h  # has a body
    empty0 = hashlib.sha256(plan.prefix + le32(0)).digest()
    empty1 = hashlib.sha256(plan.prefix + le32(1)).digest()
    assert empty0 != empty1


def test_slim_runs_only_the_corpus_chosen_chunks():
    plan, _ = plan_for(TWO, "all_pairs", chunk_size=1 << 10)  # m = 64
    r = plan.run("slim")
    assert r.chunks_total == 64
    assert [c.index for c in r.chunks] == slim_base_set(plan.suite.id, 64) == r.slim_chunks
    assert r.inputs_sha256 is None and r.expected_sha256 is None  # cannot be recomputed
    chosen = r.chunks
    assert r.slim_inputs_sha256 == chunk_digest_framing([c.inputs for c in chosen])
    assert r.slim_expected_sha256 == chunk_digest_framing([c.expected for c in chosen])
    assert r.v1_inputs_sha256 is None  # only a full run reproduces the whole-suite digest
    full = plan.run("full")
    assert full.slim_inputs_sha256 == r.slim_inputs_sha256  # same chunks, same digests


def test_slim_of_a_small_suite_is_the_whole_suite():
    plan, _ = plan_for(TWO, "all_pairs")  # one chunk
    assert plan.run("slim").slim_chunks == [0]


def test_explicit_chunk_selection():
    plan, _ = plan_for(TWO, "all_pairs", chunk_size=1 << 10)
    r = plan.run(chunks=[5, 9])
    assert [c.index for c in r.chunks] == [5, 9] and r.inputs_sha256 is None


def test_predicate_suite_has_no_expected_stream_and_counts_calls():
    plan, _ = plan_for(TWO, "relational", chunk_size=1 << 12)
    r = plan.run("full")
    assert r.kind == "predicate" and r.expected_sha256 is None
    assert r.v1_expected_sha256 is None and r.chunks[0].expected is None
    assert r.expected_calls == 2 * 65536  # two subject calls per admitted case
    assert len(r.chunks[0].line().split()) == 5  # no expected digest column


def test_chunk_file_line_format():
    plan, _ = plan_for(TWO, "all_pairs", chunk_size=1 << 15)
    r = plan.run("full")
    parts = r.chunks[1].line().split(" ")
    assert parts[:4] == ["1", "32768", "32768", "32768"]
    assert len(parts) == 6 and all(len(p) == 64 for p in parts[4:])


def test_chunk_index_bounds():
    plan, _ = plan_for(TWO, "all_pairs", chunk_size=1 << 15)
    with pytest.raises(IndexError):
        plan.run_chunk(2)
    with pytest.raises(IndexError):
        plan.run_chunk(-1)


# -- construction errors ----------------------------------------------------------------


def test_chunk_size_must_be_a_power_of_two_in_range():
    for bad in (0, 1000, 512, 1 << 17, 3 << 10):
        with pytest.raises(OracleError):
            plan_for(TWO, "all_pairs", chunk_size=bad)


def test_more_than_4096_chunks_is_refused():
    src = b"warrant big\n  op f(a: U8, b: U8, c: U8) -> U8\n  claim c3:\n    forall a: U8, b: U8, c: U8. f(a, b, c) == f(a, b, c)\n"
    plan, _ = plan_for(src, "c3", chunk_size=1 << 16)
    assert plan.chunks_total == 256 <= MAX_CHUNKS
    with pytest.raises(OracleError):
        plan_for(src, "c3", chunk_size=1 << 10)  # 16,384 chunks


def test_a_suite_that_admits_nothing_is_a_generation_error():
    src = (
        b"warrant z\n  op f(a: U8) -> U8\n  claim c:\n    forall a: U8 | a > 255u8. f(a) == f(a)\n"
    )
    plan, _ = plan_for(src, "c")
    with pytest.raises(OracleError) as err:
        plan.run("full")
    assert err.value.code == "generation"
    const = b"warrant z\n  op f(a: U8) -> U8\n  claim c:\n    forall a: U8 | 1 == 2. f(a) == f(a)\n"
    with pytest.raises(OracleError):
        plan_for(const, "c")


def test_unsupported_suite_cannot_be_planned():
    # ``ir.fixed`` is evaluated since S10c-1, so the suite of a level the oracle lacks is one of the
    # reserved ``ir.list`` (a list literal, refused at lowering since S11a)
    spec = lower_document(
        parse_source(
            b"warrant u v1\n  op f(a: U8) -> U8\n  claim c:\n    forall a: U8. f(a) == [a]\n"
        )
    )
    with pytest.raises(OracleError) as err:
        SuitePlan(spec, build_program(spec), spec.suites[0])
    assert err.value.code == "level" and err.value.level == "ir.list"


def test_budget_demotion_and_refusal_flow_through_the_plan():
    src = b"warrant w v1\n  op f(a: U16, b: U16) -> U16\n  claim c:\n    forall a: U16, b: U16. f(a, b) == f(a, b)\n"
    plan, _ = plan_for(src, "c", budget=1 << 26, chunk_size=1 << 16)
    assert plan.demoted == ["0"] and plan.product == 281 * 65536
    assert plan.run("slim").demoted == ["0"]
    with pytest.raises(OracleError):
        plan_for(src, "c", budget=1 << 10)


# -- evaluation inside enumeration ---------------------------------------------------------

WRAP = b"""warrant wr v1
  op f(a: U8) -> U8
  model plus(a: U8) -> U8 = a + 200u8
  claim c:
    forall a: U8. f(a) == plus(a)
"""


def test_wraps_are_counted_on_subject_independent_nodes():
    plan, _ = plan_for(WRAP, "c")
    r = plan.run("full")
    assert r.wraps == 200  # a + 200 wraps for a >= 56
    assert r.cases == 256


GUARD = b"""warrant g
  error E = BAD
  op h(a: U8) -> Result<U8, E>
  claim guarded:
    forall a: U8. h(a) == (if a == 0u8 then Err(E.BAD) else h(a))
  claim guarded_pred:
    forall a: U8. if a == 0u8 then true else h(a) == h(a)
"""


def test_guarded_call_counts_are_computed_on_the_pure_trace():
    plan, _ = plan_for(GUARD, "guarded_pred")
    assert callable(plan.calls)
    r = plan.run("full")
    assert r.expected_calls == 255 * 2  # a == 0 takes the call-free branch
    assert sum(c.calls for c in r.chunks) == 510
    plan, _ = plan_for(GUARD, "guarded")
    assert plan.run("full").expected_calls == 256 + 255


def test_a_per_case_refinement_with_dynamic_calls_and_expected():
    src = (
        b"warrant d\n  error E = BAD\n  op h(a: U8) -> Result<U8, E>\n"
        b"  claim c:\n    forall a: U8 | a != 7u8. h(a) == (if a == 0u8 then Err(E.BAD) else Ok(a))\n"
    )
    plan, _ = plan_for(src, "c")
    r = plan.run("full")
    assert r.cases == 255 and r.expected_calls == 255
    assert r.result_type == "Result<U8,E>"


def test_a_claim_with_no_binders_has_one_case():
    src = b"warrant n\n  op f() -> Bool\n  claim c:\n    f() == true\n"
    plan, _ = plan_for(src, "c")
    r = plan.run("full")
    assert (r.cases, r.product, r.chunks_total, r.expected_calls) == (1, 1, 1, 1)
    assert r.chunks[0].inputs == hashlib.sha256(plan.prefix + le32(0)).digest()


def test_composite_binders_enumerate_through_nth():
    src = (
        b"warrant t v1\n  error E = A\n  record P(x: Bool, y: Bool)\n"
        b"  op g(t: (Bool, Bool)) -> Bool\n  op h(p: P, o: (Bool, Bool)) -> Bool\n"
        b"  claim tup:\n    forall t: (Bool, Bool). g(t) == true\n"
        b"  claim rec:\n    forall p: P, o: (Bool, Bool). h(p, o) == true\n"
    )
    plan, _ = plan_for(src, "tup")
    r = plan.run("full")
    assert r.cases == 4 and r.header == "warrant-stream-v2"
    assert r.descriptors[0]["d"] == "tuple" and r.label == "full"
    body = b"".join(
        b"\x64" + le32(2) + bytes([x]) + bytes([y]) for x, y in itertools.product((0, 1), repeat=2)
    )
    assert r.chunks[0].inputs == hashlib.sha256(plan.prefix + le32(0) + body).digest()
    # Slice S11a lowered records but did not evaluate them (section 16.14 R37: ``level ir.data`` at evaluation
    # until S11c-1), so the planner refused the suite that has a record binder; S11c-1 enumerates it: the record
    # is the Cartesian product of its fields, last field fastest, and a tuple value carries the 64 tag.
    plan, _ = plan_for(src, "rec")
    r = plan.run("full")
    assert r.cases == 16 and r.header == "warrant-stream-v2"
    assert [d["d"] for d in r.descriptors] == ["tuple", "tuple"] and r.product == 16


def test_suite_row_reports_demotions_labels_and_descriptors():
    plan, _ = plan_for(TINY, "bump_is_inc")
    r = plan.run("full")
    assert r.descriptors == [{"d": "u8_all", "size": 256}]
    assert r.binder_types == ["U8"] and r.demoted == []


# -- CV2-F17 O3: a claim has at most 8 binders (VER04r 5.3, 14, R75) ------------------------


def _claim_with_binders(n: int) -> bytes:
    names = ", ".join(f"b{i}: Bool" for i in range(n))
    return (
        "warrant many v1\n"
        "  op f(a: Bool) -> Bool\n"
        "  model keep(a: Bool) -> Bool = a\n"
        f"  claim wide:\n    forall {names}. f(b0) == keep(b0)\n"
    ).encode()


def test_eight_binders_are_accepted_and_nine_are_refused_before_any_descriptor():
    plan, _ = plan_for(_claim_with_binders(8), "wide")
    assert len(plan.binders) == 8 and plan.product == 256
    with pytest.raises(OracleError) as e:
        plan_for(_claim_with_binders(9), "wide")
    assert e.value.code == "limit"
    assert str(e.value).endswith("a claim has more than 8 binders (VER04r 5.3)")
