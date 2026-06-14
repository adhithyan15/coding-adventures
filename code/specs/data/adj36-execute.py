#!/usr/bin/env python3
"""ADJ36 — executable proof of the clinical demo.

Reproduces the LP19e log-odds arithmetic shown in ADJ36, computes
the VOI on the unobserved precipitator, decides commit vs.
kickback, and prints the defensible derivation a clinician would
read.

Run: python3 adj36-execute.py

This script is the operational ground truth for the spec. If any
number in ADJ36 disagrees with this script's output, this script
wins.
"""

import math
import sys
from pathlib import Path


def logit(p: float) -> float:
    """log(p / (1 - p))."""
    return math.log(p / (1.0 - p))


def sigmoid(x: float) -> float:
    """Numerically stable sigmoid."""
    if x >= 0.0:
        z = math.exp(-x)
        return 1.0 / (1.0 + z)
    z = math.exp(x)
    return z / (1.0 + z)


# ---------------------------------------------------------------------------
# Step 1 — verify the fixture
# ---------------------------------------------------------------------------

FIXTURE = "62yo M, ED for chest discomfort x 2h. Pressure-like, mild diaphoresis. No clear precipitator. PMH: HTN, smoker. Vitals normal. ECG: no acute ST changes."
assert len(FIXTURE) == 152, f"fixture is {len(FIXTURE)} bytes, expected 152"


# ---------------------------------------------------------------------------
# Step 2 — the IR (extracted by Claude; sentence-level spans verified
# by string operations so the byte indices in ADJ36 stay honest)
# ---------------------------------------------------------------------------

SENTENCES = [
    (0, 38, "62yo M, ED for chest discomfort x 2h. "),
    (38, 71, "Pressure-like, mild diaphoresis. "),
    (71, 94, "No clear precipitator. "),
    (94, 112, "PMH: HTN, smoker. "),
    (112, 127, "Vitals normal. "),
    (127, 152, "ECG: no acute ST changes."),
]

# Verify the IR tiles the source exactly
total = 0
for start, end, text in SENTENCES:
    assert FIXTURE[start:end] == text, (
        f"sentence span [{start}, {end}) mismatch: "
        f"got {FIXTURE[start:end]!r}, expected {text!r}"
    )
    assert end - start == len(text)
    assert total == start
    total = end
assert total == len(FIXTURE), f"IR tiling gap: {total} != {len(FIXTURE)}"
print(f"✓ IR coverage check passed: 6 sentences tile {len(FIXTURE)} bytes exactly")


# ---------------------------------------------------------------------------
# Step 3 — the rulebook (ADJ14 grammar; each entry is a contribute
# clause with a numerical LR and a citation string for audit)
# ---------------------------------------------------------------------------

PRIOR = {
    "acs": 0.10,  # Pope et al., NEJM 1995 — ED chest-pain ACS prevalence
}

# (LR, evidence_atom, citation)
CONTRIBUTIONS = [
    (2.5, "symptom_quality(pressure_like)",        "Rational Clinical Examination, JAMA 1998"),
    (2.0, "associated_symptom(diaphoresis)",       "Rational Clinical Examination, JAMA 1998"),
    (1.5, "pmh(hypertension)",                     "HEART Score, Six et al., Neth Heart J 2008 [empirical]"),
    (1.8, "pmh(smoker)",                           "HEART Score, Six et al., Neth Heart J 2008 [empirical]"),
    (0.5, "vital_signs(within_normal_limits)",     "Rational Clinical Examination, JAMA 1998"),
    (0.4, "denied(ecg_acute_st_changes)",          "Pope et al., NEJM 1995"),
    (2.5, "precipitator(exertional)",              "Diamond & Forrester, NEJM 1979"),
    (0.6, "precipitator(rest)",                    "Diamond & Forrester, NEJM 1979"),
    (0.8, "precipitator(positional)",              "[empirical]"),
]

# Joint contributions: (LR_extra, [evidence_set...], conclusion, citation)
JOINTS = [
    (1.3, ["symptom_quality(pressure_like)", "associated_symptom(diaphoresis)"], "acs",
     "[empirical] pressure + diaphoresis synergy"),
]

