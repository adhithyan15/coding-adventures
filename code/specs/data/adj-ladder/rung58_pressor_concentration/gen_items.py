"""Generate rung-58 (vasopressor bag concentration) items.json for the ADJ-LADDER.

Rung 58 opens the **critical-care / vasopressor-infusion** panel on the quantitative band — the arithmetic of mixing a
pressor drip. A nurse draws several ampoules of a drug into a bag: the total drug MASS is the amount per ampoule times
the number of ampoules (a PRODUCT), and the total fluid VOLUME is the drug volume plus the diluent volume (a SUM). The
bag's concentration is the total mass divided by the total volume. Dividing a product by a sum introduces a genuinely
NEW arithmetic shape on the ladder: a **product over a sum** — `(a · b) / (c + d)` — a parenthesised product in the
numerator over a parenthesised sum in the denominator.

The setup: a bag is mixed from `ampoule_count` ampoules of `amount_per_ampoule` each, drawn up in `drug_volume` of drug
solution and topped up with `diluent_volume` of diluent. The concentration is the total mass over the total volume:

  CONCENTRATION   (amount_per_ampoule · ampoule_count) / (drug_volume + diluent_volume)   [ the mixed bag strength ]
  TOTAL MASS      amount_per_ampoule · ampoule_count                                       [ the product: total drug ]
  TOTAL VOLUME    drug_volume + diluent_volume                                             [ the sum: total fluid ]

The **concentration** is what makes this rung distinctive — it is the ladder's first **product over a sum**: a
parenthesised product divided by a parenthesised sum. Contrast the neighbours already on the ladder: rung-15 was a
*product over a PRODUCT* `(a·b)/(c·d)` and rung-37 a *sum over a sum* `(a+b)/(c+d)`; neither divided a PRODUCT by a SUM.
(The total mass `amount_per_ampoule · ampoule_count` and the total volume `drug_volume + diluent_volume` ride alongside
as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-57 shipped their component
products/sums/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including the parenthesised product and sum and their quotient — and the harness reads the
scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../56/57. This
rung exercises the engine across a **product divided by a sum** — the fact that `(a·b)/(c+d)` is NOT `a·b/c + d` and NOT
`(a·b)/d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `·`, `+` and `/`
— **no structural constants** — so no numeric literal appears in any program, and neither the total mass, the total
volume, nor any concentration figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`amount_per_ampoule`, `ampoule_count`, `drug_volume`, `diluent_volume`) so
no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  MISGROUPED           (amount_per_ampoule · ampoule_count) / drug_volume + diluent_volume   divide the mass by only the
                                                                                             DRUG volume, then add the
                                                                                             diluent on
                                                                                             (`… / drug + diluent`, not
                                                                                             `… / (drug + diluent)`), and
  MASS OVER DILUENT    (amount_per_ampoule · ampoule_count) / diluent_volume                 divide the mass by the
                                                                                             DILUENT volume alone,
                                                                                             forgetting the drug volume,

which are exactly the mistakes a student makes (breaking the grouping so the mass divides only the first volume term, or
dividing by one volume component instead of the total). Gold rotates A-E by index. QUERIED (used as gold) = the three
real readouts; all five always appear as options.

Distinctness: all four observed quantities are strictly positive, so every product, sum and quotient is positive; the
tables below are chosen so the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (AMOUNT_PER_AMPOULE, AMPOULE_COUNT, DRUG_VOLUME, DILUENT_VOLUME) — the drug amount in one ampoule and the number of
# ampoules (their product is the total mass), and the drug and diluent volumes (their sum is the total fluid), all
# plain positive numbers. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (4, 5, 20, 80),
    (8, 3, 12, 88),
    (2, 6, 10, 40),
    (10, 4, 40, 60),
    (5, 8, 20, 30),
    (6, 5, 30, 70),
    (16, 2, 8, 42),
]

# The option family (5 members), all built from the four observed quantities via *, + and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "concentration",
        "the bag concentration (the total drug mass over the total fluid volume)",
        "(amount_per_ampoule * ampoule_count) / (drug_volume + diluent_volume)",
    ),
    (
        "total_mass",
        "the total drug mass (amount per ampoule times the number of ampoules)",
        "amount_per_ampoule * ampoule_count",
    ),
    (
        "total_volume",
        "the total fluid volume (drug volume plus diluent volume)",
        "drug_volume + diluent_volume",
    ),
    (
        "misgrouped",
        "the mass divided by only the DRUG volume, with diluent volume added on (broken grouping)",
        "(amount_per_ampoule * ampoule_count) / drug_volume + diluent_volume",
    ),
    (
        "mass_over_diluent",
        "the mass divided by the DILUENT volume alone (forgetting the drug volume)",
        "(amount_per_ampoule * ampoule_count) / diluent_volume",
    ),
]
QUERIED = ["concentration", "total_mass", "total_volume"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(amount_per_ampoule, ampoule_count, drug_volume, diluent_volume):
    # Operation order mirrors the ADJ programs exactly (a parenthesised product over a parenthesised sum; and, for the
    # misgrouped slip, the mass-over-drug quotient binds tighter than the trailing addition), so the Python option value
    # and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    mass = amount_per_ampoule * ampoule_count
    volume = drug_volume + diluent_volume
    return {
        "concentration": mass / volume,
        "total_mass": mass,
        "total_volume": volume,
        "misgrouped": mass / drug_volume + diluent_volume,
        "mass_over_diluent": mass / diluent_volume,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for amount_per_ampoule, ampoule_count, drug_volume, diluent_volume in TABLES:
        assert (
            amount_per_ampoule > 0
            and ampoule_count > 0
            and drug_volume > 0
            and diluent_volume > 0
        ), (amount_per_ampoule, ampoule_count, drug_volume, diluent_volume)
        fv = family_values(amount_per_ampoule, ampoule_count, drug_volume, diluent_volume)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    amount_per_ampoule,
                    ampoule_count,
                    drug_volume,
                    diluent_volume,
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
                amount_per_ampoule,
                ampoule_count,
                drug_volume,
                diluent_volume,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r58conc-{idx + 1:02d}",
                "qtype": "pressor_concentration",
                "stem": (
                    f"A pressor bag is mixed from {num(ampoule_count)} ampoules of {num(amount_per_ampoule)} mg each, "
                    f"drawn up in {num(drug_volume)} mL of drug solution and topped up with {num(diluent_volume)} mL of "
                    f"diluent. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe amount_per_ampoule({num(amount_per_ampoule)})\n"
                    f"observe ampoule_count({num(ampoule_count)})\n"
                    f"observe drug_volume({num(drug_volume)})\n"
                    f"observe diluent_volume({num(diluent_volume)})\n"
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
            "ADJ-LADDER rung 58 — vasopressor bag concentration from four stated quantities (a NEW panel: critical "
            "care / vasopressor infusion). From an amount per ampoule and an ampoule count (their product is the total "
            "drug mass) and a drug volume and a diluent volume (their sum is the total fluid), compute the concentration "
            "((amount_per_ampoule*ampoule_count)/(drug_volume+diluent_volume)), the total mass "
            "(amount_per_ampoule*ampoule_count), or the total volume (drug_volume+diluent_volume). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW shape, PRODUCT OVER A SUM (a*b)/(c+d), the first on the ladder to divide a product by a "
            "sum (distinct from rung-15 product-over-product (a*b)/(c*d) and rung-37 sum-over-sum (a+b)/(c+d)) — and the "
            "harness matches the scalar to the printed options. Contamination-safe: every index is built only from the "
            "four observed quantities via *, + and / — no constant leaks, and neither the total mass, the total volume, "
            "nor any concentration figure ever appears as a literal (each is computed) — and the observed quantities "
            "carry digit-free identifiers so no numeral hides inside a variable name. The five options are a family over "
            "the same four quantities, so the distractors are exactly the slips students make: breaking the grouping so "
            "the mass divides only the drug volume ((a*b)/c+d, not (a*b)/(c+d)), and dividing by the diluent volume "
            "alone ((a*b)/d). The core confusion tested is dividing a product by a grouped sum."
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
