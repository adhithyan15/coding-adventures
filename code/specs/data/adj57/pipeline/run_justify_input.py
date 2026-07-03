#!/usr/bin/env python3
"""ADJ62 driver — report BOTH input-side gates on a decomposition.

  COVERAGE       (stage.gate_text):   every input byte is used or discarded-with-reason
                                      — nothing DROPPED.
  EXTRACTION     (justify_gate.grade): every fact the decomposer claims to have extracted
                                      or inferred is byte-anchored AND justified by the
                                      cited bytes — nothing MIS-extracted.

Coverage alone says "you touched every byte"; the extraction gate says "every fact you
pulled out is actually proven by the bytes you cite, and an interpretation is filed as
inferred, not smuggled in as extracted." Together they make the IR a faithful function of
the input bytes — the input-side dual of ADJ61.

Run: python run_justify_input.py <input-results.json>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import justify_gate  # noqa: E402
import stage  # noqa: E402


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    case_text = res["case_text"]
    facts = res["graded_facts"]

    print("=" * 74)
    print("  ADJ62 — input justification (extract/infer -> which bytes -> why)")
    print("=" * 74)

    # ---- COVERAGE: nothing dropped ----
    segs = [({"text": s["text"], "kind": "used", "produced": "fact"} if s.get("kind") == "fact"
             else {"text": s["text"], "kind": "discard", "reason": s.get("reason", "")}) for s in res["segments"]]
    cov = stage.gate_text("decompose", case_text, segs)
    print("\n## COVERAGE — every input byte accounted for (nothing dropped)")
    if cov.covered:
        print(f"   100% COVERAGE: {cov.n_used} fact-segments + {cov.n_discard} discards = all {cov.n_input} bytes. clean={cov.clean}")
    else:
        print(f"   COVERAGE VIOLATION at byte {cov.detail.get('first_divergence')}")

    # ---- EXTRACTION JUSTIFICATION: nothing mis-extracted ----
    r = justify_gate.grade([case_text], facts)
    breakdown = " + ".join(f"{v} {k}" for k, v in r["by_kind"].items()) or "0"
    print("\n## EXTRACTION — every fact byte-anchored AND justified (nothing mis-extracted)")
    print(f"   facts: {r['n_grounded']}/{r['n_claims']} grounded ({breakdown}); {r['n_rejected']} rejected.  fully={r['fully_grounded']}")

    print("\n## Kickback history (the gate forcing each fact to anchor AND justify):")
    for a in res.get("attempts", []):
        tag = "clean" if a["n_rejected"] == 0 else f"{a['n_rejected']} rejected -> kicked back"
        print(f"   attempt {a['attempt']}: {a['n_facts']} facts; {tag}")

    print("\n## EXTRACTED facts (the bytes STATE these directly):")
    for gfact in [x for x in r["grounded"] if x["kind"] == "extracted"]:
        print(f"     - {gfact['claim'][:54]:54s}  <- {gfact['spans'][0][:40]!r}")
    print("\n## INFERRED facts (DERIVED from the bytes — justified interpretations):")
    for gfact in [x for x in r["grounded"] if x["kind"] == "inferred"]:
        print(f"     - {gfact['claim'][:56]}")
        print(f"         from: {', '.join(s[:22] for s in gfact['spans'][:4])}")
        print(f"         why:  {gfact['justification'][:88]}")
    if r["rejected"]:
        print("\n## REJECTED (gave up after the attempt budget):")
        for u in r["rejected"]:
            print(f"     - [{u['kind']}] {u['claim'][:50]}: {u['reason'][:46]}")

    both = cov.covered and r["fully_grounded"]
    out = {
        "case_text_len": len(case_text),
        "coverage": {"covered": cov.covered, "clean": cov.clean, "facts": cov.n_used, "discards": cov.n_discard, "bytes": cov.n_input},
        "extraction": {"n_facts": r["n_claims"], "n_grounded": r["n_grounded"], "n_rejected": r["n_rejected"], "by_kind": r["by_kind"], "fully_grounded": r["fully_grounded"]},
        "attempts": res.get("attempts", []),
        "input_provenance_complete": both,
        "facts": r["grounded"], "rejected": r["rejected"],
    }
    (Path(__file__).resolve().parent.parent / "input-justified.json").write_text(json.dumps(out, indent=2))
    print(f"\n   >>> INPUT PROVENANCE {'COMPLETE — every byte covered AND every fact justified by its cited bytes.' if both else 'INCOMPLETE (see above).'}")
    sys.exit(0 if both else 3)


if __name__ == "__main__":
    main()
