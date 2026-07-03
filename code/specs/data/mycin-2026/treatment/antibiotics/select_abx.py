#!/usr/bin/env python3
"""select_abx.py - empiric antibiotic SELECTION + dosing + timing. 0 model calls.

MYCIN-2026 therapy layer / the "immediate actions" stage of the ER pathway. Given a
patient profile (the decomposed factors), this:

  1. runs the grounded component-selection rulebook (meningitis-abx.adj) through the
     engine -> which regimen components to INCLUDE, each with its firing rule's
     citation (the audit);
  2. solves the renal-adjusted vancomycin dose + the door-to-antibiotic TIME window
     with the constraint solver (the same one the diagnosis uses);
  3. assembles the immediate-action order set: drug, dose, route, and "give within
     the window" - every line traceable to a grounded rule.

This is MYCIN's original purpose (recommend the therapy) rebuilt as a grounded,
auditable, one-edit-correctable, constraint-solved program - no model in the loop.

Usage:  python3 select_abx.py [--profile elderly|young|allergic]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402

RULEBOOK = HERE / "meningitis-abx.adj"

# Concrete order for each component (drug, dose, route) - the dose is the standard
# adult empiric dose; vancomycin is computed/renal-adjusted below.
ORDERS = {
    "give_vancomycin": ("Vancomycin", "<computed> IV (renal-adjusted)", "covers resistant S. pneumoniae"),
    "give_ceftriaxone": ("Ceftriaxone", "2 g IV q12h", "3rd-gen cephalosporin, base coverage"),
    "give_ampicillin": ("Ampicillin", "2 g IV q4h", "Listeria cover"),
    "give_betalactam_sparing_alt": ("Moxifloxacin 400 mg IV + Aztreonam 2 g IV q6-8h (+ TMP-SMX for Listeria)",
                                    "beta-lactam-sparing", "severe beta-lactam allergy alternative"),
    "give_dexamethasone": ("Dexamethasone", "10 mg IV q6h (with/before first abx dose)", "adjunct for pneumococcal"),
}

PROFILES = {
    "young": {"meningitis_dx": "empiric", "age_band": "under_50", "betalactam_allergy": "none",
              "suspected_pneumococcal": "yes", "immunocompromised": "no",
              "weight_kg": 70, "crcl": 95, "desc": "19yo, gram-positive diplococci, no allergy"},
    "elderly": {"meningitis_dx": "empiric", "age_band": "over_50", "betalactam_allergy": "none",
                "suspected_pneumococcal": "yes", "immunocompromised": "no",
                "weight_kg": 80, "crcl": 45, "desc": "68yo, pneumococcal, mild renal impairment"},
    "allergic": {"meningitis_dx": "empiric", "age_band": "under_50", "betalactam_allergy": "severe",
                 "suspected_pneumococcal": "no", "immunocompromised": "no",
                 "weight_kg": 60, "crcl": 90, "desc": "30yo, severe penicillin anaphylaxis"},
}


def cli_json(cli: Path, program: str) -> dict:
    p = HERE / "_tmp_abx.adj"
    p.write_text(program)
    try:
        r = subprocess.run([str(cli), str(p)], capture_output=True, text=True)
        assert r.returncode == 0, r.stderr
        return json.loads(r.stdout)
    finally:
        p.unlink(missing_ok=True)


def select_components(cli: Path, profile: dict) -> list[dict]:
    """Run the rulebook with the profile observed; return the included components
    (posterior > 0.5) each with the citation of the rule that fired."""
    obs = "".join(f"observe {k}({profile[k]})\n" for k in
                  ("meningitis_dx", "age_band", "immunocompromised", "betalactam_allergy", "suspected_pneumococcal")
                  if k in profile)
    out = cli_json(cli, RULEBOOK.read_text() + "\n" + obs)
    included = []
    for r in out["ranked"]:
        if r["posterior"] > 0.5:
            # the firing rule = the contribution step (not the prior) with a citation.
            fired = [s for s in r.get("proof", []) if s.get("kind") == "contribution"]
            cite = fired[-1]["source"] if fired else "(prior)"
            included.append({"component": r["hypothesis"], "why": cite})
    return included


def vanc_dose(cli: Path, weight_kg: float, crcl: float) -> dict:
    """Solve the vancomycin per-dose mg (15 mg/kg) and pick the renal interval band."""
    prog = (f"symbol dose_mg : scalar\nobserve weight({weight_kg})\n"
            f"constrain dose_mg = weight * 15\nsolve for {{ dose_mg }}\n")
    out = cli_json(cli, prog).get("solve", {})
    mg = next((a["value"] for a in out.get("assignments", []) if a["name"] == "dose_mg"), None)
    # interval by CrCl (cited rule; the solver does the mg, the band is a guideline lookup).
    interval = "q8-12h" if crcl >= 50 else "q12-24h (renal-adjusted; monitor levels)"
    return {"per_dose_mg": mg, "interval": interval, "crcl": crcl}


def door_to_antibiotic(cli: Path) -> dict:
    """Prove waiting for culture (48h) violates the door-to-antibiotic window (1h)."""
    prog = ("symbol wait : scalar\nobserve culture_hours(48)\nobserve deadline_hours(1)\n"
            "constrain wait = culture_hours\nconstrain wait <= deadline_hours\ncheck\n")
    return cli_json(cli, prog).get("check", {})


def recommend(cli: Path, profile: dict) -> None:
    print(f"PATIENT: {profile.get('desc', '')}")
    comps = select_components(cli, profile)
    vd = vanc_dose(cli, profile["weight_kg"], profile["crcl"])
    timing = door_to_antibiotic(cli)
    must_now = timing.get("outcome") == "unsat"

    print(f"\nIMMEDIATE ACTIONS (give within {1 if must_now else '?'} h"
          + (f"; culture takes 48h > 1h deadline [time constraint UNSAT, IIS {timing.get('core')}] "
             "-> do NOT wait" if must_now else "") + "):")
    for c in comps:
        drug, dose, role = ORDERS[c["component"]]
        if c["component"] == "give_vancomycin":
            dose = f"{vd['per_dose_mg']:.0f} mg IV {vd['interval']} (15 mg/kg; CrCl {vd['crcl']})"
        print(f"  - {drug:14s} {dose}")
        print(f"      ({role}; rule: {c['why']})")
    print("  answer-time model calls: 0\n")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--profile", choices=list(PROFILES) + ["all"], default="all")
    args = ap.parse_args()
    cli = decide_mod.find_cli()
    if cli is None:
        print("select_abx: adj-lang-cli not built", file=sys.stderr)
        return 3
    for name in (PROFILES if args.profile == "all" else [args.profile]):
        print("=" * 72)
        recommend(cli, PROFILES[name])
    return 0


if __name__ == "__main__":
    sys.exit(main())
