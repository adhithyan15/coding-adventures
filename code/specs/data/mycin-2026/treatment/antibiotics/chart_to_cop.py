#!/usr/bin/env python3
"""chart_to_cop.py — compile a patient CHART into a constraint program (CC-1).

This is the first slice of CHART-AS-CONSTRAINTS.md: *everything in the chart becomes a
constraint.* Today's treatment derivation is a min-cost **set-cover** over the likely
organisms. That is the SPECIAL CASE. The general form reads the constraints off the
chart:

    chart facts ──(this compiler, deterministic)──▶  COP inputs
                                                      (organisms, exclusions, defeated)
                                                          │  native_setcover.solve
                                                          ▼  adj-constraint-solver
                                                      regimen  OR  INFEASIBLE(conflict)

CC-1 covers the constraint families that already have grounded backing in the formulary
(coverage, drug-class exclusion, culture-defeated edges), and proves it reproduces the
existing meningitis regimens (adult / over-50 / post-neurosurgical / β-lactam-allergic)
from a chart-fact IR. CC-2..7 (the spec) add dose feasibility, contraindication /
interaction grounding, the cost+side-effect objective, the wait-vs-treat decision, and
insurance step-therapy on top of this same compiler.

DESIGN INVARIANTS (from the spec):
  - Every chart fact must land as a CONSTRAINT or be an explicit DISCARD with a reason —
    nothing in the chart is silently ignored ("no unaccounted bytes", applied to the chart).
  - Every constraint carries PROVENANCE: which chart fact produced it, via which rule.
  - INFEASIBLE(minimal conflict) is a first-class answer — the compiler never invents a
    regimen. Decision support only; the physician edits any constraint.
  - The clinical FACTS the rules lean on (which drugs are β-lactams, what covers what,
    which organisms by scenario) are already spider-grounded in the formulary CAS; the
    mapping logic here is structural/definitional, not new authored clinical fact.

Usage:  python3 chart_to_cop.py        (demo: compile + solve the four canonical charts)
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent.parent / "warm"))
import decide as decide_mod  # noqa: E402  (find_cli)
import derive_regimen as reg  # noqa: E402  (grounded formulary: SCENARIOS, DRUGS, candidates)
import native_setcover as nsc  # noqa: E402  (the COP emitter/solver)


# --------------------------------------------------------------------------
# The chart-fact IR — the typed output of decomposing a chart (one fact per finding).
# --------------------------------------------------------------------------
@dataclass(frozen=True)
class ChartFact:
    """One typed fact read off the patient chart.

    kind ∈ {age_band, immune_status, setting, allergy, culture_resistance}; `value`
    is the closed-vocabulary value for that kind (e.g. age_band='older_adult'); `span`
    is the source text it was decomposed from (provenance back to the chart)."""
    kind: str
    value: str
    span: str = ""


@dataclass
class Cop:
    """The compiled constraint program: the COP inputs + a provenance/discard trace."""
    organisms: list[str] = field(default_factory=list)
    exclusions: set[str] = field(default_factory=set)
    defeated: set[tuple[str, str]] = field(default_factory=set)
    constraints: list[dict] = field(default_factory=list)   # provenance per constraint
    discards: list[dict] = field(default_factory=list)       # facts not mapped + reason


# Drug classes whose members an allergy excludes. β-lactam allergy → the formulary's
# `betalactam_allergy_severe` exclusion token (reg.candidates() drops β-lactam drugs).
_ALLERGY_EXCLUSION = {
    "penicillin": "betalactam_allergy_severe",
    "betalactam": "betalactam_allergy_severe",
    "cephalosporin": "betalactam_allergy_severe",
}


def compile_cop(facts: list[ChartFact]) -> Cop:
    """Compile chart facts into COP inputs, recording provenance for each constraint
    and an explicit discard (with reason) for any fact that maps to nothing."""
    cop = Cop()
    has_older = has_immuno = has_neurosurg = False

    for f in facts:
        if f.kind == "age_band":
            has_older = has_older or f.value == "older_adult"
            cop.constraints.append({"type": "scenario_input", "from": f"age_band={f.value}",
                                    "rule": "age band selects the empiric organism set", "span": f.span})
        elif f.kind == "immune_status":
            has_immuno = has_immuno or f.value == "immunocompromised"
            cop.constraints.append({"type": "scenario_input", "from": f"immune_status={f.value}",
                                    "rule": "immune status selects the empiric organism set", "span": f.span})
        elif f.kind == "setting":
            has_neurosurg = has_neurosurg or f.value in ("post_neurosurgical", "csf_shunt")
            cop.constraints.append({"type": "scenario_input", "from": f"setting={f.value}",
                                    "rule": "care setting selects the empiric organism set", "span": f.span})
        elif f.kind == "allergy":
            tok = _ALLERGY_EXCLUSION.get(f.value)
            if tok:
                cop.exclusions.add(tok)
                cop.constraints.append({"type": "exclusion", "from": f"allergy={f.value}",
                                        "rule": f"{f.value} allergy excludes the β-lactam drug class",
                                        "detail": tok, "span": f.span})
            else:
                cop.discards.append({"fact": f"allergy={f.value}",
                                     "reason": "no grounded drug-class exclusion rule for this allergen yet (CC-3)"})
        elif f.kind == "culture_resistance":
            # value is "drug:organism" — an in-vitro resistance result voids that edge.
            drug, _, org = f.value.partition(":")
            if drug and org:
                cop.defeated.add((drug, org))
                cop.constraints.append({"type": "defeated_edge", "from": f"culture_resistance={f.value}",
                                        "rule": "isolate resistant to drug → drop that coverage edge",
                                        "detail": [drug, org], "span": f.span})
            else:
                cop.discards.append({"fact": f"culture_resistance={f.value}",
                                     "reason": "malformed culture_resistance value (want drug:organism)"})
        else:
            cop.discards.append({"fact": f"{f.kind}={f.value}",
                                 "reason": f"no constraint rule for chart-fact kind '{f.kind}' yet"})

    # Scenario → organism set (the coverage constraint's RHS). Most-specific wins, and
    # the chosen scenario is itself a provenance-bearing constraint.
    if has_neurosurg:
        scenario = "post_neurosurgical_or_shunt"
    elif has_older or has_immuno:
        scenario = "over_50_or_immunocompromised"
    else:
        scenario = "adult_community"
    cop.organisms = list(reg.SCENARIOS[scenario])
    cop.constraints.append({"type": "coverage", "from": "scenario", "rule": scenario,
                            "detail": cop.organisms})
    return cop


def derive(cli: Path, facts: list[ChartFact]) -> dict:
    """Compile the chart → COP, solve it, and return the regimen with provenance.
    On INFEASIBLE, the engine's conflict core is surfaced (honest abstention)."""
    cop = compile_cop(facts)
    res = nsc.solve(cli, cop.organisms, cop.exclusions, cop.defeated)
    return {
        "regimen": res["regimen"], "outcome": res["outcome"],
        "cost": res.get("cost"), "conflict": res.get("iis"),
        "organisms": cop.organisms, "exclusions": sorted(cop.exclusions),
        "defeated": sorted(map(list, cop.defeated)),
        "constraints": cop.constraints, "discards": cop.discards,
    }


