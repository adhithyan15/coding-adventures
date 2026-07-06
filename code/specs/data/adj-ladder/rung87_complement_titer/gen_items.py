"""Generate rung-87 (immunology complement-titer index) items.json for the ADJ-LADDER.

Rung 87 opens the **immunology / complement-titer** panel on the quantitative band — the arithmetic of a complement
titer. A `serum_dilution` is multiplied by a `cascade_gain` AND by a `membrane_factor` — a THREE-FACTOR product — and the
whole product is then DIVIDED by a `decay_divisor`. A three-factor product divided by a bare term introduces a genuinely
NEW arithmetic shape on the ladder: a **three-factor product DIVIDED by a term** — `a*b*c/d`, i.e. `((a*b*c) / d)`.

This is genuinely new: it PAIRS with rung-86's three-factor product MINUS a term (`a*b*c-d`), but no shipped shape ever
DIVIDES a three-factor product. Every prior shape that divided (rung-70 `a*b/c-d`, rung-80 `a*b-c/d`, rung-83 `a-b*c/d`,
etc.) divided a product of at most TWO observed factors, or divided a single quantity; rung-87 is the first to divide a
THREE-factor product by a fourth quantity. The operator order matters: `a*b*c/d` is `((a*b*c) / d)` by precedence (all
three multiplies bind first, left-to-right, then the single division applies), NOT `a*b/(c*d)` (dividing the two-factor
product by the PRODUCT of the last two, i.e. folding the third factor into the denominator) and NOT `a*b*d/c` (dividing
by the WRONG one of the two trailing quantities) — the two distractors exploit exactly those confusions.

The setup: a `serum_dilution`, a `cascade_gain`, a `membrane_factor`, and a `decay_divisor`. The complement titer is:

  COMPLEMENT TITER  serum_dilution * cascade_gain * membrane_factor / decay_divisor  [ three-factor product over a term ]
  GROSS PRODUCT     serum_dilution * cascade_gain * membrane_factor                  [ the three-factor product, before decay ]
  BASE PRODUCT      serum_dilution * cascade_gain                                    [ the first two factors, before the membrane factor ]

The **complement titer** is what makes this rung distinctive — it is the ladder's first **three-factor product DIVIDED
by a term**. (The gross product `a*b*c` and the base product `a*b` ride alongside as component readouts, so the panel
teaches the whole calculation — exactly as rungs 47-86 shipped their component sums/products/differences/ratios beside
the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the serum dilution by the cascade gain by the membrane factor (three
factors, left-to-right), then the division by the decay divisor (the multiplies before the divide) — and the harness
reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../85/86. This rung exercises the engine across **a three-factor product divided by a term** — the fact that
`a*b*c/d` is `((a*b*c) / d)` and NOT `a*b/(c*d)` and NOT `a*b*d/c` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the gross product, the base product,
nor any titer figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`serum_dilution`, `cascade_gain`, `membrane_factor`, `decay_divisor`) so no numeral hides
inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    serum_dilution * cascade_gain / (membrane_factor * decay_divisor)   divide the base product by the PRODUCT of
                                                                                 the membrane factor and the decay divisor,
                                                                                 folding the third factor into the
                                                                                 denominator instead of multiplying it in
                                                                                 first (the classic `a*b*c/d` vs
                                                                                 `a*b/(c*d)` error), and
  SWAPPED    serum_dilution * cascade_gain * decay_divisor / membrane_factor     multiply by the decay divisor and divide
                                                                                 by the membrane factor, swapping which of
                                                                                 the two trailing quantities is the divisor
                                                                                 (`a*b*d/c` instead of `a*b*c/d`),

which are exactly the mistakes a student makes (folding the last multiply into the denominator, or dividing by the wrong
trailing quantity). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear
as options.

Distinctness and positivity: the tables keep the guards — every quantity `>= 2`, `membrane_factor != decay_divisor` (so
the titer never collapses onto the base product, the swapped never collapses onto the base product, and the titer never
collapses onto the swapped), and `decay_divisor != membrane_factor * membrane_factor` (so the gross product never
collapses onto the swapped) — so every family member, including the headline titer `a*b*c/d`, is strictly positive (a
product/quotient of positives); the five family values are pairwise distinct with a comfortable margin, asserted at build
time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (SERUM_DILUTION, CASCADE_GAIN, MEMBRANE_FACTOR, DECAY_DIVISOR) — a serum dilution to multiply by the cascade gain and the
# membrane factor (a three-factor product), and a decay divisor to divide that product by, all plain positive numbers
# >= 2. The tables satisfy the guards: membrane_factor != decay_divisor (titer != base product, swapped != base product,
# titer != swapped) and decay_divisor != membrane_factor*membrane_factor (gross product != swapped). The five family
# values are asserted pairwise-distinct below.
TABLES = [
    (3, 4, 2, 3),
    (4, 3, 3, 2),
    (5, 4, 2, 5),
    (6, 2, 4, 3),
    (4, 5, 3, 2),
    (7, 3, 2, 3),
    (5, 6, 4, 3),
]

# The option family (5 members), all built from the four observed quantities via * and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "complement_titer",
        "net complement titer (the three-factor product divided by the decay divisor)",
        "serum_dilution * cascade_gain * membrane_factor / decay_divisor",
    ),
    (
        "gross_product",
        "the gross product (the serum dilution times the cascade gain times the membrane factor, before decay)",
        "serum_dilution * cascade_gain * membrane_factor",
    ),
    (
        "base_product",
        "the base product (the serum dilution times the cascade gain, before the membrane factor)",
        "serum_dilution * cascade_gain",
    ),
    (
        "crossed",
        "the serum dilution times the cascade gain, all divided by the membrane factor TIMES the decay divisor, folding the third factor into the denominator instead of multiplying it in first (a wrong grouping)",
        "serum_dilution * cascade_gain / (membrane_factor * decay_divisor)",
    ),
    (
        "swapped",
        "the serum dilution times the cascade gain times the decay divisor, all divided by the membrane factor, dividing by the membrane factor instead of the decay divisor (a wrong divisor)",
        "serum_dilution * cascade_gain * decay_divisor / membrane_factor",
    ),
]
QUERIED = ["complement_titer", "gross_product", "base_product"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(serum_dilution, cascade_gain, membrane_factor, decay_divisor):
    # Operation order mirrors the ADJ programs exactly (all three multiplies bind first, left-to-right, then the single
    # division applies, so a*b*c/d evaluates as ((a*b*c)/d)), so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "complement_titer": serum_dilution * cascade_gain * membrane_factor / decay_divisor,
        "gross_product": serum_dilution * cascade_gain * membrane_factor,
        "base_product": serum_dilution * cascade_gain,
        "crossed": serum_dilution * cascade_gain / (membrane_factor * decay_divisor),
        "swapped": serum_dilution * cascade_gain * decay_divisor / membrane_factor,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for serum_dilution, cascade_gain, membrane_factor, decay_divisor in TABLES:
        assert (
            serum_dilution > 0
            and cascade_gain > 0
            and membrane_factor > 0
            and decay_divisor > 0
        ), (serum_dilution, cascade_gain, membrane_factor, decay_divisor)
        fv = family_values(serum_dilution, cascade_gain, membrane_factor, decay_divisor)
        # The tables satisfy the guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, serum_dilution, cascade_gain, membrane_factor, decay_divisor, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    serum_dilution,
                    cascade_gain,
                    membrane_factor,
                    decay_divisor,
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
                serum_dilution,
                cascade_gain,
                membrane_factor,
                decay_divisor,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r87ctr-{idx + 1:02d}",
                "qtype": "complement_titer_index",
                "stem": (
                    f"A complement assay reads a serum dilution of {num(serum_dilution)} times a cascade gain of "
                    f"{num(cascade_gain)} times a membrane factor of {num(membrane_factor)}, with a decay divisor of "
                    f"{num(decay_divisor)} dividing the product. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe serum_dilution({num(serum_dilution)})\n"
                    f"observe cascade_gain({num(cascade_gain)})\n"
                    f"observe membrane_factor({num(membrane_factor)})\n"
                    f"observe decay_divisor({num(decay_divisor)})\n"
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
            "ADJ-LADDER rung 87 — immunology complement-titer index from four stated quantities (a NEW panel: "
            "immunology / complement-titer). From a serum dilution to multiply by the cascade gain and the membrane "
            "factor (a three-factor product) and a decay divisor to divide by, compute the complement titer "
            "(serum_dilution*cascade_gain*membrane_factor/decay_divisor), the gross product "
            "(serum_dilution*cascade_gain*membrane_factor), or the base product (serum_dilution*cascade_gain). Each item "
            "is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW shape, THREE-FACTOR PRODUCT DIVIDED BY A TERM a*b*c/d (multiply a by b by c, divide by "
            "d, so a*b*c/d = ((a*b*c)/d); it pairs with rung-86's a*b*c-d, but no prior shape divides a three-factor "
            "product — every earlier divide, e.g. rung-80 a*b-c/d and rung-83 a-b*c/d, divided at most two observed "
            "factors) — and the harness matches the scalar to the printed options. Contamination-safe: every index is "
            "built only from the four observed quantities via * and / — no constant leaks, and neither the gross product, "
            "the base product, nor any titer figure ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same four quantities, so the distractors are exactly the slips students make: folding the "
            "membrane factor into the denominator (a*b/(c*d), a wrong grouping) and dividing by the membrane factor "
            "instead of the decay divisor (a*b*d/c, a wrong divisor). The core confusion tested is that a*b*c/d is "
            "((a*b*c)/d), not a*b/(c*d) and not a*b*d/c."
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