# Observed evidence from the IR's Affirmed Fact + Denied Fact nodes.
# The Uncertainty node for precipitator is NOT in this set (that's
# the kickback point).
OBSERVED = {
    "symptom_quality(pressure_like)",
    "associated_symptom(diaphoresis)",
    "pmh(hypertension)",
    "pmh(smoker)",
    "vital_signs(within_normal_limits)",
    "denied(ecg_acute_st_changes)",
}


# ---------------------------------------------------------------------------
# Step 4 — LP19e log-odds aggregation (closed-form, linear in
# number of contributors)
# ---------------------------------------------------------------------------

def lr_aggregate(conclusion: str, observed: set[str]) -> dict:
    """Compute P(conclusion | observed) via LP19e log-odds composition."""
    p_prior = PRIOR[conclusion]
    lam = logit(p_prior)
    trace = [{
        "step": "prior",
        "term": conclusion,
        "logit_delta": lam,
        "running_logit": lam,
    }]

    for lr, ev, cite in CONTRIBUTIONS:
        if ev in observed:
            delta = math.log(lr)
            lam += delta
            trace.append({
                "step": "contribution",
                "term": ev,
                "lr": lr,
                "logit_delta": delta,
                "running_logit": lam,
                "citation": cite,
            })

    for lr_extra, ev_set, conc, cite in JOINTS:
        if conc != conclusion:
            continue
        if all(e in observed for e in ev_set):
            delta = math.log(lr_extra)
            lam += delta
            trace.append({
                "step": "joint_contribution",
                "term": ev_set,
                "lr_extra": lr_extra,
                "logit_delta": delta,
                "running_logit": lam,
                "citation": cite,
            })

    return {
        "posterior_logit": lam,
        "posterior_probability": sigmoid(lam),
        "trace": trace,
    }


# ---------------------------------------------------------------------------
# Step 5 — VOI computation on the unresolved precipitator atom
# ---------------------------------------------------------------------------

def voi_for_precipitator(observed: set[str]) -> dict:
    """Compute VOI (continuous probability-shift formulation per ADJ18)."""
    baseline = lr_aggregate("acs", observed)
    p0 = baseline["posterior_probability"]

    # π_a (prior probabilities for each resolution given this
    # patient's demographics).
    pi = {
        "exertional": 0.35,
        "rest":       0.45,
        "positional": 0.10,
        "other":      0.10,
    }
    # 'other' means none of the LR contributors fire — verdict
    # equals baseline.

    posteriors_under = {}
    for resolution in ("exertional", "rest", "positional"):
        hypothetical = observed | {f"precipitator({resolution})"}
        result = lr_aggregate("acs", hypothetical)
        posteriors_under[resolution] = result["posterior_probability"]
    posteriors_under["other"] = p0

    voi = sum(
        pi[r] * abs(posteriors_under[r] - p0)
        for r in pi
    )

    return {
        "baseline_posterior": p0,
        "pi": pi,
        "posteriors_under": posteriors_under,
        "voi": voi,
    }


# ---------------------------------------------------------------------------
# Step 6 — decision rule (ADJ18 thresholds: kickback=0.10, warn=0.03)
# ---------------------------------------------------------------------------

KICKBACK_THRESHOLD = 0.10
WARN_THRESHOLD = 0.03


def decide(observed: set[str]) -> dict:
    baseline = lr_aggregate("acs", observed)
    voi = voi_for_precipitator(observed)
    if voi["voi"] >= KICKBACK_THRESHOLD:
        verdict = "kickback"
    elif voi["voi"] >= WARN_THRESHOLD:
        verdict = "committed_with_warning"
    else:
        verdict = "committed"
    return {
        "verdict": verdict,
        "posterior": baseline["posterior_probability"],
        "voi": voi["voi"],
        "voi_detail": voi,
        "trace": baseline["trace"],
    }


# ---------------------------------------------------------------------------
# Step 7 — print the defensible derivation
# ---------------------------------------------------------------------------

