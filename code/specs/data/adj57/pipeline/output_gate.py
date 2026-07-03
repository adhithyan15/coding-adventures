#!/usr/bin/env python3
"""ADJ60 — the output-grounding gate (the dual of input coverage).

ADJ57/58 enforce INPUT coverage: every input byte is used or discarded-with-reason
(nothing dropped). This gate enforces the other half — OUTPUT grounding: every output
claim must trace back to input bytes (nothing invented). A claim whose supporting
citation is not a verbatim span of the input is UNGROUNDED — it came from outside the
input (e.g. recalled from training) — and is rejected for kick-back.

Together the two gates make the output a pure FUNCTION of the input bytes, traceable
in both directions: every input byte accounted for, AND every output claim grounded in
input bytes.

A claim is GROUNDED iff at least one of its `grounded_by` citations is a verbatim
substring of the allowed input (the case text + the used-fact terms). Verbatim, because
a citation must point at real bytes you can retrieve — the same rule the CAS `cite()`
enforces on sources.

Usage (library): ground_output(input_units, claims) -> report.
CLI: python output_gate.py <case.txt> <claims.json>
  claims.json: [{"claim": "...", "grounded_by": ["verbatim input span", ...]}]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path


def ground_output(input_units: list[str], claims: list[dict]) -> dict:
    """input_units: the spans the output is ALLOWED to draw on (case text + used-fact
    terms). claims: the atomic assertions composing the output, each with citations.
    Every citation must be a verbatim substring of some input unit."""
    hay = "\n".join(input_units)
    grounded, ungrounded = [], []
    for c in claims:
        spans = [s for s in (c.get("grounded_by") or []) if s]
        retrievable = [s for s in spans if s in hay]
        if retrievable:
            grounded.append({"claim": c.get("claim", ""), "retrievable_spans": retrievable})
        else:
            ungrounded.append({
                "claim": c.get("claim", ""),
                "reason": ("citation not verbatim in input — "
                           + (f"cited {spans!r} but none retrieves" if spans else "NO citation at all")),
            })
    n = len(claims)
    return {
        "n_claims": n,
        "n_grounded": len(grounded),
        "n_ungrounded": len(ungrounded),
        "fully_grounded": n > 0 and not ungrounded,
        "grounded": grounded,
        "ungrounded": ungrounded,
    }


def main() -> None:
    case_text = Path(sys.argv[1]).read_text()
    claims = json.loads(Path(sys.argv[2]).read_text())
    r = ground_output([case_text], claims)
    print(json.dumps({k: v for k, v in r.items() if k not in ("grounded",)}, indent=2))
    if r["fully_grounded"]:
        print(f"\n>>> OUTPUT FULLY GROUNDED: all {r['n_claims']} claims trace to verbatim input bytes.")
        sys.exit(0)
    print(f"\n>>> OUTPUT-GROUNDING VIOLATION: {r['n_ungrounded']}/{r['n_claims']} claims ungrounded — "
          "REJECT and kick back to re-derive (ground or drop):")
    for u in r["ungrounded"]:
        print(f"    - {u['claim'][:70]!r}: {u['reason']}")
    sys.exit(3)


if __name__ == "__main__":
    main()
