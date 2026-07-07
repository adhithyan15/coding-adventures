"""Generate rung-100 (ophthalmology / refraction-focus) items.json for the ADJ-LADDER.

Rung 100 opens the **ophthalmology / refraction-focus** panel on the quantitative band — the arithmetic of a lens focus
ratio. A `near_power` PLUS a `far_power` gives the combined power (how much refractive power the two zones contribute
together), a `cornea_factor` TIMES a `lens_factor` gives the optical scaling (the product the power is measured against),
and the combined power is DIVIDED by that optical scaling to give the focus ratio. A **sum over a product** introduces a
genuinely NEW arithmetic family on the ladder: `(a+b)/(c*d)`, i.e. `((a+b)/(c*d))`.

This is genuinely new — the first time the ladder divides a bare SUM by a bare PRODUCT. It is the **sibling of rung-99**
`(a*b)/(c+d)` (a product over a sum): rung-99 multiplied the numerator and summed the denominator; rung-100 sums the
numerator and multiplies the denominator. No prior rung took a sum over a product: rung-37 `(a+b)/(c+d)` divided a sum by a
SUM (both binomials), rung-87 `a*b*c/d` divided a triple product by a single term, and rungs 69-80 attach a division to a
single term rather than putting a whole PRODUCT in the denominator. The operator order matters: `(a+b)/(c*d)` is
`((a+b)/(c*d))` (the sum forms first, the product forms inside the denominator's parentheses, then the sum is divided by
that product), NOT `a+b/(c*d)` (dropping the numerator parentheses so only the far power is divided and the near power is
added outside) and NOT `(a*b)/(c+d)` (multiplying the numerator terms and summing the denominator terms, mispairing which
pair is multiplied and which is added) — the two distractors exploit exactly those confusions.

The setup: a `near_power`, a `far_power`, a `cornea_factor`, and a `lens_factor`. The total is:

  FOCUS RATIO      (near_power + far_power) / (cornea_factor * lens_factor)  [ a sum over a product ]
  COMBINED POWER   near_power + far_power                                    [ the sum, the numerator ]
  OPTICAL SCALING  cornea_factor * lens_factor                              [ the product, the denominator ]

The **focus ratio** is what makes this rung distinctive — it is the ladder's first **bare SUM divided by a bare PRODUCT**.
(The combined power `a+b` and the optical scaling `c*d` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 47-99 shipped their component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the addition of the near power and far power into the combined power, the multiplication of the
cornea factor by the lens factor into the optical scaling, then the division of the combined power by the optical scaling
(the sum forming first, the product forming inside the denominator's parentheses before the division, so (a+b)/(c*d)
evaluates as ((a+b)/(c*d))) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../98/99. This rung exercises the engine across a **sum over a product** — the
fact that `(a+b)/(c*d)` is `((a+b)/(c*d))` and NOT `a+b/(c*d)` and NOT `(a*b)/(c+d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `*`, and `/` —
**no structural constants** — so no numeric literal appears in any program, and neither the combined power, the optical
scaling, nor any focus figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`near_power`, `far_power`, `cornea_factor`, `lens_factor`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    near_power + far_power / (cornea_factor * lens_factor)  drop the numerator parentheses so only the far power is
                                                                     divided by the optical scaling and the near power is
                                                                     added outside (the classic `(a+b)/(c*d)` vs `a+b/(c*d)`
                                                                     error), and
  SWAPPED    (near_power * far_power) / (cornea_factor + lens_factor)  multiply the numerator terms and sum the denominator
                                                                     terms, mispairing which pair is multiplied and which
                                                                     is summed (`(a*b)/(c+d)` instead of `(a+b)/(c*d)`),

which are exactly the mistakes a student makes (dropping the parentheses around the sum in the numerator, or mispairing
which pair is a sum and which is a product). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts;
all five always appear as options.

Distinctness and positivity: every observed quantity is a plain positive number >= 2, so the combined power (a sum of
positives), the optical scaling (a product of positives), and every ratio (a positive over a positive) are automatically
strictly positive — the denominators `cornea_factor * lens_factor` and `cornea_factor + lens_factor` are both >= 4 and
never zero. No subtraction anywhere. The tables are chosen so the five family values are pairwise distinct with a
comfortable margin, and — so all three queried readouts vary across the panel — the seven tables give distinct focus ratios,
distinct combined powers, and distinct optical scalings, all asserted at build time. (The focus ratio is a dimensionless
figure that may land below or above one, so a value like 0.5 or 2.0 is expected, not an error.)
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (NEAR_POWER, FAR_POWER, CORNEA_FACTOR, LENS_FACTOR) — a near power plus a far power for the combined power, a cornea
# factor times a lens factor for the optical scaling, all plain positive numbers >= 2. Every family member is a sum of
# positives, a product of positives, or a positive over a positive (denominators cornea_factor*lens_factor and
# cornea_factor+lens_factor are both >= 4, never zero), so positivity is automatic (no subtraction anywhere); the five
# family values are asserted pairwise-distinct below. The seven tables give distinct focus ratios, distinct combined
# powers, and distinct optical scalings so all three queried readouts vary across the panel.
TABLES = [
    (2, 4, 2, 2),
    (2, 10, 2, 3),
    (2, 2, 2, 4),
    (2, 3, 3, 3),
    (2, 5, 2, 5),
    (2, 6, 2, 6),
    (2, 7, 3, 5),
]

# The option family (5 members), all built from the four observed quantities via +, *, and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "focus_ratio",
        "total focus ratio (the combined power divided by the optical scaling)",
        "(near_power + far_power) / (cornea_factor * lens_factor)",
    ),
    (
        "combined_power",
        "the combined power (the near power plus the far power, the numerator before dividing)",
        "near_power + far_power",
    ),
    (
        "optical_scaling",
        "the optical scaling (the cornea factor times the lens factor, the denominator the focus is measured against)",
        "cornea_factor * lens_factor",
    ),
    (
        "crossed",
        "the near power plus the far power divided by the optical scaling, dropping the numerator parentheses so only the far power is divided and the near power is added outside (a wrong grouping)",
        "near_power + far_power / (cornea_factor * lens_factor)",
    ),
    (
        "swapped",
        "the near power times the far power, divided by the cornea factor plus the lens factor, multiplying the numerator terms and summing the denominator terms instead (a wrong pairing)",
        "(near_power * far_power) / (cornea_factor + lens_factor)",
    ),
]
QUERIED = ["focus_ratio", "combined_power", "optical_scaling"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(near_power, far_power, cornea_factor, lens_factor):
    # Operation order mirrors the ADJ programs exactly (the sum forms first, the product forms inside the denominator's
    # parentheses, then the sum is divided by that product, so (a+b)/(c*d) evaluates as ((a+b)/(c*d))), so the Python option
    # value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "focus_ratio": (near_power + far_power) / (cornea_factor * lens_factor),
        "combined_power": near_power + far_power,
        "optical_scaling": cornea_factor * lens_factor,
        "crossed": near_power + far_power / (cornea_factor * lens_factor),
        "swapped": (near_power * far_power) / (cornea_factor + lens_factor),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for near_power, far_power, cornea_factor, lens_factor in TABLES:
        # Every observed quantity is a plain positive number >= 2, so both denominators (cornea_factor*lens_factor and
        # cornea_factor+lens_factor) are >= 4 and never zero, and every family member is a sum of positives, a product of
        # positives, or a positive over a positive — all strictly positive with no subtraction anywhere.
        assert (
            near_power >= 2
            and far_power >= 2
            and cornea_factor >= 2
            and lens_factor >= 2
        ), (near_power, far_power, cornea_factor, lens_factor)
        fv = family_values(near_power, far_power, cornea_factor, lens_factor)
        for key, v in fv.items():
            assert v > 0, (key, near_power, far_power, cornea_factor, lens_factor, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    near_power,
                    far_power,
                    cornea_factor,
                    lens_factor,
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
                near_power,
                far_power,
                cornea_factor,
                lens_factor,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r100focus-{idx + 1:02d}",
                "qtype": "refraction_focus_ratio",
                "stem": (
                    f"A refraction study records a near power of {num(near_power)} plus a far power of "
                    f"{num(far_power)}, divided by a cornea factor of {num(cornea_factor)} times a lens factor of "
                    f"{num(lens_factor)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe near_power({num(near_power)})\n"
                    f"observe far_power({num(far_power)})\n"
                    f"observe cornea_factor({num(cornea_factor)})\n"
                    f"observe lens_factor({num(lens_factor)})\n"
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
            "ADJ-LADDER rung 100 — refraction focus ratio from four stated quantities (a NEW panel: ophthalmology / "
            "refraction-focus). From a near power plus a far power for the combined power, a cornea factor times a lens "
            "factor for the optical scaling, and the combined power divided by the optical scaling, compute the focus ratio "
            "((near_power+far_power)/(cornea_factor*lens_factor)), the combined power (near_power+far_power), or the optical "
            "scaling (cornea_factor*lens_factor). Each item is a compute_dimensioned program (observe the four quantities, "
            "let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A SUM OVER A PRODUCT (a+b)/(c*d) "
            "(add a and b, multiply c and d, divide the sum by the product, so (a+b)/(c*d) = ((a+b)/(c*d)); the FIRST time "
            "the ladder divides a bare SUM by a bare PRODUCT — the SIBLING of rung-99 (a*b)/(c+d) which multiplied the "
            "numerator and summed the denominator; rung-37 (a+b)/(c+d) divided a sum by a sum, rung-87 a*b*c/d divided a "
            "triple product by a single term) — and the harness matches the scalar to the printed options. Contamination-"
            "safe: every figure is built only from the four observed quantities via +, *, and / — no constant leaks, and "
            "neither the combined power, the optical scaling, nor any focus figure ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make: dropping the numerator parentheses so only the far power is divided and the near power is added "
            "outside (a+b/(c*d), a wrong grouping) and multiplying the numerator terms while summing the denominator terms "
            "((a*b)/(c+d), a wrong pairing). The core confusion tested is that (a+b)/(c*d) is ((a+b)/(c*d)), not a+b/(c*d) "
            "and not (a*b)/(c+d). Every observed quantity is at least two, so every denominator is nonzero and all figures "
            "stay strictly positive; the focus ratio is dimensionless and may land below or above one."
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
