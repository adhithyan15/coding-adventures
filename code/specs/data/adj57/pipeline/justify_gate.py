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

KINDS = ("evidence", "conclusion")


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
        rec = {
            "claim": c.get("claim", ""),
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
                reason = f"unknown claim kind {kind!r} (must be evidence|conclusion)"
            else:  # anchored but not justified
                reason = ("cited bytes do NOT justify this claim — "
                          + ("evidence not supported by the cited spans"
                             if kind == "evidence"
                             else "conclusion not warranted by the cited evidence, or over-asserted (not hedged)"))
            rec["reason"] = reason
            rejected.append(rec)

    n = len(graded_claims)
    ev = [g for g in grounded if g["kind"] == "evidence"]
    con = [g for g in grounded if g["kind"] == "conclusion"]
    return {
        "n_claims": n,
        "n_grounded": len(grounded),
        "n_rejected": len(rejected),
        "n_evidence": len(ev),
        "n_conclusion": len(con),
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
        print(f"\n>>> ALL {r['n_claims']} claims grounded "
              f"({r['n_evidence']} evidence + {r['n_conclusion']} conclusion) — "
              "byte-anchored AND justified by the cited bytes.")
        sys.exit(0)
    print(f"\n>>> JUSTIFICATION-GATE VIOLATION: {r['n_rejected']}/{r['n_claims']} rejected — "
          "kick back (cite real bytes that justify it, hedge an over-asserted conclusion, or drop):")
    for u in r["rejected"]:
        print(f"    - [{u['kind']}] {u['claim'][:64]!r}: {u['reason']}")
    sys.exit(3)


if __name__ == "__main__":
    main()
