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
    risks: set[str] = field(default_factory=set)             # CC-2 dose-ceiling risks
    weight: float = 70.0                                      # kg; for the mg dose window
    contraindicated: set[str] = field(default_factory=set)   # CC-3 drugs excluded by a contraindication
    weights: tuple[int, int] = reg.DEFAULT_WEIGHTS            # CC-4 objective blend (w_cost, w_tox)
    culture_status: str = ""                                 # CC-5 pending | resulted (timing decision)
    clinical_status: str = ""                                # CC-5 critical | unstable | stable
    step_therapy: set[tuple[str, str]] = field(default_factory=set)  # CC-6 (restricted, prerequisite)
    tried: set[str] = field(default_factory=set)             # CC-6 drugs already tried/failed
    constraints: list[dict] = field(default_factory=list)   # provenance per constraint
    discards: list[dict] = field(default_factory=list)       # facts not mapped + reason


# Drug classes whose members an allergy excludes. β-lactam allergy → the formulary's
# `betalactam_allergy_severe` exclusion token (reg.candidates() drops β-lactam drugs).
_ALLERGY_EXCLUSION = {
    "penicillin": "betalactam_allergy_severe",
    "betalactam": "betalactam_allergy_severe",
    "cephalosporin": "betalactam_allergy_severe",
}

# Drugs a chart contraindication removes by name (CC-3 — grounded in
# grounding/treatment-constraints-grounding.json). Fluoroquinolones (moxifloxacin) and
# TMP-SMX are contraindicated in pregnancy; the grounded byte-quotes justify each exclusion.
_PREGNANCY_CONTRAINDICATED = {"moxifloxacin", "tmp_smx"}

# CC-4: a chart `objective_priority` fact → the (w_cost, w_tox) objective blend the
# set-cover minimizes. "cost" is the historical default (toxicity ignored); raising w_tox
# lets a pricier-but-safer regimen win for a patient where side-effect burden matters.
_OBJECTIVE_WEIGHTS = {"cost": (1, 0), "balanced": (1, 1), "low_toxicity": (1, 3)}

