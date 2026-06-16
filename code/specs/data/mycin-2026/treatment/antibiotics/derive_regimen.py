#!/usr/bin/env python3
"""derive_regimen.py - DERIVE the regimen from grounded drug facts. 0 model calls.

The "drastically beyond MYCIN" move: 1976 MYCIN hard-coded ~600 therapy rules (the
guideline's pre-solved answers). Here we store only FACTS (the formulary: what each
drug covers, its contraindications, its dose window) and the engine DERIVES the
regimen as a constraint solve:

  1. SET-COVER (minimum preference-cost): pick the cheapest set of CSF-penetrant,
     non-contraindicated drugs whose coverage spans every likely organism. This
     GENERALIZES - add Pseudomonas risk and cefepime appears with NO new rule;
     flag a severe beta-lactam allergy and every beta-lactam is excluded and the
     cover re-derives over what's left.
  2. DOSE WINDOW (per chosen drug): solve floor <= dose <= ceiling, where the
     ceiling SHRINKS as renal/interaction risks stack. When the efficacy floor
     exceeds the safe ceiling the window is empty -> the solver returns UNSAT with
     an IIS: "there is no safe effective dose" - exactly the call a human misses
     under load. The system surfaces it and defers to switch/adjust.

Usage:  python3 derive_regimen.py   (runs the demo scenarios)
"""

from __future__ import annotations

import json
import subprocess
import sys
from itertools import combinations
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402

def load_formulary() -> tuple[dict, dict, list, str]:
    """Prefer the GROUNDED, gated formulary from the CAS (formulary_build.py output);
    fall back to the authored draft if the CAS hasn't been built yet. Returns
    (drugs, organisms_by_scenario, combinations, provenance)."""
    authored = json.loads((HERE / "formulary.json").read_text())
    # CC-4 side-effect / toxicity weights live in a separate authored-debt layer
    # (formulary.json "side_effects" map; "_doc" excluded). Until CC-4b grounds them
    # they are merged onto every drug as `side_effects` (default 0 if unlisted), so the
    # cost+side-effect objective has a weight for each candidate.
    se = {k: v for k, v in authored.get("side_effects", {}).items() if k != "_doc"}
    reg = HERE / "cas" / "registry.json"
    if reg.exists():
        root = json.loads(reg.read_text())["root"]
        man = json.loads((HERE / "cas" / "objects" / f"{root}.json").read_text())
        drugs = {d: {"covers": v["covers_accepted"], "csf_penetrant": v["csf_penetrant"],
                     "contraindications": v["contraindications"], "betalactam": v["betalactam"],
                     "tier": v["tier"], "dose": v["dose"],
                     "side_effects": v.get("side_effects", se.get(d, 0)),
                     "source": f"grounded (formulary CAS {root})"}
                 for d, v in man["drugs"].items()}
        return (drugs, authored["organisms_by_scenario"], man.get("combinations", []),
                f"CAS object {root} (spider-grounded + gated)")
    drugs = {d: {**v, "side_effects": se.get(d, 0)} for d, v in authored["drugs"].items()}
    return (drugs, authored["organisms_by_scenario"], authored.get("combinations", []),
            "authored draft (formulary.json; CAS not built)")


# CC-4: the regimen objective is a weighted blend of preference COST (tier) and
# SIDE-EFFECT burden. `weights = (w_cost, w_tox)`; the per-drug objective coefficient is
# `w_cost·tier + w_tox·side_effects` (a non-negative integer, so the engine's INTEGER
# optimizer still applies). The default (1, 0) is exactly the historical tier-only
# set-cover — so every existing consumer is unchanged until a policy raises w_tox.
DEFAULT_WEIGHTS = (1, 0)


def drug_weight(drug: str, weights: tuple[int, int] = DEFAULT_WEIGHTS) -> int:
    w_cost, w_tox = weights
    return w_cost * DRUGS[drug]["tier"] + w_tox * DRUGS[drug].get("side_effects", 0)


DRUGS, SCENARIOS, COMBINATIONS, FORMULARY_SOURCE = load_formulary()


def coverage_of(combo: tuple[str, ...] | list[str]) -> set[str]:
    """Organisms covered by a set of drugs: each drug's single-agent spectrum, PLUS
    any organism a grounded COMBINATION rule covers when its whole drug-set is present
    (e.g. resistant pneumococcus is covered only by vancomycin + a cephalosporin)."""
    cov = set().union(*(DRUGS[d]["covers"] for d in combo)) if combo else set()
    for rule in COMBINATIONS:
        if set(rule["drugs"]) <= set(combo):
            cov.add(rule["covers"])
    return cov


def candidates(exclusions: set[str]) -> list[str]:
    """Drugs that reach the CSF and are not contraindicated for this patient."""
    return [d for d, f in DRUGS.items()
            if f["csf_penetrant"] and not (set(f["contraindications"]) & exclusions)]


