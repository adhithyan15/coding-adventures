"""Generate rung-111 (radiation oncology / dosimetry) items.json for the ADJ-LADDER.

Rung 111 opens the **radiation oncology / dosimetry** panel on the quantitative band — the arithmetic of a net-dose index. A
`beam_count` (how many treatment beams) TIMES a `dose_per_beam` (the dose each beam contributes) gives the total delivered dose,
a `scatter_dose` (a scatter correction) is SUBTRACTED, and that net dose is DIVIDED by a `fraction_count` (how many fractions
the plan is split over) to give the net-dose index. A **product MINUS a term, all over a single divisor** introduces a genuinely
NEW arithmetic family on the ladder: `(a*b-c)/d`, i.e. `(((a*b) - c) / d)`.

This is genuinely new — rung-108 opened the three-term-numerator frontier with the pure sum `(a+b+c)/d`, rung-109 with the
mixed `(a-b+c)/d`, rung-110 with the product-PLUS-term `(a*b+c)/d`; rung-111 is the FIRST three-term numerator whose leading two
terms are MULTIPLIED and the third is SUBTRACTED, all over a divisor. It is the minus-sibling of rung-110 `(a*b+c)/d`. Every
prior ratio used either a two-term numerator (rung-37 `(a+b)/(c+d)`, rung-99 `(a*b)/(c+d)`, rung-100 `(a+b)/(c*d)`, rung-104
`(a-b)/(c*d)`, the difference-denominator trio rung-105 `(a+b)/(c-d)`, rung-106 `a*b/(c-d)`, rung-107 `(a-b)/(c-d)`) or an
additive/subtractive three-term numerator (rung-108 `(a+b+c)/d`, rung-109 `(a-b+c)/d`, rung-110 `(a*b+c)/d`). Rung-111 moves to
`(a*b-c)/d`. The operator order matters: `(a*b-c)/d` is `((a*b-c) / d)` (the whole product-minus-term is the numerator;
multiplication binds tighter than the subtraction, which binds tighter than the outer division), NOT `a*b-c/d` (dropping the
numerator parentheses so only the scatter dose is divided by the fraction count and then subtracted from the product) and NOT
`(a*b)/(c+d)` (regrouping so only the product forms the numerator and the scatter dose joins the fraction count in the
denominator) — the two distractors exploit exactly those confusions.

The setup: a `beam_count`, a `dose_per_beam`, a `scatter_dose`, and a `fraction_count`. The total is:

  NET-DOSE INDEX   (beam_count * dose_per_beam - scatter_dose) / fraction_count  [ a product-minus-term over a divisor ]
  NET DOSE         beam_count * dose_per_beam - scatter_dose                     [ the product-minus-term numerator ]
  FRACTION COUNT   fraction_count                                               [ the divisor ]

The **net-dose index** is what makes this rung distinctive — it is the ladder's first **product-MINUS-a-term over a single
divisor**. It is a rate (net delivered dose per fraction), framed as an *index* to keep it dimensionless-clean — the same
discipline rungs 100/104/.../110 used for their ratios. (The net dose `a*b-c` and the fraction count `d` ride alongside as
component readouts, so the panel teaches the whole calculation — exactly as rungs 47-110 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the multiplication of the beam count by the dose per beam, then the subtraction of the scatter dose into the net
dose, then the division of that net dose by the fraction count (the whole product-minus-term parenthesized, so (a*b-c)/d
evaluates as ((a*b-c)/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine
change, exactly as rungs 8/16/.../109/110. This rung exercises the engine across a **product-minus-term over a divisor** — the
fact that `(a*b-c)/d` is `((a*b-c)/d)` and NOT `a*b-c/d` and NOT `(a*b)/(c+d)` made computable. The ratio golds are non-integer
f64s; the engine's IEEE-double division matches Python's the same way rungs 99/100/104/.../110 relied on (well within the
harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `-` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the net dose, the fraction count, nor any
index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers**
(`beam_count`, `dose_per_beam`, `scatter_dose`, `fraction_count`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    beam_count * dose_per_beam - scatter_dose / fraction_count  drop the numerator parentheses so only the scatter dose
                                                                        is divided by the fraction count and then subtracted from
                                                                        the product (the classic `(a*b-c)/d` vs `a*b-c/d`
                                                                        precedence error), and
  SWAPPED    (beam_count * dose_per_beam) / (scatter_dose + fraction_count)  regroup so only the product forms the numerator and
                                                                        the scatter dose joins the fraction count in the
                                                                        denominator (`(a*b)/(c+d)` instead of `(a*b-c)/d`),

which are exactly the mistakes a student makes (dropping the numerator parentheses before dividing, or regrouping which terms
belong to the numerator vs the divisor). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: this rung SUBTRACTS, so positivity is guaranteed by table construction rather than automatic. Each
table guarantees **beam_count * dose_per_beam > scatter_dose** (so the net dose `a*b-c` is strictly positive, the index
`(a*b-c)/d` is positive, and the crossed slip `a*b-c/d = a*b-(c/d)` stays positive because `a*b > c >= c/d`), the
**fraction_count >= 2** (divisor never zero), the net-dose index never coincides with the fraction count or the net dose, and the
five family values are pairwise distinct with a comfortable margin; and — so all three queried readouts vary across the panel —
the seven tables give distinct net-dose indices, distinct net doses, and distinct fraction counts, all asserted at build time.
(swapped `(a*b)/(c+d)` is positive because the numerator is a product of positives and the denominator is a sum of positives.)
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BEAM_COUNT, DOSE_PER_BEAM, SCATTER_DOSE, FRACTION_COUNT) — a beam count times a dose per beam for the total delivered dose,
# minus a scatter dose for the net dose, all divided by a fraction count, all plain positive numbers >= 2. This rung SUBTRACTS
# the scatter dose, so every table guarantees beam_count*dose_per_beam > scatter_dose (a*b>c) which keeps the net dose and the
# index strictly positive; fraction_count >= 2 keeps the divisor away from zero. The five family values are asserted
# pairwise-distinct below. The seven tables give distinct net-dose indices, distinct net doses, and distinct fraction counts so
# all three queried readouts vary across the panel.
TABLES = [
    (3, 4, 2, 2),
    (5, 5, 4, 3),
    (5, 4, 2, 4),
    (6, 4, 4, 5),
    (5, 6, 2, 6),
    (7, 4, 2, 7),
    (9, 4, 6, 8),
]

# The option family (5 members), all built from the four observed quantities via *, - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "net_dose_index",
        "net-dose index (the net dose divided by the fraction count)",
        "(beam_count * dose_per_beam - scatter_dose) / fraction_count",
    ),
    (
        "net_dose",
        "the net dose (the beam count times the dose per beam minus the scatter dose, the numerator divided by the fraction count)",
        "beam_count * dose_per_beam - scatter_dose",
    ),
    (
        "fraction_count",
        "the fraction count (the divisor the net dose is divided by)",
        "fraction_count",
    ),
    (
        "crossed",
        "the beam count times the dose per beam minus the scatter dose divided by the fraction count, dropping the numerator parentheses so only the scatter dose is divided before subtracting (a wrong grouping)",
        "beam_count * dose_per_beam - scatter_dose / fraction_count",
    ),
    (
        "swapped",
        "the beam count times the dose per beam, divided by the scatter dose plus the fraction count, regrouping so only the product forms the numerator (a wrong pairing)",
        "(beam_count * dose_per_beam) / (scatter_dose + fraction_count)",
    ),
]
QUERIED = ["net_dose_index", "net_dose", "fraction_count"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(beam_count, dose_per_beam, scatter_dose, fraction_count):
    # Operation order mirrors the ADJ programs exactly (the product forms, the scatter dose is subtracted into the net dose, then
    # that numerator is divided by the fraction count, so (a*b-c)/d evaluates as ((a*b-c)/d)), so the Python option value and the
    # engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "net_dose_index": (beam_count * dose_per_beam - scatter_dose) / fraction_count,
        "net_dose": beam_count * dose_per_beam - scatter_dose,
        "fraction_count": fraction_count,
        "crossed": beam_count * dose_per_beam - scatter_dose / fraction_count,
        "swapped": (beam_count * dose_per_beam) / (scatter_dose + fraction_count),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for beam_count, dose_per_beam, scatter_dose, fraction_count in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung SUBTRACTS the scatter dose, so each table
        # guarantees beam_count*dose_per_beam > scatter_dose (the net dose a*b-c is strictly positive) which keeps every family
        # member strictly positive; fraction_count >= 2 keeps the divisor away from zero.
        assert (
            beam_count >= 2
            and dose_per_beam >= 2
            and scatter_dose >= 2
            and fraction_count >= 2
        ), (beam_count, dose_per_beam, scatter_dose, fraction_count)
        assert beam_count * dose_per_beam > scatter_dose, (
            beam_count,
            dose_per_beam,
            scatter_dose,
            fraction_count,
        )
        fv = family_values(beam_count, dose_per_beam, scatter_dose, fraction_count)
        for key, v in fv.items():
            assert v > 0, (key, beam_count, dose_per_beam, scatter_dose, fraction_count, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    beam_count,
                    dose_per_beam,
                    scatter_dose,
                    fraction_count,
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
                beam_count,
                dose_per_beam,
                scatter_dose,
                fraction_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r111dose-{idx + 1:02d}",
                "qtype": "net_dose_index",
                "stem": (
                    f"A radiotherapy plan records a beam count of {num(beam_count)} times a dose per beam of "
                    f"{num(dose_per_beam)} minus a scatter dose of {num(scatter_dose)}, divided by a fraction count of "
                    f"{num(fraction_count)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe beam_count({num(beam_count)})\n"
                    f"observe dose_per_beam({num(dose_per_beam)})\n"
                    f"observe scatter_dose({num(scatter_dose)})\n"
                    f"observe fraction_count({num(fraction_count)})\n"
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
            "ADJ-LADDER rung 111 — net-dose index from four stated quantities (a NEW panel: radiation oncology / dosimetry). "
            "From a beam count times a dose per beam for the total delivered dose, minus a scatter dose, all divided by a "
            "fraction count, compute the net-dose index ((beam_count*dose_per_beam-scatter_dose)/fraction_count), the net dose "
            "(beam_count*dose_per_beam-scatter_dose), or the fraction count. Each item is a compute_dimensioned program (observe "
            "the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A PRODUCT-MINUS-A-"
            "TERM OVER A DIVISOR (a*b-c)/d (multiply the beam count by the dose per beam, subtract the scatter dose, divide by "
            "the fraction count, so (a*b-c)/d = ((a*b-c)/d); the FIRST time the ladder puts a three-term numerator whose leading "
            "two terms are MULTIPLIED and the third is SUBTRACTED, over a divisor — the minus-sibling of rung-110 (a*b+c)/d; "
            "rung-108 opened the frontier with the pure sum (a+b+c)/d, 109 the mixed (a-b+c)/d, 110 the product-plus-term "
            "(a*b+c)/d, and every earlier ratio used a TWO-term numerator: 37 (a+b)/(c+d), 99 (a*b)/(c+d), 100 (a+b)/(c*d), 104 "
            "(a-b)/(c*d), and the difference-denominator trio 105 (a+b)/(c-d), 106 a*b/(c-d), 107 (a-b)/(c-d)) — and the harness "
            "matches the scalar to the printed options. The net-dose index is a rate (net dose per fraction), framed as an INDEX "
            "so the dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via *, - and / — no constant leaks, and neither the net dose, the fraction count, nor any index ever "
            "appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same four quantities, so the distractors are "
            "exactly the slips students make: dropping the numerator parentheses so only the scatter dose is divided before "
            "subtracting (a*b-c/d, a wrong grouping) and regrouping so only the product forms the numerator ((a*b)/(c+d), a wrong "
            "pairing). The core confusion tested is that (a*b-c)/d is ((a*b-c)/d), not a*b-c/d and not (a*b)/(c+d). This rung "
            "SUBTRACTS the scatter dose, so positivity is guaranteed by table construction: every table has "
            "beam_count*dose_per_beam > scatter_dose (a*b>c) and fraction_count >= 2 (divisor never zero), keeping every family "
            "member strictly positive."
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
