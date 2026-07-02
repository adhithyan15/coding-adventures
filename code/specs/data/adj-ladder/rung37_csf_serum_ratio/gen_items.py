"""Generate rung-37 (CSF:serum protein ratio) items.json for the ADJ-LADDER.

Rung 37 opens the **neurology / cerebrospinal-fluid analysis** panel on the quantitative band — the arithmetic of
comparing the total protein in the CSF against the total protein in serum. A CSF:serum ratio is how the lab tells
whether protein in the spinal fluid reflects the blood (a leaky barrier) or is made locally: you add up the CSF's
protein fractions, add up the serum's, and divide. It uses the same contamination-safe shape as the
transfusion-pooling rung (36), the dialysis rung (35), and the admixture rung (34): a small table of *observed*
protein fractions and a tight family of mutually-confusable formulas built **only from those observed
quantities** (no numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a paired CSF-and-serum protein panel. FOUR quantities are measured — two CSF fractions and
two serum fractions (all in mg/dL):

  CSF_ALBUMIN      albumin fraction in the CSF          (low — the barrier keeps most protein out)
  CSF_GLOBULIN     globulin fraction in the CSF         (low)
  SERUM_ALBUMIN    albumin fraction in serum            (high)
  SERUM_GLOBULIN   globulin fraction in serum           (high)

The CSF:serum ratio is the **combined CSF protein over the combined serum protein** — a *ratio of two sums* —
`(CSF_ALBUMIN + CSF_GLOBULIN) / (SERUM_ALBUMIN + SERUM_GLOBULIN)`. That is what makes this rung distinctive: it
is a NEW arithmetic shape on the ladder — a quotient whose numerator AND denominator are each their own SUM
(rung-32 divided one difference by another; rung-36 divided a sum-of-products by a sum; this rung divides one
sum by another). The core confusion this rung tests is adding each side's own two fractions before dividing,
rather than crossing the compartments or multiplying the fractions:

  CSF:SERUM RATIO   (CSF_ALB + CSF_GLOB) / (SERUM_ALB + SERUM_GLOB)   [ total CSF protein ÷ total serum protein ]
  CSF TOTAL         CSF_ALB + CSF_GLOB                                [ the combined CSF protein, the numerator ]
  SERUM TOTAL       SERUM_ALB + SERUM_GLOB                            [ the combined serum protein, the denominator ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/35/36. This rung exercises the engine across a
DIVISION of two parenthesised SUMS.

Contamination-safe by construction: every formula is built only from the four observed quantities via `+`, `/`,
`*` — **no structural constants** — so every program literal is grounded in the stem. Neither the CSF total nor
the serum total ever appears as a literal (each is computed from the observed fractions). The observed
quantities carry **digit-free identifiers** (`csf_albumin`, `csf_globulin`, `serum_albumin`, `serum_globulin`)
so no numeral hides inside a variable name. The five options are a tight family over the same quantities: the
three real indices plus the two classic slips —

  CROSSED RATIO   (CSF_ALB + SERUM_GLOB) / (SERUM_ALB + CSF_GLOB)   each sum built from one CSF and one serum fraction, and
  PRODUCT RATIO   (CSF_ALB * CSF_GLOB) / (SERUM_ALB * SERUM_GLOB)   the fractions MULTIPLIED within each side instead of added,

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the ratio is small (CSF protein is far below serum, so the quotient is order 0.01), the CSF total
is tens of mg/dL, the serum total is thousands, the crossed ratio is order 1 (one CSF + one serum fraction on
each side), and the product ratio is tiny (order 1e-4); the tables below are chosen so the five family values
are pairwise distinct — with a comfortable margin — for every item, asserted at build time (all four fractions
positive so both sums are positive and no division by zero, and no two family values collide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CSF_ALBUMIN, CSF_GLOBULIN, SERUM_ALBUMIN, SERUM_GLOBULIN) observed protein fractions, all in mg/dL. The CSF
# fractions are far below the serum fractions (the blood-CSF barrier keeps protein out), so the CSF:serum ratio
# is well below 1. All four are strictly positive, so both sums are positive (no division by zero). The five
# family values are asserted pairwise-distinct (with margin) below.
#   csf_alb / csf_glob   = CSF albumin / globulin      (low)
#   serum_alb / serum_glob = serum albumin / globulin  (high)
TABLES = [
    (20, 30, 3000, 2000),
    (40, 20, 2500, 3500),
    (15, 45, 4000, 2000),
    (30, 25, 2200, 2800),
    (50, 20, 3600, 2400),
    (25, 35, 2000, 4000),
    (45, 15, 3000, 3500),
]

# The option family (5 members), all built from the observed quantities via `+` / `/` / `*`. Every identifier
# is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all
# five always appear as the options.
FAMILY = [
    (
        "csf_serum_ratio",
        "CSF:serum protein ratio",
        "(csf_albumin + csf_globulin) / (serum_albumin + serum_globulin)",
    ),
    (
        "csf_total",
        "total CSF protein",
        "csf_albumin + csf_globulin",
    ),
    (
        "serum_total",
        "total serum protein",
        "serum_albumin + serum_globulin",
    ),
    (
        "crossed_ratio",
        "crossed ratio (each sum built from one CSF and one serum fraction)",
        "(csf_albumin + serum_globulin) / (serum_albumin + csf_globulin)",
    ),
    (
        "product_ratio",
        "product ratio (fractions multiplied within each side instead of added)",
        "(csf_albumin * csf_globulin) / (serum_albumin * serum_globulin)",
    ),
]
QUERIED = ["csf_serum_ratio", "csf_total", "serum_total"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(ca, cg, sa, sg):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    csf_total = ca + cg
    serum_total = sa + sg
    return {
        "csf_serum_ratio": csf_total / serum_total,
        "csf_total": csf_total,
        "serum_total": serum_total,
        "crossed_ratio": (ca + sg) / (sa + cg),
        "product_ratio": (ca * cg) / (sa * sg),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for ca, cg, sa, sg in TABLES:
        assert ca > 0 and cg > 0 and sa > 0 and sg > 0, (ca, cg, sa, sg)
        fv = family_values(ca, cg, sa, sg)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (ca, cg, sa, sg, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, ca, cg, sa, sg, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r37csf-{idx + 1:02d}",
                "qtype": "csf_serum_ratio",
                "stem": (
                    f"A paired CSF-and-serum protein panel shows a CSF albumin of {num(ca)} mg/dL and a CSF "
                    f"globulin of {num(cg)} mg/dL; the serum albumin is {num(sa)} mg/dL and the serum globulin "
                    f"is {num(sg)} mg/dL. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe csf_albumin({num(ca)})\n"
                    f"observe csf_globulin({num(cg)})\n"
                    f"observe serum_albumin({num(sa)})\n"
                    f"observe serum_globulin({num(sg)})\n"
                    f"let answer = {formula_of[key]}\n"
                    "? answer\n"
                ),
                "answer_from": {"type": "compute_dimensioned", "name": "answer"},
                "options": options,
                "gold_letter": LETTERS[gold_pos],
            })
            idx += 1
    return {
        "description": (
            "ADJ-LADDER rung 37 — CSF:serum protein ratio from paired CSF and serum protein fractions (a NEW "
            "panel: neurology / cerebrospinal-fluid analysis). From four stated fractions (CSF albumin, CSF "
            "globulin, serum albumin, serum globulin) compute the CSF:serum ratio "
            "((CSF_ALB+CSF_GLOB)/(SERUM_ALB+SERUM_GLOB)), the CSF total (CSF_ALB+CSF_GLOB), or the serum total "
            "(SERUM_ALB+SERUM_GLOB). Each item is a compute_dimensioned program (observe the four quantities, "
            "let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, a RATIO OF TWO SUMS "
            "((CSF_ALB+CSF_GLOB)/(SERUM_ALB+SERUM_GLOB)), so one parenthesised sum is divided by another — and "
            "the harness matches the scalar to the printed options. Contamination-safe: every index is built "
            "only from the four observed fractions via +, / and * — no constant leaks (a CSF:serum ratio is a "
            "pure quotient), and neither the CSF total nor the serum total ever appears as a literal (each is "
            "computed from the observed fractions) — and the observed quantities carry digit-free identifiers so "
            "no numeral hides inside a variable name. The five options are a family over the same quantities, so "
            "the distractors are exactly the slips students make: the crossed ratio "
            "((CSF_ALB+SERUM_GLOB)/(SERUM_ALB+CSF_GLOB), each sum mixing one CSF and one serum fraction) and the "
            "product ratio ((CSF_ALB*CSF_GLOB)/(SERUM_ALB*SERUM_GLOB), fractions multiplied within each side "
            "instead of added). The core confusion tested is summing each side's own two fractions before "
            "dividing."
        ),
        "items": items,
    }


if __name__ == "__main__":
    doc = build()
    with open("items.json", "w") as f:
        json.dump(doc, f, indent=2)
        f.write("\n")
    print("wrote items.json:", len(doc["items"]), "items")
    for it in doc["items"]:
        print(it["id"], it["qtype"], "gold", it["gold_letter"],
              "=", round(it["options"][it["gold_letter"]]["value"], 6))
