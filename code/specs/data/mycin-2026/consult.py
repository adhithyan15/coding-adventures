#!/usr/bin/env python3
"""consult.py - the composed MYCIN-2026 consultation: diagnose -> what-to-check -> treat.

This is the join of every module, the end-to-end physician-facing flow. One
patient's chart, run against the grounded importable libraries (diagnostic
rulebook + drug formulary), produces ONE audit trail with three reads off the
same composed program:

  [1] DECOMPOSE   the messy chart -> typed findings        (the ONE model call)
  [2] DIAGNOSE    differential over the grounded rulebook   (0 model calls, byte-cited)
  [3] CHECK-NEXT  value-of-information: the unobserved test  (0 model calls)
                  that would most shift the diagnosis
  [4] TREAT       if a bacterial diagnosis leads, the drug   (0 model calls, IDSA-grounded,
                  regimen customized to THIS patient         constraint-solved doses + timing)

The therapy is *derived from* the diagnosis (the Gram stain that drives the
differential also drives the dexamethasone decision). Everything after the single
decompose is CPU-bound, so the patient's data never needs to leave the machine
(privacy / HIPAA by architecture). The physician audits this trail and makes the call.

Usage:  python3 consult.py [--case case_bacterial_culture] [--age over_50|under_50]
                           [--allergy none|severe] [--weight 70] [--crcl 95]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

MYCIN = Path(__file__).resolve().parent
sys.path.insert(0, str(MYCIN / "warm"))
sys.path.insert(0, str(MYCIN / "treatment" / "antibiotics"))
import decide as decide_mod  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402
import select_abx as abx  # noqa: E402
import voi as voi_mod  # noqa: E402

IR_DIR = MYCIN / "ir"
CASES = MYCIN / "cases" / "cases.json"


def bar(t: str) -> None:
    print("\n" + "=" * 74 + f"\n{t}\n" + "=" * 74)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--case", default="case_bacterial_culture")
    ap.add_argument("--age", default="under_50", choices=["over_50", "under_50"])
    ap.add_argument("--allergy", default="none", choices=["none", "severe"])
    ap.add_argument("--immuno", default="no", choices=["yes", "no"])
    ap.add_argument("--weight", type=float, default=70)
    ap.add_argument("--crcl", type=float, default=95)
    args = ap.parse_args()

    cli = decide_mod.find_cli()
    if cli is None:
        print("consult: adj-lang-cli not built", file=sys.stderr)
        return 3
    domains = ir_mod.load_domains()
    cases = {c["id"]: c for c in json.loads(CASES.read_text())["cases"]}
    ir = json.loads((IR_DIR / f"{args.case}.json").read_text())

    bar(f"MYCIN-2026 CONSULTATION — {args.case}")
    print(f"chart: {cases.get(args.case, {}).get('vignette', '')[:300]}")
    print(f"structured: age={args.age}, drug-allergy={args.allergy}, "
          f"immunocompromised={args.immuno}, weight={args.weight}kg, CrCl={args.crcl}")

    # [1] decompose (the IR was produced by the small model, one call; reused here)
    observe_adj, kept, dropped = ir_mod.ir_to_adj(ir, domains)
    bar("[1] DECOMPOSITION  (1 model call — small local model; data stays on-device)")
    print("  findings:", ", ".join(kept))
    if dropped:
        print("  dropped at the vocabulary gate:", [d["term"] for d in dropped])

    # [2] differential diagnosis
    res = decide_mod.decide(args.case, observe_adj, cli)
    leader = res["leader"]
    bar("[2] DIFFERENTIAL DIAGNOSIS  (0 model calls — engine over the grounded rulebook)")
    for hyp, p in sorted(res["posteriors"].items(), key=lambda kv: -kv[1]):
        print(f"  {hyp:24s} P = {p:.4f}{'   <- leading' if hyp == leader else ''}")
    print(f"  decision: {res['decision'].get('type')}  | evidence for leader: {res['n_evidence_for_leader']}")

    # [3] value-of-information
    bar("[3] WHAT TO CHECK NEXT  (value-of-information; 0 model calls)")
    rows = voi_mod.voi(args.case, observe_adj, set(kept), cli)
    for r in rows[:3]:
        flip = "  *would FLIP the diagnosis*" if r["flips_leader"] else ""
        print(f"  order {r['order']:34s} dmargin={r['margin_delta']:+.4f}{flip}")

    # [4] therapy — gated on a bacterial diagnosis with real evidence
    bar("[4] EMPIRIC THERAPY  (0 model calls — IDSA-grounded, constraint-solved)")
    if leader != "bacterial_meningitis" or res["decision"].get("type") == "insufficient_evidence":
        print("  No empiric antibacterial therapy indicated by the leading diagnosis "
              f"({leader}); reassess / await results. (The system abstains rather than treat.)")
        return 0
    # therapy inputs DERIVED from the diagnosis + the structured chart:
    pneumococcal = "yes" if "csf_gram_stain(positive)" in kept else "no"
    profile = {"meningitis_dx": "empiric", "age_band": args.age, "immunocompromised": args.immuno,
               "betalactam_allergy": args.allergy, "suspected_pneumococcal": pneumococcal,
               "weight_kg": args.weight, "crcl": args.crcl,
               "desc": f"{args.case}, age {args.age}, allergy {args.allergy}"}
    comps = abx.select_components(cli, profile)
    vd = abx.vanc_dose(cli, args.weight, args.crcl)
    timing = abx.door_to_antibiotic(cli)
    must_now = timing.get("outcome") == "unsat"
    print(f"  GIVE WITHIN 1 h"
          + (f" — culture takes 48h > 1h deadline [time constraint UNSAT, IIS {timing.get('core')}]"
             if must_now else "") + ":")
    for c in comps:
        drug, dose, role = abx.ORDERS[c["component"]]
        if c["component"] == "give_vancomycin":
            dose = f"{vd['per_dose_mg']:.0f} mg IV {vd['interval']} (15 mg/kg; CrCl {vd['crcl']})"
        print(f"    - {drug:14s} {dose}")
        print(f"        ({role}; rule: {c['why']})")

    bar("PHYSICIAN REVIEW — every line above is grounded + overridable; you make the call.")
    print("  answer-time model calls across diagnosis + VOI + therapy: 0")
    return 0


if __name__ == "__main__":
    sys.exit(main())
