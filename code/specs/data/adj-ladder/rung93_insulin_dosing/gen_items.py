"""Generate rung-93 (endocrinology / insulin-dosing total) items.json for the ADJ-LADDER.

Rung 93 opens the **endocrinology / insulin-dosing** panel on the quantitative band — the arithmetic of a total insulin
dose. A `basal_rate` times `active_hours` gives the basal infusion load, and a `correction_units` and a `carb_units` are
ADDED as two bolus terms, all three ADDing into the total. A **product at the FRONT of two added terms** introduces a
genuinely NEW arithmetic family on the ladder: `a*b+c+d`, i.e. `(((a*b)+c)+d)`.

This is genuinely new and COMPLETES the additive-chain-plus-one-product family. Rung 91 shipped `a+b+c*d` (product
LAST) and rung 92 shipped `a+b*c+d` (product in the MIDDLE). Rung 93 puts the single product at the **FRONT**:
`a*b+c+d` — the product forms first, then two bare terms are added to its right. Together the three rungs exhaust the
three positions a lone product can take in a flat four-term add-chain (last / middle / first). No prior shape led with a
product and then added TWO independent bare terms in one flat chain — rung-79 `a*b+c/d`/rung-80 `a*b-c/d` attached ONE
quotient to the leading product, rung-85 `a*b-c-d` SUBTRACTED two bare terms from it. `a*b+c+d` is the ladder's first
**leading-product-plus-two-added-terms**. The operator order matters: `a*b+c+d` is `(((a*b)+c)+d)` (the product forms
first by precedence, then the flat sum), NOT `a*(b+c)+d` (folding the first added term into the product) and NOT
`a+b*c+d` (multiplying the WRONG pair and adding the basal rate bare) — the two distractors exploit exactly those
confusions.

The setup: a `basal_rate`, `active_hours`, `correction_units`, and `carb_units`. The total is:

  TOTAL INSULIN     basal_rate * active_hours + correction_units + carb_units  [ a product at the front plus two added terms ]
  INFUSION LOAD     basal_rate * active_hours                                  [ the basal product, before the boluses ]
  BOLUS SUM         correction_units + carb_units                             [ the two bolus terms, before the product ]

The **total insulin** is what makes this rung distinctive — it is the ladder's first
**leading-product-plus-two-added-terms**. (The infusion load `a*b` and the bolus sum `c+d` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-92 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the basal rate by the active hours into the infusion load, then the flat
addition of that product, the correction units, and the carb units (the product forming before the sum, so a*b+c+d
evaluates as (((a*b)+c)+d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../91/92. This rung exercises the engine across a
**leading-product-plus-two-added-terms** — the fact that `a*b+c+d` is `(((a*b)+c)+d)` and NOT `a*(b+c)+d` and NOT
`a+b*c+d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*` and `+` — **no
structural constants** — so no numeric literal appears in any program, and neither the infusion load, the bolus sum, nor
any total figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`basal_rate`, `active_hours`, `correction_units`, `carb_units`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    basal_rate * (active_hours + correction_units) + carb_units  fold the FIRST added term (correction) into the
                                                                          product instead of leaving it added (the classic
                                                                          `a*b+c+d` vs `a*(b+c)+d` error), and
  SWAPPED    basal_rate + active_hours * correction_units + carb_units    multiply the WRONG pair (active_hours × correction)
                                                                          and add the basal rate bare (`a+b*c+d` instead of
                                                                          `a*b+c+d`),

which are exactly the mistakes a student makes (folding a neighbouring term into the product, or multiplying the wrong
adjacent pair). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: every quantity is a plain positive number >= 2, so every family member — a sum of positive
terms and positive products — is automatically strictly positive; the tables are chosen so the five family values are
pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BASAL_RATE, ACTIVE_HOURS, CORRECTION_UNITS, CARB_UNITS) — a basal rate times active hours for the infusion load, and a
# correction units and a carb units to add as two bolus terms, all plain positive numbers >= 2. Every family member is a
# sum of positive terms / positive products, so positivity is automatic; the five family values are asserted
# pairwise-distinct below (the tables avoid basal_rate == correction_units, which would collide the total with the
# swapped slip, and basal_rate*active_hours == correction_units+carb_units, which would collide the infusion load with
# the bolus sum).
TABLES = [
    (3, 2, 5, 4),
    (2, 5, 3, 4),
    (3, 3, 2, 4),
    (2, 4, 6, 3),
    (3, 4, 2, 5),
    (5, 2, 3, 4),
    (2, 6, 3, 2),
]

# The option family (5 members), all built from the four observed quantities via * and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "total_insulin",
        "total insulin delivered (the basal infusion load plus the correction and carb boluses)",
        "basal_rate * active_hours + correction_units + carb_units",
    ),
    (
        "infusion_load",
        "the basal infusion load (the basal rate times the active hours, before adding the boluses)",
        "basal_rate * active_hours",
    ),
    (
        "bolus_sum",
        "the bolus sum (the correction units plus the carb units, the two boluses before adding the infusion load)",
        "correction_units + carb_units",
    ),
    (
        "crossed",
        "the basal rate times the active hours and correction units together, plus the carb units, folding the first bolus into the product instead of leaving it added (a wrong grouping)",
        "basal_rate * (active_hours + correction_units) + carb_units",
    ),
    (
        "swapped",
        "the basal rate plus the active hours times the correction units, plus the carb units, multiplying the wrong pair (a wrong pairing)",
        "basal_rate + active_hours * correction_units + carb_units",
    ),
]
QUERIED = ["total_insulin", "infusion_load", "bolus_sum"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(basal_rate, active_hours, correction_units, carb_units):
    # Operation order mirrors the ADJ programs exactly (the product forms first by precedence, then the flat sum, so
    # a*b+c+d evaluates as (((a*b)+c)+d)), so the Python option value and the engine result are the same IEEE-double
    # (well within the harness's 1e-9 match tolerance).
    return {
        "total_insulin": basal_rate * active_hours + correction_units + carb_units,
        "infusion_load": basal_rate * active_hours,
        "bolus_sum": correction_units + carb_units,
        "crossed": basal_rate * (active_hours + correction_units) + carb_units,
        "swapped": basal_rate + active_hours * correction_units + carb_units,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for basal_rate, active_hours, correction_units, carb_units in TABLES:
        assert (
            basal_rate > 0
            and active_hours > 0
            and correction_units > 0
            and carb_units > 0
        ), (basal_rate, active_hours, correction_units, carb_units)
        fv = family_values(basal_rate, active_hours, correction_units, carb_units)
        # Every family member is a sum of positive terms / positive products, so every value is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, basal_rate, active_hours, correction_units, carb_units, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    basal_rate,
                    active_hours,
                    correction_units,
                    carb_units,
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
                basal_rate,
                active_hours,
                correction_units,
                carb_units,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r93insulin-{idx + 1:02d}",
                "qtype": "insulin_dosing_total",
                "stem": (
                    f"An insulin order records a basal rate of {num(basal_rate)} times active hours of "
                    f"{num(active_hours)}, plus correction units of {num(correction_units)} plus carb units of "
                    f"{num(carb_units)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe basal_rate({num(basal_rate)})\n"
                    f"observe active_hours({num(active_hours)})\n"
                    f"observe correction_units({num(correction_units)})\n"
                    f"observe carb_units({num(carb_units)})\n"
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
            "ADJ-LADDER rung 93 — insulin-dosing total from four stated quantities (a NEW panel: endocrinology / "
            "insulin-dosing). From a basal rate times active hours for the infusion load and a correction units and a "
            "carb units to add as two bolus terms, compute the total insulin "
            "(basal_rate*active_hours+correction_units+carb_units), the infusion load (basal_rate*active_hours), or the "
            "bolus sum (correction_units+carb_units). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A LEADING PRODUCT "
            "PLUS TWO ADDED TERMS a*b+c+d (multiply a by b, then add c and d, so a*b+c+d = (((a*b)+c)+d); this COMPLETES "
            "the additive-chain-plus-one-product family — rung-91 a+b+c*d put the product LAST, rung-92 a+b*c+d put it in "
            "the MIDDLE, rung-93 puts it at the FRONT — and no prior shape led with a product and then added TWO "
            "independent bare terms in one flat chain, e.g. rung-79 a*b+c/d attached one quotient, rung-85 a*b-c-d "
            "subtracted two bare terms) — and the harness matches the scalar to the printed options. Contamination-safe: "
            "every figure is built only from the four observed quantities via * and + — no constant leaks, and neither the "
            "infusion load, the bolus sum, nor any total figure ever appears as a literal (each is computed) — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options "
            "are a family over the same four quantities, so the distractors are exactly the slips students make: folding "
            "the first bolus into the product (a*(b+c)+d, a wrong grouping) and multiplying the wrong pair with the basal "
            "rate added bare (a+b*c+d, a wrong pairing). The core confusion tested is that a*b+c+d is (((a*b)+c)+d), not "
            "a*(b+c)+d and not a+b*c+d."
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
