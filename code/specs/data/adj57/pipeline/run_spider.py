#!/usr/bin/env python3
"""ADJ66 driver — re-run the decision on spider-grounded weights and show the shift.

The principle: a weight may not be asserted unless it is grounded in bytes — input or
rulebook. ADJ65 flagged that the decision rested on `assumed` weights; the spider fetched a
source for each load-bearing one and grounded it in a verbatim passage. This driver:
  - re-runs the sensitivity engine on the GROUNDED weights -> new decision + margin;
  - compares to the BEFORE decision (the original assumed model);
  - prints the rulebook (each grounded weight -> its source passage);
  - reports whether the margin now rests on grounded weights.

Run: python run_spider.py <spider-results.json> [<before sensitivity-results.json>]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sensitivity as s  # noqa: E402


def _decide(hyps, evidence):
    return s.assess(hyps, evidence)


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    hyps, evidence = res["hypotheses"], res["evidence"]

    print("=" * 74)
    print("  ADJ66 — the spider: decision on rulebook-GROUNDED weights")
    print("=" * 74)

    # BEFORE (optional): original assumed model
    if len(sys.argv) > 2:
        before = json.loads(Path(sys.argv[2]).read_text())
        before = before.get("result", before)
        rb = _decide(before["hypotheses"], before["evidence"])
        print(f"\n## BEFORE (assumed weights): {rb['decision']}  "
              f"({rb['posteriors'][rb['decision']] * 100:.1f}%, margin {rb['margin_db']:+.1f} dB)")
        gt_rank_b = next((i + 1 for i, r in enumerate(rb["ranked"]) if "rucell" in r["hypothesis"]), "?")
        print(f"   (Brucellosis ranked #{gt_rank_b})")

    # AFTER: grounded model
    r = _decide(hyps, evidence)
    print(f"\n## AFTER (spider-grounded weights): {r['decision']}  "
          f"({r['posteriors'][r['decision']] * 100:.1f}%, margin {r['margin_db']:+.1f} dB)")
    print("\n## Ranking now:")
    for row in r["ranked"]:
        h = row["hypothesis"]
        bar = "#" * int(round(r["posteriors"][h] * 40))
        mark = "  <-- held-aside truth" if "rucell" in h else ""
        print(f"   {h[:34]:34s} {r['posteriors'][h] * 100:5.1f}%  {row['score_db']:+6.1f} dB  {bar}{mark}")

    # how grounded is the margin now?
    n_grounded = sum(1 for e in evidence if e.get("source") == "grounded")
    print(f"\n## Provenance: {n_grounded}/{len(evidence)} facts now grounded; "
          f"margin rests on assumed weights = {r['margin_rests_on_assumed']}")
    if r["assumed_load_bearing"]:
        print("   still-assumed load-bearing (next to fetch): " + ", ".join(r["assumed_load_bearing"][:5]))

    # the rulebook (grounded weight -> source byte)
    print("\n## RULEBOOK (each grounded weight traces to a fetched source passage):")
    for c in res.get("rulebook", [])[:14]:
        print(f"   [{c.get('fact','')[:26]:26s} -> {c.get('hypothesis','')[:24]:24s}] {c.get('derived_weight_db', 0):+.0f} dB")
        print(f"       {c.get('url','')[:78]}")
        print(f"       \"{(c.get('quote','') or '')[:96]}\"")

    out = {
        "before_decision": (sys.argv[2] and rb["decision"]) if len(sys.argv) > 2 else None,
        "after_decision": r["decision"], "after_margin_db": r["margin_db"],
        "after_ranked": r["ranked"], "n_grounded": n_grounded, "n_facts": len(evidence),
        "margin_rests_on_assumed": r["margin_rests_on_assumed"],
        "rulebook_size": len(res.get("rulebook", [])),
        "ground_truth": res.get("ground_truth", ""),
    }
    (Path(__file__).resolve().parent.parent / "spider.json").write_text(json.dumps(out, indent=2))
    print(f"\n   ground truth (held aside): {res.get('ground_truth','')}")
    shifted = (len(sys.argv) > 2) and (rb["decision"] != r["decision"])
    print(f"\n   >>> decision {'SHIFTED off the assumed answer' if shifted else 'unchanged'} after grounding; "
          f"{n_grounded}/{len(evidence)} weights now traced to rulebook bytes")
    sys.exit(0)


if __name__ == "__main__":
    main()
