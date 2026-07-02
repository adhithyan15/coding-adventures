"""Generate rung-45 (weight-and-time-normalized drug elimination rate) items.json for the ADJ-LADDER.

Rung 45 opens the **pharmacokinetic elimination-rate** panel on the quantitative band — the arithmetic of how fast a
drug leaves the body once you normalise the amount eliminated to the patient's body weight AND the elapsed time.
Between two measurements the amount eliminated is `administered_dose − residual_dose`; expressing that as a rate per
kilogram per hour divides it by the PRODUCT of the body weight and the elapsed time. This rung introduces a
genuinely NEW arithmetic shape on the ladder: **difference-over-product** — `(a − b) / (c · d)` — a difference in the
numerator divided by a product in the denominator.

The setup: a drug is given (`administered_dose`, mg) and later `residual_dose` (mg) remains; the patient weighs
`body_weight` (kg) and `elapsed_time` (h) has passed. The weight-and-time-normalised elimination rate is the amount
that left divided by the weight-times-time product:

  ELIMINATION RATE      (administered_dose − residual_dose) / (body_weight · elapsed_time)   [ mg per kg per hour ]
  ELIMINATED AMOUNT     administered_dose − residual_dose                                     [ the numerator: what left ]
  WEIGHT·TIME PRODUCT   body_weight · elapsed_time                                            [ the denominator ]

The **elimination rate** is what makes this rung distinctive — it is the ladder's first **difference-over-product**:
a parenthesised difference divided by a parenthesised product. Contrast the neighbours already on the ladder: rung-41
was a *difference-over-sum* `(a−b)/(a+b)`, rung-42 a *sum-over-difference* `(a+b)/(c−d)`, rung-44 a *product-over-sum*
`(a·b)/(c+d)`; none divided a DIFFERENCE by a PRODUCT. (The eliminated amount `administered_dose − residual_dose` and
the weight·time product `body_weight · elapsed_time` ride alongside as the two component quantities, so the panel
teaches the whole calculation — exactly as rung-44 shipped its injected mass and distribution volume beside the
headline concentration.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — including the inner `(administered_dose − residual_dose)` difference and the
`(body_weight · elapsed_time)` product — and the harness reads the scalar via the existing `compute_dimensioned`
extractor. No harness/engine change, exactly as rungs 8/16/.../43/44. This rung exercises the engine across a
**difference divided by a product**.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `−`, `·` and
`/` — **no structural constants** — so no numeric literal appears in any program, and neither the eliminated amount,
the weight·time product, nor any rate is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`administered_dose`, `residual_dose`, `body_weight`, `elapsed_time`) so
no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic
slips —

  WEIGHT ONLY         (administered_dose − residual_dose) / body_weight                       normalise by weight
                                                                                              alone, forgetting the
                                                                                              elapsed time, and
  SUMMED DENOMINATOR  (administered_dose − residual_dose) / (body_weight + elapsed_time)      ADD the weight and time
                                                                                              instead of multiplying
                                                                                              them,

which are exactly the mistakes a student makes (dropping a normaliser, or adding two quantities that should be
multiplied). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness: the tables below are chosen with `administered_dose > residual_dose` (so the eliminated amount — and
every numerator — is strictly positive) and all four quantities positive (so every denominator is strictly positive,
no division by zero); the five family values are asserted pairwise-distinct with a comfortable margin at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (ADMINISTERED_DOSE, RESIDUAL_DOSE, BODY_WEIGHT, ELAPSED_TIME) — doses in mg, weight in kg, time in h.
# ADMINISTERED_DOSE > RESIDUAL_DOSE > 0 so the eliminated amount (every numerator) is strictly positive, and
# BODY_WEIGHT, ELAPSED_TIME > 0 so every denominator is strictly positive. The five family values are asserted
# pairwise-distinct (with margin) below.
TABLES = [
    (100, 20, 4, 2),
    (120, 30, 3, 3),
    (90, 10, 5, 2),
    (60, 12, 3, 4),
    (80, 20, 4, 3),
    (150, 30, 6, 2),
    (200, 40, 4, 4),
]

# The option family (5 members), all built from the four observed quantities via -, * and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "elimination_rate",
        "weight-and-time-normalised elimination rate (amount eliminated over weight times time)",
        "(administered_dose - residual_dose) / (body_weight * elapsed_time)",
    ),
    (
        "eliminated_amount",
        "the amount of drug eliminated (administered minus residual)",
        "administered_dose - residual_dose",
    ),
    (
        "weight_time_product",
        "the weight-times-time product (the normalising denominator)",
        "body_weight * elapsed_time",
    ),
    (
        "weight_only",
        "amount eliminated over the body weight ALONE, forgetting the elapsed time (a wrong denominator)",
        "(administered_dose - residual_dose) / body_weight",
    ),
    (
        "summed_denominator",
        "amount eliminated over weight PLUS time (the two normalisers added instead of multiplied)",
        "(administered_dose - residual_dose) / (body_weight + elapsed_time)",
    ),
]
QUERIED = ["elimination_rate", "eliminated_amount", "weight_time_product"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(administered_dose, residual_dose, body_weight, elapsed_time):
    # Operation order mirrors the ADJ programs exactly (difference in the numerator, product in the denominator),
    # so the Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9
    # match tolerance).
    eliminated = administered_dose - residual_dose
    product = body_weight * elapsed_time
    return {
        "elimination_rate": eliminated / product,
        "eliminated_amount": eliminated,
        "weight_time_product": product,
        "weight_only": eliminated / body_weight,
        "summed_denominator": eliminated / (body_weight + elapsed_time),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for administered_dose, residual_dose, body_weight, elapsed_time in TABLES:
        assert administered_dose > residual_dose > 0 and body_weight > 0 and elapsed_time > 0, (
            administered_dose,
            residual_dose,
            body_weight,
            elapsed_time,
        )
        fv = family_values(administered_dose, residual_dose, body_weight, elapsed_time)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    administered_dose,
                    residual_dose,
                    body_weight,
                    elapsed_time,
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
                administered_dose,
                residual_dose,
                body_weight,
                elapsed_time,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r45elr-{idx + 1:02d}",
                "qtype": "elimination_rate",
                "stem": (
                    f"A {num(administered_dose)} mg dose is given to a {num(body_weight)} kg patient; after "
                    f"{num(elapsed_time)} h, {num(residual_dose)} mg remains. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe administered_dose({num(administered_dose)})\n"
                    f"observe residual_dose({num(residual_dose)})\n"
                    f"observe body_weight({num(body_weight)})\n"
                    f"observe elapsed_time({num(elapsed_time)})\n"
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
            "ADJ-LADDER rung 45 — weight-and-time-normalised drug elimination rate from four stated quantities (a "
            "NEW panel: pharmacokinetic elimination rate). From an administered and a residual dose plus the body "
            "weight and elapsed time compute the elimination rate ((administered_dose-residual_dose)/(body_weight*"
            "elapsed_time)), the eliminated amount (administered_dose-residual_dose), or the weight*time product "
            "(body_weight*elapsed_time). Each item is a compute_dimensioned program (observe the four quantities, "
            "let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, DIFFERENCE-OVER-PRODUCT "
            "(a-b)/(c*d), the first quotient on the ladder to divide a parenthesised difference by a parenthesised "
            "product (distinct from rung-41 difference-over-sum (a-b)/(a+b), rung-42 sum-over-difference (a+b)/(c-d), "
            "and rung-44 product-over-sum (a*b)/(c+d)) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via -, * and / — no "
            "constant leaks, and neither the eliminated amount, the weight*time product, nor any rate ever appears "
            "as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. The five options are a family over the same four quantities, so "
            "the distractors are exactly the slips students make: normalising by the body weight alone (dropping the "
            "elapsed time), and ADDING the weight and time instead of multiplying them. The core confusion tested is "
            "dividing the eliminated amount by the weight-times-time product."
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
