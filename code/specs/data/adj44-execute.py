#!/usr/bin/env python3
"""ADJ44 — MYCIN-2026 meningitis-differential executor.

Reproduces the LP19e log-odds arithmetic from ADJ44 §"Step 5",
computes VOI on the pending lumbar-puncture findings (U1),
shows the kickback question + the post-LP completion under each
plausible LP outcome.

Run: python3 adj44-execute.py
"""

import math


def logit(p): return math.log(p / (1.0 - p))
def sigmoid(x):
    if x >= 0: return 1.0 / (1.0 + math.exp(-x))
    z = math.exp(x); return z / (1.0 + z)


# ---------------------------------------------------------------------------
# Input IR — patient case
# ---------------------------------------------------------------------------

INPUT_FIXTURE = "28yo M, headache and fever x 6 hours. Temp 38.9C, neck stiffness noted, photophobia, no rash. No recent sick contacts known. Immunization status uncertain. No prior infections. Lumbar puncture pending."

OBSERVED = {
    "age_lt_50",                # 28yo (no Listeria coverage triggered)
    "sex_male",                 # not in current rule set
    "fever",                    # temp 38.9C
    "neck_stiffness",
    "photophobia",
    # Denied facts (apply LR− = 1/LR+):
    # - petechial_rash: DENIED → use complement
}

DENIED = {
    "petechial_rash",
    "recent_sick_contacts",
    "prior_infections",
}

# Unresolved (kickback candidates):
UNRESOLVED = {
    "csf_findings",             # U1 — primary kickback
    "immunization_status",      # U2 — secondary
}


# ---------------------------------------------------------------------------
# Rulebook IR (decomposed from adj44-rulebook.txt; provenance flagged)
# ---------------------------------------------------------------------------

PRIOR = {"bacterial": 0.35}
PRIOR_CITATION = "Tunkel 2004 IDSA + Thigpen 2011 NEJM [PROVENANCE: A−]"

# (lr, evidence_atom, citation, provenance_grade)
CONTRIBUTIONS_OBSERVED = [
    (1.3, "fever",                 "van de Beek 2006 NEJM",            "A−"),
    (1.5, "neck_stiffness",        "Brouwer 2010 Clin Microbiol Rev",  "B+"),
    (2.0, "ams",                   "van de Beek 2006 NEJM",            "A−"),
    (3.0, "classic_triad",         "van de Beek 2006 NEJM",            "A−"),
    (1.3, "photophobia",           "[empirical, LOW confidence]",      "C"),
    (5.0, "petechial_rash",        "Thigpen 2011 NEJM",                "B"),
]

# CSF findings — these only apply when LP results in
CONTRIBUTIONS_CSF = [
    (5.0, "csf_wbc_gt_1000",       "Spanos 1989 JAMA",                 "B"),
    (4.0, "csf_neutrophil_gt_80",  "Spanos 1989 JAMA",                 "B"),
    (6.0, "csf_glucose_lt_40",     "Spanos 1989 JAMA",                 "B"),
    (5.0, "csf_glucose_ratio_lt_0_4", "Spanos 1989 JAMA",              "B"),
    (3.0, "csf_protein_gt_200",    "[approximated from training]",     "C"),
    (4.0, "csf_lactate_gt_3_5",    "Sakushima 2011 J Infect",          "B"),
    (20.0, "csf_gram_stain_positive", "Tunkel 2004 IDSA",              "A"),
]


# ---------------------------------------------------------------------------
# Inference
# ---------------------------------------------------------------------------

def lr_aggregate(observed, denied):
    p0 = PRIOR["bacterial"]
    lam = logit(p0)
    trace = [{"step": "prior", "value": p0, "logit": lam,
              "citation": PRIOR_CITATION}]

    for lr, ev, cite, grade in CONTRIBUTIONS_OBSERVED:
        if ev in observed:
            delta = math.log(lr)
            lam += delta
            trace.append({"step": "contrib", "atom": ev, "lr": lr,
                          "delta": delta, "logit": lam,
                          "citation": cite, "grade": grade})
        elif ev in denied:
            # Use 1/LR+ as the negative-evidence LR.
            inv_lr = 1.0 / lr
            delta = math.log(inv_lr)
            lam += delta
            trace.append({"step": "contrib_denied", "atom": ev, "lr": inv_lr,
                          "delta": delta, "logit": lam,
                          "citation": cite, "grade": grade,
                          "note": "denied; using LR− = 1/LR+"})

    return lam, sigmoid(lam), trace


# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