def print_derivation(result: dict, observed: set[str]):
    print()
    print("=" * 72)
    print(f"P(ACS | observed evidence) = {result['posterior']*100:.1f}%")
    print(f"Framework verdict: {result['verdict'].upper()}")
    print("=" * 72)
    print()
    print("Derivation:")
    print()
    for step in result["trace"]:
        if step["step"] == "prior":
            print(f"  prior P({step['term']}) = {PRIOR[step['term']]:.2f}")
            print(f"      → prior logit λ₀ = {step['logit_delta']:.3f}")
        elif step["step"] == "contribution":
            print(f"  {step['term']}: LR {step['lr']:.1f}")
            print(f"      → +{step['logit_delta']:+.3f} log-odds  [{step['citation']}]")
        elif step["step"] == "joint_contribution":
            print(f"  joint synergy of {' + '.join(step['term'])}: LR_extra {step['lr_extra']:.1f}")
            print(f"      → {step['logit_delta']:+.3f} log-odds  [{step['citation']}]")
        print(f"      running logit: {step['running_logit']:.3f}")
        print()
    print(f"  Posterior logit: {result['trace'][-1]['running_logit']:.3f}")
    print(f"  Posterior probability: {result['posterior']*100:.1f}%")
    print()

    if result["verdict"] == "kickback":
        print("─" * 72)
        print("KICKBACK — the framework refuses to commit until the highest-")
        print("VOI unresolved atom is clarified.")
        print("─" * 72)
        print()
        v = result["voi_detail"]
        print(f"Focal atom: precipitator (currently Uncertainty)")
        print(f"VOI(precipitator) = {v['voi']:.3f}  (threshold = {KICKBACK_THRESHOLD})")
        print()
        print("Posterior under each resolution:")
        for r, p in v["posteriors_under"].items():
            print(f"  precipitator({r:11}): π={v['pi'][r]:.2f}  →  P(ACS) = {p*100:5.1f}%")
        print()
        print("Structured clarification question:")
        print("  What precipitated the patient's chest discomfort?")
        print("    [a] exertional (e.g., walking up stairs, climbing)")
        print("    [b] at rest (sitting/sleeping/no clear trigger)")
        print("    [c] positional (changed with body position)")
        print("    [d] other (please specify)")
        print()

    print("Independence assumption: conditional independence of the")
    print("listed contributions given ACS, with the explicit joint term")
    print("(pressure_like + diaphoresis) modeling clinically-recognized")
    print("synergy. Recorded in the audit trail.")
    print()


# ---------------------------------------------------------------------------
# Step 8 — illustrative completion under each clarification
# ---------------------------------------------------------------------------

def show_post_clarification():
    print()
    print("=" * 72)
    print("Illustrative completion under each clarification answer")
    print("=" * 72)
    print()
    for resolution in ("exertional", "rest", "positional"):
        new_observed = OBSERVED | {f"precipitator({resolution})"}
        r = lr_aggregate("acs", new_observed)
        print(f"  If precipitator({resolution}):")
        print(f"    posterior logit: {r['posterior_logit']:.3f}")
        print(f"    P(ACS): {r['posterior_probability']*100:.1f}%")
        if r["posterior_probability"] >= 0.40:
            tier = "urgent workup (serial troponins + cardiology consult)"
        elif r["posterior_probability"] >= 0.20:
            tier = "observation tier (ED obs + serial ECG/troponin)"
        else:
            tier = "lower-risk observation (rule-out at lower urgency)"
        print(f"    Recommended tier: {tier}")
        print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    print("ADJ36 — clinical demo executable proof")
    print()
    print(f"Source ({len(FIXTURE)} bytes):")
    print(f"  {FIXTURE!r}")
    print()
    print(f"Observed evidence: {len(OBSERVED)} atoms")
    for ev in sorted(OBSERVED):
        print(f"  - {ev}")
    print()
    print(f"Unobserved (will be VOI-scanned): precipitator(?)")
    print()
    result = decide(OBSERVED)
    print_derivation(result, OBSERVED)
    show_post_clarification()


if __name__ == "__main__":
    main()
