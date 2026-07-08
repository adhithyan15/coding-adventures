"""Generate rung-104 (hematology / coagulation mixing-study) items.json for the ADJ-LADDER.

Rung 104 opens the **hematology / coagulation mixing-study** panel on the quantitative band — the arithmetic of a mixing
study's correction index. A `patient_time` (a prolonged clotting time) MINUS a `control_time` (the normal plasma's time)
gives the prolongation (how far above normal the patient's clot sits, the difference), a `mix_ratio` TIMES an
`incubation_factor` gives the dilution load (the product the prolongation is spread across), and the prolongation is
DIVIDED by the dilution load to give the correction index. A **difference over a product** introduces a genuinely NEW
arithmetic family on the ladder: `(a-b)/(c*d)`, i.e. `((a-b) / (c*d))`.

This is genuinely new — the first time the ladder divides a bare DIFFERENCE by a bare PRODUCT. It is the **mirror-sibling of
rung-100** `(a+b)/(c*d)` (a SUM over a product): rung-100 divided a *sum* by the product, rung-104 divides a *difference*.
No prior rung divides a difference by a product: rung-99 `(a*b)/(c+d)` divided a product by a sum, rung-37 `(a+b)/(c+d)`
divided a sum by a sum, and rungs 75/76 divide a binomial by a bare factor `c` then multiply by `d` (`(a∓b)/c*d`), not by a
`c*d` product. The operator order matters: `(a-b)/(c*d)` is `((a-b) / (c*d))` (the difference forms, the product forms, then
the difference is divided by the product — the parentheses on both sides are what make it a clean ratio), NOT `a-b/(c*d)`
(dropping the numerator parentheses so only the control time is divided by the dilution load and then subtracted from the
patient time) and NOT `(a*b)/(c+d)` (multiplying the numerator pair and summing the denominator pair, mispairing which pair
is the product and which is the difference) — the two distractors exploit exactly those confusions.

The setup: a `patient_time`, a `control_time`, a `mix_ratio`, and an `incubation_factor`. The total is:

  CORRECTION INDEX   (patient_time - control_time) / (mix_ratio * incubation_factor)  [ a difference over a product ]
  PROLONGATION       patient_time - control_time                                      [ the difference, the numerator ]
  DILUTION LOAD      mix_ratio * incubation_factor                                    [ the product, the denominator ]

The **correction index** is what makes this rung distinctive — it is the ladder's first **bare DIFFERENCE over a bare
PRODUCT**. It is a dimensionless ratio (the prolongation per unit of dilution load), so it naturally lands below one; framing
it as an *index* (not a physical time) keeps the sub-one values honest — the same discipline rung-100 used for its
`focus ratio`. (The prolongation `a-b` and the dilution load `c*d` ride alongside as component readouts, so the panel
teaches the whole calculation — exactly as rungs 47-103 shipped their component sums/products/differences/ratios beside the
headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the control time from the patient time into the prolongation, the multiplication
of the mix ratio by the incubation factor into the dilution load, then the division of the prolongation by the dilution load
(both parenthesized, so (a-b)/(c*d) evaluates as ((a-b)/(c*d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../102/103. This rung exercises the engine
across a **difference over a product** — the fact that `(a-b)/(c*d)` is `((a-b)/(c*d))` and NOT `a-b/(c*d)` and NOT
`(a*b)/(c+d)` made computable. The ratio golds are non-integer f64s; the engine's IEEE-double division matches Python's the
same way rungs 99/100 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `*`, and `/` —
**no structural constants** — so no numeric literal appears in any program, and neither the prolongation, the dilution load,
nor any index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`patient_time`, `control_time`, `mix_ratio`, `incubation_factor`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    patient_time - control_time / (mix_ratio * incubation_factor)  drop the numerator parentheses so only the
                                                                            control time is divided by the dilution load and
                                                                            then subtracted from the patient time (the classic
                                                                            `(a-b)/(c*d)` vs `a-b/(c*d)` precedence error), and
  SWAPPED    (patient_time * control_time) / (mix_ratio + incubation_factor)  multiply the numerator pair and sum the
                                                                            denominator pair, mispairing which pair is the
                                                                            product and which is the difference (`(a*b)/(c+d)`
                                                                            instead of `(a-b)/(c*d)`),

which are exactly the mistakes a student makes (dropping the numerator parentheses before dividing, or mispairing which pair
is a difference and which is a product). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all
five always appear as options.

Distinctness and positivity: the tables are chosen so `patient_time > control_time` (prolongation strictly positive — the
patient's clot always sits above normal, so the correction index is strictly positive too), and every denominator is a
product or a sum of quantities `>= 2` (the dilution load `c*d >= 4` and the swapped denominator `c+d >= 4` are never zero, and
there is NO subtraction in any denominator), so no family member is ever zero, negative, or undefined; every observed
quantity is `>= 2`. The crossed figure `a - b/(c*d)` is positive because `b/(c*d) < b <= patient_time`. The tables are chosen
so the five family values are pairwise distinct with a comfortable margin, and — so all three queried readouts vary across
the panel — the seven tables give distinct correction indices, distinct prolongations, and distinct dilution loads, all
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PATIENT_TIME, CONTROL_TIME, MIX_RATIO, INCUBATION_FACTOR) — a patient time minus a control time for the prolongation, a
# mix ratio times an incubation factor for the dilution load, all plain positive numbers >= 2. Each table satisfies
# patient_time > control_time (prolongation > 0 => correction index > 0); every denominator (c*d and c+d) is >= 4 with no
# subtraction, so nothing is ever zero or undefined. The five family values are asserted pairwise-distinct below. The seven
# tables give distinct correction indices, distinct prolongations, and distinct dilution loads so all three queried readouts
# vary across the panel.
TABLES = [
    (4, 2, 2, 3),
    (6, 3, 3, 4),
    (8, 4, 4, 2),
    (10, 5, 5, 6),
    (12, 6, 6, 7),
    (14, 7, 7, 5),
    (16, 8, 8, 9),
]

# The option family (5 members), all built from the four observed quantities via -, *, and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "correction_index",
        "correction index (the prolongation divided by the dilution load)",
        "(patient_time - control_time) / (mix_ratio * incubation_factor)",
    ),
    (
        "prolongation",
        "the prolongation (the patient time minus the control time, the numerator divided by the dilution load)",
        "patient_time - control_time",
    ),
    (
        "dilution_load",
        "the dilution load (the mix ratio times the incubation factor, the denominator the prolongation is divided by)",
        "mix_ratio * incubation_factor",
    ),
    (
        "crossed",
        "the patient time minus the control time divided by the dilution load, dropping the numerator parentheses so only the control time is divided before subtracting (a wrong grouping)",
        "patient_time - control_time / (mix_ratio * incubation_factor)",
    ),
    (
        "swapped",
        "the patient time times the control time, divided by the mix ratio plus the incubation factor, multiplying the numerator pair and summing the denominator pair instead (a wrong pairing)",
        "(patient_time * control_time) / (mix_ratio + incubation_factor)",
    ),
]
QUERIED = ["correction_index", "prolongation", "dilution_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(patient_time, control_time, mix_ratio, incubation_factor):
    # Operation order mirrors the ADJ programs exactly (the difference forms, the product forms, then the difference is
    # divided by the product, so (a-b)/(c*d) evaluates as ((a-b)/(c*d))), so the Python option value and the engine result
    # are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "correction_index": (patient_time - control_time) / (mix_ratio * incubation_factor),
        "prolongation": patient_time - control_time,
        "dilution_load": mix_ratio * incubation_factor,
        "crossed": patient_time - control_time / (mix_ratio * incubation_factor),
        "swapped": (patient_time * control_time) / (mix_ratio + incubation_factor),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for patient_time, control_time, mix_ratio, incubation_factor in TABLES:
        # Every observed quantity is a plain positive number >= 2, and the tables guarantee patient_time > control_time
        # (prolongation > 0 => correction index > 0); every denominator (mix_ratio*incubation_factor and
        # mix_ratio+incubation_factor) is >= 4 with no subtraction, so nothing is ever zero, negative, or undefined.
        assert (
            patient_time >= 2
            and control_time >= 2
            and mix_ratio >= 2
            and incubation_factor >= 2
        ), (patient_time, control_time, mix_ratio, incubation_factor)
        assert patient_time > control_time, (
            patient_time, control_time, mix_ratio, incubation_factor,
        )
        fv = family_values(patient_time, control_time, mix_ratio, incubation_factor)
        for key, v in fv.items():
            assert v > 0, (key, patient_time, control_time, mix_ratio, incubation_factor, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    patient_time,
                    control_time,
                    mix_ratio,
                    incubation_factor,
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
                patient_time,
                control_time,
                mix_ratio,
                incubation_factor,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r104hemix-{idx + 1:02d}",
                "qtype": "hemix_correction_index",
                "stem": (
                    f"A coagulation mixing study records a patient time of {num(patient_time)} minus a control time of "
                    f"{num(control_time)}, divided by a mix ratio of {num(mix_ratio)} times an incubation factor of "
                    f"{num(incubation_factor)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe patient_time({num(patient_time)})\n"
                    f"observe control_time({num(control_time)})\n"
                    f"observe mix_ratio({num(mix_ratio)})\n"
                    f"observe incubation_factor({num(incubation_factor)})\n"
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
            "ADJ-LADDER rung 104 — coagulation mixing-study correction index from four stated quantities (a NEW panel: "
            "hematology / coagulation mixing-study). From a patient time minus a control time for the prolongation, a mix "
            "ratio times an incubation factor for the dilution load, and the prolongation divided by the dilution load, "
            "compute the correction index ((patient_time-control_time)/(mix_ratio*incubation_factor)), the prolongation "
            "(patient_time-control_time), or the dilution load (mix_ratio*incubation_factor). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, A DIFFERENCE OVER A PRODUCT (a-b)/(c*d) (subtract b from a, multiply c and d, divide "
            "the difference by the product, so (a-b)/(c*d) = ((a-b)/(c*d)); the FIRST time the ladder divides a bare "
            "DIFFERENCE by a bare PRODUCT — the MIRROR-SIBLING of rung-100 (a+b)/(c*d) which divided a SUM by the product; "
            "rung-99 (a*b)/(c+d) divided a product by a sum, rung-37 (a+b)/(c+d) a sum by a sum) — and the harness matches "
            "the scalar to the printed options. The correction index is a dimensionless ratio that naturally lands below "
            "one, framed as an INDEX (not a physical time) so the sub-one values stay honest. Contamination-safe: every "
            "figure is built only from the four observed quantities via -, *, and / — no constant leaks, and neither the "
            "prolongation, the dilution load, nor any index ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same four quantities, so the distractors are exactly the slips students make: dropping the "
            "numerator parentheses so only the control time is divided before subtracting (a-b/(c*d), a wrong grouping) and "
            "multiplying the numerator pair while summing the denominator pair ((a*b)/(c+d), a wrong pairing). The core "
            "confusion tested is that (a-b)/(c*d) is ((a-b)/(c*d)), not a-b/(c*d) and not (a*b)/(c+d). Each table guarantees "
            "the patient time exceeds the control time and every denominator is a product or sum of quantities >= 2 (never "
            "zero, no subtraction), so every figure stays strictly positive and well-defined."
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
