"""Generate rung-135 (admixture concentration / a PRODUCT-numerator over a SUM — divide a total amount by a pooled total) items.json.

Rung 135 opens the **admixture** panel and **carries the product-numerator onto a SUM denominator**. rung-132 put a QUOTIENT over a sum,
`(a/b)/(c+d)`; rung-135 puts a PRODUCT over a sum, `(a*b)/(c+d)`. Read the other way, it is the sum-denominator twin of the over-a-rate
trio: rung-131 put a SUM over a rate `(a+b)/(c/d)`, rung-133 a DIFFERENCE `(a-b)/(c/d)`, rung-134 a PRODUCT `(a*b)/(c/d)`; rung-135 keeps
the product numerator but swaps the divide-by-a-rate skeleton for a divide-by-a-sum one, `(a*b)/(c+d)`.

This is genuinely new. `(a*b)/(c+d)` is a PRODUCT `a*b` divided by a SUM `c+d`. The product `a*b` binds and stays grouped over the bar
(grouping), and the two-part denominator `c+d` is ONE pooled total that the whole numerator is divided by (not two separate divisors, and
not distributed across). The core confusions this rung tests are the two canonical divide-by-a-sum slips: distributing the division across
the sum (`(a*b)/(c+d)` treated as `(a*b)/c + (a*b)/d`, which is FALSE because `x/(c+d) != x/c + x/d`), and dropping the grouping on the
denominator so only the first part divides and the second is added on (`(a*b)/c + d`).

The setup: a `dose_count` of aliquots each of `dose_size` (a total dose `dose_count * dose_size`), diluted into a pooled volume formed from
a `base_volume` plus an `added_volume` (a total volume `base_volume + added_volume`). The figures are:

  ADMIX CONC    (dose_count * dose_size) / (base_volume + added_volume)   [ product-numerator OVER a sum: total dose / total volume ]
  TOTAL DOSE    dose_count * dose_size                                    [ the product numerator (divided by the total volume) ]
  TOTAL VOLUME  base_volume + added_volume                               [ the pooled sum the total dose is divided by ]

The **admixture concentration** is the ladder's first **(a product) over (a sum) as a headline** — a concentration (how much total dose
sits in each unit of pooled volume), framed as a *concentration* to keep it dimensionless-clean, the same discipline rungs 100/.../133/134
used for their ratios and spans. (The total dose `a*b` and the total volume `c+d` ride alongside as component readouts, so the panel
teaches the whole calculation — exactly as rungs 47-134 shipped their component figures beside the headline. The two components anchor the
"multiply out the dose FIRST, pool the volume, then divide the dose by the pooled volume" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication to form the total dose, the addition to form the total volume, then the division of the total dose by the
total volume to form the compound figure (so (a*b)/(c+d) evaluates as ((a*b)/(c+d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../133/134. This rung exercises the engine across a
**product divided by a sum** — the fact that `(a*b)/(c+d)` is one product over one pooled total and NOT `(a*b)/c + (a*b)/d` and NOT
`(a*b)/c + d` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same
way rungs 100/.../133/134 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `/`, and `+` — **no structural
constants** — so no numeric literal appears in any program, and neither the total dose, the total volume, nor the admixture concentration
is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`dose_count`,
`dose_size`, `base_volume`, `added_volume`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SPLIT      (dose_count * dose_size) / base_volume + (dose_count * dose_size) / added_volume   distribute the division across the sum,
                                                                treating `x/(c+d)` as `x/c + x/d` (FALSE — division does not distribute
                                                                over a sum in the denominator), and
  FLAT       (dose_count * dose_size) / base_volume + added_volume   divide the total dose by the base volume ONLY and then add the added
                                                                volume, dropping the grouping on the denominator so the second part is
                                                                added on instead of pooled into the divisor (`(a*b)/c + d`),

which are exactly the mistakes a student makes (splitting one divisor into two, or losing the parentheses on the pooled denominator). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `*`, `/`, and `+` over positive quantities, so **every figure is automatically positive**
(no subtraction anywhere) — like rungs 128/130/131/132/134, no positivity guards are needed. Every observed quantity is `>= 2`. Every
family member is asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give distinct admixture concentrations,
distinct total doses, and distinct total volumes so all three queried readouts vary across the panel; the five family values are pairwise
distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (DOSE_COUNT, DOSE_SIZE, BASE_VOLUME, ADDED_VOLUME) — a total dose (dose_count * dose_size) divided by a pooled total volume
# (base_volume + added_volume), giving the admixture concentration as a product over a sum (a*b)/(c+d). This rung uses only *, /, and + over
# positive quantities, so every figure is automatically positive; no positivity guards are needed. The seven tables give distinct total
# doses (a*b), distinct total volumes (c+d), and distinct concentrations ((a*b)/(c+d)); the five family values are asserted pairwise-distinct
# below.
TABLES = [
    (2, 3, 4, 5),     # dose = 6,  vol = 9,  conc = 0.666...
    (2, 4, 3, 7),     # dose = 8,  vol = 10, conc = 0.8
    (2, 5, 4, 7),     # dose = 10, vol = 11, conc = 0.909...
    (3, 4, 5, 3),     # dose = 12, vol = 8,  conc = 1.5
    (2, 7, 3, 4),     # dose = 14, vol = 7,  conc = 2.0
    (3, 3, 4, 9),     # dose = 9,  vol = 13, conc = 0.692...
    (2, 8, 3, 2),     # dose = 16, vol = 5,  conc = 3.2
]

# The option family (5 members), all built from the four observed quantities via *, /, and +. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "admix_conc",
        "admixture concentration (the total dose divided by the total volume)",
        "(dose_count * dose_size) / (base_volume + added_volume)",
    ),
    (
        "total_dose",
        "the total dose (the dose count times the dose size, the numerator that is divided by the total volume)",
        "dose_count * dose_size",
    ),
    (
        "total_volume",
        "the total volume (the base volume plus the added volume, the pooled total the dose is divided by)",
        "base_volume + added_volume",
    ),
    (
        "split",
        "the total dose divided by the base volume plus the total dose divided by the added volume, distributing the division across the sum instead of pooling the volumes first (a wrong operation)",
        "(dose_count * dose_size) / base_volume + (dose_count * dose_size) / added_volume",
    ),
    (
        "flat",
        "the total dose divided by the base volume and then the added volume added on, dropping the grouping so the second volume is added instead of pooled into the divisor (a wrong operation)",
        "(dose_count * dose_size) / base_volume + added_volume",
    ),
]
QUERIED = ["admix_conc", "total_dose", "total_volume"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(dose_count, dose_size, base_volume, added_volume):
    # Operation order mirrors the ADJ programs exactly (the multiplication forms the total dose, the addition forms the total volume, then
    # the total dose is divided by the total volume to form the compound figure, so (a*b)/(c+d) evaluates as ((a*b)/(c+d))), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    dose = dose_count * dose_size
    vol = base_volume + added_volume
    return {
        "admix_conc": dose / vol,
        "total_dose": dose,
        "total_volume": vol,
        "split": dose / base_volume + dose / added_volume,
        "flat": dose / base_volume + added_volume,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for dose_count, dose_size, base_volume, added_volume in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only *, /, and + over positive quantities, so positivity
        # is automatic — no positivity guards are needed.
        assert (
            dose_count >= 2
            and dose_size >= 2
            and base_volume >= 2
            and added_volume >= 2
        ), (dose_count, dose_size, base_volume, added_volume)
        fv = family_values(dose_count, dose_size, base_volume, added_volume)
        for key, v in fv.items():
            assert v > 0, (key, dose_count, dose_size, base_volume, added_volume, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    dose_count,
                    dose_size,
                    base_volume,
                    added_volume,
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
                dose_count,
                dose_size,
                base_volume,
                added_volume,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r135aca-{idx + 1:02d}",
                "qtype": "admix_conc",
                "stem": (
                    f"An admixture study records a dose count of {num(dose_count)} aliquots each of dose size "
                    f"{num(dose_size)}, diluted into a base volume of {num(base_volume)} plus an added volume of "
                    f"{num(added_volume)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe dose_count({num(dose_count)})\n"
                    f"observe dose_size({num(dose_size)})\n"
                    f"observe base_volume({num(base_volume)})\n"
                    f"observe added_volume({num(added_volume)})\n"
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
            "ADJ-LADDER rung 135 — admixture concentration from four stated quantities (a NEW panel: admixture, carrying the PRODUCT "
            "numerator onto a SUM denominator). rung-132 put a quotient over a sum (a/b)/(c+d); rung-135 puts a PRODUCT over a sum "
            "(a*b)/(c+d) — and it is the sum-denominator twin of the over-a-rate trio (131 sum, 133 difference, 134 product over (c/d)). "
            "From a total dose (dose_count * dose_size) divided by a total volume (base_volume + added_volume), compute the admixture "
            "concentration ((dose_count*dose_size)/(base_volume+added_volume)), the total dose (dose_count*dose_size), or the total volume "
            "(base_volume+added_volume). Each item is a compute_dimensioned program (observe the four quantities, let answer = formula); "
            "the ADJ engine carries the arithmetic — a NEW family, a PRODUCT NUMERATOR OVER A SUM (a*b)/(c+d) (multiply out the dose, pool "
            "the volume, then divide the dose by the pooled volume — the two-part denominator is ONE total, not two divisors). The "
            "divide-by-a-sum slips ride alongside as distractors. The harness matches the scalar to the printed options. The admixture "
            "concentration is a concentration (how much total dose sits in each unit of pooled volume), framed as a CONCENTRATION so the "
            "dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed quantities via *, /, "
            "and + — no constant leaks, and neither the total dose, the total volume, nor the admixture concentration ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the slips students make: "
            "distributing the division across the sum ((a*b)/c + (a*b)/d, FALSE because x/(c+d) != x/c + x/d, a wrong operation) and "
            "dropping the grouping so the second volume is added on instead of pooled ((a*b)/c + d, a wrong operation). The core confusion "
            "tested is that (a*b)/(c+d) is one product over one pooled total, not (a*b)/c + (a*b)/d and not (a*b)/c + d. This rung uses "
            "only *, /, and + over positive quantities, so every figure is automatically positive — no positivity guards are needed — and "
            "the five family values are kept pairwise distinct with all three queried readouts varying across the panel, all asserted "
            "strictly positive at build time."
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
