"""The ct-compare gate: the oracle against the committed v1 corpus, and seeded-wrong corpora."""

from __future__ import annotations

import copy
import hashlib
import json

import pytest
from conftest import V1_CORPUS, adopter_bytes

from coding_adventures_warrant_oracle.corpus_v1 import (
    load_json,
    typed_json,
    v1_binder,
    v1_faults,
    verify_v1,
)
from coding_adventures_warrant_oracle.domains import TupleDesc, leaf_bool
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source

SPEC = adopter_bytes("ct-compare")
RAW = V1_CORPUS.read_bytes()
CORPUS = json.loads(RAW)


def gate(corpus=None, spec=SPEC, digests=True):
    raw = RAW if corpus is None else json.dumps(corpus).encode()
    return verify_v1(spec, raw, digests=digests)


def codes(report):
    return {(m["code"], m["where"]) for m in report.mismatches}


def test_the_real_corpus_passes_all_three_checks():
    report, results = gate()
    assert report.ok and report.mismatches == []
    assert sum(s["cases"] for s in report.suites.values()) == 572_109
    assert len(report.suites) == len(results) == 9


def test_v1_and_v2_digests_are_both_reproduced():
    report, results = gate()
    by = {s["claim"]: s for s in CORPUS["suites"]}
    for r in results:
        row = by[r.claim]
        assert (
            r.v1_inputs_sha256 == row["inputs_sha256"]
            and r.v1_expected_sha256 == row["expected_sha256"]
        )
        assert r.inputs_sha256 and r.expected_sha256 and r.chunks  # the v2 chunk listing exists too
        assert r.header == "warrant-stream-v1"  # every ct-compare suite is in the v1 type set
        assert sum(c.cases for c in r.chunks) == row["cases"]
    assert report.suites["eq_agrees"]["chunks"] == 2


def seeded(mutate, digests=False):
    """Run the gate on a mutated corpus; only digest tests pay for the enumeration."""
    corpus = copy.deepcopy(CORPUS)
    mutate(corpus)
    return gate(corpus, digests=digests)[0]


def test_a_flipped_inputs_digest_fails_the_enumeration_check():
    def flip(c):
        d = c["suites"][0]["inputs_sha256"]
        c["suites"][0]["inputs_sha256"] = ("0" if d[0] != "0" else "1") + d[1:]

    report = seeded(flip, digests=True)
    assert ("enumeration", "suites.eq_agrees.inputs_sha256") in codes(report)


def test_a_flipped_expected_digest_and_wrong_counts_fail():
    def mutate(c):
        c["suites"][6]["expected_sha256"] = "0" * 64
        c["suites"][1]["cases"] += 1
        c["suites"][2]["discarded"] = 5
        c["suites"][3]["expected_calls"] = 0

    got = codes(seeded(mutate, digests=True))
    assert ("enumeration", "suites.select_picks.expected_sha256") in got
    assert ("enumeration", "suites.eq_fixed_agrees_w0.cases") in got
    assert ("enumeration", "suites.eq_fixed_agrees_w1.discarded") in got
    assert ("enumeration", "suites.eq_fixed_agrees_w2.expected_calls") in got


def test_a_wrong_lowering_is_caught_in_a_model_a_suite_and_the_admit():
    def mutate(c):
        c["models"]["same"]["body"][0] = "ne"  # a == b lowered as a != b
        c["suites"][6]["admit"] = ["ne"] + c["suites"][6]["admit"][1:]
        c["suites"][0]["rhs"] = [
            "model_call",
            "same",
            [["var", "b"], ["var", "a"]],
        ]  # operands swapped

    got = codes(seeded(mutate))
    assert ("lowering", "models.same") in got
    assert ("lowering", "suites.select_picks.admit") in got
    assert ("lowering", "suites.eq_agrees.rhs") in got


def test_a_generator_lowering_a_minus_b_as_b_minus_a_would_be_caught():
    # ct-compare has no subtraction, so show the mechanism on an edited spec: the
    # corpus keeps the original IR; the oracle's lowering of the edited source differs.
    edited = SPEC.replace(
        b"model same(a: Bytes, b: Bytes) -> Bool = a == b",
        b"model same(a: Bytes, b: Bytes) -> Bool = b == a",
    )
    report = gate(spec=edited, digests=False)[0]
    assert not report.ok
    assert ("spec-digest", "spec_sha256") in codes(report)
    assert ("lowering", "models.same") in codes(report)


