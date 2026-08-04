"""Generate rung-130 (relative-potency ratio / COMPLEX FRACTION — a ratio of two ratios) items.json.

Rung 130 opens the **relative-potency** panel and takes the ladder's first step into a **complex fraction**: a quotient in BOTH the
numerator and the denominator at once, `(a/b)/(c/d)`. Rungs 126-129 completed the full 2x2 matrix of a SINGLE embedded quotient — {add,
subtract} x {denominator, numerator}: `a/(b+c/d)`, `a/(b-c/d)`, `(a+b/c)/d`, `(a-b/c)/d`. Rung 130 is the natural next tier: TWO embedded
quotients, one over the other — a ratio of two ratios.

This is genuinely new. `(a/b)/(c/d)` is a fraction whose numerator is itself a fraction `a/b` and whose denominator is itself a fraction
`c/d`. Dividing by a fraction is multiplying by its reciprocal, so `(a/b)/(c/d) = (a/b) * (d/c) = (a*d)/(b*c)` — the classic **invert the
DENOMINATOR fraction and multiply**. The core confusions this rung tests are the two canonical complex-fraction slips: multiplying the two
ratios straight across WITHOUT inverting (`(a/b) * (c/d) = (a*c)/(b*d)`), and inverting the WHOLE ratio — dividing the reference by the
treatment instead of the treatment by the reference (`(c/d)/(a/b) = (b*c)/(a*d)`).

The setup: a `drug_amount` in a `carrier_volume` (a treatment concentration `drug_amount/carrier_volume`), measured RELATIVE to a
`ref_amount` in a `ref_volume` (a reference concentration `ref_amount/ref_volume`). The figures are:

  RELATIVE POTENCY          (drug_amount / carrier_volume) / (ref_amount / ref_volume)   [ complex fraction: treatment conc OVER ref conc ]
  TREATMENT CONCENTRATION   drug_amount / carrier_volume                                 [ the numerator ratio (the top fraction) ]
  REFERENCE CONCENTRATION   ref_amount / ref_volume                                      [ the denominator ratio (the bottom fraction) ]

The **relative potency** is the ladder's first **complex fraction (a ratio of two ratios) as a headline** — a dimensionless relative
figure (how many times as concentrated the treatment is versus the reference), framed as a *potency ratio* to keep it dimensionless-clean,
the same discipline rungs 100/.../128/129 used for their ratios. (The treatment concentration `a/b` and the reference concentration `c/d`
ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-129 shipped their component figures
beside the headline. The two component ratios anchor the "form each concentration FIRST, then divide the treatment by the reference"
structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the two divisions to form the treatment and reference concentrations, then the division of the treatment concentration by the
reference concentration to form the complex fraction (so (a/b)/(c/d) evaluates as ((a/b)/(c/d)) = (a*d)/(b*c)) — and the harness reads the
scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../128/129. This rung exercises
the engine across a **ratio of two ratios (complex fraction)** — the fact that `(a/b)/(c/d)` is `(a*d)/(b*c)` and NOT `(a*c)/(b*d)` and NOT
`(b*c)/(a*d)` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same
way rungs 100/.../128/129 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/` and `*` — **no structural
constants** — so no numeric literal appears in any program, and neither the treatment concentration, the reference concentration, nor the
relative potency is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`drug_amount`, `carrier_volume`, `ref_amount`, `ref_volume`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic complex-fraction slips —

  STRAIGHT      (drug_amount / carrier_volume) * (ref_amount / ref_volume)   multiply the two concentrations straight across WITHOUT
                                                                  inverting the denominator fraction (the classic "divide fractions =
                                                                  multiply straight" error, evaluating `(a*c)/(b*d)`), and
  RECIPROCAL    (ref_amount / ref_volume) / (drug_amount / carrier_volume)   invert the WHOLE ratio — divide the reference by the
                                                                  treatment instead of the treatment by the reference (`(c/d)/(a/b) =
                                                                  (b*c)/(a*d)`, the upside-down complex fraction),

which are exactly the mistakes a student makes (multiplying instead of inverting-and-dividing, or dividing the wrong way round). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `/` and `*` over positive quantities, so **every figure is automatically positive** (no
subtraction anywhere) — like rung-128, no positivity guards are needed. Every observed quantity is `>= 2`. Every family member is asserted
`> 0` at build time as a belt-and-suspenders check. The seven tables give distinct relative potencies, distinct treatment concentrations,
and distinct reference concentrations so all three queried readouts vary across the panel; the five family values are pairwise distinct
with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (DRUG_AMOUNT, CARRIER_VOLUME, REF_AMOUNT, REF_VOLUME) — a treatment concentration (drug_amount/carrier_volume) measured RELATIVE to a
# reference concentration (ref_amount/ref_volume), giving the relative potency as a complex fraction (a/b)/(c/d) = (a*d)/(b*c). This rung
# uses only / and * over positive quantities, so every figure is automatically positive; no positivity guards are needed. The seven
# tables give distinct treatment concentrations (a/b), distinct reference concentrations (c/d), and distinct relative potencies
# ((a/b)/(c/d)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (6, 2, 2, 4),     # a/b = 3.0,  c/d = 0.5,  potency = 6.0
    (8, 2, 6, 2),     # a/b = 4.0,  c/d = 3.0,  potency = 1.333...
    (5, 2, 3, 4),     # a/b = 2.5,  c/d = 0.75, potency = 3.333...
    (10, 2, 8, 4),    # a/b = 5.0,  c/d = 2.0,  potency = 2.5
    (3, 2, 8, 2),     # a/b = 1.5,  c/d = 4.0,  potency = 0.375
    (7, 2, 5, 4),     # a/b = 3.5,  c/d = 1.25, potency = 2.8
    (12, 2, 3, 2),    # a/b = 6.0,  c/d = 1.5,  potency = 4.0
]

# The option family (5 members), all built from the four observed quantities via / and *. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "relative_potency",
        "relative potency (the treatment concentration divided by the reference concentration, a complex fraction)",
        "(drug_amount / carrier_volume) / (ref_amount / ref_volume)",
    ),
    (
        "treatment_concentration",
        "the treatment concentration (the drug amount per carrier volume, the numerator ratio of the complex fraction)",
        "drug_amount / carrier_volume",
    ),
    (
        "reference_concentration",
        "the reference concentration (the reference amount per reference volume, the denominator ratio of the complex fraction)",
        "ref_amount / ref_volume",
    ),
    (
        "straight",
        "the treatment concentration times the reference concentration, multiplying the two ratios straight across without inverting the denominator fraction (a wrong operation)",
        "(drug_amount / carrier_volume) * (ref_amount / ref_volume)",
    ),
    (
        "reciprocal",
        "the reference concentration divided by the treatment concentration, inverting the whole ratio so it divides the wrong way round (a wrong operation)",
        "(ref_amount / ref_volume) / (drug_amount / carrier_volume)",
    ),
]
QUERIED = ["relative_potency", "treatment_concentration", "reference_concentration"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(drug_amount, carrier_volume, ref_amount, ref_volume):
    # Operation order mirrors the ADJ programs exactly (the two divisions form the treatment and reference concentrations, then the
    # treatment concentration is divided by the reference concentration to form the complex fraction, so (a/b)/(c/d) evaluates as
    # ((a/b)/(c/d)) = (a*d)/(b*c)), so the Python option value and the engine result are the same IEEE-double (well within the 1e-9
    # tolerance).
    return {
        "relative_potency": (drug_amount / carrier_volume) / (ref_amount / ref_volume),
        "treatment_concentration": drug_amount / carrier_volume,
        "reference_concentration": ref_amount / ref_volume,
        "straight": (drug_amount / carrier_volume) * (ref_amount / ref_volume),
        "reciprocal": (ref_amount / ref_volume) / (drug_amount / carrier_volume),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for drug_amount, carrier_volume, ref_amount, ref_volume in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only / and * over positive quantities, so positivity is
        # automatic — no positivity guards are needed.
        assert (
            drug_amount >= 2
            and carrier_volume >= 2
            and ref_amount >= 2
            and ref_volume >= 2
        ), (drug_amount, carrier_volume, ref_amount, ref_volume)
        fv = family_values(drug_amount, carrier_volume, ref_amount, ref_volume)
        for key, v in fv.items():
            assert v > 0, (key, drug_amount, carrier_volume, ref_amount, ref_volume, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    drug_amount,
                    carrier_volume,
                    ref_amount,
                    ref_volume,
                    ORDER[i],
                    ORDER[j],
                    fv,
                )
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (
                key,
                drug_amount,
                carrier_volume,
                ref_amount,
                ref_volume,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r130rpa-{idx + 1:02d}",
                "qtype": "relative_potency",
                "stem": (
                    f"A relative-potency study records a drug amount of {num(drug_amount)} in a carrier volume of "
                    f"{num(carrier_volume)}, measured relative to a reference amount of {num(ref_amount)} in a reference volume of "
                    f"{num(ref_volume)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe drug_amount({num(drug_amount)})\n"
                    f"observe carrier_volume({num(carrier_volume)})\n"
                    f"observe ref_amount({num(ref_amount)})\n"
                    f"observe ref_volume({num(ref_volume)})\n"
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
            "ADJ-LADDER rung 130 — relative-potency ratio from four stated quantities (a NEW panel: relative potency, and the ladder's "
            "first COMPLEX FRACTION — a quotient in BOTH the numerator and the denominator at once). Rungs 126-129 completed the 2x2 "
            "matrix of a single embedded quotient ({add,subtract} x {denominator,numerator}); rung 130 is the next tier: a ratio of two "
            "ratios (a/b)/(c/d). From a treatment concentration (drug_amount/carrier_volume) measured relative to a reference "
            "concentration (ref_amount/ref_volume), compute the relative potency "
            "((drug_amount/carrier_volume)/(ref_amount/ref_volume)), the treatment concentration (drug_amount/carrier_volume), or the "
            "reference concentration (ref_amount/ref_volume). Each item is a compute_dimensioned program (observe the four quantities, "
            "let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a COMPLEX FRACTION (a/b)/(c/d) (form each "
            "concentration, then divide the treatment by the reference, so (a/b)/(c/d) = (a*d)/(b*c) — dividing by a fraction is "
            "multiplying by its reciprocal, invert the DENOMINATOR fraction). The invert-and-multiply slips ride alongside as "
            "distractors. The harness matches the scalar to the printed options. The relative potency is a dimensionless relative figure "
            "(how many times as concentrated the treatment is versus the reference), framed as a POTENCY RATIO so the dimensionless "
            "value stays honest. Contamination-safe: every figure is built only from the four observed quantities via / and * — no "
            "constant leaks, and neither the treatment concentration, the reference concentration, nor the relative potency ever appears "
            "as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a "
            "variable name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make: multiplying the two ratios straight across without inverting the denominator fraction ((a/b)*(c/d) = "
            "(a*c)/(b*d), a wrong operation) and inverting the whole ratio so it divides the wrong way round ((c/d)/(a/b) = (b*c)/(a*d), "
            "a wrong operation). The core confusion tested is that (a/b)/(c/d) is (a*d)/(b*c), not (a*c)/(b*d) and not (b*c)/(a*d). This "
            "rung uses only / and * over positive quantities, so every figure is automatically positive — no positivity guards are "
            "needed — and the five family values are kept pairwise distinct with all three queried readouts varying across the panel, "
            "all asserted strictly positive at build time."
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
