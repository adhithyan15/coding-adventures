"""Generate rung-99 (nephrology / renal-clearance-fraction) items.json for the ADJ-LADDER.

Rung 99 opens the **nephrology / clearance-fraction** panel on the quantitative band — the arithmetic of a renal
clearance index. A `filtered_solute` TIMES a `flow_factor` gives the filtered mass (how much solute the glomerulus
delivers), a `plasma_baseline` PLUS a `plasma_rise` gives the plasma pool (the concentration the clearance is measured
against), and the filtered mass is DIVIDED by that plasma pool to give the clearance index. A **product over a sum**
introduces a genuinely NEW arithmetic family on the ladder: `(a*b)/(c+d)`, i.e. `((a*b)/(c+d))`.

This is genuinely new — the first time the ladder divides a bare PRODUCT by a bare SUM. No prior rung took a product over a
sum: rung-37 `(a+b)/(c+d)` divided a SUM by a sum (both binomials), rung-87 `a*b*c/d` divided a triple product by a single
term, rung-67 `(a-b)*c/d` and rung-68 `(a+b)*c/d` divided a binomial-times-factor by a single term, and rungs 69-80 attach
a division to a single term rather than putting a whole SUM in the denominator. The operator order matters: `(a*b)/(c+d)`
is `((a*b)/(c+d))` (the product forms first, the sum forms inside the denominator's parentheses, then the product is
divided by that sum), NOT `a*b/c+d` (dropping the denominator parentheses so only the baseline divides and the rise is
added outside) and NOT `(a+b)/(c*d)` (summing the numerator terms and multiplying the denominator terms, mispairing which
pair is multiplied and which is added) — the two distractors exploit exactly those confusions.

The setup: a `filtered_solute`, a `flow_factor`, a `plasma_baseline`, and a `plasma_rise`. The total is:

  CLEARANCE INDEX  (filtered_solute * flow_factor) / (plasma_baseline + plasma_rise)  [ a product over a sum ]
  FILTERED MASS    filtered_solute * flow_factor                                      [ the product, the numerator ]
  PLASMA POOL      plasma_baseline + plasma_rise                                       [ the sum, the denominator ]

The **clearance index** is what makes this rung distinctive — it is the ladder's first **bare PRODUCT divided by a bare
SUM**. (The filtered mass `a*b` and the plasma pool `c+d` ride alongside as component readouts, so the panel teaches the
whole calculation — exactly as rungs 47-98 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the filtered solute by the flow factor into the filtered mass, the addition
of the plasma baseline and plasma rise into the plasma pool, then the division of the filtered mass by the plasma pool (the
product forming first, the sum forming inside the denominator's parentheses before the division, so (a*b)/(c+d) evaluates
as ((a*b)/(c+d))) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine
change, exactly as rungs 8/16/.../97/98. This rung exercises the engine across a **product over a sum** — the fact that
`(a*b)/(c+d)` is `((a*b)/(c+d))` and NOT `a*b/c+d` and NOT `(a+b)/(c*d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `+`, and `/` —
**no structural constants** — so no numeric literal appears in any program, and neither the filtered mass, the plasma pool,
nor any clearance figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`filtered_solute`, `flow_factor`, `plasma_baseline`, `plasma_rise`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (filtered_solute * flow_factor) / plasma_baseline + plasma_rise  drop the denominator parentheses so only the
                                                                             baseline divides the filtered mass and the
                                                                             plasma rise is added outside (the classic
                                                                             `(a*b)/(c+d)` vs `a*b/c+d` error), and
  SWAPPED    (filtered_solute + flow_factor) / (plasma_baseline * plasma_rise)  sum the numerator terms and multiply the
                                                                             denominator terms, mispairing which pair is
                                                                             multiplied and which is added (`(a+b)/(c*d)`
                                                                             instead of `(a*b)/(c+d)`),

which are exactly the mistakes a student makes (dropping the parentheses around the sum in the denominator, or mispairing
which pair is a product and which is a sum). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts;
all five always appear as options.

Distinctness and positivity: every observed quantity is a plain positive number >= 2, so the filtered mass (a product of
positives), the plasma pool (a sum of positives), and every ratio (a positive over a positive) are automatically strictly
positive — the denominators `plasma_baseline + plasma_rise`, `plasma_baseline`, and `plasma_baseline * plasma_rise` are all
>= 2 and never zero. No subtraction anywhere. The tables are chosen so the five family values are pairwise distinct with a
comfortable margin (and, for readability, the clearance index is a clean integer that varies 2..8 across the seven tables),
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (FILTERED_SOLUTE, FLOW_FACTOR, PLASMA_BASELINE, PLASMA_RISE) — a filtered solute times a flow factor for the filtered
# mass, a plasma baseline plus a plasma rise for the plasma pool, all plain positive numbers >= 2. Every family member is a
# product of positives, a sum of positives, or a positive over a positive (denominators plasma_baseline+plasma_rise,
# plasma_baseline, plasma_baseline*plasma_rise are all >= 2, never zero), so positivity is automatic (no subtraction
# anywhere); the five family values are asserted pairwise-distinct below. The seven tables give distinct clearance indices
# (2..8), distinct plasma pools (4..10), and a range of filtered masses so all three queried readouts vary.
TABLES = [
    (2, 4, 2, 2),
    (3, 5, 2, 3),
    (2, 12, 2, 4),
    (5, 7, 2, 5),
    (4, 12, 2, 6),
    (7, 9, 2, 7),
    (8, 10, 2, 8),
]

# The option family (5 members), all built from the four observed quantities via *, +, and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "clearance_index",
        "total clearance index (the filtered mass divided by the plasma pool)",
        "(filtered_solute * flow_factor) / (plasma_baseline + plasma_rise)",
    ),
    (
        "filtered_mass",
        "the filtered mass (the filtered solute times the flow factor, the numerator before dividing)",
        "filtered_solute * flow_factor",
    ),
    (
        "plasma_pool",
        "the plasma pool (the plasma baseline plus the plasma rise, the denominator the clearance is measured against)",
        "plasma_baseline + plasma_rise",
    ),
    (
        "crossed",
        "the filtered mass divided by the plasma baseline, plus the plasma rise, dropping the denominator parentheses so only the baseline divides and the rise is added outside (a wrong grouping)",
        "(filtered_solute * flow_factor) / plasma_baseline + plasma_rise",
    ),
    (
        "swapped",
        "the filtered solute plus the flow factor, divided by the plasma baseline times the plasma rise, summing the numerator terms and multiplying the denominator terms instead (a wrong pairing)",
        "(filtered_solute + flow_factor) / (plasma_baseline * plasma_rise)",
    ),
]
QUERIED = ["clearance_index", "filtered_mass", "plasma_pool"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(filtered_solute, flow_factor, plasma_baseline, plasma_rise):
    # Operation order mirrors the ADJ programs exactly (the product forms first, the sum forms inside the denominator's
    # parentheses, then the product is divided by that sum, so (a*b)/(c+d) evaluates as ((a*b)/(c+d))), so the Python option
    # value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "clearance_index": (filtered_solute * flow_factor) / (plasma_baseline + plasma_rise),
        "filtered_mass": filtered_solute * flow_factor,
        "plasma_pool": plasma_baseline + plasma_rise,
        "crossed": (filtered_solute * flow_factor) / plasma_baseline + plasma_rise,
        "swapped": (filtered_solute + flow_factor) / (plasma_baseline * plasma_rise),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for filtered_solute, flow_factor, plasma_baseline, plasma_rise in TABLES:
        # Every observed quantity is a plain positive number >= 2, so every denominator
        # (plasma_baseline+plasma_rise, plasma_baseline, plasma_baseline*plasma_rise) is >= 2 and never zero, and every
        # family member is a product of positives, a sum of positives, or a positive over a positive — all strictly
        # positive with no subtraction anywhere.
        assert (
            filtered_solute >= 2
            and flow_factor >= 2
            and plasma_baseline >= 2
            and plasma_rise >= 2
        ), (filtered_solute, flow_factor, plasma_baseline, plasma_rise)
        fv = family_values(filtered_solute, flow_factor, plasma_baseline, plasma_rise)
        for key, v in fv.items():
            assert v > 0, (key, filtered_solute, flow_factor, plasma_baseline, plasma_rise, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    filtered_solute,
                    flow_factor,
                    plasma_baseline,
                    plasma_rise,
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
                filtered_solute,
                flow_factor,
                plasma_baseline,
                plasma_rise,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r99clr-{idx + 1:02d}",
                "qtype": "renal_clearance_fraction",
                "stem": (
                    f"A renal-clearance study records a filtered solute of {num(filtered_solute)} times a flow factor "
                    f"of {num(flow_factor)}, divided by a plasma baseline of {num(plasma_baseline)} plus a plasma rise "
                    f"of {num(plasma_rise)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe filtered_solute({num(filtered_solute)})\n"
                    f"observe flow_factor({num(flow_factor)})\n"
                    f"observe plasma_baseline({num(plasma_baseline)})\n"
                    f"observe plasma_rise({num(plasma_rise)})\n"
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
            "ADJ-LADDER rung 99 — renal clearance fraction from four stated quantities (a NEW panel: nephrology / "
            "clearance-fraction). From a filtered solute times a flow factor for the filtered mass, a plasma baseline plus "
            "a plasma rise for the plasma pool, and the filtered mass divided by the plasma pool, compute the clearance "
            "index ((filtered_solute*flow_factor)/(plasma_baseline+plasma_rise)), the filtered mass "
            "(filtered_solute*flow_factor), or the plasma pool (plasma_baseline+plasma_rise). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, A PRODUCT OVER A SUM (a*b)/(c+d) (multiply a and b, add c and d, divide the product "
            "by the sum, so (a*b)/(c+d) = ((a*b)/(c+d)); the FIRST time the ladder divides a bare PRODUCT by a bare SUM — "
            "rung-37 (a+b)/(c+d) divided a sum by a sum, rung-87 a*b*c/d divided a triple product by a single term, rung-67 "
            "(a-b)*c/d and rung-68 (a+b)*c/d divided a binomial-times-factor by a single term) — and the harness matches the "
            "scalar to the printed options. Contamination-safe: every figure is built only from the four observed "
            "quantities via *, +, and / — no constant leaks, and neither the filtered mass, the plasma pool, nor any "
            "clearance figure ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the same four "
            "quantities, so the distractors are exactly the slips students make: dropping the denominator parentheses so "
            "only the baseline divides and the rise is added outside (a*b/c+d, a wrong grouping) and summing the numerator "
            "terms while multiplying the denominator terms ((a+b)/(c*d), a wrong pairing). The core confusion tested is that "
            "(a*b)/(c+d) is ((a*b)/(c+d)), not a*b/c+d and not (a+b)/(c*d). Every observed quantity is at least two, so every "
            "denominator is nonzero and all figures stay strictly positive."
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
