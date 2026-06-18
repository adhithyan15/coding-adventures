#!/usr/bin/env python3
"""test_board_offline.py — the zero-online-call board pipeline (MYCIN-2026).

Pins three contracts:
  1. offline_guard.no_network() actually trips on outbound egress (and permits
     loopback / local IPC), so "no online call" is enforced, not asserted.
  2. decompose_query turns prose into a legal ADJ recall query, and any junk the
     local model emits degrades to None → an abstention, never a fabricated answer.
  3. board_offline answers covered prose items correctly through the NATIVE engine
     with online_calls == 0, abstains on uncovered ones, and never fabricates.

Run:  python3 test_board_offline.py
"""

from __future__ import annotations

import json
import socket
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import board_offline as bo  # noqa: E402
import decompose_query as dq  # noqa: E402
from offline_guard import OnlineCallError, no_network, proves_offline  # noqa: E402


# ---- 1. the network-egress tripwire -------------------------------------------

def test_no_network_blocks_outbound_connect() -> None:
    raised = False
    with no_network():
        try:
            socket.create_connection(("example.com", 80), timeout=1)
        except OnlineCallError:
            raised = True
    assert raised, "an outbound connection inside no_network() must raise OnlineCallError"


def test_no_network_blocks_socket_connect_method() -> None:
    g = no_network()
    raised = False
    with g:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        try:
            s.connect(("93.184.216.34", 80))  # a routable address, not loopback
        except OnlineCallError:
            raised = True
        finally:
            s.close()
    assert raised
    assert len(g.attempts) == 1, "the guard records each blocked egress attempt"


def test_no_network_permits_loopback() -> None:
    # A local model server / IPC on loopback is 'offline' and must NOT be blocked.
    g = no_network()
    with g:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(0.01)
        try:
            s.connect_ex(("127.0.0.1", 9))  # discard port; refusal is fine, not a block
        except OnlineCallError:
            raise AssertionError("loopback must be permitted inside no_network()")
        finally:
            s.close()
    assert g.attempts == [], "loopback is not an egress attempt"


def test_proves_offline_decorator_passes_through_result() -> None:
    @proves_offline
    def compute() -> int:
        return 21 * 2
    assert compute() == 42


# ---- 2. decomposition (prose → ADJ query), local-model-agnostic ---------------

def test_parse_query_accepts_prose_wrapped_json() -> None:
    raw = 'Sure! Here is the query:\n{"relation": "deficient_in", "subject": "Tay-Sachs", "var": "Enzyme"} done'
    q = dq.parse_query(raw)
    assert q == {"relation": "deficient_in", "subject": "tay_sachs", "var": "Enzyme"}


def test_parse_query_rejects_illegal_relation() -> None:
    assert dq.parse_query('{"relation": "cures", "subject": "x", "var": "Y"}') is None
    assert dq.parse_query("no json here") is None


def test_parse_query_pins_conventional_variable() -> None:
    # Even if the model names the wrong variable, the relation's conventional var wins.
    q = dq.parse_query('{"relation": "factor_deficiency", "subject": "hemophilia_a", "var": "Wrong"}')
    assert q == {"relation": "factor_deficiency", "subject": "hemophilia_a", "var": "Factor"}


def test_build_vocab_lists_canonical_subjects() -> None:
    vocab = dq.build_vocab()
    assert "tay_sachs" in vocab["deficient_in"]
    assert "hemophilia_a" in vocab["factor_deficiency"]


def test_faithfulness_gate_attests_subject_in_stem() -> None:
    # The chosen subject must be grounded in the stem's bytes (byte-provenance on the query).
    assert dq.attested_in_stem("von_gierke", "Von Gierke disease is diagnosed. Which enzyme?")
    assert dq.attested_in_stem("hereditary_spherocytosis", "...hereditary spherocytosis... finding?")
    # A mis-map to a different (valid) entity is NOT attested → rejected → abstention,
    # which is how the gate turns a confident wrong answer into an honest UNKNOWN.
    assert not dq.attested_in_stem("fabry", "Von Gierke disease is diagnosed. Which enzyme?")


def test_faithfulness_gate_turns_misdecomposition_into_abstention() -> None:
    # A model that confidently emits a DIFFERENT valid subject than the stem names must
    # NOT produce a wrong answer — the gate rejects the un-attested query → abstain.
    stem = "An infant with Von Gierke disease. Which enzyme is deficient?"
    vocab = dq.build_vocab()
    mis = lambda _p: '{"relation": "deficient_in", "subject": "fabry", "var": "Enzyme"}'  # noqa: E731
    assert dq.decompose(stem, mis, vocab, faithful=True) is None
    # Without the gate, the (legal, valid) query would pass through.
    assert dq.decompose(stem, mis, vocab, faithful=False) == {
        "relation": "deficient_in", "subject": "fabry", "var": "Enzyme"}


