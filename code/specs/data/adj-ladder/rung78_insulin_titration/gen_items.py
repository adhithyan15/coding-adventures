"""Generate rung-78 (endocrinology insulin-titration index) items.json for the ADJ-LADDER.

Rung 78 opens the **endocrinology / insulin-titration** panel on the quantitative band — the arithmetic of a net
insulin-titration dose. A titration forms a `correction component` (a `glucose_excess` divided by a `sensitivity_factor`)
and, independently, an `offset component` (a `carb_load` times a `coverage_gain` already delivered), then SUBTRACTS the
offset from the correction. Two INDEPENDENT binary terms — one a pure quotient, one a pure product — with the product
subtracted from the quotient introduces a genuinely NEW arithmetic shape on the ladder: a **quotient MINUS a product** —
`a/b-c*d`, i.e. `(a/b)-(c*d)`.

This is the deliberate MINUS-counterpart of rung-77's `a/b+c*d` (a quotient plus a product). Like rung-77 — and unlike
rungs 69-74, which chained the `+`/`-` and the `*`/`/` through a SHARED operand — the two sides of the `-` are DISJOINT
two-operand terms: `a/b` uses only the first pair, `c*d` only the second, so the shape is a difference of two
independent binary sub-results. The operation order matters: `a/b-c*d` is `(a/b)-(c*d)` by precedence (divide and
multiply bind before subtract), NOT `a/(b-c)*d` (subtracting into the denominator) and NOT `a*b-c/d` (swapping which
pair divides and which multiplies) — the two distractors exploit exactly those confusions.

The setup: a `glucose_excess`, a `sensitivity_factor`, a `carb_load`, and a `coverage_gain`. The net titration is:

  NET TITRATION         glucose_excess / sensitivity_factor - carb_load * coverage_gain   [ quotient minus product ]
  CORRECTION COMPONENT  glucose_excess / sensitivity_factor                               [ the quotient term ]
  OFFSET COMPONENT      carb_load * coverage_gain                                         [ the product term ]

The **net titration** is what makes this rung distinctive — it is the ladder's first **quotient MINUS a product** (a
difference of two disjoint binary terms). (The correction component `glucose_excess / sensitivity_factor` and the offset
component `carb_load * coverage_gain` ride alongside as component readouts, so the panel teaches the whole calculation —
exactly as rungs 47-77 shipped their component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the division of the glucose excess by the sensitivity factor, the multiplication of the carb
load by the coverage gain, and the subtraction of the two independent terms (multiply/divide before subtract) — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../76/77. This rung exercises the engine across **a quotient minus a product** — the fact that `a/b-c*d` is
`(a/b)-(c*d)` and NOT `a/(b-c)*d` and NOT `a*b-c/d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `*`, and `-`
— **no structural constants** — so no numeric literal appears in any program, and neither the correction component, the
offset component, nor any net figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`glucose_excess`, `sensitivity_factor`, `carb_load`, `coverage_gain`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    glucose_excess / (sensitivity_factor - carb_load) * coverage_gain   SUBTRACT the carb load FROM the
                                                                                 sensitivity factor in the denominator
                                                                                 instead of keeping two independent
                                                                                 terms (the classic `a/b-c*d` vs
                                                                                 `a/(b-c)*d` error), and
  SWAPPED    glucose_excess * sensitivity_factor - carb_load / coverage_gain     MULTIPLY the first pair and DIVIDE the
                                                                                 second — swapping which pair divides
                                                                                 and which multiplies (`a*b-c/d`
                                                                                 instead of `a/b-c*d`),

which are exactly the mistakes a student makes (folding the second operand into the denominator, or swapping the divide
and the multiply between the two pairs). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts;
all five always appear as options.

Distinctness: all four observed quantities are strictly positive; the sensitivity factor exceeds the carb load
(`sensitivity_factor > carb_load`) so the crossed denominator stays positive, and the correction component exceeds the
offset component (`glucose_excess / sensitivity_factor > carb_load * coverage_gain`) so the **headline net titration
stays positive**; the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (GLUCOSE_EXCESS, SENSITIVITY_FACTOR, CARB_LOAD, COVERAGE_GAIN) — a glucose excess to divide, a sensitivity factor to
# divide by, a carb load to multiply, and a coverage gain to scale it by, all plain positive numbers with
# sensitivity_factor > carb_load (so the crossed denominator stays positive) and glucose_excess / sensitivity_factor >
# carb_load * coverage_gain (so the headline net titration stays positive). The five family values are asserted
# pairwise-distinct below.
TABLES = [
    (40, 4, 2, 3),
    (60, 5, 2, 4),
    (36, 6, 1, 5),
    (50, 5, 3, 2),
    (45, 9, 1, 3),
    (48, 4, 2, 4),
    (54, 6, 2, 3),
]

# The option family (5 members), all built from the four observed quantities via /, *, and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "net_titration",
        "net insulin-titration dose (the correction component minus the offset component)",
        "glucose_excess / sensitivity_factor - carb_load * coverage_gain",
    ),
    (
        "correction_component",
        "the correction component (glucose excess over the sensitivity factor)",
        "glucose_excess / sensitivity_factor",
    ),
    (
        "offset_component",
        "the offset component (carb load times the coverage gain)",
        "carb_load * coverage_gain",
    ),
    (
        "crossed",
        "the glucose excess divided by the DIFFERENCE of the sensitivity factor and carb load, then scaled by the coverage gain, not two independent terms (a wrong grouping)",
        "glucose_excess / (sensitivity_factor - carb_load) * coverage_gain",
    ),
    (
        "swapped",
        "the glucose excess MULTIPLIED by the sensitivity factor minus the carb load DIVIDED by the coverage gain, the operations swapped (a wrong grouping)",
        "glucose_excess * sensitivity_factor - carb_load / coverage_gain",
    ),
]
QUERIED = ["net_titration", "correction_component", "offset_component"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(glucose_excess, sensitivity_factor, carb_load, coverage_gain):
    # Operation order mirrors the ADJ programs exactly (the divide and the multiply bind before the subtract, per
    # precedence), so the Python option value and the engine result are the same IEEE-double (well within the harness's
    # 1e-9 match tolerance).
    return {
        "net_titration": glucose_excess / sensitivity_factor - carb_load * coverage_gain,
        "correction_component": glucose_excess / sensitivity_factor,
        "offset_component": carb_load * coverage_gain,
        "crossed": glucose_excess / (sensitivity_factor - carb_load) * coverage_gain,
        "swapped": glucose_excess * sensitivity_factor - carb_load / coverage_gain,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for glucose_excess, sensitivity_factor, carb_load, coverage_gain in TABLES:
        assert (
            glucose_excess > 0
            and sensitivity_factor > 0
            and carb_load > 0
            and coverage_gain > 0
        ), (glucose_excess, sensitivity_factor, carb_load, coverage_gain)
        # Sensitivity factor exceeds the carb load so the crossed denominator stays positive, and the correction
        # component exceeds the offset component so the headline net titration stays positive.
        assert sensitivity_factor > carb_load, (glucose_excess, sensitivity_factor, carb_load, coverage_gain)
        assert glucose_excess / sensitivity_factor > carb_load * coverage_gain, (
            glucose_excess,
            sensitivity_factor,
            carb_load,
            coverage_gain,
        )
        fv = family_values(glucose_excess, sensitivity_factor, carb_load, coverage_gain)
        for key, v in fv.items():
            assert v > 0, (key, glucose_excess, sensitivity_factor, carb_load, coverage_gain, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    glucose_excess,
                    sensitivity_factor,
                    carb_load,
                    coverage_gain,
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
                glucose_excess,
                sensitivity_factor,
                carb_load,
                coverage_gain,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r78ins-{idx + 1:02d}",
                "qtype": "insulin_titration",
                "stem": (
                    f"An insulin titration records a glucose excess of {num(glucose_excess)}, a sensitivity factor of "
                    f"{num(sensitivity_factor)} to divide by, a carb load of {num(carb_load)} and a coverage gain of "
                    f"{num(coverage_gain)} to scale it by. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe glucose_excess({num(glucose_excess)})\n"
                    f"observe sensitivity_factor({num(sensitivity_factor)})\n"
                    f"observe carb_load({num(carb_load)})\n"
                    f"observe coverage_gain({num(coverage_gain)})\n"
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
            "ADJ-LADDER rung 78 — endocrinology insulin-titration index from four stated quantities (a NEW panel: "
            "endocrinology / insulin-titration). From a glucose excess to divide, a sensitivity factor to divide by, a "
            "carb load to multiply, and a coverage gain to scale by, compute the net titration "
            "(glucose_excess/sensitivity_factor - carb_load*coverage_gain), the correction component "
            "(glucose_excess/sensitivity_factor), or the offset component (carb_load*coverage_gain). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW shape, QUOTIENT MINUS A PRODUCT a/b-c*d (two INDEPENDENT binary terms — a pure "
            "quotient and a pure product — with the product subtracted from the quotient, divide/multiply before "
            "subtract; the minus-counterpart of rung-77 a/b+c*d; contrast rungs 69-74 which chained the +/- and */÷ "
            "through a SHARED operand; here the two sides of the - are disjoint 2-operand terms, so a/b-c*d = "
            "(a/b)-(c*d), not a/(b-c)*d and not a*b-c/d) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via /, *, and - — no "
            "constant leaks, and neither the correction component, the offset component, nor any net figure ever "
            "appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. The five options are a family over the same four quantities, so the "
            "distractors are exactly the slips students make: SUBTRACTING the carb load INTO the denominator "
            "(a/(b-c)*d, a wrong grouping) and SWAPPING the multiply and divide between the two pairs (a*b-c/d, a wrong "
            "grouping). The core confusion tested is that a/b-c*d is (a/b)-(c*d), not a/(b-c)*d and not a*b-c/d."
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
