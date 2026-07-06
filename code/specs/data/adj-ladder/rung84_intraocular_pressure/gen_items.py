"""Generate rung-84 (ophthalmology intraocular-pressure index) items.json for the ADJ-LADDER.

Rung 84 opens the **ophthalmology / intraocular-pressure** panel on the quantitative band — the arithmetic of a net
intraocular pressure. An `aqueous_inflow` and a `venous_bias` are SUMMED (the two pressure sources feeding the globe),
that sum is scaled by a `resistance_factor` (the outflow resistance the pressure builds against), and an `outflow_drain`
is SUBTRACTED (the trabecular egress). A grouped SUM times a factor, minus a fourth term, introduces a genuinely NEW
arithmetic shape on the ladder: a **grouped-sum product MINUS a term** — `(a+b)*c-d`, i.e. `((a+b)*c) - d`.

This is genuinely new: rung-68 was `(a+b)*c/d` (`((a+b)*c)/d`, the grouped-sum product DIVIDED by the fourth term), and
rung-81 was `(a+b)/c-d` (the grouped sum DIVIDED by c then minus d); here the grouped-sum product `(a+b)*c` has the
fourth term SUBTRACTED from it. The operator order matters: `(a+b)*c-d` is `((a+b)*c) - d` by precedence (the
parenthesised sum is formed first, multiplied by `c`, and only then is `d` subtracted), NOT `(a+b)*(c-d)` (subtracting
`d` from `c` INSIDE the second factor) and NOT `(a-b)*c-d` (a MINUS inside the group instead of a plus) — the two
distractors exploit exactly those confusions (and `(a-b)*c-d` is the same *outer* shape with the group's operator
flipped, the classic wrong-grouping slip).

The setup: an `aqueous_inflow`, a `venous_bias`, a `resistance_factor`, and an `outflow_drain`. The intraocular pressure
is:

  INTRAOCULAR PRESSURE   (aqueous_inflow + venous_bias) * resistance_factor - outflow_drain   [ grouped-sum product minus a term ]
  GROSS PRESSURE         (aqueous_inflow + venous_bias) * resistance_factor                   [ the grouped-sum product, before draining ]
  INFLOW SUM             aqueous_inflow + venous_bias                                          [ the summed inflow, before scaling ]

The **intraocular pressure** is what makes this rung distinctive — it is the ladder's first **grouped-sum product MINUS
a term**. (The gross pressure `(a+b)*c` and the inflow sum `a+b` ride alongside as component readouts, so the panel
teaches the whole calculation — exactly as rungs 47-83 shipped their component sums/products/differences/ratios beside
the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the summation of the aqueous inflow and the venous bias, the multiplication of that sum by the
resistance factor, and the subtraction of the outflow drain (the parenthesised sum before the multiply, the multiply
before the subtract) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../82/83. This rung exercises the engine across **a grouped-sum product
minus a term** — the fact that `(a+b)*c-d` is `((a+b)*c) - d` and NOT `(a+b)*(c-d)` and NOT `(a-b)*c-d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `*`, and `-`
— **no structural constants** — so no numeric literal appears in any program, and neither the gross pressure, the inflow
sum, nor any pressure figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`aqueous_inflow`, `venous_bias`, `resistance_factor`, `outflow_drain`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (aqueous_inflow + venous_bias) * (resistance_factor - outflow_drain)   subtract the outflow drain from the
                                                                                    resistance factor INSIDE the second
                                                                                    factor, instead of after the product
                                                                                    (the classic `(a+b)*c-d` vs
                                                                                    `(a+b)*(c-d)` error), and
  SWAPPED    (aqueous_inflow - venous_bias) * resistance_factor - outflow_drain     SUBTRACT the venous bias from the
                                                                                    aqueous inflow inside the group
                                                                                    instead of adding it (`(a-b)*c-d`
                                                                                    instead of `(a+b)*c-d`, the group's
                                                                                    operator flipped),

which are exactly the mistakes a student makes (folding the final subtraction into the second factor, or mis-signing the
grouped sum). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: the tables keep the guards — `aqueous_inflow > venous_bias` (so the swapped group
`(a-b)` stays positive), `resistance_factor > outflow_drain` (so the crossed factor `(c-d)` stays positive, and by at
least two so the crossed value never collapses onto the inflow sum), `(aqueous_inflow+venous_bias)*resistance_factor >
outflow_drain` (intraocular pressure positive), and `(aqueous_inflow-venous_bias)*resistance_factor > outflow_drain`
(swapped positive) — so every family member, including the headline intraocular pressure `(a+b)*c-d`, is strictly
positive; the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (AQUEOUS_INFLOW, VENOUS_BIAS, RESISTANCE_FACTOR, OUTFLOW_DRAIN) — an aqueous inflow and a venous bias to sum, a
# resistance factor to scale that sum by, and an outflow drain to subtract from the scaled product, all plain positive
# numbers. The tables satisfy the guards: aqueous_inflow > venous_bias (swapped group > 0), resistance_factor >
# outflow_drain with a margin of at least two (crossed factor > 0 and crossed != inflow sum),
# (aqueous_inflow+venous_bias)*resistance_factor > outflow_drain (intraocular pressure > 0), and
# (aqueous_inflow-venous_bias)*resistance_factor > outflow_drain (swapped > 0). The five family values are asserted
# pairwise-distinct below.
TABLES = [
    (6, 2, 5, 3),
    (8, 2, 6, 4),
    (5, 3, 7, 4),
    (9, 3, 6, 4),
    (10, 4, 7, 5),
    (7, 5, 8, 6),
    (8, 6, 9, 5),
]

# The option family (5 members), all built from the four observed quantities via +, *, and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "intraocular_pressure",
        "net intraocular pressure (the scaled inflow sum minus the outflow drain)",
        "(aqueous_inflow + venous_bias) * resistance_factor - outflow_drain",
    ),
    (
        "gross_pressure",
        "the gross pressure (the summed inflow times the resistance factor, before draining)",
        "(aqueous_inflow + venous_bias) * resistance_factor",
    ),
    (
        "inflow_sum",
        "the inflow sum (the aqueous inflow plus the venous bias)",
        "aqueous_inflow + venous_bias",
    ),
    (
        "crossed",
        "the summed inflow scaled by the resistance factor MINUS the outflow drain, with the drain taken off the resistance factor inside the second factor instead of off the product (a wrong grouping)",
        "(aqueous_inflow + venous_bias) * (resistance_factor - outflow_drain)",
    ),
    (
        "swapped",
        "the aqueous inflow MINUS the venous bias, scaled by the resistance factor then the outflow drain subtracted, the grouped sum's operator flipped (a wrong grouping)",
        "(aqueous_inflow - venous_bias) * resistance_factor - outflow_drain",
    ),
]
QUERIED = ["intraocular_pressure", "gross_pressure", "inflow_sum"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(aqueous_inflow, venous_bias, resistance_factor, outflow_drain):
    # Operation order mirrors the ADJ programs exactly (the parenthesised sum is formed first, multiplied by the
    # resistance factor, and only then the outflow drain subtracted, per precedence), so the Python option value and the
    # engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "intraocular_pressure": (aqueous_inflow + venous_bias) * resistance_factor - outflow_drain,
        "gross_pressure": (aqueous_inflow + venous_bias) * resistance_factor,
        "inflow_sum": aqueous_inflow + venous_bias,
        "crossed": (aqueous_inflow + venous_bias) * (resistance_factor - outflow_drain),
        "swapped": (aqueous_inflow - venous_bias) * resistance_factor - outflow_drain,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for aqueous_inflow, venous_bias, resistance_factor, outflow_drain in TABLES:
        assert (
            aqueous_inflow > 0
            and venous_bias > 0
            and resistance_factor > 0
            and outflow_drain > 0
        ), (aqueous_inflow, venous_bias, resistance_factor, outflow_drain)
        fv = family_values(aqueous_inflow, venous_bias, resistance_factor, outflow_drain)
        # The tables satisfy the guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, aqueous_inflow, venous_bias, resistance_factor, outflow_drain, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    aqueous_inflow,
                    venous_bias,
                    resistance_factor,
                    outflow_drain,
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
                aqueous_inflow,
                venous_bias,
                resistance_factor,
                outflow_drain,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r84iop-{idx + 1:02d}",
                "qtype": "intraocular_pressure_index",
                "stem": (
                    f"An eye is fed by an aqueous inflow of {num(aqueous_inflow)} plus a venous bias of "
                    f"{num(venous_bias)}, this summed inflow building against a resistance factor of "
                    f"{num(resistance_factor)}, with an outflow drain of {num(outflow_drain)} carried off. What is the "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe aqueous_inflow({num(aqueous_inflow)})\n"
                    f"observe venous_bias({num(venous_bias)})\n"
                    f"observe resistance_factor({num(resistance_factor)})\n"
                    f"observe outflow_drain({num(outflow_drain)})\n"
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
            "ADJ-LADDER rung 84 — ophthalmology intraocular-pressure index from four stated quantities (a NEW panel: "
            "ophthalmology / intraocular-pressure). From an aqueous inflow and a venous bias to sum, a resistance factor "
            "to scale that sum by, and an outflow drain to subtract, compute the intraocular pressure "
            "((aqueous_inflow+venous_bias)*resistance_factor-outflow_drain), the gross pressure "
            "((aqueous_inflow+venous_bias)*resistance_factor), or the inflow sum (aqueous_inflow+venous_bias). Each item "
            "is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine "
            "carries the arithmetic — a NEW shape, GROUPED-SUM PRODUCT MINUS A TERM (a+b)*c-d (sum a and b, times c, "
            "subtract d, so (a+b)*c-d = ((a+b)*c)-d; distinct from rung-68 (a+b)*c/d = ((a+b)*c)/d and from rung-81 "
            "(a+b)/c-d = ((a+b)/c)-d) — and the harness matches the scalar to the printed options. Contamination-safe: "
            "every index is built only from the four observed quantities via +, *, and - — no constant leaks, and "
            "neither the gross pressure, the inflow sum, nor any pressure figure ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make: taking the outflow drain off the resistance factor inside the second factor "
            "((a+b)*(c-d), a wrong grouping) and mis-signing the grouped sum ((a-b)*c-d, a wrong grouping). The core "
            "confusion tested is that (a+b)*c-d is ((a+b)*c)-d, not (a+b)*(c-d) and not (a-b)*c-d."
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