# Canonical charts (the four meningitis profiles) expressed as chart-fact IRs.
CHARTS = {
    "adult_community": [ChartFact("age_band", "adult", "45-year-old")],
    "over_50_or_immunocompromised": [ChartFact("age_band", "older_adult", "aged 68")],
    "post_neurosurgical_or_shunt": [ChartFact("setting", "post_neurosurgical", "POD#3 craniotomy")],
    "betalactam_allergic_adult": [ChartFact("age_band", "adult", "adult"),
                                  ChartFact("allergy", "penicillin", "anaphylaxis to penicillin")],
}


def main() -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("chart_to_cop: adj-lang-cli not built", file=sys.stderr)
        return 3
    print("compiling each CHART into a constraint program, then solving with the "
          "adj-constraint-solver (set-cover is the special case):\n")
    for name, facts in CHARTS.items():
        r = derive(cli, facts)
        print("=" * 74 + f"\n{name}")
        print(f"  chart facts → organisms: {r['organisms']}"
              + (f"  exclusions: {r['exclusions']}" if r["exclusions"] else ""))
        if r["regimen"] is None:
            print(f"  ENGINE: NO REGIMEN ({r['outcome']}) — conflict: {r['conflict']}")
        else:
            print(f"  regimen: {r['regimen']}  (cost {r['cost']})")
        if r["discards"]:
            print(f"  discards: {r['discards']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
