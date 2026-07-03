"""Generate rung-53 (enteral-feed caloric density) items.json for the ADJ-LADDER.

Rung 53 opens the **nutrition / enteral-feeding** panel on the quantitative band — the arithmetic of how many calories a
tube feed delivers per kilogram of the patient. A feed's calories come from three macronutrients — carbohydrate,
protein and fat — and the caloric density per kilogram is their TOTAL divided by the patient's weight. Dividing a
parenthesised sum of THREE terms by a fourth quantity introduces a genuinely NEW arithmetic shape on the ladder: a
**sum-of-three over one** — `(a + b + c) / d` — three quantities added, then the total divided by a fourth.

The setup: a feed supplies `carb_calories` from carbohydrate, `protein_calories` from protein and `fat_calories` from
fat, for a patient weighing `body_weight`. The caloric density is the total calories over the weight:

  CALORIC DENSITY     (carb_calories + protein_calories + fat_calories) / body_weight   [ kcal/kg — the whole figure ]
  TOTAL CALORIES      carb_calories + protein_calories + fat_calories                   [ the numerator: all calories ]
  CARB + PROTEIN      carb_calories + protein_calories                                  [ one partial sum ]

The **caloric density** is what makes this rung distinctive — it is the ladder's first **sum-of-three over one**: a
three-term sum divided by a fourth quantity. Contrast the neighbours already on the ladder: rung-37 was a *ratio of two
SUMS* `(a+b)/(c+d)` and rung-43 a *sum of three PRODUCTS*; neither divided a three-term sum by a single quantity. (The
total calories `carb_calories + protein_calories + fat_calories` and the carb-plus-protein partial sum ride alongside
as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-52 shipped their component
sums/products/differences beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including the parenthesised `(carb_calories + protein_calories + fat_calories)` numerator over
`body_weight` — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine
change, exactly as rungs 8/16/.../51/52. This rung exercises the engine across a **three-term sum divided by a single
quantity** — the whole-over-one made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `/` —
**no structural constants** — so no numeric literal appears in any program, and neither the total calories, the
carb-plus-protein partial sum, nor any caloric-density figure is ever a literal (each is computed from the observed
quantities). The observed quantities carry **digit-free identifiers** (`carb_calories`, `protein_calories`,
`fat_calories`, `body_weight`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SUM ALL       carb_calories + protein_calories + fat_calories + body_weight   ADD the weight into the calorie total
                                                                                instead of DIVIDING by it, and
  MISGROUPED    carb_calories + protein_calories + fat_calories / body_weight    divide ONLY the fat term (forgetting to
                                                                                group the three-term numerator, so
                                                                                `+ fat_calories / body_weight`, not
                                                                                `(… + fat_calories) / body_weight`),

which are exactly the mistakes a student makes (folding the divisor into the sum, or breaking the grouping so only the
last term is divided). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always
appear as options.

Distinctness: all four observed quantities are strictly positive, so every sum, quotient and partial is positive; the
tables below are chosen so the five family values are pairwise distinct with a comfortable margin, asserted at build
time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CARB_CALORIES, PROTEIN_CALORIES, FAT_CALORIES, BODY_WEIGHT) — three macronutrient calorie contributions plus the
# patient's weight, all plain positive numbers. The five family values are asserted pairwise-distinct (with margin)
# below.
TABLES = [
    (200, 100, 120, 20),
    (300, 150, 200, 25),
    (240, 160, 180, 40),
    (180, 90, 150, 30),
    (400, 200, 300, 45),
    (150, 120, 90, 18),
    (360, 140, 220, 36),
]

# The option family (5 members), all built from the four observed quantities via + and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "caloric_density",
        "caloric density (the total calories over the patient's weight)",
        "(carb_calories + protein_calories + fat_calories) / body_weight",
    ),
    (
        "total_calories",
        "the total calories (carbohydrate plus protein plus fat)",
        "carb_calories + protein_calories + fat_calories",
    ),
    (
        "carb_plus_protein",
        "the carbohydrate-plus-protein calories (one partial sum, before adding fat)",
        "carb_calories + protein_calories",
    ),
    (
        "sum_all",
        "the weight ADDED into the calorie total instead of dividing by it (a wrong caloric density)",
        "carb_calories + protein_calories + fat_calories + body_weight",
    ),
    (
        "misgrouped",
        "only the fat calories divided by the weight, forgetting to group the three-term numerator",
        "carb_calories + protein_calories + fat_calories / body_weight",
    ),
]
QUERIED = ["caloric_density", "total_calories", "carb_plus_protein"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(carb_calories, protein_calories, fat_calories, body_weight):
    # Operation order mirrors the ADJ programs exactly (parenthesised three-term sum over the weight; and, for the
    # misgrouped slip, division of only the fat term binds tighter than the additions), so the Python option value and
    # the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    total = carb_calories + protein_calories + fat_calories
    return {
        "caloric_density": total / body_weight,
        "total_calories": total,
        "carb_plus_protein": carb_calories + protein_calories,
        "sum_all": total + body_weight,
        "misgrouped": carb_calories + protein_calories + fat_calories / body_weight,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for carb_calories, protein_calories, fat_calories, body_weight in TABLES:
        assert (
            carb_calories > 0
            and protein_calories > 0
            and fat_calories > 0
            and body_weight > 0
        ), (carb_calories, protein_calories, fat_calories, body_weight)
        fv = family_values(carb_calories, protein_calories, fat_calories, body_weight)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    carb_calories,
                    protein_calories,
                    fat_calories,
                    body_weight,
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
                carb_calories,
                protein_calories,
                fat_calories,
                body_weight,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r53cal-{idx + 1:02d}",
                "qtype": "caloric_density",
                "stem": (
                    f"A tube feed supplies {num(carb_calories)} kcal from carbohydrate, {num(protein_calories)} kcal "
                    f"from protein and {num(fat_calories)} kcal from fat, for a patient weighing {num(body_weight)} kg. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe carb_calories({num(carb_calories)})\n"
                    f"observe protein_calories({num(protein_calories)})\n"
                    f"observe fat_calories({num(fat_calories)})\n"
                    f"observe body_weight({num(body_weight)})\n"
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
            "ADJ-LADDER rung 53 — enteral-feed caloric density from four stated quantities (a NEW panel: nutrition / "
            "enteral-feeding). From carbohydrate, protein and fat calories plus a body weight compute the caloric "
            "density ((carb_calories+protein_calories+fat_calories)/body_weight), the total calories "
            "(carb_calories+protein_calories+fat_calories), or the carb-plus-protein partial sum "
            "(carb_calories+protein_calories). Each item is a compute_dimensioned program (observe the four quantities, "
            "let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, SUM-OF-THREE OVER ONE "
            "(a+b+c)/d, the first on the ladder to divide a three-term sum by a single quantity (distinct from rung-37 "
            "ratio-of-two-sums (a+b)/(c+d) and rung-43 sum-of-three-products) — and the harness matches the scalar to "
            "the printed options. Contamination-safe: every index is built only from the four observed quantities via + "
            "and / — no constant leaks, and neither the total calories, the carb-plus-protein partial sum, nor any "
            "caloric-density figure ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the "
            "same four quantities, so the distractors are exactly the slips students make: ADDING the weight into the "
            "calorie total instead of dividing by it, and breaking the grouping so only the fat term is divided "
            "(a+b+c/d, not (a+b+c)/d). The core confusion tested is dividing a grouped three-term sum by a single "
            "quantity."
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
