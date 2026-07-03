"""Generate rung-46 (daily per-patient drug cost) items.json for the ADJ-LADDER.

Rung 46 opens the **pharmacoeconomics / cost-per-patient-day** panel on the quantitative band — the arithmetic of
what a two-drug regimen costs per patient per day once you spread the combined cost over the patient-days it covers.
The combined cost of the two drugs is `cost_first + cost_second`; expressing it per patient per day divides that sum
by the PRODUCT of the patient count and the day count (the patient-days). This rung introduces a genuinely NEW
arithmetic shape on the ladder: **sum-over-product** — `(a + b) / (c · d)` — a sum in the numerator divided by a
product in the denominator.

The setup: a regimen combines two drugs costing `cost_first` and `cost_second` (dollars) and is given to
`patient_count` patients for `day_count` days. The daily per-patient cost is the combined cost divided by the
patient-days:

  DAILY PER-PATIENT COST   (cost_first + cost_second) / (patient_count · day_count)   [ dollars per patient per day ]
  COMBINED COST            cost_first + cost_second                                    [ the numerator: both drugs ]
  PATIENT-DAYS             patient_count · day_count                                   [ the denominator ]

The **daily per-patient cost** is what makes this rung distinctive — it is the ladder's first **sum-over-product**:
a parenthesised sum divided by a parenthesised product. Contrast the neighbours already on the ladder: rung-42 was a
*sum-over-difference* `(a+b)/(c−d)`, rung-44 a *product-over-sum* `(a·b)/(c+d)`, rung-45 a *difference-over-product*
`(a−b)/(c·d)`; none divided a SUM by a PRODUCT. (The combined cost `cost_first + cost_second` and the patient-days
`patient_count · day_count` ride alongside as the two component quantities, so the panel teaches the whole
calculation — exactly as rung-45 shipped its eliminated amount and weight·time product beside the headline rate.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — including the inner `(cost_first + cost_second)` sum and the `(patient_count ·
day_count)` product — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../44/45. This rung exercises the engine across a **sum divided by a
product**.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `·` and
`/` — **no structural constants** — so no numeric literal appears in any program, and neither the combined cost, the
patient-days, nor any per-patient-day figure is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`cost_first`, `cost_second`, `patient_count`, `day_count`) so
no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic
slips —

  PER PATIENT ONLY    (cost_first + cost_second) / patient_count                       normalise by the patient
                                                                                       count ALONE, forgetting the
                                                                                       days, and
  SUMMED DENOMINATOR  (cost_first + cost_second) / (patient_count + day_count)         ADD the patient count and day
                                                                                       count instead of multiplying
                                                                                       them,

which are exactly the mistakes a student makes (dropping a normaliser, or adding two quantities that should be
multiplied). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness: all four observed quantities are positive, so every sum and product is positive and every denominator
is strictly positive (no division by zero); the tables below are chosen so the five family values are pairwise
distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (COST_FIRST, COST_SECOND, PATIENT_COUNT, DAY_COUNT) — costs in dollars, counts are plain positive integers. All
# four quantities are strictly positive, so every denominator (patient-days) is strictly positive. The five family
# values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (80, 40, 4, 3),
    (60, 30, 3, 5),
    (100, 20, 5, 2),
    (50, 40, 3, 3),
    (90, 30, 4, 5),
    (70, 50, 6, 2),
    (40, 20, 3, 4),
]

# The option family (5 members), all built from the four observed quantities via +, * and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "daily_per_patient_cost",
        "daily per-patient cost (combined cost spread over the patient-days)",
        "(cost_first + cost_second) / (patient_count * day_count)",
    ),
    (
        "combined_cost",
        "the combined cost of both drugs (the two costs added)",
        "cost_first + cost_second",
    ),
    (
        "patient_days",
        "the patient-days (the patient count times the day count)",
        "patient_count * day_count",
    ),
    (
        "per_patient_only",
        "combined cost over the patient count ALONE, forgetting the days (a wrong denominator)",
        "(cost_first + cost_second) / patient_count",
    ),
    (
        "summed_denominator",
        "combined cost over patient count PLUS day count (the two normalisers added instead of multiplied)",
        "(cost_first + cost_second) / (patient_count + day_count)",
    ),
]
QUERIED = ["daily_per_patient_cost", "combined_cost", "patient_days"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(cost_first, cost_second, patient_count, day_count):
    # Operation order mirrors the ADJ programs exactly (sum in the numerator, product in the denominator), so the
    # Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match
    # tolerance).
    combined = cost_first + cost_second
    patient_days = patient_count * day_count
    return {
        "daily_per_patient_cost": combined / patient_days,
        "combined_cost": combined,
        "patient_days": patient_days,
        "per_patient_only": combined / patient_count,
        "summed_denominator": combined / (patient_count + day_count),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for cost_first, cost_second, patient_count, day_count in TABLES:
        assert cost_first > 0 and cost_second > 0 and patient_count > 0 and day_count > 0, (
            cost_first,
            cost_second,
            patient_count,
            day_count,
        )
        fv = family_values(cost_first, cost_second, patient_count, day_count)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    cost_first,
                    cost_second,
                    patient_count,
                    day_count,
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
                cost_first,
                cost_second,
                patient_count,
                day_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r46cpd-{idx + 1:02d}",
                "qtype": "cost_per_patient_day",
                "stem": (
                    f"A regimen combines two drugs costing ${num(cost_first)} and ${num(cost_second)}, given to "
                    f"{num(patient_count)} patients for {num(day_count)} days. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe cost_first({num(cost_first)})\n"
                    f"observe cost_second({num(cost_second)})\n"
                    f"observe patient_count({num(patient_count)})\n"
                    f"observe day_count({num(day_count)})\n"
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
            "ADJ-LADDER rung 46 — daily per-patient drug cost from four stated quantities (a NEW panel: "
            "pharmacoeconomics / cost-per-patient-day). From two drug costs plus a patient count and a day count "
            "compute the daily per-patient cost ((cost_first+cost_second)/(patient_count*day_count)), the combined "
            "cost (cost_first+cost_second), or the patient-days (patient_count*day_count). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW shape, SUM-OVER-PRODUCT (a+b)/(c*d), the first quotient on the ladder to divide a "
            "parenthesised sum by a parenthesised product (distinct from rung-42 sum-over-difference (a+b)/(c-d), "
            "rung-44 product-over-sum (a*b)/(c+d), and rung-45 difference-over-product (a-b)/(c*d)) — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every index is built only from the four "
            "observed quantities via +, * and / — no constant leaks, and neither the combined cost, the patient-days, "
            "nor any per-patient-day figure ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are "
            "a family over the same four quantities, so the distractors are exactly the slips students make: "
            "normalising by the patient count alone (dropping the days), and ADDING the patient count and day count "
            "instead of multiplying them. The core confusion tested is dividing the combined cost by the "
            "patient-days product."
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
