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

ATTACK edges (AD-6) — a paper's DIALECTIC, not just its support. When a later paragraph REBUTS an
earlier conclusion, the gold-object carries an `attacks` list; these metrics score whether the model
recovered the rebuttal faithfully AND in the right direction:
  - attack_precision / attack_recall / attack_f1 — set overlap of predicted vs gold attack edges by
    identity (kind, directed winner/loser CONCLUSIONS, winner/loser CONTEXTS, normalized-span): an
    attack matches only if it names the right conflict AND says the right side wins.
  - attack_wrong_direction (VETO, must be 0) — COUNT of predicted attacks that REVERSE a gold attack
    (predicted winner == gold loser and vice-versa): the model saw the conflict but backed the loser,
    which would make the engine withdraw the CORRECT conclusion. The dangerous failure.
  - attack_fabrication (VETO, must be 0) — COUNT of predicted attacks whose precedence-establishing
    span is NOT in the note: an invented warrant for a withdrawal the paragraph never licenses.
  - attack_resolution — 0/1: does the predicted argument, once built with its functional head and
    context_order and RUN, produce the engine's ADJ73 verdict gold expects — the winner GOVERNS and
    the loser is WITHDRAWN? `None` when there is no gold attack or the binaries are absent. This is
    the attack counterpart of thesis_derivation: the engine's governing output is the ground truth.
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


def _attack_key(a: dict) -> tuple:
    """An attack edge's identity (AD-6): its kind, the winner/loser CONCLUSIONS, the winner/loser
    CONTEXTS, and its normalized span. Both the conclusions and the contexts encode the precedence
    DIRECTION, so an attack that names the right pair but the wrong winner is a different key — it
    does not match gold."""
    return (a.get("kind"),
            _norm(a.get("winner_conclusion", "")), _norm(a.get("loser_conclusion", "")),
            a.get("winner_context"), a.get("loser_context"), _norm(a.get("span", "")))


def _to_attack_builder_spec(predicted: dict, gold: dict) -> dict:
    """Extend `_to_builder_spec` with the attack surface: carry each predicted inference's `context`
    tag, take the `functional` head from GOLD (structural, like the thesis), and derive a
    `context_order` edge from each PREDICTED rebut attack. Building from the PREDICTED precedence is
    what lets the resolution gate catch a reversed attack — a flipped context_order makes the engine
    withdraw the wrong conclusion, so the gate returns 0."""
    spec = _to_builder_spec(predicted, gold.get("thesis", ""))
    for src, dst in zip(predicted.get("inferences", []) or [], spec["inferences"]):
        if src.get("context"):
            dst["context"] = src["context"]
    spec["functional"] = gold.get("functional")
    spec["context_order"] = [
        (a["winner_context"], a["loser_context"])
        for a in (predicted.get("attacks") or [])
        if a.get("kind") == "rebut" and a.get("winner_context") and a.get("loser_context")
    ]
    return spec


def attack_resolution(predicted: dict, gold: dict, note: str) -> int | None:
    """The real gate for attack edges (AD-6): build the predicted argument WITH its functional head
    and context_order, run the engine, and check its ADJ73 verdict matches gold — for every gold
    rebut attack, the winner conclusion GOVERNS and the loser is WITHDRAWN. Returns 1/0, or `None`
    when there is no gold attack to resolve or the binaries are not built. A reversed or dropped
    attack scores 0 here: the engine's governing output is the ground truth, not the model's claim."""
    gold_attacks = [a for a in (gold.get("attacks") or []) if a.get("kind") == "rebut"]
    if not gold_attacks or not (gad.CLI.exists() and gad.VERIFY.exists()):
        return None
    spec = _to_attack_builder_spec(predicted, gold)
    sb = note.encode("utf-8")
    try:
        adj_text, _ = gad.build_argument_adj(spec, sb)
    except gad.SpanNotFound:
        return 0  # a fabricated citation can't even be built.
    try:
        answers = gad.governing_answers_for(adj_text)
    except gad.BinariesMissing:
        return None
    by_term = {_norm(a["term"]): a for a in answers}
    for atk in gold_attacks:
        win = by_term.get(_norm(atk.get("winner_conclusion", "")), {})
        lose = by_term.get(_norm(atk.get("loser_conclusion", "")), {})
        if win.get("status") != "governing" or lose.get("status") != "defeated":
            return 0
    return 1


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

    # ATTACK edges (AD-6). Precision/recall/f1 by full identity (kind + directed conclusions/contexts
    # + span), plus two vetoes mirroring the premise/inference ones:
    pred_atk = predicted.get("attacks", []) or []
    gold_atk = gold.get("attacks", []) or []
    ap, ar = _pr([_attack_key(a) for a in pred_atk], {_attack_key(a) for a in gold_atk})
    # The DIRECTED conclusion pairs gold asserts, so a REVERSAL (predicted winner == gold loser and
    # vice-versa) is detectable — the single most dangerous attack error, since it would make the
    # engine withdraw the correct conclusion.
    gold_pairs = {(_norm(a.get("winner_conclusion", "")), _norm(a.get("loser_conclusion", "")))
                  for a in gold_atk}
    attack_wrong_direction = sum(
        1 for a in pred_atk
        if (_norm(a.get("loser_conclusion", "")), _norm(a.get("winner_conclusion", ""))) in gold_pairs
        and (_norm(a.get("winner_conclusion", "")), _norm(a.get("loser_conclusion", ""))) not in gold_pairs
    )
    # A predicted attack whose precedence sentence is not in the note at all — an invented warrant
    # for a withdrawal the paragraph never licenses.
    attack_fabrication = sum(1 for a in pred_atk
                             if a.get("span") and _norm(a["span"]) not in note_n)

    out = {
        "premise_precision": pp, "premise_recall": pr_, "premise_f1": _f1(pp, pr_),
        "inference_precision": ip, "inference_recall": ir, "inference_f1": _f1(ip, ir),
        "span_faithfulness": span_faithfulness,
        "discard_recall": discard_recall, "discard_precision": discard_precision,
        "attack_precision": ap, "attack_recall": ar, "attack_f1": _f1(ap, ar),
        "near_miss_violations": near_miss_violations,
        "fabrication": fabrication,
        "attack_wrong_direction": attack_wrong_direction,
        "attack_fabrication": attack_fabrication,
    }
    out["thesis_derivation"] = thesis_derivation(predicted, gold, note) if run_gate else None
    out["attack_resolution"] = attack_resolution(predicted, gold, note) if run_gate else None
    return out


def aggregate(scores: list[dict]) -> dict:
    """Mean each ratio metric, SUM the veto counts, mean thesis_derivation over the examples that
    ran it (skips `None`). Empty input → {}."""
    if not scores:
        return {}
    ratio = ("premise_precision", "premise_recall", "premise_f1",
             "inference_precision", "inference_recall", "inference_f1",
             "span_faithfulness", "discard_recall", "discard_precision",
             "attack_precision", "attack_recall", "attack_f1")
    counts = ("near_miss_violations", "fabrication",
              "attack_wrong_direction", "attack_fabrication")
    out = {m: sum(s[m] for s in scores) / len(scores) for m in ratio}
    out.update({m: sum(s[m] for s in scores) for m in counts})
    gated = [s["thesis_derivation"] for s in scores if s.get("thesis_derivation") is not None]
    out["thesis_derivation"] = (sum(gated) / len(gated)) if gated else None
    resolved = [s["attack_resolution"] for s in scores if s.get("attack_resolution") is not None]
    out["attack_resolution"] = (sum(resolved) / len(resolved)) if resolved else None
    out["n"] = len(scores)
    return out
