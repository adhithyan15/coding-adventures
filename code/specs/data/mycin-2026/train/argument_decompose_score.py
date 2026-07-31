#!/usr/bin/env python3
"""argument_decompose_score.py — score an ARGUMENT decomposer's fidelity (model-free).

The counterpart to decompose_score.py for the OPEN-VOCAB `argument` shape
(ADJ-ARGUMENT-DECOMPOSER.md §4). Where decompose_score scores flat findings/chart-facts,
this scores a decomposed ARGUMENT: did the model extract the right premises and inference
steps, cite VERBATIM spans, reach the paragraph's thesis, and — the headline — NOT coin a
premise from something the gold says to DISCARD, and NOT fabricate a citation whose bytes
are not in the paragraph at all?

It is a PURE function over `(predicted, gold, note)` — no model, no network — so it is
fully unit-testable on synthetic predictions. `predicted` and `gold` are gold-objects in the
AD-2 training-row `gold` schema (ADJ-ARGUMENT-DECOMPOSER.md §3.2):
    {premises:[{name,kind,term,span,type}], inferences:[{name,connective,conclusion,from,span,type}],
     thesis:"…", discard:[{span,reason}]}

Metrics (all ratios in [0, 1] except the integer VETO counts):
  - premise_precision / premise_recall / premise_f1 — set overlap of predicted vs gold premises
    by identity (kind, term, normalized-span): a premise matches only if its SPAN is right too, so
    a fluent-but-unfaithful premise scores 0.
  - inference_precision / inference_recall / inference_f1 — same over inferences by identity
    (connective, conclusion, {from…}, normalized-span): an inference must cite the right premises
    AND the right connective bytes.
  - span_faithfulness — fraction of predicted premise+inference spans that are a VERBATIM
    (whitespace/case-normalized) substring of the note. Byte-provenance measured directly.
  - discard_recall / discard_precision — overlap of predicted vs gold discard spans.
  - near_miss_violations (VETO, must be 0) — COUNT of predicted premises/inferences whose span is
    a GOLD DISCARD span: the model turned a set-aside near-miss into a premise.
  - fabrication (VETO, must be 0) — COUNT of predicted premise/inference spans NOT present in the
    note at all: invented bytes, the strongest faithfulness failure.
  - thesis_derivation — 0/1: does the predicted argument, once compiled, actually DERIVE the gold
    thesis and BYTE-ANCHOR its citations (the real 3-part gate, via gen_argument_data.verify_gold)?
    `None` when the adj-lang-cli / adj-verify binaries are not built (skipped, like AD-2).
"""

from __future__ import annotations

import gen_argument_data as gad


def _norm(s: str) -> str:
    """Lowercase + collapse whitespace — the framework's citation-matching normalization
    (mirrors decompose_score._norm / gen_data._norm)."""
    return " ".join(str(s).lower().split())


def _f1(precision: float, recall: float) -> float:
    return 0.0 if precision + recall == 0 else 2 * precision * recall / (precision + recall)


def _prem_key(p: dict) -> tuple:
    """A premise's identity: its kind, its term, and its NORMALIZED span. Span is part of the
    key so a premise only matches when it is grounded on the right bytes."""
    return (p.get("kind"), _norm(p.get("term", "")), _norm(p.get("span", "")))


def _infer_key(i: dict) -> tuple:
    """An inference's identity: its connective, its conclusion, the SET of premises it cites, and
    its normalized span. `from` is a set so citation ORDER doesn't change identity."""
    return (i.get("connective"), _norm(i.get("conclusion", "")),
            frozenset(i.get("from", []) or []), _norm(i.get("span", "")))


def _pr(pred_keys: list, gold_keys: set) -> tuple[float, float]:
    """(precision, recall) with the abstain-friendly empty-set conventions decompose_score uses:
    no predicted AND no gold → (1, 1); no predicted but some gold → (1, 0); predicted but no gold
    → (0, 1)."""
    matched = [k for k in pred_keys if k in gold_keys]
    precision = 1.0 if not pred_keys else len(matched) / len(pred_keys)
    recall = 1.0 if not gold_keys else len({k for k in matched}) / len(gold_keys)
    return precision, recall


def _to_builder_spec(predicted: dict, thesis: str) -> dict:
    """Map a predicted gold-object to a gen_argument_data.build_argument_adj spec — the gold
    schema names the cited bytes `span`, the builder names them `quote`. Uses the GOLD `thesis`
    as the query, so the derivation check asks "do the predicted premises/inferences reach the
    paragraph's thesis?" A default doc/trust is fine: the byte-anchor is what's under test."""
    return {
        "name": "predicted",
        "doc": "predicted argument",
        "trust": "authoritative",
        "premises": [
            {"name": p.get("name"), "kind": p.get("kind"), "term": p.get("term"),
             "quote": p.get("span", "")}
            for p in predicted.get("premises", []) or []
        ],
        "inferences": [
            {"name": i.get("name"), "connective": i.get("connective"),
             "conclusion": i.get("conclusion"), "from": i.get("from", []) or [],
             "quote": i.get("span", "")}
            for i in predicted.get("inferences", []) or []
        ],
        "thesis": thesis,
    }


