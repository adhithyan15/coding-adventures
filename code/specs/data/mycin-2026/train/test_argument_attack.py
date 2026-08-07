#!/usr/bin/env python3
"""test_argument_attack.py — guard the ATTACK-edge round-trip (AD-6).

A paper's argument is rarely monotone: a later paragraph often REBUTS an earlier conclusion. AD-6
teaches the model-free scaffold to GENERATE such a dialectic as gold, VERIFY the engine resolves it
(the rebutted conclusion is WITHDRAWN), and SCORE a decomposer's recovery of the attack — including
the two failure modes that matter most: reversing the precedence, and inventing one.

Layers (mirroring test_gen_argument_data.py / test_argument_decompose_score.py):

  PURE (always run, no binaries):
    - every attack seed's premise/inference/attack span is a verbatim slice of the paragraph;
    - the emitted `.adj` carries the two `context:` tags, the `functional` head, and the
      `context_order` edge;
    - the attack training row round-trips through JSON and matches the AD-6 schema;
    - the scorer, given the gold as its own prediction, is all-perfect on the attack metrics with
      zero vetoes;
    - a REVERSED attack trips the wrong-direction veto; a FABRICATED attack trips its veto.

  GATE (skipped when adj-lang-cli / adj-verify are not built):
    - the seed RESOLVES: the engine byte-anchors every citation AND withdraws the defeated
      conclusion (winner governs, loser defeated_by the winner);
    - the scorer's attack_resolution gate is 1 for the gold and 0 for a reversed attack.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import argument_decompose_score as s  # noqa: E402
import gen_argument_data as gad  # noqa: E402

# The attack gold-objects + their notes — the fixtures every check scores against.
_ATTACK = [
    (gad.to_attack_training_row(spec, gad.source_bytes_for(spec).decode("utf-8")),
     gad.source_bytes_for(spec).decode("utf-8"))
    for spec in gad.ATTACK_SEED
]


# ---------------------------------------------------------------------------
# PURE — generation faithfulness.
# ---------------------------------------------------------------------------

@pytest.mark.parametrize("spec", gad.ATTACK_SEED, ids=[s["id"] for s in gad.ATTACK_SEED])
def test_attack_spans_are_faithful_and_adj_carries_the_attack_surface(spec):
    """Every premise/inference/attack quote is a verbatim slice of the paragraph, and the emitted
    `.adj` expresses the attack: each step's `context:` tag, the `functional` head, and one
    `context_order { winner > loser }` edge per attack."""
    sb = gad.source_bytes_for(spec)
    source = sb.decode("utf-8")
    for item in spec["premises"] + spec["inferences"] + spec["attacks"]:
        assert item["quote"] in source, f"{spec['id']}: quote not verbatim: {item['quote']!r}"
    adj_text, _ = gad.build_argument_adj(spec, sb)
    for inf in spec["inferences"]:
        assert f'context: {inf["context"]}' in adj_text, "each infer carries its context tag"
    assert f'functional {spec["functional"]}' in adj_text, "the functional head is emitted"
    for hi, lo in spec["context_order"]:
        assert f'context_order {{ {hi} > {lo} }}' in adj_text, "the precedence edge is emitted"


@pytest.mark.parametrize("spec", gad.ATTACK_SEED, ids=[s["id"] for s in gad.ATTACK_SEED])
def test_attack_training_row_matches_schema(spec):
    """The AD-6 row is JSON-clean and carries the attack fields: inferences with a `context`, a
    `functional` head, and an `attacks` list whose edges name the directed winner/loser."""
    source = gad.source_bytes_for(spec).decode("utf-8")
    row = gad.to_attack_training_row(spec, source)
    assert json.loads(json.dumps(row)) == row  # JSON-clean
    gold = row["gold"]
    assert gold["functional"] and gold["attacks"], "an attack row must have a functional head + edges"
    for i in gold["inferences"]:
        assert "context" in i, "each inference records its context tag"
    for a in gold["attacks"]:
        assert set(a) == {"kind", "defeater", "defeated", "winner_context", "loser_context",
                          "winner_conclusion", "loser_conclusion", "span"}
        assert a["winner_conclusion"] != a["loser_conclusion"], "an attack pits two conclusions"


# ---------------------------------------------------------------------------
# PURE — scoring: self-consistency + the two attack vetoes.
# ---------------------------------------------------------------------------

def test_attack_gold_scored_as_itself_is_perfect():
    """Self-consistency: scoring the attack gold against itself is perfect on the attack metrics with
    no vetoes (mirrors the support self-check)."""
    for row, note in _ATTACK:
        gold = row["gold"]
        m = s.score(gold, gold, note, run_gate=False)
        assert m["attack_precision"] == 1.0 and m["attack_recall"] == 1.0 and m["attack_f1"] == 1.0
        assert m["attack_wrong_direction"] == 0 and m["attack_fabrication"] == 0


def test_reversed_attack_trips_the_wrong_direction_veto():
    """A prediction that keeps the right conflict but backs the LOSER (swaps winner/loser) trips the
    wrong-direction veto — the single most dangerous attack error, since the engine would withdraw
    the correct conclusion. The span is still verbatim, so only the veto catches it."""
    row, note = _ATTACK[0]
    gold = row["gold"]
    atk = dict(gold["attacks"][0])
    atk["winner_conclusion"], atk["loser_conclusion"] = atk["loser_conclusion"], atk["winner_conclusion"]
    atk["winner_context"], atk["loser_context"] = atk["loser_context"], atk["winner_context"]
    predicted = {**gold, "attacks": [atk]}
    m = s.score(predicted, gold, note, run_gate=False)
    assert m["attack_wrong_direction"] == 1, "a reversed attack must be vetoed"
    assert m["attack_fabrication"] == 0, "the span is real — not a fabrication"
    assert m["attack_recall"] < 1.0, "a reversed attack does not match the gold edge"


def test_fabricated_attack_trips_the_fabrication_veto():
    """A prediction asserting a precedence whose sentence is not in the paragraph trips the
    attack-fabrication veto — an invented warrant for a withdrawal the text never licenses."""
    row, note = _ATTACK[0]
    gold = row["gold"]
    atk = dict(gold["attacks"][0])
    atk["span"] = "the committee unanimously overruled the original authors"  # never in the note
    predicted = {**gold, "attacks": [atk]}
    m = s.score(predicted, gold, note, run_gate=False)
    assert m["attack_fabrication"] == 1, "an attack span absent from the note must be vetoed"


# ---------------------------------------------------------------------------
# GATE — the engine actually resolves the dialectic (needs the built binaries).
# ---------------------------------------------------------------------------

_HAVE_BINS = gad.CLI.exists() and gad.VERIFY.exists()


@pytest.mark.skipif(not _HAVE_BINS, reason="adj-lang-cli / adj-verify not built")
@pytest.mark.parametrize("spec", gad.ATTACK_SEED, ids=[s["id"] for s in gad.ATTACK_SEED])
def test_seed_resolves_the_dialectic(spec):
    """The full attack gate: the seed's gold `.adj` byte-anchors every citation AND the engine
    withdraws the defeated conclusion — winner governs, loser defeated_by the winner."""
    res = gad.verify_attack_gold(spec)
    assert res["derive_ok"] and res["verify_ok"] and res["verified"] is True
    assert res["quotes_verified"] == gad.total_citations(spec), "every citation byte-anchors"
    assert res["winner_governs"], "the superseding conclusion must govern"
    assert res["loser_defeated"] and res["defeated_by_winner"], "the rebutted conclusion is withdrawn"


@pytest.mark.skipif(not _HAVE_BINS, reason="adj-lang-cli / adj-verify not built")
def test_attack_resolution_gate_rewards_the_gold_and_fails_a_reversal():
    """With the binaries built: the gold decomposition RESOLVES (attack_resolution == 1), and a
    reversed precedence makes the engine withdraw the wrong conclusion, so the gate returns 0."""
    row, note = _ATTACK[0]
    gold = row["gold"]
    assert s.attack_resolution(gold, gold, note) == 1, "the gold attack must resolve"
    atk = dict(gold["attacks"][0])
    atk["winner_context"], atk["loser_context"] = atk["loser_context"], atk["winner_context"]
    atk["winner_conclusion"], atk["loser_conclusion"] = atk["loser_conclusion"], atk["winner_conclusion"]
    reversed_pred = {**gold, "attacks": [atk]}
    assert s.attack_resolution(reversed_pred, gold, note) == 0, "a reversed context_order must not resolve"
