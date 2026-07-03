#!/usr/bin/env python3
"""ADJ64 driver — report the underdetermination gate on a graded run.

Re-adjudicates with the deterministic gate (underdetermination.assess), then prints which
rivals are resolved by present bytes, which are OPEN because their discriminating
observation is missing, the named provenance holes (queries to fetch), and the honest
disjunctive answer that replaces the over-attributed one.

Run: python run_underdetermination.py <underdetermination-results.json>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import underdetermination as u  # noqa: E402


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    r = u.assess([res["case_text"]], res["rivals"])

    print("=" * 74)
    print("  ADJ64 — underdetermination gate (over-attribution under missing data)")
    print("=" * 74)

    print(f"\n## Rivals to the leading conclusion: {r['n_rivals']} "
          f"({r['n_resolved']} resolved by present bytes, {r['n_open']} OPEN)")
    print(f"## Conclusion is {'DETERMINED' if r['determined'] else 'UNDERDETERMINED'} by the input bytes.")

    if r["resolved"]:
        print("\n## RESOLVED rivals (ruled out by a present, cited datum):")
        for x in r["resolved"]:
            print(f"     - {x['hypothesis'][:52]:52s}  <- {x['citation'][:36]!r}")
    if r["open"]:
        print("\n## OPEN rivals (cannot be ruled out — the deciding datum is MISSING):")
        for x in r["open"]:
            print(f"     - rival: {x['hypothesis'][:60]}")
            print(f"         would need: {x['discriminating_observation'][:64]}")
            print(f"         status: {x['why'][:60]}")

    if r["holes"]:
        print("\n## NAMED PROVENANCE HOLES (queries the spider/CAS must fetch to decide):")
        for h in r["holes"]:
            print(f"     ? {h}")

    print("\n## BEFORE (ADJ63 — over-attributed, single cause):")
    print(f"   {res['leading_answer'][:300]}")
    print("\n## AFTER (ADJ64 — honest disjunction + named missing data):")
    print(f"   {res['corrected_answer'][:600]}")

    out = {
        "determined": r["determined"], "n_rivals": r["n_rivals"],
        "n_resolved": r["n_resolved"], "n_open": r["n_open"],
        "holes": r["holes"], "resolved": r["resolved"], "open": r["open"],
        "before": res["leading_answer"], "after": res["corrected_answer"],
    }
    (Path(__file__).resolve().parent.parent / "underdetermination.json").write_text(json.dumps(out, indent=2))
    print(f"\n   >>> {'DETERMINED — no missing discriminators.' if r['determined'] else 'UNDERDETERMINED — the gate refused to single out a cause and named the missing data instead of guessing.'}")
    sys.exit(0)


if __name__ == "__main__":
    main()
