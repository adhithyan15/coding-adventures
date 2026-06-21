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
import contraindications as ci  # noqa: E402  (ADJ-native: derive_contraindications via the engine)
import dose_caps as dc  # noqa: E402  (ADJ-native: derive_dose_caps — conjunctive caps via the engine)
import decide as decide_mod  # noqa: E402  (find_cli)
import derive_regimen as reg  # noqa: E402  (grounded formulary: SCENARIOS, DRUGS, candidates)
import native_setcover as nsc  # noqa: E402  (the COP emitter/solver)
import step_therapy as st  # noqa: E402  (ADJ-native: derive_blocked via the engine, NAF)
import timing as timing_mod  # noqa: E402  (ADJ-native: derive_timing via the precedence engine)


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
    active_contexts: set[str] = field(default_factory=set)   # CC-3 patient's active clinical contexts (pregnancy, …)
    contraindicated: set[str] = field(default_factory=set)   # CC-3 drugs the ENGINE derives as contraindicated
    weights: tuple[int, int] = reg.DEFAULT_WEIGHTS            # CC-4 objective blend (w_cost, w_tox)
    culture_status: str = ""                                 # CC-5 pending | resulted (timing decision)
    clinical_status: str = ""                                # CC-5 critical | unstable | stable
    step_therapy: set[tuple[str, str]] = field(default_factory=set)  # CC-6 (restricted, prerequisite)
    tried: set[str] = field(default_factory=set)             # CC-6 drugs already tried/failed
    constraints: list[dict] = field(default_factory=list)   # provenance per constraint
    discards: list[dict] = field(default_factory=list)       # facts not mapped + reason


# CC-3b — allergy no longer maps to a blanket "betalactam_allergy_severe" exclusion token.
# An allergy activates a CONTEXT (penicillin_allergy / cephalosporin_allergy / betalactam_allergy);
# the engine then derives which drugs that context contraindicates from the grounded
# contraindication rulebook, which is SIDE-CHAIN-scoped (a penicillin allergy excludes only
# penicillins — cephalosporins/carbapenems/aztreonam stay available, cross-reactivity <1-2%,
# record ci_betalactam_sidechain_mechanism). The allergen → context map lives in
# `_CONTEXT_FROM_FACT` below alongside the other contexts.

# CC-3 — the contraindication knowledge is NO LONGER a Python set. It lives in the ADJ
# rulebook `contraindications.adj` (generated from grounding/treatment-constraints-grounding.json
# by contraindications_build.py) as grounded `relate` facts + two generic, context-scoped
# `rule { head: … when: … }` clauses. This compiler's only job is to translate chart facts
# into the patient's active CONTEXTS (e.g. pregnancy=present → active_context "pregnancy");
# `derive()` then asks the ENGINE which drugs that makes contraindicated. The reasoning moved
# out of Python and into the language (project_adj_native_no_python_middle).
#
# A chart-fact `kind` → the clinical CONTEXT it activates. Adding a new context (QT
# prolongation, G6PD deficiency, …) is a data edit here + a grounded fact in the rulebook —
# no new Python branch, because the generic rule already joins any active context.
_CONTEXT_FROM_FACT = {
    ("pregnancy", "present"): "pregnancy",
    ("pregnancy", "pregnant"): "pregnancy",
    ("pregnancy", "true"): "pregnancy",
    # CC-3b allergy contexts. "penicillin" → exclude penicillins only (the literature-correct
    # narrow exclusion); "cephalosporin" → exclude cephalosporins; "betalactam" is an
    # UNSPECIFIED/severe whole-class allergy → exclude penicillins+cephalosporins+carbapenems
    # (aztreonam, a monobactam, stays available — the grounded safe choice in β-lactam allergy).
    ("allergy", "penicillin"): "penicillin_allergy",
    ("allergy", "cephalosporin"): "cephalosporin_allergy",
    ("allergy", "betalactam"): "betalactam_allergy",
}

# CC-4: a chart `objective_priority` fact → the (w_cost, w_tox) objective blend the
# set-cover minimizes. "cost" is the historical default (toxicity ignored); raising w_tox
# lets a pricier-but-safer regimen win for a patient where side-effect burden matters.
_OBJECTIVE_WEIGHTS = {"cost": (1, 0), "balanced": (1, 1), "low_toxicity": (1, 3)}

