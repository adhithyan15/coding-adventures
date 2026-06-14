#!/usr/bin/env python3
"""voi.py - value-of-information: "what should we order next?" 0 model calls.

MYCIN-2026 M7. Given a case's observed findings, rank the UNOBSERVED findings by
how much observing each would move the differential. This is the engine's
value-of-information made into MYCIN's "order-next" output: for every
finding(value) in the closed dictionary that the case has not yet observed,
re-decide with it added and measure the change in the leader's posterior and the
between-hypothesis margin. The finding whose observation would most change or
most confirm the leading diagnosis comes first - each cited to the rulebook
clause that would fire. Pure CPU (re-runs the CLI); the model is not involved.

Usage:  python3 voi.py <case_id>     (reads ir/<case_id>.json)
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "warm"))
import decide as decide_mod  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402

DICT = ROOT / "warm" / "dictionary.json"
IR_DIR = ROOT / "ir"


def margin(res: dict) -> float:
    """Signed margin = leader posterior - runner-up posterior (the differential gap)."""
    post = sorted(res["posteriors"].values(), reverse=True)
    return post[0] - post[1] if len(post) >= 2 else post[0]


def voi(case_id: str, observe_adj: str, observed_terms: set[str], cli) -> list[dict]:
    d = json.loads(DICT.read_text())
    base = decide_mod.decide(case_id, observe_adj, cli)
    base_leader, base_margin = base["leader"], margin(base)

    ranked = []
    for f in d["findings"]:
        for v in f["value_domain"]:
            term = f"{f['functor']}({v})"
            if term in observed_terms:
                continue
            trial = decide_mod.decide(f"{case_id}_voi", observe_adj + f"observe {term}\n", cli)
            ranked.append({
                "order": term,
                "would_make_leader": trial["leader"],
                "flips_leader": trial["leader"] != base_leader,
                "new_margin": round(margin(trial), 4),
                "margin_delta": round(margin(trial) - base_margin, 4),
                "posteriors_if_observed": {k: round(x, 4) for k, x in trial["posteriors"].items()},
            })
    # Rank by absolute decision impact (flips first, then largest margin move).
    ranked.sort(key=lambda r: (not r["flips_leader"], -abs(r["margin_delta"])))
    return ranked


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: voi.py <case_id>", file=sys.stderr)
        return 2
    cli = decide_mod.find_cli()
    if cli is None:
        print("voi: adj-lang-cli not built", file=sys.stderr)
        return 3
    case_id = argv[0]
    ir = json.loads((IR_DIR / f"{case_id}.json").read_text())
    domains = ir_mod.load_domains()
    observe_adj, kept, _ = ir_mod.ir_to_adj(ir, domains)
    observed = set(kept)

    base = decide_mod.decide(case_id, observe_adj, cli)
    rows = voi(case_id, observe_adj, observed, cli)

    print(f"case {case_id}: leader={base['leader']} margin={margin(base):.4f} "
          f"(answer-time model calls: 0)")
    print("ORDER-NEXT (most decision-relevant unobserved findings):")
    for r in rows[:5]:
        flip = " *FLIPS*" if r["flips_leader"] else ""
        print(f"  {r['order']:38s} Δmargin={r['margin_delta']:+.4f} "
              f"→ {r['would_make_leader']}{flip}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
