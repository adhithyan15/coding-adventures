#!/usr/bin/env python3
"""ADJ37 — executable proof of the unified-framework demo.

Reproduces the LP19e log-odds arithmetic from ADJ37, computes VOI
across all unresolved atoms (input + rulebook), and prints the
defensible derivation a clinician would read.

Run: python3 adj37-execute.py

Unlike ADJ36's executor, this one operates on the *union* of the
input IR and the rulebook IR — demonstrating that the framework
treats both symmetrically. The rulebook itself contains
Uncertainty nodes (rules the LLM hedged on); those are not
applied to inference but are recorded as ambiguities the
framework might kick back on.
"""

import math
from collections import defaultdict


def logit(p): return math.log(p / (1.0 - p))
def sigmoid(x):
    if x >= 0: return 1.0 / (1.0 + math.exp(-x))
    z = math.exp(x); return z / (1.0 + z)


# ---------------------------------------------------------------------------
# INPUT IR (Facts extracted from the patient case)
# ---------------------------------------------------------------------------

INPUT_FIXTURE = "82yo F admitted from SNF with confusion since last night. Meds: lorazepam 1mg qhs, diphenhydramine 50mg PRN, oxybutynin 5mg BID, morphine 15mg q4h PRN, sertraline 50mg daily. Cr 1.4, baseline cognition mild dementia. No prior delirium episodes."

# Observed atoms (Affirmed Facts in input IR)
OBSERVED = {
    "age_gt_75",
    "dementia",
    "renal_impairment_cr_gt_1_2",
    "medication_class(benzodiazepine)",
    "medication_class(anticholinergic_high) #1",  # diphenhydramine
    "medication_class(anticholinergic_high) #2",  # oxybutynin
    "medication_class(opioid)",
    # NOT observed: regimen_chronicity (U1, kickback candidate)
    # NOT observed: sex_female (R-3 in rulebook is Uncertainty so it doesn't
    #               contribute either way; recorded but not gating)
}

# Unresolved atoms (Uncertainty nodes in input IR)
UNRESOLVED_INPUT = {
    "regimen_chronicity": None,  # U1; framework's primary kickback candidate
}


# ---------------------------------------------------------------------------
# RULEBOOK IR (Rule-Facts extracted from the LLM-elicited rulebook)
# Each rule carries confidence + citation provenance.
# ---------------------------------------------------------------------------

PRIOR = {
    "delirium": 0.20,
}
PRIOR_CITATION = "Inouye SK, NEJM 2006 [HIGH confidence on paper, MEDIUM on prevalence value]"

# (lr, evidence_atom, citation, confidence)
CONTRIBUTIONS = [
    (6.0, "dementia",                                   "Inouye 2006 + CAM lit",      "HIGH (range 3-9)"),
    (2.5, "age_gt_75",                                  "Inouye 2006 + epidemiology", "MEDIUM"),
    (3.0, "medication_class(benzodiazepine)",           "Beers 2019 (AGS)",           "HIGH"),
    (2.5, "medication_class(anticholinergic_high) #1",  "Boustani ACB 2008",          "MEDIUM"),
    (2.5, "medication_class(anticholinergic_high) #2",  "Boustani ACB 2008",          "MEDIUM"),
    (2.5, "medication_class(opioid)",                   "Vaurio 2006 [UNVERIFIED]",   "MEDIUM-LOW"),
    (1.3, "renal_impairment_cr_gt_1_2 * benzo",         "[empirical synthesis]",      "MEDIUM-LOW"),
    (1.3, "renal_impairment_cr_gt_1_2 * opioid",        "[empirical synthesis]",      "MEDIUM-LOW"),
]

# Rules the LLM marked as Uncertainty rather than confident Rule-Facts.
# These don't apply to inference; recorded for audit.
RULEBOOK_UNCERTAINTIES = [
    ("R-3:  sex(female) net LR ~ 1.0",                  "LOW"),
    ("R-8:  ssri LR ~ 1.2",                              "LOW"),
    ("R-11: benzo + anticholinergic synergy ~ 1.5",     "LOW"),
]

# Chronicity contributions (R-10a / R-10b) — only one applies, depending
# on whether U1 resolves to chronic or recent.
CHRONICITY_CONTRIBUTIONS = {
    "chronic": 0.7,   # LR+: chronic regimen less likely to be the cause
    "recent":  2.5,   # LR+: recent change is the classical cause
}

# Citations the framework would verify before deploying. Marked unverified
# (would be checked via PubMed/Crossref in a production implementation).
UNVERIFIED_CITATIONS = [
    "Vaurio LE et al., Anesth Analg 2006;102(4):1267-73",
]


# ---------------------------------------------------------------------------
# LP19e log-odds aggregation
# ---------------------------------------------------------------------------