def run():
    print("=" * 72)
    print("ADJ44 — MYCIN-2026 meningitis differential")
    print("=" * 72)
    print()
    print(f"Patient case ({len(INPUT_FIXTURE)} bytes):")
    print(f"  {INPUT_FIXTURE!r}")
    print()
    print(f"Observed Facts: {len(OBSERVED)}")
    for a in sorted(OBSERVED):
        print(f"  - {a}")
    print(f"Denied Facts: {len(DENIED)}")
    for a in sorted(DENIED):
        print(f"  - {a}")
    print(f"Unresolved (Uncertainty): {len(UNRESOLVED)}")
    for a in sorted(UNRESOLVED):
        print(f"  - {a}  ← kickback candidate")
    print()

    # --- Pre-LP inference ---
    print("=" * 72)
    print("Pre-LP inference (CSF findings unresolved):")
    print("=" * 72)
    print()

    lam, p, trace = lr_aggregate(OBSERVED, DENIED)
    for step in trace:
        if step["step"] == "prior":
            print(f"  prior P(bacterial) = {step['value']:.2f} → λ₀ = {step['logit']:+.3f}")
            print(f"    [{step['citation']}]")
        elif step["step"] == "contrib":
            print(f"  +{step['atom']}: LR {step['lr']:.1f} → {step['delta']:+.3f}")
            print(f"    [{step['grade']}] {step['citation']}")
        elif step["step"] == "contrib_denied":
            print(f"  −{step['atom']}: LR− {step['lr']:.2f} → {step['delta']:+.3f}")
            print(f"    [{step['grade']}] {step['citation']} ({step['note']})")
    print()
    print(f"  Posterior logit (pre-LP): {lam:.3f}")
    print(f"  P(bacterial | pre-LP):    {p*100:.1f}%")
    print()

    # --- VOI on CSF findings ---
    print("=" * 72)
    print("VOI on U1 (pending CSF findings):")
    print("=" * 72)
    print()
    print("Posterior P(bacterial) under each plausible CSF outcome:")
    print()

    for lr, ev, cite, grade in CONTRIBUTIONS_CSF:
        # If observed (positive bacterial direction)
        lam_pos = lam + math.log(lr)
        p_pos = sigmoid(lam_pos)
        # If denied (viral direction; use 1/LR)
        lam_neg = lam + math.log(1.0 / lr)
        p_neg = sigmoid(lam_neg)
        print(f"  {ev}:")
        print(f"    if positive: P(bacterial) = {p_pos*100:5.1f}%  (LR {lr:.0f})")
        print(f"    if negative: P(bacterial) = {p_neg*100:5.1f}%  (LR− {1/lr:.2f})")
        print()

    print()
    print("=" * 72)
    print("FRAMEWORK KICKBACK")
    print("=" * 72)
    print()
    print(f"Pre-LP P(bacterial) = {p*100:.0f}%; insufficient to commit to a")
    print("treatment decision. The LP findings are the dominant")
    print("unresolved variable.")
    print()
    print("Action required:")
    print("  1. Obtain CSF Gram stain (highest single-test LR)")
    print("  2. Obtain CSF WBC count + differential")
    print("  3. Obtain CSF glucose (with concurrent serum glucose for ratio)")
    print("  4. Obtain CSF protein")
    print("  5. Obtain CSF lactate (if available)")
    print()
    print("Empiric coverage while LP is pending:")
    print("  Per IDSA 2004 (Tunkel et al.) + de Gans 2002 NEJM (European")
    print("  Dexamethasone Study), do NOT delay empiric antibiotics for LP")
    print("  in patients with clinical concern for bacterial meningitis.")
    print()
    print("  Start: ceftriaxone 2g IV q12h + vancomycin 15-20 mg/kg IV q8-12h")
    print("         + dexamethasone 0.15 mg/kg IV q6h")
    print("  Age 28 + immunocompetent → Listeria coverage not indicated")
    print()

    # --- Post-LP illustrative scenarios ---
    print("=" * 72)
    print("Post-LP illustrative scenarios:")
    print("=" * 72)
    print()

    scenarios = [
        ("Strong bacterial picture",
         {"csf_wbc_gt_1000", "csf_neutrophil_gt_80", "csf_glucose_lt_40",
          "csf_protein_gt_200", "csf_lactate_gt_3_5"},
         set()),
        ("Strong viral picture",
         set(),
         {"csf_wbc_gt_1000", "csf_neutrophil_gt_80", "csf_glucose_lt_40",
          "csf_protein_gt_200", "csf_lactate_gt_3_5"}),
        ("Gram stain positive (definitive)",
         {"csf_gram_stain_positive", "csf_wbc_gt_1000",
          "csf_neutrophil_gt_80", "csf_glucose_lt_40"},
         set()),
    ]

    for label, csf_pos, csf_neg in scenarios:
        lam_s = lam
        for lr, ev, _, _ in CONTRIBUTIONS_CSF:
            if ev in csf_pos:
                lam_s += math.log(lr)
            elif ev in csf_neg:
                lam_s += math.log(1.0 / lr)
        p_s = sigmoid(lam_s)
        print(f"  {label}:")
        print(f"    posterior P(bacterial) = {p_s*100:.1f}%")
        if p_s >= 0.80:
            tier = "CONFIRM bacterial; continue empiric coverage; adjust per culture/sensitivity"
        elif p_s >= 0.50:
            tier = "PROBABLE bacterial; continue empiric coverage; await cultures"
        elif p_s >= 0.20:
            tier = "INDETERMINATE; await cultures + clinical course"
        else:
            tier = "PROBABLE viral; consider de-escalation of empiric antibiotics after cultures negative"
        print(f"    Clinical interpretation: {tier}")
        print()


if __name__ == "__main__":
    run()
