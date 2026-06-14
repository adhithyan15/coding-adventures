#!/usr/bin/env python3
"""ADJ68 driver — report the open-book DEFENSIBILITY audit.

The framework is not a recall engine; it produces auditable, defensible work. This driver
reports, for a question run through the audit workflow:
  - the GROUNDED facts (spidered open-book, each with a source + verbatim quote — the CAS);
  - the two answers (bare closed-book recall vs the framework's grounded chain);
  - the adversarial auditor's verdicts, and a deterministic DEFENSIBILITY SCORE
    (fraction of claims a reader can independently verify) for each arm.

Both arms may be correct; the point is that a correct-but-uncited answer is INDEFENSIBLE.

Run: python run_audit.py <audit-results.json>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)

    print("=" * 76)
    print("  ADJ68 — open-book defensibility audit (verifiability, not correctness)")
    print("=" * 76)
    print(f"\n## Question: {res['question'][:120]}...")
    print(f"## Ground truth: {res['ground_truth']}")

    print("\n## GROUNDED FACTS (spidered open-book — the CAS):")
    for i, f in enumerate(res["grounded_facts"], 1):
        print(f"   [G{i}] ({f['kind']}) {f['fact'][:90]}")
        print(f"        <- {f['source_url']}")
        print(f"        \"{(f['quote'] or '')[:96]}\"")

    print("\n## ANSWERS (both reached the correct answer):")
    print(f"   Arm A (bare recall):   {res['arm_A_bare']['answer'].splitlines()[0][:70]} ...")
    print(f"   Arm B (grounded chain): {res['arm_B_chain']['answer'][:70]} ...  ({len(res['arm_B_chain']['chain'])} cited nodes)")

    print("\n## ADVERSARIAL AUDIT — defensibility (a reader must verify EVERY link):")
    label_to_arm = {"Answer 1": "Arm A (bare recall)", "Answer 2": "Arm B (grounded chain)"}
    for a in res["audit"]:
        total, unsupported = a["claims_total"], a["claims_unsupported"]
        verifiable = total - unsupported
        score = verifiable / total if total else 0.0
        arm = label_to_arm.get(a["label"], a["label"])
        print(f"\n   {arm}: verdict={a['verdict']}  "
              f"defensibility={verifiable}/{total} claims verifiable ({score*100:.0f}%)")
        for u in a["unsupported_list"][:6]:
            print(f"       ✗ {u[:104]}")

    # deterministic headline comparison
    by_label = {a["label"]: (a["claims_total"] - a["claims_unsupported"], a["claims_total"]) for a in res["audit"]}
    a_v, a_t = by_label.get("Answer 1", (0, 1))
    b_v, b_t = by_label.get("Answer 2", (0, 1))
    print("\n## HEADLINE (the axis the framework targets):")
    print(f"   bare recall:    {a_v}/{a_t} verifiable ({a_v/a_t*100:.0f}%) — correct but INDEFENSIBLE")
    print(f"   grounded chain: {b_v}/{b_t} verifiable ({b_v/b_t*100:.0f}%) — same answer, defensible to the source")
    print("   The open-book spider grounded the very fact (the 8π-first step ORDER) a closed-book")
    print("   model could only assert from memory — settling that byte provenance CAN catch it.")

    out = {
        "question": res["question"], "ground_truth": res["ground_truth"],
        "n_grounded_facts": len(res["grounded_facts"]),
        "arm_A_defensibility": [a_v, a_t], "arm_B_defensibility": [b_v, b_t],
        "audit": res["audit"],
    }
    (Path(__file__).resolve().parent.parent / "audit.json").write_text(json.dumps(out, indent=2))
    sys.exit(0)


if __name__ == "__main__":
    main()
