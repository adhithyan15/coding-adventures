#!/usr/bin/env python3
"""ADJ65 — uncertainty as a first-class primitive (weight of evidence + decision sensitivity).

The gates so far answer YES/NO questions: is this claim grounded? is the conclusion
determined? But a real decision is rarely a clean yes/no — it is a *competition* between
hypotheses, each pushed on by the evidence with some strength. This module makes that
competition first-class, and then asks the question Adhithya posed:

    "If we make some probability shift, how would the decision shift?"

THE MODEL (Good's weight of evidence, the formulation behind MYCIN's certainty factors).
Each piece of grounded evidence E_j contributes, for each hypothesis H_i, a *weight* in
**decibans** — ten times the log10 likelihood ratio:

    w_ij = 10 · log10  P(E_j | H_i) / P(E_j | baseline)

Decibans are additive log-odds: +10 dB means the evidence makes H_i ten times more likely;
−10 dB, ten times less. The score of a hypothesis is its prior log-odds plus the evidence
it has collected:

    score_i = prior_i + Σ_j w_ij          (decibans)

The DECISION is the argmax of the scores — deterministic, no temperature, no softmax. (We
expose posteriors via softmax purely as a human-readable *view*; nothing downstream depends
on them, so there is no entropy knob to tune — the past softmax mistake is not repeated.)

THE PRIMITIVE WE ACTUALLY WANT IS SENSITIVITY. A single number for the winner is the least
interesting output. What a decision-maker needs is:

  - MARGIN  M = score(leader) − score(runner-up), in decibans. The decision survives any
    single weight perturbation smaller than M. M *is* the robustness of the call.
  - LOAD-BEARING evidence: ranked by how much each fact pushes leader-over-runner
    (d_j = w_leader,j − w_runner,j). The fact with d_j closest to M is carrying the call.
  - INDIVIDUALLY DECISIVE evidence: those whose removal alone flips the decision (d_j > M).
  - TIPPING POINT: how far you must shift a weight (or the prior) to change the answer, and
    WHICH hypothesis it changes to.
  - PROVENANCE OF THE MARGIN: of the load-bearing weights, which are `grounded` (cite a real
    likelihood ratio) and which are `assumed` (an ungrounded estimate). A decision whose
    margin rests on an *assumed* weight is fragile in the one way that matters — go ground
    that weight first (it is an ADJ64 hole, now prioritized by how much the decision depends
    on it).

This is why sensitivity, not a point probability, is the honest primitive: the weights are
the soft underbelly (often interpretation, not byte-grounded), and the right response to
soft inputs is not false precision — it is to report exactly how much the softness can move
the answer.

Usage (library): assess(hyps, evidence, prior=None) -> report.
  evidence: [{"name","weights":{hyp: decibans}, "source":"grounded"|"assumed", "citation":""}]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path


def _scores(hyps: list[str], prior: dict, evidence: list[dict]) -> dict:
    """score_i = prior_i + Σ_j w_ij, in decibans."""
    s = {h: float(prior.get(h, 0.0)) for h in hyps}
    for e in evidence:
        for h, w in (e.get("weights") or {}).items():
            if h in s:
                s[h] += float(w)
    return s


def _rank(scores: dict) -> list[tuple]:
    return sorted(scores.items(), key=lambda kv: -kv[1])


def posteriors(scores: dict) -> dict:
    """A human-readable VIEW only — softmax over the log-odds scores (decibans -> prob).
    The decision never depends on this; it is argmax of `scores`."""
    if not scores:
        return {}
    m = max(scores.values())
    exps = {h: 10 ** ((v - m) / 10.0) for h, v in scores.items()}  # /10: decibans -> bans
    z = sum(exps.values())
    return {h: exps[h] / z for h in scores}


def assess(hyps: list[str], evidence: list[dict], prior: dict | None = None) -> dict:
    """Run the weight-of-evidence decision and its full sensitivity analysis."""
    prior = prior or {}
    scores = _scores(hyps, prior, evidence)
    ranked = _rank(scores)
    leader, s1 = ranked[0]
    runner, s2 = ranked[1] if len(ranked) > 1 else (None, float("-inf"))
    margin = s1 - s2  # decibans

    # per-evidence push of leader over runner-up (load-bearing ranking)
    contrib = []
    for e in evidence:
        w = e.get("weights") or {}
        d = float(w.get(leader, 0.0)) - (float(w.get(runner, 0.0)) if runner else 0.0)
        contrib.append({
            "name": e.get("name", ""),
            "push_for_leader": round(d, 2),          # decibans of leader-over-runner support
            "decisive_alone": d > margin,            # removing this alone would flip the call
            "source": e.get("source", "assumed"),
            "citation": e.get("citation", ""),
        })
    contrib.sort(key=lambda c: -c["push_for_leader"])

    # one-out: which single removals actually flip the leader
    flips = []
    for j, e in enumerate(evidence):
        sub_leader = _rank(_scores(hyps, prior, evidence[:j] + evidence[j + 1:]))[0][0]
        if sub_leader != leader:
            flips.append({"remove": e.get("name", ""), "new_leader": sub_leader})

    # minimal NUMBER of supporting facts that must fail to flip the decision (greedy on push)
    pos = [c for c in contrib if c["push_for_leader"] > 0]
    acc, k_to_flip, eroded = 0.0, None, []
    for c in pos:
        acc += c["push_for_leader"]
        eroded.append(c["name"])
        if acc > margin:
            k_to_flip = len(eroded)
            break

    # provenance of the margin: is the call resting on ungrounded weights?
    assumed_load = [c for c in pos if c["source"] != "grounded"]
    margin_rests_on_assumed = bool(assumed_load) and (pos and pos[0]["source"] != "grounded")

    return {
        "hypotheses": hyps,
        "scores": {h: round(v, 2) for h, v in scores.items()},
        "posteriors": {h: round(p, 4) for h, p in posteriors(scores).items()},
        "decision": leader,
        "runner_up": runner,
        "margin_db": round(margin, 2),
        "margin_odds": round(10 ** (margin / 10.0), 1),   # leader is this-many times the runner-up
        "ranked": [{"hypothesis": h, "score_db": round(v, 2)} for h, v in ranked],
        "load_bearing": contrib,
        "one_out_flips": flips,
        "min_facts_to_flip": k_to_flip,                   # None if no single-evidence set can flip it
        "would_flip_to": runner,
        "assumed_load_bearing": [c["name"] for c in assumed_load],
        "margin_rests_on_assumed": margin_rests_on_assumed,
    }


def tip(hyps: list[str], evidence: list[dict], prior: dict, evidence_name: str) -> dict:
    """The literal "probability shift -> decision shift": sweep ONE evidence's leader-weight
    downward and report the threshold at which the decision changes, and to what."""
    prior = prior or {}
    base = assess(hyps, evidence, prior)
    leader, margin = base["decision"], base["margin_db"]
    idx = next((i for i, e in enumerate(evidence) if e.get("name") == evidence_name), None)
    if idx is None:
        return {"error": f"no evidence named {evidence_name!r}"}
    w0 = float((evidence[idx].get("weights") or {}).get(leader, 0.0))
    # lowering this one weight by > margin flips the leader — but only possible if w0 > margin
    return {
        "evidence": evidence_name,
        "current_weight_for_leader_db": round(w0, 2),
        "flip_needs_drop_db": round(margin, 2),     # decibans this weight must lose to flip
        "can_flip_alone": w0 > margin,
        "flips_to": base["runner_up"] if w0 > margin else None,
        "note": (f"dropping this weight below {round(w0 - margin, 2)} dB flips the decision to "
                 f"{base['runner_up']!r}" if w0 > margin
                 else "this single weight cannot flip the decision (its support is smaller than the margin)"),
    }


def main() -> None:
    """CLI: python sensitivity.py <model.json>   (model: {hypotheses, prior?, evidence})"""
    model = json.loads(Path(sys.argv[1]).read_text())
    r = assess(model["hypotheses"], model["evidence"], model.get("prior"))
    print(json.dumps(r, indent=2))
    sys.exit(0)


if __name__ == "__main__":
    main()
