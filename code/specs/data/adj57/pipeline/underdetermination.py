#!/usr/bin/env python3
"""ADJ64 — the underdetermination gate (over-attribution under missing evidence).

ADJ60/61 stop *invented evidence*: a claim cannot cite bytes that are not there. But the
ADJ63 axle run exposed a *second* failure the byte-anchor does not catch. When the datum
that would decide between rival explanations is **absent from the input**, the model does
not fabricate it (good) — it lets the conclusion **drift to the loudest present lever** and
singles out one cause anyway (bad). The axle answer named "machining/surface" as the root
cause; the held-aside truth was "operating stress exceeded the fatigue limit". Both fit the
*same* bytes — the discriminating measurement (operating stress vs. fatigue limit) was
simply never decomposed. Every claim was byte-grounded; the *selection among causes* was
not.

So invention is not the only way to be wrong. A conclusion can be **underdetermined**: more
than one hypothesis fits the present bytes, and the observation that would separate them is
missing. The honest move is not to guess — it is to (a) keep the conclusion as a disjunction
over the live rivals, and (b) emit the missing discriminating observation as a *named
provenance hole* — a query the spider/CAS can go fetch. "If the data is not present we
cannot reason over it" is true for a single step; naming the hole is how the *loop* turns
absence into the next thing to retrieve.

THE GATE. For each rival hypothesis that fits the same bytes, the model supplies the single
**discriminating observation** that would settle leading-vs-rival, and whether that
observation is PRESENT in the input (with a verbatim citation) or ABSENT. The deterministic
rule:

  - a rival is RESOLVED iff its discriminating observation is marked present AND the citation
    is verbatim in the input (you cannot *claim* the data is there without a real byte);
  - otherwise the rival is OPEN — the discriminating observation is a HOLE (absent, or a
    fabricated "present").

The conclusion is DETERMINED iff no rival is open. If any rival is open, the conclusion is
underdetermined: it must soften to the disjunction over the open rivals, and the holes are
reported as required-but-missing data. The model proposes; this module adjudicates — and,
exactly as on every other gate, "present" is only ever as good as a verbatim byte.

Usage (library): assess(input_units, rivals) -> report.
  rivals: [{"hypothesis","discriminating_observation","present":bool,"citation":"verbatim span"}]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path


def assess(input_units: list[str], rivals: list[dict]) -> dict:
    """input_units: the bytes the answer may draw on. rivals: alternative explanations that
    fit the same bytes, each with the observation that would discriminate it from the leading
    answer and a claim about whether that observation is present in the input."""
    hay = "\n".join(input_units)
    resolved, open_ = [], []
    for r in rivals:
        obs = (r.get("discriminating_observation") or "").strip()
        citation = (r.get("citation") or "").strip()
        cite_ok = bool(citation) and citation in hay
        present = bool(r.get("present")) and cite_ok
        rec = {
            "hypothesis": r.get("hypothesis", ""),
            "discriminating_observation": obs,
            "present": present,
            "citation": citation if cite_ok else "",
        }
        if present and obs:
            resolved.append(rec)
        else:
            rec["why"] = (
                "discriminating observation ABSENT from the input — a required-but-missing datum"
                if not r.get("present")
                else "claimed present but citation is not verbatim in the input — treated as absent"
                if not cite_ok
                else "no discriminating observation supplied"
            )
            open_.append(rec)

    holes = [o["discriminating_observation"] for o in open_ if o["discriminating_observation"]]
    return {
        "n_rivals": len(rivals),
        "n_resolved": len(resolved),
        "n_open": len(open_),
        # determined iff no rival is left open (no rivals at all -> vacuously determined).
        "determined": not open_,
        "resolved": resolved,
        "open": open_,
        "holes": holes,
    }


def main() -> None:
    """CLI: python underdetermination.py <case.txt> <rivals.json>"""
    input_units = [Path(sys.argv[1]).read_text()]
    rivals = json.loads(Path(sys.argv[2]).read_text())
    r = assess(input_units, rivals)
    print(json.dumps({k: v for k, v in r.items() if k not in ("resolved", "open")}, indent=2))
    if r["determined"]:
        print(f"\n>>> DETERMINED: all {r['n_rivals']} rival(s) discriminated by present, cited bytes.")
        sys.exit(0)
    print(f"\n>>> UNDERDETERMINED: {r['n_open']}/{r['n_rivals']} rival(s) cannot be ruled out — "
          "the conclusion must stay a disjunction; these discriminating data are MISSING (go fetch):")
    for o in r["open"]:
        print(f"    - to rule out {o['hypothesis'][:48]!r}: {o['discriminating_observation'][:70]} ({o['why'][:40]})")
    sys.exit(3)


if __name__ == "__main__":
    main()
