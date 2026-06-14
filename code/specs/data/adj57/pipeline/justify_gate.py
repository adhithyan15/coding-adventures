#!/usr/bin/env python3
"""ADJ61 — the justification gate (the refinement of ADJ60's output-grounding gate).

ADJ60 asked one question of every output claim: *is a citation a verbatim substring of
the input?* That syntactic test is at once too tight and too loose:

  - too TIGHT — a true fact synthesized from SEVERAL input bytes ("disseminated zoonotic
    infection", combining hepatosplenomegaly + granuloma + travel + sterile-cultures) has
    no single verbatim span, so an honest synthesis looks ungrounded; and it gagged the
    legitimate CONCLUSION ("neurobrucellosis") because the answer *name* is not a byte.
  - too LOOSE — it checks a citation *exists*, never that the citation *supports* the
    claim. A claim could cite any present-but-irrelevant span and pass.

The refinement (Adhithya's): a claim is grounded iff you may **combine multiple input
bytes into one fact, AND the combination justifies the fact.** Grounding is
JUSTIFICATION-by-cited-bytes, not substring matching. So the gate splits into two layers:

  LAYER 1 — byte-anchor (deterministic, here): EVERY cited span must be verbatim in the
    input. Citations may only point at real bytes; you cannot pad a claim with a
    fabricated citation. (Strictly stronger than ADJ60's "at least one retrieves".)

  LAYER 2 — justification (semantic, an adversarial verifier agent supplies the verdict):
    do the cited bytes, taken together, justify the claim? Combining bytes is allowed.
    The verdict depends on the claim's KIND:
      * evidence   — a statement ABOUT the input. The cited bytes must state or directly
                     imply it. ("it is tremolitized" / "Brucella serology positive" — no
                     cited byte supports it → REJECT. This is invention.)
      * conclusion — an INFERENCE from the evidence. The cited bytes must collectively
                     make it the warranted reading, and it must be hedged AS an inference
                     (a leading hypothesis), not asserted as a byte-fact. ("neurobrucellosis
                     is the most likely diagnosis given <these byte-grounded findings>" —
                     ALLOWED.)

A claim is GROUNDED iff byte-anchored AND justified. Anything else is rejected and kicked
back (the ADJ06 loop): cite real bytes that justify it, soften an over-asserted
conclusion to a hedged inference, or drop it.

This module owns LAYER 1 + the aggregation of LAYER 2 verdicts. The semantic verdict is
produced by the verifier in the workflow and passed in as `justified`/`justification` —
the gate never invents it.

Usage (library): grade(input_units, graded_claims) -> report.
  graded_claims: [{"claim","kind","grounded_by":[spans],"justified":bool,"justification":str}]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# The gate is STAGE-SYMMETRIC. Every kind is either STRICT (the claim is a statement
# directly supported by its cited bytes) or an INFERENCE (it is derived from them, and is
# allowed only as a hedged hypothesis). The same two-layer test then applies to both ends
# of the pipeline:
#   OUTPUT stage (ADJ61): evidence (strict)  / conclusion (inference)
#   INPUT  stage (ADJ62): extracted (strict) / inferred   (inference)
# The input split answers the question ADJ62 asks the decomposer: "what did you extract or
# infer from these bytes, which bytes, and why do those bytes PROVE your extraction?" An
# *extracted* fact must be stated by its cited bytes; an *inferred* fact must be warranted
# by them and flagged as an inference — exactly mirroring evidence vs conclusion.
STRICT_KINDS = ("evidence", "extracted")
INFERENCE_KINDS = ("conclusion", "inferred")
KINDS = STRICT_KINDS + INFERENCE_KINDS


def anchor(input_units: list[str], grounded_by: list[str]) -> dict:
    """LAYER 1 — byte-anchor. EVERY cited span must be a verbatim substring of the input
    (the case text + the used-fact terms). Returns which spans retrieve and which do not.
    A claim is anchored iff it cites ≥1 span and ALL cited spans are verbatim."""
    hay = "\n".join(input_units)
    spans = [s for s in (grounded_by or []) if s]
    present = [s for s in spans if s in hay]
    missing = [s for s in spans if s not in hay]
    return {
        "anchored": bool(spans) and not missing,
        "n_spans": len(spans),
        "present": present,
        "missing": missing,
    }


def grade(input_units: list[str], graded_claims: list[dict]) -> dict:
    """Combine LAYER 1 (byte-anchor, computed here) with LAYER 2 (the justification verdict
    supplied per claim) into a final report. A claim is GROUNDED iff anchored AND justified.

    Each graded_claim: {claim, kind, grounded_by, justified(bool), justification(str)}.
    """
    grounded, rejected = [], []
    for c in graded_claims:
        kind = c.get("kind", "evidence")
        a = anchor(input_units, c.get("grounded_by"))
        justified = bool(c.get("justified"))
        # stage-symmetric: the output stage labels the assertion "claim", the input stage
        # labels it "fact" — accept either as the assertion text.
        rec = {
            "claim": c.get("claim") or c.get("fact") or "",
            "kind": kind,
            "anchored": a["anchored"],
            "justified": justified,
            "spans": a["present"],
            "justification": c.get("justification", ""),
        }
        if a["anchored"] and justified and kind in KINDS:
            grounded.append(rec)
        else:
            if not a["anchored"]:
                reason = ("NO citation at all" if a["n_spans"] == 0
                          else f"fabricated citation(s) — not verbatim in input: {a['missing']!r}")
            elif kind not in KINDS:
                reason = f"unknown claim kind {kind!r} (must be one of {KINDS})"
            else:  # anchored but not justified
                detail = (f"{kind} not supported by the cited spans" if kind in STRICT_KINDS
                          else f"{kind} not warranted by the cited bytes, or over-asserted (not hedged)")
                reason = "cited bytes do NOT justify this claim — " + detail
            rec["reason"] = reason
            rejected.append(rec)

    n = len(graded_claims)
    by_kind: dict[str, int] = {}
    for g in grounded:
        by_kind[g["kind"]] = by_kind.get(g["kind"], 0) + 1
    return {
        "n_claims": n,
        "n_grounded": len(grounded),
        "n_rejected": len(rejected),
        "by_kind": by_kind,
        "n_strict": sum(v for k, v in by_kind.items() if k in STRICT_KINDS),
        "n_inference": sum(v for k, v in by_kind.items() if k in INFERENCE_KINDS),
        # back-compat aliases for the output-stage driver (ADJ61)
        "n_evidence": by_kind.get("evidence", 0),
        "n_conclusion": by_kind.get("conclusion", 0),
        "fully_grounded": n > 0 and not rejected,
        "grounded": grounded,
        "rejected": rejected,
    }


def main() -> None:
    """CLI: python justify_gate.py <case.txt> <graded_claims.json>"""
    input_units = [Path(sys.argv[1]).read_text()]
    claims = json.loads(Path(sys.argv[2]).read_text())
    r = grade(input_units, claims)
    print(json.dumps({k: v for k, v in r.items() if k not in ("grounded", "rejected")}, indent=2))
    if r["fully_grounded"]:
        breakdown = " + ".join(f"{v} {k}" for k, v in r["by_kind"].items()) or "0"
        print(f"\n>>> ALL {r['n_claims']} claims grounded "
              f"({breakdown}) — byte-anchored AND justified by the cited bytes.")
        sys.exit(0)
    print(f"\n>>> JUSTIFICATION-GATE VIOLATION: {r['n_rejected']}/{r['n_claims']} rejected — "
          "kick back (cite real bytes that justify it, hedge an over-asserted conclusion, or drop):")
    for u in r["rejected"]:
        print(f"    - [{u['kind']}] {u['claim'][:64]!r}: {u['reason']}")
    sys.exit(3)


if __name__ == "__main__":
    main()