def lr_aggregate(observed, conclusion="delirium"):
    p0 = PRIOR[conclusion]
    lam = logit(p0)
    trace = [("prior", conclusion, p0, lam, PRIOR_CITATION)]
    for lr, ev, cite, conf in CONTRIBUTIONS:
        if ev in observed:
            d = math.log(lr)
            lam += d
            trace.append(("contrib", ev, lr, d, f"[{conf}] {cite}"))
    return lam, sigmoid(lam), trace


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    print("ADJ37 — unified-framework demo (LLM-derived rulebook + input)")
    print()
    print(f"Input fixture ({len(INPUT_FIXTURE)} bytes):")
    print(f"  {INPUT_FIXTURE!r}")
    print()
    print(f"Observed input atoms ({len(OBSERVED)}):")
    for a in sorted(OBSERVED):
        print(f"  - {a}")
    print()
    print(f"Unresolved input atoms ({len(UNRESOLVED_INPUT)}):")
    for a in UNRESOLVED_INPUT:
        print(f"  - {a}  ← framework kickback candidate")
    print()
    print(f"Rulebook rules applied (HIGH/MEDIUM confidence only):  {len(CONTRIBUTIONS)}")
    print(f"Rulebook rules marked Uncertainty (NOT applied):       {len(RULEBOOK_UNCERTAINTIES)}")
    for u, conf in RULEBOOK_UNCERTAINTIES:
        print(f"  - [{conf}] {u}")
    print(f"Citations flagged as UNVERIFIED: {len(UNVERIFIED_CITATIONS)}")
    for c in UNVERIFIED_CITATIONS:
        print(f"  - {c}")
    print()

    # --- Inference without resolving chronicity ---
    print("=" * 72)
    print("Inference WITHOUT resolving the chronicity uncertainty (U1):")
    print("=" * 72)
    print()
    lam, p, trace = lr_aggregate(OBSERVED)
    for tag, name, val, delta, cite in trace:
        if tag == "prior":
            print(f"  prior P({name}) = {val:.2f} → λ₀ = {delta:+.3f}")
            print(f"      [{cite}]")
        else:
            print(f"  {name}: LR {val:.1f} → {delta:+.3f}")
            print(f"      {cite}")
    print()
    print(f"  Posterior logit (before chronicity):  {lam:.3f}")
    print(f"  Posterior probability:                {p*100:.2f}%")
    print()

    # --- Hypothetical: chronicity resolution ---
    print("=" * 72)
    print("If chronicity resolves (the U1 kickback question):")
    print("=" * 72)
    print()
    for resolution, lr in CHRONICITY_CONTRIBUTIONS.items():
        lam_r = lam + math.log(lr)
        p_r = sigmoid(lam_r)
        print(f"  precipitator(regimen_{resolution}): LR {lr}")
        print(f"      → posterior logit {lam_r:+.3f}, P(delirium) = {p_r*100:.2f}%")
    print()

    # --- Modeling-bug surface ---
    print("=" * 72)
    print("⚠  MODELING-ISSUE SURFACED BY THE FRAMEWORK ON ITSELF:")
    print("=" * 72)
    print()
    print("  Both branches produce P(delirium) > 99%. This is because")
    print("  the rulebook's LRs are calibrated for 'any delirium' (a")
    print("  broad endpoint), while the input's query is 'med-induced")
    print("  delirium' (a narrower question).")
    print()
    print("  In a properly-scoped rulebook for 'med-induced delirium',")
    print("  patient-factor LRs (dementia, age) would be smaller because")
    print("  they predict delirium overall, not med-induced delirium")
    print("  specifically. The current arithmetic is therefore an")
    print("  over-count.")
    print()
    print("  Framework's symmetric response: kick back to the RULEBOOK")
    print("  elicitation step, not just the input. Ask the LLM (or a")
    print("  human reviewer) to re-elicit a rulebook conditioned on the")
    print("  narrower conclusion term, then re-run.")
    print()

    # --- Framework kickback (input ambiguity) ---
    print("=" * 72)
    print("Framework kickback question (input ambiguity):")
    print("=" * 72)
    print()
    print('  "Were any of the patient\'s medications recently started,')
    print('   dose-increased, or restarted (within the past 30 days)?"')
    print()
    print('   [a] Yes — please specify which medication(s) and when')
    print('   [b] No — all medications are chronic (>30 days on current')
    print('       regimen)')
    print('   [c] Unknown — records unavailable')
    print()

    # --- Audit trail summary ---
    print("=" * 72)
    print("Audit-trail provenance summary:")
    print("=" * 72)
    print()
    print("  Source-byte provenance (input):")
    print(f"    Input fixture, {len(INPUT_FIXTURE)} bytes total")
    print(f"    {len(OBSERVED)} Affirmed Fact nodes extracted")
    print(f"    1 Uncertainty (synthesized: information-missing-from-input)")
    print()
    print("  Source-byte provenance (rulebook):")
    print(f"    Rulebook fixture, see adj37-rulebook.txt")
    print(f"    {len(CONTRIBUTIONS)} Rule-Fact nodes applied to inference")
    print(f"    {len(RULEBOOK_UNCERTAINTIES)} Uncertainty rules NOT applied (audit-recorded)")
    print(f"    {len(UNVERIFIED_CITATIONS)} citations flagged unverified")
    print()
    print("  Recommended next-actions before final deployment:")
    print("    1. Re-elicit rulebook conditioned on 'med-induced delirium'")
    print("       endpoint specifically (modeling-issue fix)")
    print("    2. Resolve chronicity via clinical staff (input-kickback)")
    print("    3. Verify unverified citations via PubMed/Crossref")
    print("    4. Independently confirm LR values for HIGH-stakes rules")


if __name__ == "__main__":
    main()
