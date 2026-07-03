#!/usr/bin/env python3
"""ADJ65 driver — report the weight-of-evidence decision and its sensitivity.

Answers "if we make some probability shift, how would the decision shift?" by printing:
the decision + posteriors (a view), the MARGIN (robustness, in decibans), the load-bearing
evidence, which single facts would flip the call, how far one weight must move to flip it,
and — the honest part — whether the margin rests on ungrounded (assumed) weights that should
be fetched/grounded first.

Run: python run_sensitivity.py <sensitivity-results.json>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sensitivity as s  # noqa: E402


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    hyps, evidence = res["hypotheses"], res["evidence"]
    r = s.assess(hyps, evidence)

    print("=" * 74)
    print("  ADJ65 — uncertainty as a primitive (weight of evidence + sensitivity)")
    print("=" * 74)

    print("\n## Posterior view (softmax of the log-odds — a VIEW; the decision is argmax):")
    for row in r["ranked"]:
        h = row["hypothesis"]
        bar = "#" * int(round(r["posteriors"][h] * 40))
        print(f"   {h[:30]:30s} {r['posteriors'][h] * 100:5.1f}%  {row['score_db']:+6.1f} dB  {bar}")

    print(f"\n## DECISION: {r['decision']}")
    print(f"   runner-up: {r['runner_up']}")
    print(f"   MARGIN: {r['margin_db']:+.1f} dB  (leader is ~{r['margin_odds']:.1f}x the runner-up's odds)")
    print(f"   => the decision survives any single weight perturbation smaller than {r['margin_db']:.1f} dB")

    print("\n## Load-bearing evidence (push of leader over runner-up, decibans):")
    for c in r["load_bearing"][:8]:
        tag = "" if c["push_for_leader"] <= 0 else (" [DECISIVE ALONE]" if c["decisive_alone"] else "")
        src = "grounded" if c["source"] == "grounded" else "ASSUMED"
        print(f"   {c['name'][:42]:42s} {c['push_for_leader']:+6.1f} dB  ({src}){tag}")

    print("\n## What would flip the decision:")
    if r["one_out_flips"]:
        for f in r["one_out_flips"]:
            print(f"   - remove '{f['remove']}' -> leader becomes {f['new_leader']}")
    else:
        print("   - no SINGLE fact's removal flips it")
    print(f"   - minimum supporting facts that must fail to flip it: {r['min_facts_to_flip']}"
          f"{'' if r['min_facts_to_flip'] else ' (no single-evidence erosion can flip it)'}")
    if r["load_bearing"] and r["load_bearing"][0]["push_for_leader"] > 0:
        top = r["load_bearing"][0]["name"]
        t = s.tip(hyps, evidence, {}, top)
        print(f"   - top lever '{top}': weight {t['current_weight_for_leader_db']:+.1f} dB; "
              f"{'drop it by >' + str(t['flip_needs_drop_db']) + ' dB -> ' + str(t['flips_to']) if t['can_flip_alone'] else 'cannot flip the call alone'}")

    print("\n## PROVENANCE OF THE MARGIN (the honest part):")
    if r["margin_rests_on_assumed"]:
        print("   ⚠ the decision's margin RESTS ON ASSUMED (ungrounded) weights:")
        for n in r["assumed_load_bearing"][:8]:
            print(f"       ? {n}  — fetch a real likelihood ratio for this before trusting the margin")
    else:
        print("   the load-bearing weights are grounded.")

    out = {
        "decision": r["decision"], "runner_up": r["runner_up"],
        "margin_db": r["margin_db"], "margin_odds": r["margin_odds"],
        "posteriors": r["posteriors"], "ranked": r["ranked"],
        "load_bearing": r["load_bearing"], "one_out_flips": r["one_out_flips"],
        "min_facts_to_flip": r["min_facts_to_flip"],
        "margin_rests_on_assumed": r["margin_rests_on_assumed"],
        "assumed_load_bearing": r["assumed_load_bearing"],
        "ground_truth": res.get("ground_truth", ""),
    }
    (Path(__file__).resolve().parent.parent / "sensitivity.json").write_text(json.dumps(out, indent=2))
    print(f"\n   ground truth (held aside): {res.get('ground_truth','')}")
    print(f"\n   >>> decision={r['decision']}; margin={r['margin_db']:+.1f} dB; "
          f"{'margin rests on assumed weights -> fetch LRs' if r['margin_rests_on_assumed'] else 'margin grounded'}")
    sys.exit(0)


if __name__ == "__main__":
    main()