# CC-5: the wait-vs-treat-now decision (§4). Empiric-now vs await-culture is a real
# tradeoff: treating now buys broad coverage (more cost + side-effect burden) but
# `delay_risk = 0`; awaiting the culture is cheaper/narrower but incurs delay_risk. The
# disease's TIME-CRITICALITY decides which dominates: for bacterial meningitis it is a
# neurologic emergency and antibiotics must start AS SOON AS POSSIBLE — delay worsens outcome —
# so empiric-now dominates regardless of cost/toxicity. `routine` acuity (a non-time-critical
# infection) leaves room to await a culture when the patient is stable. The decision is
# reusable: a function of (disease acuity, culture status, clinical stability), not
# meningitis-specific.
#
# CC-5b — GROUNDED (was AUTHORED-DEBT). The time-criticality of bacterial meningitis is now
# carried by VERBATIM guideline quotes, not a paraphrase. HONESTY CORRECTION: the IDSA
# meningitis guideline (Tunkel 2004) says start antimicrobials "as soon as possible" and that
# it is "a neurologic emergency" — it does NOT set a hard numeric door-to-antibiotic threshold
# for meningitis. The earlier "≤60 min / ≤1 hour" figure was an unsupported overclaim (a
# sepsis/quality-bundle number, not IDSA meningitis guidance), so we represent the urgency
# QUALITATIVELY exactly as the source states (`treat_target: as_soon_as_possible`) rather than
# asserting a threshold no cited source supports — grounding catching a mis-asserted pivot value
# is the point. See `time-criticality.SOURCES.md` for the retrieval trail.
_TIME_CRITICALITY = {
    "meningitis": {
        "acuity": "time_critical",
        # Qualitative target faithful to the source — NOT a numeric ≤60-min claim (see above).
        "treat_target": "as_soon_as_possible",
        "source": (
            "Begin antimicrobial therapy as soon as possible: \"When a patient presents with "
            "suspected acute bacterial meningitis, the physician should begin antimicrobial "
            "therapy as soon as possible.\" Bacterial meningitis is a neurologic emergency: "
            "\"Bacterial meningitis is a neurologic emergency; progression to more severe "
            "disease reduces the patient's likelihood of a full recovery.\""
        ),
        "locator": (
            "IDSA Practice Guidelines for the Management of Bacterial Meningitis (Tunkel et al., "
            "Clin Infect Dis 2004;39:1267); AAFP summary (Am Fam Physician 2005;71(10):2003)"
        ),
        "trust": "authoritative",
    },
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
            # CC-3b: a drug allergy activates an allergy CONTEXT. It does NOT decide which drugs
            # are out — the engine derives that from the side-chain-scoped contraindication
            # rulebook (a penicillin allergy excludes penicillins, NOT all β-lactams).
            context = _CONTEXT_FROM_FACT.get((f.kind, f.value))
            if context:
                cop.active_contexts.add(context)
                cop.constraints.append({"type": "context", "from": f"allergy={f.value}",
                                        "rule": f"{f.value} allergy → active clinical context "
                                                f"'{context}' (gates the contraindication rules)",
                                        "detail": context, "span": f.span})
            else:
                cop.discards.append({"fact": f"allergy={f.value}",
                                     "reason": "no grounded allergy context for this allergen yet (CC-3b)"})
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
        elif f.kind == "hepatic_status":
            # CC-2b: hepatic impairment is tracked as an active risk token, but per the
            # ceftriaxone FDA label hepatic dysfunction ALONE needs no dose adjustment — only
            # the CONJUNCTION of hepatic + significant renal impairment caps the dose. That
            # conjunction is DERIVED BY THE ENGINE (dose_caps.adj) in derive(), not here: this
            # handler only asserts the raw risk token, the rulebook owns the reasoning.
            if f.value in ("hepatic_severe", "hepatic_moderate"):
                cop.risks.add(f.value)
                cop.constraints.append({"type": "dose_risk", "from": f"hepatic_status={f.value}",
                                        "rule": "hepatic impairment alone needs no adjustment; "
                                                "combined with renal impairment it caps the dose",
                                        "span": f.span})
            else:
                cop.discards.append({"fact": f"hepatic_status={f.value}",
                                     "reason": "unrecognized hepatic status (want hepatic_severe/hepatic_moderate)"})
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
            # CC-3: a pregnancy fact activates the "pregnancy" clinical CONTEXT. It does NOT
            # decide which drugs are contraindicated — the engine does that in derive(), by
            # running the grounded contraindication rulebook under this active context.
            context = _CONTEXT_FROM_FACT.get((f.kind, f.value))
            if context:
                cop.active_contexts.add(context)
                cop.constraints.append({"type": "context", "from": f"pregnancy={f.value}",
                                        "rule": f"pregnancy → active clinical context '{context}' "
                                                "(gates the contraindication rules)",
                                        "detail": context, "span": f.span})
            else:
                cop.discards.append({"fact": f"pregnancy={f.value}",
                                     "reason": "pregnancy value not 'present' → no context activated"})
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
    # NOTE: the CC-2b conjunctive cap (hepatic ∧ renal → `hepatorenal`) is NOT computed here.
    # It is DERIVED BY THE ENGINE in derive() from dose_caps.adj, so compile_cop stays a pure
    # chart→COP translation and the conjunction reasoning lives in the language, not Python.
    return cop


# A decision → its human-readable rationale. The DECISION is engine-derived (timing.adj
# precedence ladder); this is presentation only, keyed by what governed. treat_now's rationale
# depends on the governing tier (authoritative time-critical/unstable vs the conservative
# default), so it is keyed by (decision, delay_risk).
_TIMING_RATIONALE = {
    ("targeted_culture_directed", "none"): "culture resulted → narrow to the isolate's susceptibilities",
    ("treat_now_empiric", "high"): ("time-critical (or unstable): start empiric antibiotics now; "
                                    "awaiting culture would save cost/side-effects but the delay "
                                    "raises mortality above the grounded threshold"),
    ("await_culture", "low"): ("stable + non-time-critical + culture pending: await the result "
                               "and give narrow targeted therapy — cheaper, fewer side effects"),
    ("treat_now_empiric", "moderate"): "insufficient evidence the delay is safe → treat empirically (conservative)",
}


def decide_timing(cli: Path, disease: str, culture_status: str, clinical_status: str) -> dict:
    """CC-5 (§4) — ADJ-NATIVE: the empiric-now vs await-culture DECISION is now derived by the
    engine from the `timing.adj` precedence ladder (`timing.derive_timing`), not a Python
    if/elif. This wrapper supplies the disease's acuity (from the flagged `_TIME_CRITICALITY`
    input table) and dresses the engine verdict with the threshold provenance + a
    human-readable rationale (presentation). The reasoning lives in the language.

      culture resulted        → targeted (mandatory tier; the wait question is moot)
      time-critical / unstable→ treat_now_empiric (authoritative tier; delay_risk high)
      stable+routine+pending  → await_culture (specific tier; delay_risk low)
      otherwise               → treat_now_empiric (default tier; delay_risk moderate)"""
    tc = _TIME_CRITICALITY.get(disease, {"acuity": "routine"})
    acuity = tc.get("acuity", "routine")
    res = timing_mod.derive_timing(cli, culture_status, clinical_status, acuity)
    decision, delay_risk = res["decision"], res["delay_risk"]
    out = {
        "decision": decision,
        "delay_risk": delay_risk,
        "standing": res.get("standing"),
        "disease_acuity": acuity,
        "culture_status": culture_status or "unknown",
        "rationale": _TIMING_RATIONALE.get((decision, delay_risk), ""),
    }
    # Carry the grounded time-criticality basis (verbatim guideline quote + locator + trust +
    # the qualitative treat target) when one is recorded for this disease.
    threshold = {k: tc[k] for k in ("treat_target", "source", "locator", "trust") if k in tc}
    if threshold:
        out["threshold"] = threshold
    return out


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


def derive(cli: Path, facts: list[ChartFact], disease: str = "meningitis") -> dict:
    """Compile the chart → COP, solve it, and return the regimen with provenance.
    Dose feasibility (CC-2) is folded into the cover: a drug with no safe-and-effective
    dose under the chart's renal/interaction risks is excluded, so the optimizer
    re-derives around it or abstains. On INFEASIBLE the engine's conflict core is
    surfaced (honest abstention). CC-5: the wait-vs-treat-now decision is computed from
    the chart's culture/clinical status against the disease's grounded time-criticality."""
    cop = compile_cop(facts)
    # CC-2b: ask the ENGINE which COMPOUND risks the patient's active risk tokens trigger, by
    # running the grounded dose-cap rulebook (? derived_risk / ? dose_capped). The hepatorenal
    # conjunction (hepatic ∧ renal) is the engine's, not a Python `if` — a single risk derives
    # nothing (hepatic alone needs no adjustment). Fold any derived compound risk into the COP's
    # risk set BEFORE the dose-window solve so its grounded ceiling penalty fires; each capped
    # drug carries its FDA byte-quote into the provenance.
    derived_risks, dose_cap_info = dc.derive_dose_caps(cli, cop.risks)
    cop.risks |= derived_risks
    for drug, info in sorted(dose_cap_info.items()):
        cop.constraints.append({"type": "dose_risk", "from": f"derived_risk({info['risk']})",
                                "rule": f"{drug} dose-capped under {info['risk']} "
                                        "(engine-derived conjunction of patient risk factors)",
                                "detail": drug, "source": info.get("source"),
                                "locator": info.get("locator"), "trust": info.get("trust")})
    # CC-2: drop drugs that can't be safely + effectively dosed for this patient.
    undosable = dose_infeasible(cli, reg.candidates(cop.exclusions), cop.risks, cop.weight)
    for d, w in undosable.items():
        cop.constraints.append({
            "type": "dose_infeasible", "from": f"risks={sorted(cop.risks)}", "detail": d,
            "rule": f"no safe+effective dose: floor {w['floor_per_kg']} > ceiling "
                    f"{w['ceiling_per_kg']} mg/kg"})
    # CC-3: ask the ENGINE which drugs the patient's active contexts make contraindicated, by
    # running the grounded contraindication rulebook (? contraindicated($D, $C)). The set is
    # derived, not looked up in Python; each derivation carries its grounded byte-quote.
    derived_ci = ci.derive_contraindications(cli, cop.active_contexts)
    cop.contraindicated = set(derived_ci)
    for drug, info in sorted(derived_ci.items()):
        cop.constraints.append({"type": "contraindication",
                                "from": f"active_context({info['context']})",
                                "rule": f"{drug} is contraindicated in {info['context']} "
                                        "(engine-derived from the grounded rulebook)",
                                "detail": drug, "source": info.get("source"),
                                "locator": info.get("locator"), "trust": info.get("trust")})
    # Every exclusion is an EXPLICIT engine constraint (`x_d <= 0`), keyed by its reason, so
    # the emitted program is self-documenting: a drug with no safe dose (CC-2) or one that is
    # contraindicated (CC-3) is pinned out by the solver, not pre-removed in Python.
    forced_zero = {d: "dose-infeasible" for d in undosable}
    forced_zero.update({d: "contraindicated" for d in cop.contraindicated})
    # CC-4: solve under the chart's cost/side-effect objective blend (default tier-only).
    # `regimen` is the CLINICALLY optimal one — what the physician should give, ignoring
    # the payer. (CC-4 objective blend applies.)
    res = nsc.solve(cli, cop.organisms, cop.exclusions, cop.defeated, cop.weights, forced_zero=forced_zero)
    # CC-5: the empiric-now vs await-culture decision — DERIVED BY THE ENGINE from the timing
    # precedence ladder (timing.adj), keyed by the chart's culture/clinical status + acuity.
    timing = decide_timing(cli, disease, cop.culture_status, cop.clinical_status)
    # CC-6: when the chart carries payer step-therapy rules, ALSO solve the reimbursement-
    # feasible regimen (the clinically-best drugs whose step-therapy prerequisite is unmet
    # are excluded) and surface BOTH so the tradeoff — and any medical-necessity appeal — is
    # explicit. Reimbursement infeasibility is distinct from clinical infeasibility.
    reimbursement = None
    if cop.step_therapy:
        # CC-6: the ENGINE derives which drugs the payer blocks, via the step-therapy
        # precedence rule (negation-as-failure over the per-case requires/tried facts) —
        # not a Python set-difference. The precedence reasoning lives in the language.
        blocked = st.derive_blocked(cli, cop.step_therapy, cop.tried)
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
    # CC-3b: a PENICILLIN allergy (even anaphylactic) is FEASIBLE — a 3rd-gen cephalosporin
    # (ceftriaxone) has <1% cross-reactivity, so vancomycin + ceftriaxone stands.
    "penicillin_allergic_adult": [ChartFact("age_band", "adult", "adult"),
                                  ChartFact("allergy", "penicillin", "anaphylaxis to penicillin")],
    # An UNSPECIFIED whole-class β-lactam allergy excludes penicillins/cephalosporins/
    # carbapenems (only aztreonam survives, which can't cover S. pneumoniae) → honest abstention.
    "betalactam_allergic_adult": [ChartFact("age_band", "adult", "adult"),
                                  ChartFact("allergy", "betalactam", "severe reaction to all beta-lactams")],
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
