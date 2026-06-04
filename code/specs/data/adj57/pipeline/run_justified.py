#!/usr/bin/env python3
"""ADJ61 driver — report the justification gate on a graded run.

Re-grades the workflow's output with the deterministic gate (justify_gate.grade), then
prints, for each claim: its KIND (evidence|conclusion), whether it is byte-anchored, the
justification verdict, and the input bytes it draws on. Shows the kickback history and the
final answer — a CONCLUSION the framework is now allowed to STATE (hedged) because the
combined cited bytes justify it, not because the name appears verbatim.

Run: python run_justified.py <justified-results.json>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import justify_gate  # noqa: E402


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    input_units = [res["case_text"]] + res.get("facts", [])
    claims = res["graded_claims"]

    print("=" * 74)
    print("  ADJ61 — the justification gate (combine bytes -> justified fact)")
    print("=" * 74)

    r = justify_gate.grade(input_units, claims)
    print(f"\n## Gate result: {r['n_grounded']}/{r['n_claims']} grounded "
          f"({r['n_evidence']} evidence + {r['n_conclusion']} conclusion); "
          f"{r['n_rejected']} rejected.  fully_grounded={r['fully_grounded']}")

    print("\n## Kickback history (the gate forcing each claim to anchor AND justify):")
    for a in res.get("attempts", []):
        tag = "clean" if a["n_rejected"] == 0 else f"{a['n_rejected']} rejected -> kicked back"
        print(f"   attempt {a['attempt']}: \"{a['leading_answer'][:58]}\"  ({a['n_claims']} claims; {tag})")

    print(f"\n## ANSWER (a justified, hedged conclusion): {res['leading_answer']}")

    print("\n## EVIDENCE claims (statements about the input — byte-grounded):")
    for g in [x for x in r["grounded"] if x["kind"] == "evidence"]:
        print(f"     - {g['claim'][:60]:60s}  <- {g['spans'][0][:42]!r}")
    print("\n## CONCLUSION claims (inferences justified by COMBINING the cited bytes):")
    for g in [x for x in r["grounded"] if x["kind"] == "conclusion"]:
        print(f"     - {g['claim'][:70]}")
        print(f"         justified by: {', '.join(s[:24] for s in g['spans'][:4])}")
        print(f"         verifier: {g['justification'][:90]}")
    if r["rejected"]:
        print("\n## STILL REJECTED (gave up after the attempt budget):")
        for u in r["rejected"]:
            print(f"     - [{u['kind']}] {u['claim'][:54]}: {u['reason'][:48]}")

    out = {
        "leading_answer": res["leading_answer"],
        "n_claims": r["n_claims"], "n_grounded": r["n_grounded"], "n_rejected": r["n_rejected"],
        "n_evidence": r["n_evidence"], "n_conclusion": r["n_conclusion"],
        "fully_grounded": r["fully_grounded"], "attempts": res.get("attempts", []),
        "grounded": r["grounded"], "rejected": r["rejected"],
    }
    (Path(__file__).resolve().parent.parent / "justified.json").write_text(json.dumps(out, indent=2))
    print(f"\n   >>> {'ALL claims byte-anchored AND justified — conclusion reached WITHOUT inventing a single evidence byte.' if r['fully_grounded'] else 'INCOMPLETE (see rejected above).'}")
    sys.exit(0 if r["fully_grounded"] else 3)


if __name__ == "__main__":
    main()
