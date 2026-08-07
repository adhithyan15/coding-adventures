#!/usr/bin/env python3
"""test_eval_argument_decompose.py — guard the held-out ARGUMENT eval set + runner (AD-4).

Pure checks (the gate self-skips when the binaries are absent):
  - the curated eval gold is SELF-CONSISTENT: every gold, scored as its own prediction, is
    all-perfect (every ratio 1.0, every veto 0) — `--self-check` passes.
  - every gold span is a VERBATIM substring of its note (byte-provenance holds by construction).
  - a missing prediction scores as an empty argument (penalized, not skipped).
  - the eval domains are DISJOINT from the AD-2 seed (a genuine held-out set).
  - GATE (binaries built): every eval gold DERIVES its thesis and byte-anchors its citations.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import argument_decompose_score as ads  # noqa: E402
import eval_argument_decompose as ev  # noqa: E402
import gen_argument_data as gad  # noqa: E402

_RECORDS = ev.load_eval()


def test_the_eval_set_is_nonempty_and_well_formed():
    assert _RECORDS, "the held-out eval set must not be empty"
    for r in _RECORDS:
        assert r["shape"] == "argument"
        g = r["gold"]
        assert g["premises"] and g["inferences"] and g["thesis"]


def test_gold_is_self_consistent_offline():
    """Every gold, scored as its own prediction, is all-perfect (the offline --self-check)."""
    assert ev.self_check(_RECORDS, run_gate=False) == 0


def test_every_gold_span_is_verbatim_in_its_note():
    for r in _RECORDS:
        note = r["note"]
        for item in r["gold"]["premises"] + r["gold"]["inferences"]:
            assert item["span"] in note, f"{r['id']}: span not verbatim: {item['span']!r}"
        for d in r["gold"].get("discard", []):
            assert d["span"] in note, f"{r['id']}: discard span not verbatim: {d['span']!r}"


def test_missing_prediction_scores_as_empty_not_skipped():
    """An id with no prediction is scored as an empty argument — recall drops, not silently 1.0."""
    rows, agg = ev.score_predictions(_RECORDS, {}, run_gate=False)
    assert agg["n"] == len(_RECORDS)
    # Every record had gold premises but no prediction → recall 0 everywhere.
    assert all(row["premise_recall"] == 0.0 for row in rows)


def test_eval_domains_are_disjoint_from_the_seed():
    """A genuine held-out set: its domains don't overlap the AD-2 seed's."""
    eval_domains = {r.get("domain") for r in _RECORDS}
    seed_domains = {s["domain"] for s in gad.SEED}
    assert eval_domains and not (eval_domains & seed_domains), \
        f"held-out domains must be disjoint from seed: {eval_domains} vs {seed_domains}"


def test_scored_prediction_of_the_gold_is_perfect():
    """score_predictions with the gold as prediction yields perfect aggregate fidelity."""
    preds = {r["id"]: r["gold"] for r in _RECORDS}
    _, agg = ev.score_predictions(_RECORDS, preds, run_gate=False)
    assert agg["premise_f1"] == 1.0 and agg["inference_f1"] == 1.0
    assert agg["near_miss_violations"] == 0 and agg["fabrication"] == 0


# ---------------------------------------------------------------------------
# GATE — the real 3-part correctness check (needs the built binaries).
# ---------------------------------------------------------------------------

_HAVE_BINS = gad.CLI.exists() and gad.VERIFY.exists()


@pytest.mark.skipif(not _HAVE_BINS, reason="adj-lang-cli / adj-verify not built")
def test_every_eval_gold_derives_its_thesis():
    for r in _RECORDS:
        assert ads.thesis_derivation(r["gold"], r["gold"], r["note"]) == 1, \
            f"{r['id']}: held-out gold must derive its thesis + byte-anchor"