def thesis_derivation(predicted: dict, gold: dict, note: str) -> int | None:
    """Run the real 3-part gate on the predicted argument against the GOLD thesis: it must compile,
    let adj-lang-cli derive a NON-EMPTY answer for the gold thesis, and let adj-verify byte-anchor
    every citation. Returns 1/0, or `None` when the binaries are not built (not scored)."""
    if not (gad.CLI.exists() and gad.VERIFY.exists()):
        return None
    spec = _to_builder_spec(predicted, gold.get("thesis", ""))
    sb = note.encode("utf-8")
    try:
        adj_text, _ = gad.build_argument_adj(spec, sb)
    except gad.SpanNotFound:
        # A fabricated citation can't even be built — it cannot faithfully derive anything.
        return 0
    try:
        res = gad.verify_gold(adj_text, sb)
    except gad.BinariesMissing:
        return None
    derived = res["derive_ok"] and '"abstained":false' in res["derive_stdout"]
    anchored = res["verified"] is True and res["quotes_verified"] == gad.total_citations(spec)
    return 1 if (derived and anchored) else 0


def score(predicted: dict, gold: dict, note: str, *, run_gate: bool = True) -> dict:
    """Score one predicted argument against the gold argument for `note`. Pure over the metrics
    documented above; the optional `thesis_derivation` shells out to the built binaries (skipped
    when `run_gate` is False or the binaries are absent). Returns the metrics dict."""
    note_n = _norm(note)
    pred_prem = predicted.get("premises", []) or []
    gold_prem = gold.get("premises", []) or []
    pred_inf = predicted.get("inferences", []) or []
    gold_inf = gold.get("inferences", []) or []

    pp, pr_ = _pr([_prem_key(p) for p in pred_prem], {_prem_key(p) for p in gold_prem})
    ip, ir = _pr([_infer_key(i) for i in pred_inf], {_infer_key(i) for i in gold_inf})

    # Every predicted premise/inference makes a byte-provenance claim (a cited span).
    cited = [x for x in (pred_prem + pred_inf) if x.get("span")]
    grounded = [x for x in cited if _norm(x["span"]) in note_n]
    span_faithfulness = 1.0 if not cited else len(grounded) / len(cited)

    pred_disc = {_norm(d.get("span", "")) for d in (predicted.get("discard") or []) if d.get("span")}
    gold_disc = {_norm(d.get("span", "")) for d in (gold.get("discard") or []) if d.get("span")}
    matched_disc = pred_disc & gold_disc
    discard_recall = 1.0 if not gold_disc else len(matched_disc) / len(gold_disc)
    discard_precision = 1.0 if not pred_disc else len(matched_disc) / len(pred_disc)

    # VETOES. near-miss: a predicted premise/inference whose span the gold says to DISCARD.
    near_miss_violations = sum(1 for x in (pred_prem + pred_inf)
                               if x.get("span") and _norm(x["span"]) in gold_disc)
    # fabrication: a predicted span not present in the note at all (invented bytes).
    fabrication = sum(1 for x in cited if _norm(x["span"]) not in note_n)

    out = {
        "premise_precision": pp, "premise_recall": pr_, "premise_f1": _f1(pp, pr_),
        "inference_precision": ip, "inference_recall": ir, "inference_f1": _f1(ip, ir),
        "span_faithfulness": span_faithfulness,
        "discard_recall": discard_recall, "discard_precision": discard_precision,
        "near_miss_violations": near_miss_violations,
        "fabrication": fabrication,
    }
    out["thesis_derivation"] = thesis_derivation(predicted, gold, note) if run_gate else None
    return out


def aggregate(scores: list[dict]) -> dict:
    """Mean each ratio metric, SUM the veto counts, mean thesis_derivation over the examples that
    ran it (skips `None`). Empty input → {}."""
    if not scores:
        return {}
    ratio = ("premise_precision", "premise_recall", "premise_f1",
             "inference_precision", "inference_recall", "inference_f1",
             "span_faithfulness", "discard_recall", "discard_precision")
    counts = ("near_miss_violations", "fabrication")
    out = {m: sum(s[m] for s in scores) / len(scores) for m in ratio}
    out.update({m: sum(s[m] for s in scores) for m in counts})
    gated = [s["thesis_derivation"] for s in scores if s.get("thesis_derivation") is not None]
    out["thesis_derivation"] = (sum(gated) / len(gated)) if gated else None
    out["n"] = len(scores)
    return out