# CC-5: the wait-vs-treat-now decision (§4). Empiric-now vs await-culture is a real
# tradeoff: treating now buys broad coverage (more cost + side-effect burden) but
# `delay_risk = 0`; awaiting the culture is cheaper/narrower but incurs delay_risk. The
# disease's TIME-CRITICALITY decides which dominates. For bacterial meningitis the door-to-
# antibiotic window is ~60 min — delay raises mortality — so empiric-now dominates regardless
# of cost/toxicity. This threshold is AUTHORED-DEBT (the IDSA recommendation), flagged for
# CC-5b spider grounding; `routine` acuity (a non-time-critical infection) leaves room to
# await a culture when the patient is stable. The decision is reusable: it's a function of
# (disease acuity, culture status, clinical stability), not meningitis-specific.
_TIME_CRITICALITY = {
    "meningitis": {"acuity": "time_critical", "treat_within_min": 60,
                   "source": "Suspected bacterial meningitis is a medical emergency; "
                             "empiric antibiotics should be started without delay (target "
                             "≤1 hour), as delay increases mortality and morbidity (IDSA).",
                   "trust": "consensus"},  # [FLAG: authored — ground in CC-5b]
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
        elif f.kind == "renal_status":
            # renal impairment shrinks the safe dose ceiling (CC-2 dose feasibility).
            if f.value in ("renal_severe", "renal_moderate"):
                cop.risks.add(f.value)
                cop.constraints.append({"type": "dose_risk", "from": f"renal_status={f.value}",
                                        "rule": "renal impairment lowers the safe dose ceiling", "span": f.span})
            else:
                cop.discards.append({"fact": f"renal_status={f.value}",
                                     "reason": "unrecognized renal status (want renal_severe/renal_moderate)"})
        elif f.kind == "interaction":
            # an additive-toxicity interaction (e.g. another nephrotoxin) lowers the ceiling.
            if f.value == "nephrotoxin_interaction":
                cop.risks.add(f.value)
                cop.constraints.append({"type": "dose_risk", "from": "interaction=nephrotoxin_interaction",
                                        "rule": "additive nephrotoxicity lowers the safe dose ceiling", "span": f.span})
            else:
                cop.discards.append({"fact": f"interaction={f.value}",
                                     "reason": "no grounded dose-interaction rule for this interaction yet (CC-3)"})
        elif f.kind == "weight":
            try:
                w = float(f.value)
            except (TypeError, ValueError):
                w = float("nan")
            # A body weight feeds the displayed mg dose range — accept only a finite,
            # physiologically plausible value, else discard (don't anchor on a nonsense weight).
            if 0.0 < w < 1000.0:
                cop.weight = w
            else:
                cop.discards.append({"fact": f"weight={f.value}",
                                     "reason": "implausible or non-numeric body weight (kg)"})
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
        elif f.kind == "pregnancy":
            # CC-3: pregnancy contraindicates specific drugs (grounded rules) — exclude them
            # by name from the cover, each with its own provenance constraint.
            if f.value in ("present", "pregnant", "true"):
                cop.contraindicated |= _PREGNANCY_CONTRAINDICATED
                for d in sorted(_PREGNANCY_CONTRAINDICATED):
                    cop.constraints.append({"type": "contraindication", "from": "pregnancy=present",
                                            "rule": f"{d} is contraindicated in pregnancy (grounded)",
                                            "detail": d, "span": f.span})
            else:
                cop.discards.append({"fact": f"pregnancy={f.value}",
                                     "reason": "pregnancy value not 'present' → no contraindication applied"})
        elif f.kind == "culture_status":
            # CC-5: whether the culture is back drives the wait-vs-treat-now decision.
            if f.value in ("pending", "resulted"):
                cop.culture_status = f.value
                cop.constraints.append({"type": "timing_input", "from": f"culture_status={f.value}",
                                        "rule": "culture status drives the wait-vs-treat-now decision",
                                        "span": f.span})
            else:
                cop.discards.append({"fact": f"culture_status={f.value}",
                                     "reason": "unrecognized culture status (want pending/resulted)"})
        elif f.kind == "clinical_status":
            # CC-5: acuity — a critical/unstable patient forces empiric-now regardless.
            if f.value in ("critical", "unstable", "stable"):
                cop.clinical_status = f.value
                cop.constraints.append({"type": "timing_input", "from": f"clinical_status={f.value}",
                                        "rule": "clinical acuity drives the wait-vs-treat-now decision",
                                        "span": f.span})
            else:
                cop.discards.append({"fact": f"clinical_status={f.value}",
                                     "reason": "unrecognized clinical status (want critical/unstable/stable)"})
        elif f.kind == "step_therapy":
            # CC-6: a payer step-therapy rule "won't approve Y until X tried", value
            # "restricted:prerequisite" (e.g. "cefepime:meropenem" — wait, no: the
            # restricted drug is the one needing a prior trial). Format "Y:X" = Y requires X.
            restricted, _, prereq = f.value.partition(":")
            if restricted and prereq:
                cop.step_therapy.add((restricted, prereq))
                cop.constraints.append({"type": "step_therapy", "from": f"step_therapy={f.value}",
                                        "rule": f"payer won't reimburse {restricted} until {prereq} is tried",
                                        "detail": [restricted, prereq], "span": f.span})
            else:
                cop.discards.append({"fact": f"step_therapy={f.value}",
                                     "reason": "malformed step_therapy value (want restricted:prerequisite)"})
        elif f.kind == "prior_failed":
            # CC-6: a drug already tried (and failed) — satisfies a step-therapy prerequisite.
            cop.tried.add(f.value)
            cop.constraints.append({"type": "prior_treatment", "from": f"prior_failed={f.value}",
                                    "rule": f"{f.value} already tried/failed → satisfies its step-therapy prerequisite",
                                    "span": f.span})
        elif f.kind == "objective_priority":
            # CC-4: the chart's treatment priority selects the cost/side-effect objective
            # blend. "cost" (default) = cheapest acceptable regimen; "low_toxicity" weights
            # side effects heavily (e.g. frail/renal patient, polypharmacy); "balanced" splits.
            w = _OBJECTIVE_WEIGHTS.get(f.value)
            if w is not None:
                cop.weights = w
                cop.constraints.append({"type": "objective", "from": f"objective_priority={f.value}",
                                        "rule": f"minimize w_cost·tier + w_tox·side_effects, weights {w}",
                                        "span": f.span})
            else:
                cop.discards.append({"fact": f"objective_priority={f.value}",
                                     "reason": "unknown priority (want cost/balanced/low_toxicity)"})
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


def decide_timing(disease: str, culture_status: str, clinical_status: str) -> dict:
    """CC-5 (§4): model the empiric-now vs await-culture decision as a costed binary, with
    the grounded time-criticality threshold as the deciding factor. Returns the decision +
    its delay_risk + the rationale/threshold provenance — a reusable function of (disease
    acuity, culture status, clinical stability), not meningitis-specific.

      culture resulted        → targeted (culture-directed): the wait question is moot.
      time-critical disease   → treat_now_empiric (delay_risk high): the grounded threshold
        OR critical/unstable     says delay raises mortality; empiric coverage now dominates
                                 any cost/side-effect saving from waiting.
      stable + routine acuity → await_culture (delay_risk low): narrow, cheaper, fewer side
        + culture pending        effects — defensible only when delay is safe.
      otherwise               → treat_now_empiric (delay_risk moderate): the conservative
                                 default (don't gamble on a benign course)."""
    tc = _TIME_CRITICALITY.get(disease, {"acuity": "routine"})
    acuity = tc.get("acuity", "routine")
    if culture_status == "resulted":
        return {"decision": "targeted_culture_directed", "delay_risk": "none",
                "rationale": "culture resulted → narrow to the isolate's susceptibilities",
                "disease_acuity": acuity}
    base = {"disease_acuity": acuity, "culture_status": culture_status or "unknown",
            "threshold": {k: tc[k] for k in ("treat_within_min", "source", "trust") if k in tc}}
    if acuity == "time_critical" or clinical_status in ("critical", "unstable"):
        return {**base, "decision": "treat_now_empiric", "delay_risk": "high",
                "rationale": ("time-critical (or unstable): start empiric antibiotics now; "
                              "awaiting culture would save cost/side-effects but the delay "
                              "raises mortality above the grounded threshold")}
    if clinical_status == "stable" and acuity == "routine" and culture_status == "pending":
        return {**base, "decision": "await_culture", "delay_risk": "low",
                "rationale": ("stable + non-time-critical + culture pending: await the result "
                              "and give narrow targeted therapy — cheaper, fewer side effects")}
    return {**base, "decision": "treat_now_empiric", "delay_risk": "moderate",
            "rationale": "insufficient evidence the delay is safe → treat empirically (conservative)"}


def dose_infeasible(cli: Path, drugs: list[str], risks: set[str], weight: float) -> dict:
    """CC-2: the drugs with NO safe-and-effective dose for this patient — efficacy floor
    exceeds the toxicity ceiling once `risks` shrink it (the engine's dose_window check
    returns UNSAT). Returns {drug: window} for each excluded drug (for provenance)."""
    out = {}
    for d in drugs:
        w = reg.dose_window(cli, d, weight, risks)
        if not w["feasible"]:
            out[d] = w
    return out


def reimbursement_blocked(step_therapy: set[tuple[str, str]], tried: set[str]) -> set[str]:
    """CC-6: the drugs a payer won't reimburse because their step-therapy prerequisite
    hasn't been tried — the precedence `x_Y ≤ tried_X` realized as a reimbursement-only
    exclusion. A restricted drug Y is blocked iff its prerequisite X is not in `tried`."""
    return {restricted for restricted, prereq in step_therapy if prereq not in tried}


def derive(cli: Path, facts: list[ChartFact], disease: str = "meningitis") -> dict:
    """Compile the chart → COP, solve it, and return the regimen with provenance.
    Dose feasibility (CC-2) is folded into the cover: a drug with no safe-and-effective
    dose under the chart's renal/interaction risks is excluded, so the optimizer
    re-derives around it or abstains. On INFEASIBLE the engine's conflict core is
    surfaced (honest abstention). CC-5: the wait-vs-treat-now decision is computed from
    the chart's culture/clinical status against the disease's grounded time-criticality."""
    cop = compile_cop(facts)
    # CC-2: drop drugs that can't be safely + effectively dosed for this patient.
    undosable = dose_infeasible(cli, reg.candidates(cop.exclusions), cop.risks, cop.weight)
    for d, w in undosable.items():
        cop.constraints.append({
            "type": "dose_infeasible", "from": f"risks={sorted(cop.risks)}", "detail": d,
            "rule": f"no safe+effective dose: floor {w['floor_per_kg']} > ceiling "
                    f"{w['ceiling_per_kg']} mg/kg"})
    # Every exclusion is an EXPLICIT engine constraint (`x_d <= 0`), keyed by its reason, so
    # the emitted program is self-documenting: a drug with no safe dose (CC-2) or one that is
    # contraindicated (CC-3) is pinned out by the solver, not pre-removed in Python.
    forced_zero = {d: "dose-infeasible" for d in undosable}
    forced_zero.update({d: "contraindicated" for d in cop.contraindicated})
    # CC-4: solve under the chart's cost/side-effect objective blend (default tier-only).
    # `regimen` is the CLINICALLY optimal one — what the physician should give, ignoring
    # the payer. (CC-4 objective blend applies.)
    res = nsc.solve(cli, cop.organisms, cop.exclusions, cop.defeated, cop.weights, forced_zero=forced_zero)
    # CC-5: the empiric-now vs await-culture decision, from the chart's timing inputs.
    timing = decide_timing(disease, cop.culture_status, cop.clinical_status)
    # CC-6: when the chart carries payer step-therapy rules, ALSO solve the reimbursement-
    # feasible regimen (the clinically-best drugs whose step-therapy prerequisite is unmet
    # are excluded) and surface BOTH so the tradeoff — and any medical-necessity appeal — is
    # explicit. Reimbursement infeasibility is distinct from clinical infeasibility.
    reimbursement = None
    if cop.step_therapy:
        blocked = reimbursement_blocked(cop.step_therapy, cop.tried)
        # The precedence is enforced BY THE ENGINE: payer-blocked drugs join forced_zero
        # (reason "step-therapy") → an explicit `constrain x_Y <= 0` clause in the
        # reimbursement program. Clinical exclusions (dose/contraindication) carry over, so
        # the covered solve layers the payer constraint on top of the clinical one.
        cov_forced = {**forced_zero, **{d: "step-therapy" for d in blocked}}
        cov = nsc.solve(cli, cop.organisms, cop.exclusions, cop.defeated,
                        cop.weights, forced_zero=cov_forced)
        differs = cov["regimen"] != res["regimen"]
        if cov["regimen"] is None:
            note = ("reimbursement-INFEASIBLE under step therapy: the only regimens covering "
                    "these organisms need a drug the payer blocks until its prerequisite is "
                    "tried → physician override / appeal on medical necessity")
        elif differs:
            note = ("the payer-covered regimen differs from the clinical optimum because step "
                    "therapy blocks a clinically-preferred drug — give the clinical one on "
                    "medical necessity, or step through the prerequisite")
        else:
            note = "step therapy does not change the regimen (prerequisites already satisfied)"
        reimbursement = {
            "step_therapy": sorted(map(list, cop.step_therapy)), "tried": sorted(cop.tried),
            "blocked": sorted(blocked), "covered_regimen": cov["regimen"],
            "covered_outcome": cov["outcome"], "covered_conflict": cov.get("iis"),
            "differs_from_clinical": differs, "note": note,
        }
    return {
        "regimen": res["regimen"], "outcome": res["outcome"],
        "cost": res.get("cost"), "conflict": res.get("iis"),
        "objective": res.get("objective"), "timing": timing,
        "reimbursement": reimbursement,
        "organisms": cop.organisms, "exclusions": sorted(cop.exclusions),
        "defeated": sorted(map(list, cop.defeated)),
        "risks": sorted(cop.risks), "dose_infeasible": sorted(undosable),
        "contraindicated": sorted(cop.contraindicated),
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
