"""Generate rung-96 (urology / urodynamics detrusor work) items.json for the ADJ-LADDER.

Rung 96 opens the **urology / urodynamics** panel on the quantitative band — the arithmetic of a detrusor voiding-work
index. A `vesical_pressure` MINUS an `abdominal_pressure` gives the detrusor pressure (the pressure the bladder muscle
itself generates, subtracting the abdominal component), that detrusor pressure MULTIPLIES the `flow_rate` into the
scaled detrusor drive, and a `baseline_tone` is ADDED on to give the total detrusor work. A **binomial difference times
a bare factor, plus a term** introduces a genuinely NEW arithmetic family on the ladder: `(a-b)*c+d`, i.e.
`(((a-b)*c)+d)`.

This is genuinely new. It is the **sign-flip sibling of rung-84** `(a+b)*c-d` (a binomial SUM times a factor MINUS a
term). Rung-84 summed the two inner terms and subtracted the trailing term; rung-96 takes the DIFFERENCE of the two inner
terms and ADDS the trailing term. No prior shape multiplied a binomial DIFFERENCE by a bare factor and added a term:
rung-84 `(a+b)*c-d` used a sum and subtracted, rung-67 `(a-b)*c/d` divided a difference-product by a term (not add),
rung-95 `(a-b)*(c+d)` multiplied a difference by a SUM (both binomials), rung-31 was a difference of two differences.
The operator order matters: `(a-b)*c+d` is `(((a-b)*c)+d)` (the difference forms first inside its parentheses, then it is
multiplied by the factor, then the term is added), NOT `a-b*c+d` (dropping the parentheses so the factor multiplies only
the abdominal pressure) and NOT `(a-b)+c*d` (adding the bare detrusor pressure and multiplying the factor into the
baseline tone instead) — the two distractors exploit exactly those confusions.

The setup: a `vesical_pressure`, an `abdominal_pressure`, a `flow_rate`, and a `baseline_tone`. The total is:

  DETRUSOR WORK       (vesical_pressure - abdominal_pressure) * flow_rate + baseline_tone  [ a difference times a factor, plus a term ]
  DETRUSOR PRESSURE   vesical_pressure - abdominal_pressure                                [ the difference, before the product ]
  SCALED DETRUSOR     (vesical_pressure - abdominal_pressure) * flow_rate                  [ the difference times the factor, before adding the term ]

The **detrusor work** is what makes this rung distinctive — it is the ladder's first **binomial DIFFERENCE times a bare
factor, plus a trailing term**, the sign-flip sibling of rung-84's `(a+b)*c-d`. (The detrusor pressure `a-b` and the
scaled detrusor drive `(a-b)*c` ride alongside as component readouts, so the panel teaches the whole calculation —
exactly as rungs 47-95 shipped their component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the abdominal pressure from the vesical pressure into the detrusor pressure,
the multiplication of that difference by the flow rate into the scaled drive, then the addition of the baseline tone (the
difference forming inside its parentheses before the product, and the product forming before the trailing addition, so
(a-b)*c+d evaluates as (((a-b)*c)+d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor.
No harness/engine change, exactly as rungs 8/16/.../94/95. This rung exercises the engine across a **difference times a
factor, plus a term** — the fact that `(a-b)*c+d` is `(((a-b)*c)+d)` and NOT `a-b*c+d` and NOT `(a-b)+c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `*` and `+` —
**no structural constants** — so no numeric literal appears in any program, and neither the detrusor pressure, the scaled
detrusor drive, nor any work figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`vesical_pressure`, `abdominal_pressure`, `flow_rate`, `baseline_tone`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    vesical_pressure - abdominal_pressure * flow_rate + baseline_tone  drop the parentheses so the flow rate
                                                                               multiplies only the abdominal pressure and
                                                                               the bare vesical pressure and baseline tone
                                                                               are left added (the classic `(a-b)*c+d` vs
                                                                               `a-b*c+d` error), and
  SWAPPED    (vesical_pressure - abdominal_pressure) + flow_rate * baseline_tone  add the bare detrusor pressure and
                                                                               multiply the flow rate into the baseline
                                                                               tone instead, mispairing which terms are
                                                                               added and which are multiplied (`(a-b)+c*d`
                                                                               instead of `(a-b)*c+d`),

which are exactly the mistakes a student makes (dropping the parentheses around the binomial before applying the factor,
or mispairing the factor with the trailing term). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness and positivity: every vesical pressure exceeds its abdominal pressure by at least two
(`vesical_pressure >= abdominal_pressure + 2`) and every quantity is a plain positive number >= 2, so the detrusor
pressure is >= 2 and every family member is strictly positive — a product of a positive difference and a positive factor
plus a positive term, a positive difference, or a positive combination; the crossed slip `a - b*c + d` stays strictly
positive because the tables keep `abdominal_pressure * flow_rate < vesical_pressure + baseline_tone`. The tables are chosen
so the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (VESICAL_PRESSURE, ABDOMINAL_PRESSURE, FLOW_RATE, BASELINE_TONE) — a vesical pressure minus an abdominal pressure for the
# detrusor pressure (vesical_pressure >= abdominal_pressure + 2 so the detrusor pressure is a positive number >= 2), a flow
# rate that scales it, and a baseline tone added on, all plain positive numbers >= 2. Every family member is a product of a
# positive difference and a positive factor plus a positive term / a positive difference / a positive combination, so
# positivity is automatic once the crossed slip a-b*c+d is kept positive (abdominal_pressure*flow_rate <
# vesical_pressure+baseline_tone); the five family values are asserted pairwise-distinct below.
TABLES = [
    (8, 3, 2, 4),
    (9, 2, 3, 5),
    (7, 4, 2, 5),
    (10, 3, 2, 6),
    (6, 2, 4, 5),
    (11, 4, 2, 3),
    (8, 2, 5, 4),
]

# The option family (5 members), all built from the four observed quantities via -, * and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "detrusor_work",
        "total detrusor work (the detrusor pressure times the flow rate, plus the baseline tone)",
        "(vesical_pressure - abdominal_pressure) * flow_rate + baseline_tone",
    ),
    (
        "detrusor_pressure",
        "the detrusor pressure (the vesical pressure minus the abdominal pressure, before multiplying by the flow rate)",
        "vesical_pressure - abdominal_pressure",
    ),
    (
        "scaled_detrusor",
        "the scaled detrusor drive (the detrusor pressure times the flow rate, before adding the baseline tone)",
        "(vesical_pressure - abdominal_pressure) * flow_rate",
    ),
    (
        "crossed",
        "the vesical pressure minus the abdominal pressure times the flow rate, plus the baseline tone, dropping the parentheses so the flow rate multiplies only the abdominal pressure (a wrong grouping)",
        "vesical_pressure - abdominal_pressure * flow_rate + baseline_tone",
    ),
    (
        "swapped",
        "the detrusor pressure plus the flow rate times the baseline tone, adding the bare detrusor pressure and multiplying the flow rate into the baseline tone instead (a wrong pairing)",
        "(vesical_pressure - abdominal_pressure) + flow_rate * baseline_tone",
    ),
]
QUERIED = ["detrusor_work", "detrusor_pressure", "scaled_detrusor"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(vesical_pressure, abdominal_pressure, flow_rate, baseline_tone):
    # Operation order mirrors the ADJ programs exactly (the difference forms inside its parentheses first, then it is
    # multiplied by the flow rate, then the baseline tone is added, so (a-b)*c+d evaluates as (((a-b)*c)+d)), so the Python
    # option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "detrusor_work": (vesical_pressure - abdominal_pressure) * flow_rate + baseline_tone,
        "detrusor_pressure": vesical_pressure - abdominal_pressure,
        "scaled_detrusor": (vesical_pressure - abdominal_pressure) * flow_rate,
        "crossed": vesical_pressure - abdominal_pressure * flow_rate + baseline_tone,
        "swapped": (vesical_pressure - abdominal_pressure) + flow_rate * baseline_tone,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for vesical_pressure, abdominal_pressure, flow_rate, baseline_tone in TABLES:
        # Every vesical pressure exceeds its abdominal pressure by at least two, so the detrusor pressure is a positive
        # number >= 2, and the crossed slip a-b*c+d stays strictly positive.
        assert (
            vesical_pressure >= abdominal_pressure + 2
            and abdominal_pressure > 0
            and flow_rate > 0
            and baseline_tone > 0
            and abdominal_pressure * flow_rate < vesical_pressure + baseline_tone
        ), (vesical_pressure, abdominal_pressure, flow_rate, baseline_tone)
        fv = family_values(vesical_pressure, abdominal_pressure, flow_rate, baseline_tone)
        # Every family member is a product of a positive difference and a positive factor plus a positive term / a positive
        # difference / a positive combination, so every value is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, vesical_pressure, abdominal_pressure, flow_rate, baseline_tone, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    vesical_pressure,
                    abdominal_pressure,
                    flow_rate,
                    baseline_tone,
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
                vesical_pressure,
                abdominal_pressure,
                flow_rate,
                baseline_tone,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r96urodyn-{idx + 1:02d}",
                "qtype": "urodynamics_detrusor_work",
                "stem": (
                    f"A urodynamics study records a vesical pressure of {num(vesical_pressure)} minus an abdominal "
                    f"pressure of {num(abdominal_pressure)}, times a flow rate of {num(flow_rate)}, plus a baseline "
                    f"tone of {num(baseline_tone)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe vesical_pressure({num(vesical_pressure)})\n"
                    f"observe abdominal_pressure({num(abdominal_pressure)})\n"
                    f"observe flow_rate({num(flow_rate)})\n"
                    f"observe baseline_tone({num(baseline_tone)})\n"
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
            "ADJ-LADDER rung 96 — urodynamics detrusor work from four stated quantities (a NEW panel: urology / "
            "urodynamics). From a vesical pressure minus an abdominal pressure for the detrusor pressure, a flow rate that "
            "scales it, and a baseline tone added on, compute the detrusor work "
            "((vesical_pressure-abdominal_pressure)*flow_rate+baseline_tone), the detrusor pressure "
            "(vesical_pressure-abdominal_pressure), or the scaled detrusor drive "
            "((vesical_pressure-abdominal_pressure)*flow_rate). Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A DIFFERENCE "
            "TIMES A FACTOR, PLUS A TERM (a-b)*c+d (subtract b from a, multiply the difference by c, add d, so "
            "(a-b)*c+d = (((a-b)*c)+d); this is the SIGN-FLIP SIBLING of rung-84 (a+b)*c-d, which summed the inner terms "
            "and subtracted the trailing term — rung-96 takes the difference and adds; no prior shape multiplied a binomial "
            "DIFFERENCE by a bare factor and added a term, e.g. rung-67 (a-b)*c/d divided a difference-product by a term "
            "and rung-95 (a-b)*(c+d) multiplied a difference by a sum) — and the harness matches the scalar to the printed "
            "options. Contamination-safe: every figure is built only from the four observed quantities via -, * and + — no "
            "constant leaks, and neither the detrusor pressure, the scaled detrusor drive, nor any work figure ever appears "
            "as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same four quantities, so the distractors are "
            "exactly the slips students make: dropping the parentheses so the flow rate multiplies only the abdominal "
            "pressure (a-b*c+d, a wrong grouping) and adding the bare detrusor pressure while multiplying the flow rate into "
            "the baseline tone ((a-b)+c*d, a wrong pairing). The core confusion tested is that (a-b)*c+d is (((a-b)*c)+d), "
            "not a-b*c+d and not (a-b)+c*d. Every vesical pressure exceeds its abdominal pressure by at least two and the "
            "tables keep the abdominal-times-flow product below the vesical-plus-tone sum, so all figures stay strictly "
            "positive."
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