def test_a_stale_spec_digest_is_reported():
    def mutate(c):
        c["spec_sha256"] = hashlib.sha256(b"other").hexdigest()

    assert ("spec-digest", "spec_sha256") in codes(seeded(mutate))


def test_ops_types_faults_controls_and_examples_are_all_checked():
    def mutate(c):
        c["ops"]["eq"]["ret"] = "U8"
        c["types"]["SelectError"] = ["CT_SELECT_LENGTH_MISMATCH", "EXTRA"]
        c["faults"] = c["faults"][:-1]
        c["controls"][0]["required_claims"] = []
        c["examples"][0]["expected"] = False

    got = codes(seeded(mutate))
    assert {
        ("lowering", "ops"),
        ("lowering", "types"),
        ("faults", "faults"),
        ("controls", "controls"),
        ("lowering", "examples"),
    } <= got


def test_suite_set_and_order_are_checked():
    def drop(c):
        c["suites"].pop()

    assert ("suite-set", "suites") in codes(seeded(drop))

    def swap(c):
        c["suites"][0], c["suites"][1] = c["suites"][1], c["suites"][0]

    assert ("suite-set", "suites") in codes(seeded(swap))


def test_module_mismatch():
    def mutate(c):
        c["module"] = "other"

    assert ("profile", "module") in codes(seeded(mutate))


@pytest.mark.parametrize(
    "mutate",
    [
        lambda c: c.pop("faults"),
        lambda c: c.update(extra=1),
        lambda c: c.update(profile="warrant-corpus-v2"),
        lambda c: c.update(schema_version=2),
    ],
)
def test_structurally_wrong_corpora_are_refused(mutate):
    corpus = copy.deepcopy(CORPUS)
    mutate(corpus)
    with pytest.raises(OracleError) as err:
        gate(corpus)
    assert err.value.code == "corpus"


def test_the_corpus_reader_is_strict():
    with pytest.raises(OracleError):
        load_json(b'{"a": 1, "a": 2}')  # duplicate key
    with pytest.raises(OracleError):
        load_json(b'{"a": NaN}')
    with pytest.raises(OracleError):
        load_json(b'{"a": Infinity}')
    with pytest.raises(OracleError):
        load_json(b"\xff")
    with pytest.raises(OracleError):
        load_json(b"{")
    with pytest.raises(OracleError):
        load_json(b"[" * 100000)  # recursion bomb
    with pytest.raises(OracleError) as err:
        load_json(b"[" + b"0," * 100 + b"0]", limit=10)
    assert err.value.code == "limit"
    with pytest.raises(OracleError):
        verify_v1(SPEC, b"[]")
    assert load_json(b'{"a": [1, 2]}') == {"a": [1, 2]}


def test_v1_helpers_outside_the_profile_are_refused():
    spec = lower_document(parse_source(b"warrant x\n  op f(a: U16) -> U16\n"))
    with pytest.raises(OracleError):
        v1_faults(spec.ops)
    with pytest.raises(OracleError):
        v1_binder(TupleDesc(None, [leaf_bool()]))
    with pytest.raises(OracleError):
        typed_json(("some", 1), "Option<U8>")
    assert typed_json(("err", "X"), "Result<Bytes,E>") == {
        "type": "Result<Bytes,E>",
        "value": {"err": "X"},
    }
    assert typed_json(True, "Bool") is True


def test_a_document_with_no_claim_target_for_a_fault_is_refused():
    src = b"warrant x\n  op eq(a: Bytes, b: Bytes) -> Bool\n"
    spec = lower_document(parse_source(src))
    from coding_adventures_warrant_oracle.corpus_v1 import v1_controls

    with pytest.raises(OracleError):
        v1_controls(spec, v1_faults(spec.ops))


def test_a_corpus_for_a_spec_outside_the_v1_profile_fails_closed():
    spec_text = b"warrant ct_compare\n  op eq(a: Bytes, b: Bytes) -> Bool\n  claim c:\n    forall a: Bytes. eq(a, a) == true\n"
    corpus = copy.deepcopy(CORPUS)
    corpus["spec_sha256"] = hashlib.sha256(spec_text).hexdigest()
    report, _ = verify_v1(spec_text, json.dumps(corpus).encode())
    assert not report.ok
