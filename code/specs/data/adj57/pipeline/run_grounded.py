#!/usr/bin/env python3
"""ADJ60 driver — verify BOTH directions of byte provenance on a grounded run.

  INPUT grounding  (stage.gate_text):   every input byte used or discarded-with-reason.
  OUTPUT grounding (output_gate):       every output claim cites verbatim input bytes.

Reports the bidirectional trail + the kickback history (how the answer was forced to
ground, dropping any specificity the input did not support) + the final byte-grounded
answer with each claim mapped to its supporting input span.

Run: python run_grounded.py <grounded-results.json>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import output_gate  # noqa: E402
import stage  # noqa: E402


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    ingest, derived = res["ingest"], res["derived"]
    facts = [s["term"] for s in ingest["segments"] if s.get("kind") == "fact"]

    print("=" * 74)
    print("  ADJ60 — bidirectional byte provenance")
    print(f"  case: {ingest.get('source_url','')}")
    print("=" * 74)

    # ---- INPUT grounding ----
    segs = [({"text": s["text"], "kind": "used", "produced": s.get("term")} if s.get("kind") == "fact"
             else {"text": s["text"], "kind": "discard", "reason": s.get("reason")}) for s in ingest["segments"]]
    cov = stage.gate_text("decompose", ingest["case_text"], segs)
    print("\n## INPUT grounding — every input byte accounted for")
    if cov.covered:
        print(f"   100% COVERAGE: {cov.n_used} facts ({cov.detail and ''}) + {cov.n_discard} reasoned discards "
              f"= all {cov.n_input} bytes. clean={cov.clean}")
    else:
        print(f"   COVERAGE VIOLATION at byte {cov.detail.get('first_divergence')}")

    # ---- OUTPUT grounding ----
    og = output_gate.ground_output([ingest["case_text"]] + facts, derived["claims"])
    print("\n## OUTPUT grounding — every output claim traces to input bytes")
    print(f"   claims: {og['n_grounded']}/{og['n_claims']} grounded; "
          f"{og['n_ungrounded']} ungrounded.  fully_grounded={og['fully_grounded']}")

    # ---- the kickback history (the enforce->correct loop on OUTPUT) ----
    print("\n## Kickback history (output-grounding gate forcing the answer to ground):")
    for a in res.get("attempts", []):
        tag = "clean" if a["n_ungrounded"] == 0 else f"{a['n_ungrounded']} ungrounded -> kicked back"
        print(f"   attempt {a['attempt']}: \"{a['leading_answer'][:60]}\"  ({a['n_claims']} claims; {tag})")

    # ---- the byte-grounded answer ----
    print(f"\n## ANSWER (byte-grounded): {derived['leading_answer']}")
    print("   every claim, and the verbatim input span it traces to:")
    for g in og["grounded"]:
        span = g["retrievable_spans"][0]
        print(f"     - {g['claim'][:58]:58s}  <- input: {span[:48]!r}")
    if og["ungrounded"]:
        print("   STILL UNGROUNDED (would be rejected — gave up after the attempt budget):")
        for u in og["ungrounded"]:
            print(f"     - {u['claim'][:58]}: {u['reason'][:50]}")

    both = cov.covered and og["fully_grounded"]
    print(f"\n   ground truth (held aside): {ingest.get('ground_truth','')[:160]}")
    print(f"\n   >>> BIDIRECTIONAL PROVENANCE {'COMPLETE — every input byte accounted for AND every output claim grounded in input bytes' if both else 'INCOMPLETE (see above)'}")

    out = {"leading_answer": derived["leading_answer"],
           "input_coverage": {"covered": cov.covered, "clean": cov.clean, "facts": cov.n_used, "discards": cov.n_discard, "bytes": cov.n_input},
           "output_grounding": {"n_claims": og["n_claims"], "n_grounded": og["n_grounded"], "n_ungrounded": og["n_ungrounded"], "fully_grounded": og["fully_grounded"]},
           "attempts": res.get("attempts", []),
           "bidirectional_complete": both,
           "grounded_claims": og["grounded"]}
    (Path(__file__).resolve().parent.parent / "grounded.json").write_text(json.dumps(out, indent=2))
    sys.exit(0 if both else 3)


if __name__ == "__main__":
    main()
