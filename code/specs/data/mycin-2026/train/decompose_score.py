#!/usr/bin/env python3
"""decompose_score.py — score a decomposer's FIDELITY against a gold typed IR (model-free).

`eval_specialist.py` scores the *downstream diagnosis* (correct/wrong/abstained) a decomposed
chart produces — a coarse, end-to-end proxy that needs an MLX-served model to run. This module
scores the thing the framework actually cares about, **directly and offline**: did the model
extract the right facts, cite VERBATIM spans, discard the right distractors, and — the headline —
NOT coin a fact from something that should have been discarded (the near-miss over-extraction
failure the training data, gen_*.py, is built to prevent)?

It is a PURE function over `(predicted_ir, gold_ir, note)` — no model, no network — so it both
(a) gives `eval_specialist` a sharp fidelity metric to print alongside the diagnosis outcome, and
(b) is fully unit-testable on synthetic predictions (test_decompose_score.py). The model run that
produces `predicted_ir` stays the caller's step; the scoring is deterministic here.

Works for BOTH typed-IR shapes the decomposer emits:
  - findings  (gen_data.py):       items under "findings",   identity key (functor, value, polarity)
  - chart-facts (gen_chart_data.py): items under "chart_facts", identity key (kind, value)

Metrics (all in [0, 1] except the integer violation counts):
  - fact_precision / fact_recall / fact_f1 — set overlap of predicted vs gold facts by identity key.
  - span_faithfulness — fraction of predicted facts whose (non-empty) `span` is a VERBATIM
    (whitespace/case-normalized) substring of the note. A hallucinated/paraphrased span scores 0
    here; this is the byte-provenance discipline measured directly.
  - discard_recall / discard_precision — overlap of predicted vs gold `discard` spans (did the
    model set aside the red herrings it should, without inventing discards?).
  - near_miss_violations — COUNT of predicted facts whose span matches a GOLD DISCARD span: the
    model coined a fact from a phrase the gold says to discard (a relative's illness, a hedge, a
    pending test, …). The single most important faithfulness number — should be 0.
  - false_positive_facts — COUNT of predicted facts with no matching gold fact (over-extraction).
"""

from __future__ import annotations

FINDINGS = ("findings", ("functor", "value", "polarity"))
CHART_FACTS = ("chart_facts", ("kind", "value"))


def _norm(s: str) -> str:
    """Lowercase + collapse whitespace — the framework's citation-matching normalization
    (mirrors gen_data._norm), so a span counts as grounded under benign whitespace/case drift."""
    return " ".join(str(s).lower().split())


def _key(item: dict, fields: tuple[str, ...]) -> tuple:
    """The identity tuple a fact is matched on (e.g. (functor, value, polarity))."""
    return tuple(item.get(f) for f in fields)


def _f1(precision: float, recall: float) -> float:
    return 0.0 if precision + recall == 0 else 2 * precision * recall / (precision + recall)


def score_decompose(predicted: dict, gold: dict, note: str,
                    shape: tuple[str, tuple[str, ...]] = CHART_FACTS) -> dict:
    """Score one predicted IR against the gold IR for `note`. `shape` is FINDINGS or CHART_FACTS
    (the items field + the identity-key fields). Pure; returns the metrics dict documented above.

    Conventions for empty sets (so an honest abstain scores perfectly, not 0/0 → NaN):
      - no predicted facts AND no gold facts → precision = recall = f1 = 1.0 (correct abstention);
      - no predicted facts but some gold → precision = 1.0 (no false positives), recall = 0.0;
      - some predicted but no gold → precision = 0.0, recall = 1.0 (everything is a false positive).
    """
    items_field, key_fields = shape
    pred_items = predicted.get(items_field, []) or []
    gold_items = gold.get(items_field, []) or []
    note_n = _norm(note)

    pred_keys = [_key(i, key_fields) for i in pred_items]
    gold_keys = {_key(i, key_fields) for i in gold_items}
    matched = [k for k in pred_keys if k in gold_keys]

    fact_precision = 1.0 if not pred_keys else len(matched) / len(pred_keys)
    fact_recall = 1.0 if not gold_keys else len({k for k in matched}) / len(gold_keys)

    # Span faithfulness: every cited span must be a verbatim substring of the note. Facts with an
    # empty span (honest "inferred", no provenance) are excluded from the denominator — they make
    # no provenance claim to verify.
    cited = [i for i in pred_items if i.get("span")]
    grounded = [i for i in cited if _norm(i["span"]) in note_n]
    span_faithfulness = 1.0 if not cited else len(grounded) / len(cited)

    # Discards: span-set overlap (normalized).
    pred_disc = {_norm(d.get("span", "")) for d in (predicted.get("discard") or []) if d.get("span")}
    gold_disc = {_norm(d.get("span", "")) for d in (gold.get("discard") or []) if d.get("span")}
    matched_disc = pred_disc & gold_disc
    discard_recall = 1.0 if not gold_disc else len(matched_disc) / len(gold_disc)
    discard_precision = 1.0 if not pred_disc else len(matched_disc) / len(pred_disc)

    # The over-extraction failure: a predicted FACT whose span is one the gold says to DISCARD.
    near_miss_violations = sum(1 for i in pred_items
                               if i.get("span") and _norm(i["span"]) in gold_disc)
    false_positive_facts = sum(1 for k in pred_keys if k not in gold_keys)

    return {
        "fact_precision": fact_precision,
        "fact_recall": fact_recall,
        "fact_f1": _f1(fact_precision, fact_recall),
        "span_faithfulness": span_faithfulness,
        "discard_recall": discard_recall,
        "discard_precision": discard_precision,
        "near_miss_violations": near_miss_violations,
        "false_positive_facts": false_positive_facts,
    }


def aggregate(scores: list[dict]) -> dict:
    """Mean each ratio metric, SUM the violation counts, over a list of per-example score dicts.
    Empty input → an empty dict (the caller reports 'no examples')."""
    if not scores:
        return {}
    ratio = ("fact_precision", "fact_recall", "fact_f1", "span_faithfulness",
             "discard_recall", "discard_precision")
    counts = ("near_miss_violations", "false_positive_facts")
    out = {m: sum(s[m] for s in scores) / len(scores) for m in ratio}
    out.update({m: sum(s[m] for s in scores) for m in counts})
    out["n"] = len(scores)
    return out
