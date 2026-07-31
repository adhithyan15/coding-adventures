#!/usr/bin/env python3
"""test_argument_decompose_score.py — guard the ARGUMENT fidelity scorer (AD-3).

Pure, model-free checks (the thesis-derivation gate self-skips when the binaries are absent):
  - SELF-CONSISTENCY: the AD-2 seed gold, scored as its own prediction, is all-perfect — every
    ratio 1.0, every veto 0 (mirrors eval_decompose --self-check).
  - the NEAR-MISS veto fires when a prediction turns a gold DISCARD span into a premise.
  - the FABRICATION veto fires when a prediction cites bytes not in the paragraph.
  - a dropped premise LOWERS recall (and, via the gate when built, breaks thesis-derivation).
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import argument_decompose_score as s  # noqa: E402
import gen_argument_data as gad  # noqa: E402

# The gold-objects + notes from the AD-2 seed set — the fixtures every check scores against.
_GOLD = [
    (gad.to_training_row(spec, gad.source_bytes_for(spec).decode("utf-8")),
     gad.source_bytes_for(spec).decode("utf-8"))
    for spec in gad.SEED
]


def test_gold_scored_as_itself_is_perfect():
    """Self-consistency: scoring the gold against itself yields perfect fidelity and no vetoes."""
    for row, note in _GOLD:
        gold = row["gold"]
        m = s.score(gold, gold, note, run_gate=False)
        for ratio in ("premise_precision", "premise_recall", "premise_f1",
                      "inference_precision", "inference_recall", "inference_f1",
                      "span_faithfulness"):
            assert m[ratio] == 1.0, f"{row['id']}: {ratio} must be 1.0, got {m[ratio]}"
        assert m["near_miss_violations"] == 0 and m["fabrication"] == 0


def test_near_miss_span_turned_into_a_premise_is_vetoed():
    """A prediction that coins a premise from a gold DISCARD span trips the near-miss veto — even
    though that span IS in the paragraph (span_faithfulness stays 1.0), so only the veto catches it."""
    row = next(r for r, _ in _GOLD if r["id"] == "arg-pump-outbreak")
    note = next(n for r, n in _GOLD if r["id"] == "arg-pump-outbreak")
    gold = row["gold"]
    discard_span = gold["discard"][0]["span"]  # the coincidental-factory near-miss
    predicted = {
        "premises": gold["premises"] + [
            {"name": "pX", "kind": "extracted", "term": "changed_schedule(factory)",
             "span": discard_span, "type": "stated"}
        ],
        "inferences": gold["inferences"], "thesis": gold["thesis"], "discard": [],
    }
    m = s.score(predicted, gold, note, run_gate=False)
    assert m["near_miss_violations"] == 1, "coining a premise from a discard span must be vetoed"
    assert m["fabrication"] == 0, "the discard span IS in the note — not a fabrication"
    assert m["span_faithfulness"] == 1.0, "every span is verbatim — only the near-miss veto fires"


def test_fabricated_span_is_vetoed():
    """A prediction citing bytes absent from the paragraph trips the fabrication veto and drops
    span-faithfulness below 1.0."""
    row = next(r for r, _ in _GOLD if r["id"] == "arg-galaxy-redshift")
    note = next(n for r, n in _GOLD if r["id"] == "arg-galaxy-redshift")
    gold = row["gold"]
    predicted = {
        "premises": gold["premises"] + [
            {"name": "pF", "kind": "extracted", "term": "invented(claim)",
             "span": "this sentence never appears in the source paragraph", "type": "stated"}
        ],
        "inferences": gold["inferences"], "thesis": gold["thesis"], "discard": [],
    }
    m = s.score(predicted, gold, note, run_gate=False)
    assert m["fabrication"] == 1, "a span absent from the note must be vetoed as fabrication"
    assert m["span_faithfulness"] < 1.0, "an ungrounded span lowers span-faithfulness"


def test_dropped_premise_lowers_recall():
    """Omitting a real premise lowers premise recall (a partial decomposition is not perfect)."""
    row = next(r for r, _ in _GOLD if r["id"] == "arg-axle-fatigue")
    note = next(n for r, n in _GOLD if r["id"] == "arg-axle-fatigue")
    gold = row["gold"]
    predicted = {
        "premises": gold["premises"][:-1],  # drop one premise
        "inferences": gold["inferences"], "thesis": gold["thesis"], "discard": [],
    }
    m = s.score(predicted, gold, note, run_gate=False)
    assert m["premise_recall"] < 1.0, "a dropped premise must lower recall"
    assert m["premise_precision"] == 1.0, "the kept premises are still all correct"


def test_wrong_span_breaks_the_premise_match():
    """A premise with the right term but the WRONG span does not match gold — fidelity is about
    provenance, not just the extracted proposition."""
    row = next(r for r, _ in _GOLD if r["id"] == "arg-axle-fatigue")
    note = next(n for r, n in _GOLD if r["id"] == "arg-axle-fatigue")
    gold = row["gold"]
    tampered = [dict(p) for p in gold["premises"]]
    tampered[0]["span"] = "cyclic loading"  # a real substring, but not the premise's span
    predicted = {"premises": tampered, "inferences": gold["inferences"],
                 "thesis": gold["thesis"], "discard": []}
    m = s.score(predicted, gold, note, run_gate=False)
    assert m["premise_recall"] < 1.0, "a wrong span must break the premise match"
    assert m["span_faithfulness"] == 1.0, "the wrong span is still a real substring (verbatim)"


def test_aggregate_means_ratios_and_sums_vetoes():
    """aggregate averages ratios and sums the veto counts across examples."""
    scores = [s.score(r["gold"], r["gold"], n, run_gate=False) for r, n in _GOLD]
    agg = s.aggregate(scores)
    assert agg["n"] == len(_GOLD)
    assert agg["premise_f1"] == 1.0 and agg["inference_f1"] == 1.0
    assert agg["near_miss_violations"] == 0 and agg["fabrication"] == 0


# ---------------------------------------------------------------------------
# GATE — the real thesis-derivation check (needs the built binaries).
# ---------------------------------------------------------------------------

_HAVE_BINS = gad.CLI.exists() and gad.VERIFY.exists()


@pytest.mark.skipif(not _HAVE_BINS, reason="adj-lang-cli / adj-verify not built")
def test_gold_derives_its_thesis_and_a_dropped_premise_does_not():
    """With the binaries built: the gold argument DERIVES its thesis (thesis_derivation == 1), and a
    prediction that drops a load-bearing premise fails to derive it (== 0)."""
    row = next(r for r, _ in _GOLD if r["id"] == "arg-galaxy-redshift")
    note = next(n for r, n in _GOLD if r["id"] == "arg-galaxy-redshift")
    gold = row["gold"]
    assert s.thesis_derivation(gold, gold, note) == 1, "the gold argument must derive its thesis"
    broken = {"premises": gold["premises"][:1], "inferences": gold["inferences"],
              "thesis": gold["thesis"], "discard": []}
    assert s.thesis_derivation(broken, gold, note) == 0, "dropping a premise must break derivation"