def test_relation_cues_derive_only_from_controlled_vocab() -> None:
    # Cues are the relation name's word-parts + its conventional variable — no new
    # medical knowledge, just the words the relation is already spelled with.
    assert dq.RELATION_CUES["has_mcv"] == {"mcv", "class"}
    assert dq.RELATION_CUES["classic_finding"] == {"classic", "finding"}
    assert dq.RELATION_CUES["deficient_in"] == {"deficient", "enzyme"}  # structural "in" dropped


def test_relation_gate_attests_interrogative() -> None:
    # The stem must ASK for what the relation answers (whole-word cue match).
    assert dq.relation_attested_in_stem("classic_finding", "What is the classic smear finding?")
    # has_mcv is NOT what a finding question asks — and 'class' must not match inside 'classic'.
    assert not dq.relation_attested_in_stem("has_mcv", "What is the classic peripheral-smear finding?")
    assert dq.relation_attested_in_stem("has_mcv", "What MCV class is iron deficiency anemia?")


def test_relation_gate_turns_wrong_relation_into_abstention() -> None:
    # The documented residual: right subject, WRONG relation (has_mcv on a finding stem)
    # resolved to a real-but-wrong edge. The relation gate rejects it → abstain.
    stem = "A patient with hereditary spherocytosis. What is the classic peripheral-smear finding?"
    vocab = dq.build_vocab()
    wrong_rel = lambda _p: ('{"relation": "has_mcv", "subject": "hereditary_spherocytosis", '  # noqa: E731
                            '"var": "Class"}')
    assert dq.decompose(stem, wrong_rel, vocab, faithful=True) is None
    # The correct relation for that stem is attested and passes.
    right_rel = lambda _p: ('{"relation": "classic_finding", "subject": "hereditary_spherocytosis", '  # noqa: E731
                            '"var": "Finding"}')
    assert dq.decompose(stem, right_rel, vocab, faithful=True) == {
        "relation": "classic_finding", "subject": "hereditary_spherocytosis", "var": "Finding"}


def test_every_gold_query_passes_both_gates() -> None:
    # No gold query may be false-rejected by either gate (subject AND relation attested).
    for it in _items():
        q, stem = it["query"], it["stem"]
        assert dq.attested_in_stem(q["subject"], stem), f"subject gate false-rejects {it['id']}"
        assert dq.relation_attested_in_stem(q["relation"], stem), f"relation gate false-rejects {it['id']}"


# ---- 3. end-to-end offline scoring --------------------------------------------

def _items():
    return json.loads((HERE / "free_text_board.json").read_text())["items"]


def test_cached_mode_is_offline_and_never_wrong() -> None:
    # Holds with or without the engine: wrong == 0 and zero online calls always.
    card = bo.score(_items(), gen=None)
    s = card["summary"]
    assert s["wrong"] == 0
    assert s["online_calls"] == 0


def test_cached_mode_answers_covered_items_with_citations() -> None:
    if not bo.be.cli_available():
        return  # the native engine answers; skip in a Python-only env
    card = bo.score(_items(), gen=None)
    s = card["summary"]
    assert s["online_calls"] == 0
    assert s["defensibility"] == 1.0
    by_id = {r["id"]: r for r in card["results"]}
    assert by_id["ft_tay_sachs"]["outcome"] == "correct"
    assert by_id["ft_tay_sachs"]["answer"] == "hexosaminidase_a"
    assert by_id["ft_tay_sachs"]["trust"] == "authoritative"
    # Uncovered entities abstain, never fabricate.
    assert by_id["ft_wilson_abstain"]["outcome"] == "abstained"
    assert by_id["ft_glucagon_abstain"]["outcome"] == "abstained"


def test_stub_model_drives_the_pipeline_correctly() -> None:
    if not bo.be.cli_available():
        return
    items = _items()
    # A perfect local model: emit the gold query JSON for each stem (keyed by stem).
    gold_by_stem = {it["stem"]: it["query"] for it in items}

    def perfect_gen(prompt: str) -> str:
        # The stem is embedded near the end of the decompose prompt.
        stem = next(s for s in gold_by_stem if s in prompt)
        return json.dumps(gold_by_stem[stem])

    card = bo.score(items, gen=perfect_gen)
    s = card["summary"]
    assert s["online_calls"] == 0
    assert s["wrong"] == 0
    assert s["decompose_accuracy"] == 1.0
    assert s["mode"] == "local_model_decompose"


def test_garbage_model_abstains_never_fabricates() -> None:
    # A model that emits nothing usable must yield abstentions, not wrong answers,
    # and certainly no online calls.
    card = bo.score(_items(), gen=lambda _prompt: "I am not sure.")
    s = card["summary"]
    assert s["wrong"] == 0
    assert s["online_calls"] == 0
    assert s["correct"] == 0  # nothing decoded → nothing answered


def _run() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  PASS  {t.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"  FAIL  {t.__name__}: {exc}")
    print(f"\ntest_board_offline: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run())
