"""Generate rung-54 (leukocyte-differential neutrophil ratio) items.json for the ADJ-LADDER.

Rung 54 opens the **hematology / leukocyte-differential** panel on the quantitative band — the arithmetic of comparing
one white-cell line against the rest. A differential count breaks the leukocytes into lines; the neutrophil-to-other
ratio weighs the neutrophils against the COMBINED count of the other three lines (lymphocytes, monocytes, eosinophils).
Dividing one quantity by a sum of THREE others introduces a genuinely NEW arithmetic shape on the ladder: **one over a
sum-of-three** — `a / (b + c + d)` — a single numerator over a three-term denominator sum. This is the inverse
arrangement of rung-53's sum-of-three-over-one `(a+b+c)/d`: there the three-term sum was the numerator, here it is the
denominator.

The setup: a differential reports `neutrophil_count` neutrophils, `lymphocyte_count` lymphocytes, `monocyte_count`
monocytes and `eosinophil_count` eosinophils. The neutrophil ratio is the neutrophils over the sum of the other three:

  NEUTROPHIL RATIO   neutrophil_count / (lymphocyte_count + monocyte_count + eosinophil_count)   [ neutrophils per other ]
  OTHER LEUKOCYTES   lymphocyte_count + monocyte_count + eosinophil_count                        [ the denominator sum ]
  LYMPH + MONO       lymphocyte_count + monocyte_count                                           [ one partial sum ]

The **neutrophil ratio** is what makes this rung distinctive — it is the ladder's first **one-over-a-sum-of-three**: a
single numerator divided by a parenthesised three-term sum. (The other-leukocyte total `lymphocyte_count +
monocyte_count + eosinophil_count` and the lymph-plus-mono partial sum ride alongside as component readouts, so the
panel teaches the whole calculation — exactly as rungs 47-53 shipped their component sums/products/differences beside
the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including the parenthesised `(lymphocyte_count + monocyte_count + eosinophil_count)`
denominator under the numerator — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../52/53. This rung exercises the engine across a **single quantity over a
three-term sum** — the one-over-a-total made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `/` —
**no structural constants** — so no numeric literal appears in any program, and neither the other-leukocyte total, the
lymph-plus-mono partial sum, nor any neutrophil-ratio figure is ever a literal (each is computed from the observed
quantities). The observed quantities carry **digit-free identifiers** (`neutrophil_count`, `lymphocyte_count`,
`monocyte_count`, `eosinophil_count`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  FRACTION OF TOTAL   neutrophil_count / (neutrophil_count + lymphocyte_count + monocyte_count + eosinophil_count)
                                                                                 fold the neutrophils INTO the total and
                                                                                 divide by everything (a fraction of the
                                                                                 whole, not a ratio to the rest), and
  MISGROUPED          neutrophil_count / lymphocyte_count + monocyte_count + eosinophil_count
                                                                                 divide only by the lymphocytes,
                                                                                 forgetting to group the three-term
                                                                                 denominator (`/ lymphocyte_count +
                                                                                 …`, not `/ (lymphocyte_count + …)`),

which are exactly the mistakes a student makes (dividing by the grand total instead of the other lines, or breaking the
grouping so only the first term is a divisor). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness: all four observed quantities are strictly positive, so every sum, quotient and partial is positive; the
tables below are chosen so the five family values are pairwise distinct with a comfortable margin, asserted at build
time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (NEUTROPHIL_COUNT, LYMPHOCYTE_COUNT, MONOCYTE_COUNT, EOSINOPHIL_COUNT) — leukocyte-differential counts, all plain
# positive numbers. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (60, 25, 8, 3),
    (72, 18, 6, 4),
    (55, 30, 10, 5),
    (66, 20, 9, 2),
    (48, 35, 12, 6),
    (80, 12, 5, 3),
    (63, 22, 7, 4),
]

# The option family (5 members), all built from the four observed quantities via + and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "neutrophil_ratio",
        "neutrophil ratio (the neutrophils over the sum of the other three lines)",
        "neutrophil_count / (lymphocyte_count + monocyte_count + eosinophil_count)",
    ),
    (
        "other_leukocytes",
        "the other-leukocyte total (lymphocytes plus monocytes plus eosinophils)",
        "lymphocyte_count + monocyte_count + eosinophil_count",
    ),
    (
        "lymph_plus_mono",
        "the lymphocyte-plus-monocyte count (one partial sum, before adding eosinophils)",
        "lymphocyte_count + monocyte_count",
    ),
    (
        "fraction_of_total",
        "neutrophils over the GRAND total including themselves, not the ratio to the rest",
        "neutrophil_count / (neutrophil_count + lymphocyte_count + monocyte_count + eosinophil_count)",
    ),
    (
        "misgrouped",
        "neutrophils divided by only the lymphocytes, forgetting to group the three-term denominator",
        "neutrophil_count / lymphocyte_count + monocyte_count + eosinophil_count",
    ),
]
QUERIED = ["neutrophil_ratio", "other_leukocytes", "lymph_plus_mono"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(neutrophil_count, lymphocyte_count, monocyte_count, eosinophil_count):
    # Operation order mirrors the ADJ programs exactly (a single numerator over a parenthesised three-term sum; and,
    # for the misgrouped slip, division by only the first term binds tighter than the additions), so the Python option
    # value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    other = lymphocyte_count + monocyte_count + eosinophil_count
    return {
        "neutrophil_ratio": neutrophil_count / other,
        "other_leukocytes": other,
        "lymph_plus_mono": lymphocyte_count + monocyte_count,
        "fraction_of_total": neutrophil_count / (neutrophil_count + other),
        "misgrouped": neutrophil_count / lymphocyte_count + monocyte_count + eosinophil_count,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for neutrophil_count, lymphocyte_count, monocyte_count, eosinophil_count in TABLES:
        assert (
            neutrophil_count > 0
            and lymphocyte_count > 0
            and monocyte_count > 0
            and eosinophil_count > 0
        ), (neutrophil_count, lymphocyte_count, monocyte_count, eosinophil_count)
        fv = family_values(neutrophil_count, lymphocyte_count, monocyte_count, eosinophil_count)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    neutrophil_count,
                    lymphocyte_count,
                    monocyte_count,
                    eosinophil_count,
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
                neutrophil_count,
                lymphocyte_count,
                monocyte_count,
                eosinophil_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r54neut-{idx + 1:02d}",
                "qtype": "neutrophil_ratio",
                "stem": (
                    f"A differential reports {num(neutrophil_count)} neutrophils, {num(lymphocyte_count)} lymphocytes, "
                    f"{num(monocyte_count)} monocytes and {num(eosinophil_count)} eosinophils. What is the "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe neutrophil_count({num(neutrophil_count)})\n"
                    f"observe lymphocyte_count({num(lymphocyte_count)})\n"
                    f"observe monocyte_count({num(monocyte_count)})\n"
                    f"observe eosinophil_count({num(eosinophil_count)})\n"
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
            "ADJ-LADDER rung 54 — leukocyte-differential neutrophil ratio from four stated quantities (a NEW panel: "
            "hematology / leukocyte-differential). From neutrophil, lymphocyte, monocyte and eosinophil counts compute "
            "the neutrophil ratio (neutrophil_count/(lymphocyte_count+monocyte_count+eosinophil_count)), the "
            "other-leukocyte total (lymphocyte_count+monocyte_count+eosinophil_count), or the lymph-plus-mono partial "
            "sum (lymphocyte_count+monocyte_count). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, ONE OVER A "
            "SUM-OF-THREE a/(b+c+d), the first on the ladder to divide a single quantity by a parenthesised three-term "
            "sum (the inverse of rung-53 sum-of-three-over-one (a+b+c)/d, and distinct from rung-37 ratio-of-two-sums "
            "(a+b)/(c+d)) — and the harness matches the scalar to the printed options. Contamination-safe: every index "
            "is built only from the four observed quantities via + and / — no constant leaks, and neither the "
            "other-leukocyte total, the lymph-plus-mono partial sum, nor any neutrophil-ratio figure ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same four quantities, so the distractors "
            "are exactly the slips students make: dividing by the GRAND total (folding the neutrophils in) instead of "
            "the other lines, and breaking the grouping so only the first term is a divisor (a/b+c+d, not a/(b+c+d)). "
            "The core confusion tested is dividing one quantity by a grouped three-term sum."
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
