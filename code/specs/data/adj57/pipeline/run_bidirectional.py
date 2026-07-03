#!/usr/bin/env python3
"""ADJ63 driver — report the full bidirectional justification pipeline on a fresh case.

Verifies, end to end, every corner of the provenance grid:
  INPUT  coverage   (stage.gate_text):    every input byte used or discarded-with-reason
  INPUT  extraction (justify_gate.grade): every extracted/inferred fact byte-anchored AND justified
  OUTPUT grounding  (justify_gate.grade): every evidence/conclusion claim byte-anchored AND justified

Run: python run_bidirectional.py <bidirectional-results.json>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import justify_gate  # noqa: E402
import stage  # noqa: E402


def _breakdown(r: dict) -> str:
    return " + ".join(f"{v} {k}" for k, v in r["by_kind"].items()) or "0"


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    case_text = res["case_text"]

    print("=" * 74)
    print("  ADJ63 — bidirectional justification, end to end")
    print(f"  domain: {res.get('domain','?')}   source: {res.get('source_url','')}")
    print("=" * 74)

    # INPUT coverage
    segs = [({"text": s["text"], "kind": "used", "produced": "fact"} if s.get("kind") == "fact"
             else {"text": s["text"], "kind": "discard", "reason": s.get("reason", "")}) for s in res["segments"]]
    cov = stage.gate_text("decompose", case_text, segs)
    print("\n## [1] INPUT coverage — nothing dropped")
    print(f"   {'100% COVERAGE' if cov.covered else 'VIOLATION'}: {cov.n_used} facts + {cov.n_discard} discards = {cov.n_input} bytes. clean={cov.clean}")

    # INPUT extraction
    ri = justify_gate.grade([case_text], res["input_facts"])
    print("\n## [2] INPUT extraction — nothing mis-extracted")
    print(f"   {ri['n_grounded']}/{ri['n_claims']} facts grounded ({_breakdown(ri)}); {ri['n_rejected']} rejected. fully={ri['fully_grounded']}")
    for a in res.get("input_attempts", []):
        print(f"      attempt {a['attempt']}: {a['n']} facts; {'clean' if a['n_rejected'] == 0 else str(a['n_rejected']) + ' rejected -> kickback'}")

    # OUTPUT grounding
    ro = justify_gate.grade([case_text], res["output_claims"])
    print("\n## [3] OUTPUT grounding — nothing invented")
    print(f"   {ro['n_grounded']}/{ro['n_claims']} claims grounded ({_breakdown(ro)}); {ro['n_rejected']} rejected. fully={ro['fully_grounded']}")
    for a in res.get("output_attempts", []):
        print(f"      attempt {a['attempt']}: \"{a['leading_answer'][:52]}\" ({a['n']} claims; {'clean' if a['n_rejected'] == 0 else str(a['n_rejected']) + ' rejected -> kickback'})")

    print(f"\n## ANSWER (hedged, byte-justified): {res['leading_answer']}")
    print("\n## CONCLUSION claims (each justified by COMBINING cited bytes):")
    for g in [x for x in ro["grounded"] if x["kind"] == "conclusion"]:
        print(f"     - {g['claim'][:72]}")
        print(f"         from: {', '.join(s[:22] for s in g['spans'][:5])}")
    print("\n## INFERRED input facts (read, not stated):")
    for g in [x for x in ri["grounded"] if x["kind"] == "inferred"][:8]:
        print(f"     - {g['claim'][:68]}")

    if ri["rejected"] or ro["rejected"]:
        print("\n## LIVE REJECTIONS (the gate biting):")
        for u in ri["rejected"] + ro["rejected"]:
            print(f"     - [{u['kind']}] {u['claim'][:50]}: {u['reason'][:46]}")

    print(f"\n   ground truth (held aside): {res.get('ground_truth','')[:180]}")
    allgood = cov.covered and ri["fully_grounded"] and ro["fully_grounded"]
    print(f"\n   >>> BIDIRECTIONAL PROVENANCE {'COMPLETE — coverage + input extraction + output grounding all clean.' if allgood else 'INCOMPLETE (see above).'}")

    out = {
        "domain": res.get("domain"), "leading_answer": res["leading_answer"],
        "coverage": {"covered": cov.covered, "clean": cov.clean, "bytes": cov.n_input},
        "input_extraction": {"n": ri["n_claims"], "grounded": ri["n_grounded"], "rejected": ri["n_rejected"], "by_kind": ri["by_kind"]},
        "output_grounding": {"n": ro["n_claims"], "grounded": ro["n_grounded"], "rejected": ro["n_rejected"], "by_kind": ro["by_kind"]},
        "input_attempts": res.get("input_attempts", []), "output_attempts": res.get("output_attempts", []),
        "complete": allgood,
    }
    (Path(__file__).resolve().parent.parent / "bidirectional.json").write_text(json.dumps(out, indent=2))
    sys.exit(0 if allgood else 3)


if __name__ == "__main__":
    main()