def min_cost_cover(cands: list[str], organisms: list[str],
                   weights: tuple[int, int] = DEFAULT_WEIGHTS) -> list[str] | None:
    """Minimum-objective (then fewest drugs) set of `cands` covering every organism.
    Tiny formulary -> exhaustive is instant. Returns None if impossible. The objective is
    the CC-4 weighted blend `Σ (w_cost·tier + w_tox·side_effects)` (default (1,0) = the
    historical tier-only preference cost). This is the reference the native engine program
    must agree with. coverage_of() folds in grounded COMBINATION rules."""
    need = set(organisms)
    best, best_key = None, None
    # A 2-drug first-line regimen (objective 1+1) beats a 1-drug reserve agent (objective 4).
    for k in range(1, len(cands) + 1):
        for combo in combinations(cands, k):
            if need <= coverage_of(combo):
                key = (sum(drug_weight(d, weights) for d in combo), len(combo))
                if best_key is None or key < best_key:
                    best, best_key = list(combo), key
    return best


def dose_window(cli: Path, drug: str, weight: float, risks: set[str]) -> dict:
    """Solve floor <= dose_per_kg <= ceiling (ceiling reduced by active risks).
    Returns feasibility + the window (or UNSAT + IIS)."""
    dose = DRUGS[drug]["dose"]
    floor = dose["floor_mg_per_kg"]
    ceiling = dose["ceiling_base_mg_per_kg"] - sum(
        pen for r, pen in dose.get("ceiling_penalty_mg_per_kg", {}).items() if r in risks)
    prog = (f"symbol dpk : scalar\nobserve floor({floor})\nobserve ceiling({ceiling})\n"
            f"constrain dpk >= floor\nconstrain dpk <= ceiling\ncheck\n")
    p = HERE / "_tmp_dose.adj"
    p.write_text(prog)
    try:
        r = subprocess.run([str(cli), str(p)], capture_output=True, text=True)
        chk = json.loads(r.stdout).get("check", {})
    finally:
        p.unlink(missing_ok=True)
    feasible = chk.get("outcome") in ("sat", "sat_real")
    return {"drug": drug, "floor_per_kg": floor, "ceiling_per_kg": ceiling, "feasible": feasible,
            "mg_range": (f"{floor * weight:.0f}-{ceiling * weight:.0f} mg" if feasible else None),
            "iis": chk.get("core"), "active_risks": sorted(risks)}


def derive(cli: Path, title: str, organisms: list[str], exclusions: set[str],
           risks: set[str], weight: float) -> None:
    print("=" * 74 + f"\n{title}\n" + "=" * 74)
    print(f"  cover: {organisms}")
    if exclusions:
        print(f"  exclusions: {sorted(exclusions)}")
    cover = min_cost_cover(candidates(exclusions), organisms)
    if cover is None:
        print("  NO REGIMEN can cover all organisms under these exclusions -> escalate / specialist.")
        return
    print(f"  DERIVED REGIMEN: {' + '.join(cover)}")
    single_cov = set().union(*(DRUGS[d]["covers"] for d in cover)) if cover else set()
    for rule in COMBINATIONS:
        if set(rule["drugs"]) <= set(cover) and rule["covers"] in set(organisms) - single_cov:
            print(f"    + COMBINATION {' + '.join(rule['drugs'])} covers {rule['covers']}")
            print(f"        [{rule['source'][:120]}]")
    for d in cover:
        covered = sorted(set(DRUGS[d]["covers"]) & set(organisms))
        if covered:
            print(f"    - {d:13s} covers {covered}   [{DRUGS[d]['source']}]")
        w = dose_window(cli, d, weight, risks)
        if w["feasible"]:
            print(f"        dose window: {w['floor_per_kg']}-{w['ceiling_per_kg']} mg/kg "
                  f"-> {w['mg_range']} (risks: {w['active_risks'] or 'none'})")
        else:
            print(f"        *** DOSE UNSAT — no safe effective dose: efficacy floor "
                  f"{w['floor_per_kg']} mg/kg > safe ceiling {w['ceiling_per_kg']} mg/kg "
                  f"given {w['active_risks']} [IIS {w['iis']}] -> SWITCH AGENT / adjust ***")
    print()


def main() -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("derive_regimen: adj-lang-cli not built", file=sys.stderr)
        return 3
    print(f"formulary: {FORMULARY_SOURCE}\n")
    org = SCENARIOS
    derive(cli, "1) Adult community meningitis (no allergy, normal renal)",
           org["adult_community"], set(), set(), 70)
    derive(cli, "2) GENERALIZATION — post-neurosurgical (adds Pseudomonas/MRSA): no new rule",
           org["post_neurosurgical_or_shunt"], set(), set(), 70)
    derive(cli, "3) Severe beta-lactam allergy — beta-lactams excluded, cover re-derives",
           org["adult_community"], {"betalactam_allergy_severe"}, set(), 70)
    derive(cli, "4) DOSE UNSAT — vancomycin in severe renal failure + a nephrotoxin interaction",
           ["s_pneumoniae_resistant", "n_meningitidis"], set(), {"renal_severe", "nephrotoxin_interaction"}, 70)
    return 0


if __name__ == "__main__":
    sys.exit(main())
