"""Generate rung-77 (anesthesia gas-uptake index) items.json for the ADJ-LADDER.

Rung 77 opens the **anesthesia / gas-uptake** panel on the quantitative band — the arithmetic of an anesthetic
gas-uptake index. A gas-uptake assessment forms a `partition quotient` (an `alveolar_tension` over a `blood_solubility`)
and, independently, a `circulatory load` (a `cardiac_output` times a `partial_gradient`), then ADDS the two together.
Two INDEPENDENT binary terms — one a pure quotient, one a pure product — summed left-to-right introduces a genuinely NEW
arithmetic shape on the ladder: a **quotient PLUS a product** — `a/b+c*d`, i.e. `(a/b)+(c*d)`.

This is the deliberate contrast to every prior sum-and-scale rung. Rungs 69-74 each chained the `+`/`-` and the `*`/`/`
through a SHARED operand (`a*b/c+d`, `a/b*c+d`, `a+b/c*d`, …): one three-operand chain plus a lone fourth. Here the two
sides of the `+` are DISJOINT two-operand terms — `a/b` uses only the first pair, `c*d` only the second — so the shape is
a sum of two independent binary sub-results, the first time the ladder splits its four quantities into two clean
2-operand halves joined by `+`. The operation order matters: `a/b+c*d` is `(a/b)+(c*d)` by precedence (divide and
multiply bind before add), NOT `a/(b+c)*d` (adding into the denominator) and NOT `a*b+c/d` (swapping which pair divides
and which multiplies) — the two distractors exploit exactly those confusions.

The setup: an `alveolar_tension`, a `blood_solubility`, a `cardiac_output`, and a `partial_gradient`. The gas-uptake
index is:

  UPTAKE INDEX        alveolar_tension / blood_solubility + cardiac_output * partial_gradient   [ quotient plus product ]
  PARTITION QUOTIENT  alveolar_tension / blood_solubility                                       [ the quotient term ]
  CIRCULATORY LOAD    cardiac_output * partial_gradient                                         [ the product term ]

The **uptake index** is what makes this rung distinctive — it is the ladder's first **quotient PLUS a product** (a sum
of two disjoint binary terms). (The partition quotient `alveolar_tension / blood_solubility` and the circulatory load
`cardiac_output * partial_gradient` ride alongside as component readouts, so the panel teaches the whole calculation —
exactly as rungs 47-76 shipped their component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the division of the alveolar tension by the blood solubility, the multiplication of the cardiac
output by the partial gradient, and the addition of the two independent terms (multiply/divide before add) — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../75/76. This rung exercises the engine across **a quotient plus a product** — the fact that `a/b+c*d` is
`(a/b)+(c*d)` and NOT `a/(b+c)*d` and NOT `a*b+c/d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `*`, and `+`
— **no structural constants** — so no numeric literal appears in any program, and neither the partition quotient, the
circulatory load, nor any uptake figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`alveolar_tension`, `blood_solubility`, `cardiac_output`,
`partial_gradient`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    alveolar_tension / (blood_solubility + cardiac_output) * partial_gradient   ADD the cardiac output INTO the
                                                                                         denominator instead of keeping
                                                                                         two independent terms (the
                                                                                         classic `a/b+c*d` vs
                                                                                         `a/(b+c)*d` error), and
  SWAPPED    alveolar_tension * blood_solubility + cardiac_output / partial_gradient     MULTIPLY the first pair and
                                                                                         DIVIDE the second — swapping
                                                                                         which pair divides and which
                                                                                         multiplies (`a*b+c/d` instead
                                                                                         of `a/b+c*d`),

which are exactly the mistakes a student makes (folding the second operand into the denominator, or swapping the
divide and the multiply between the two pairs). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness: all four observed quantities are strictly positive, so the partition quotient and the circulatory load are
positive and the uptake index (their sum) exceeds each; the blood solubility exceeds one (so the partition quotient
stays below the alveolar tension and the crossed/swapped variants diverge) and the partial gradient exceeds one (so the
product term and the swapped quotient `cardiac_output/partial_gradient` differ); the five family values are pairwise
distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (ALVEOLAR_TENSION, BLOOD_SOLUBILITY, CARDIAC_OUTPUT, PARTIAL_GRADIENT) — an alveolar tension to divide, a blood
# solubility to divide by, a cardiac output to multiply, and a partial gradient to scale by, all plain positive numbers
# with blood_solubility > 1 and partial_gradient > 1. Because every family value is a positive quotient/product/sum of
# positive quantities, positivity is automatic; the five family values are asserted pairwise-distinct below.
TABLES = [
    (12, 4, 3, 5),
    (20, 5, 4, 3),
    (18, 6, 2, 4),
    (24, 8, 5, 2),
    (15, 3, 6, 2),
    (28, 7, 3, 4),
    (16, 8, 4, 5),
]

# The option family (5 members), all built from the four observed quantities via /, *, and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "uptake_index",
        "gas-uptake index (the partition quotient plus the circulatory load)",
        "alveolar_tension / blood_solubility + cardiac_output * partial_gradient",
    ),
    (
        "partition_quotient",
        "the partition quotient (alveolar tension over the blood solubility)",
        "alveolar_tension / blood_solubility",
    ),
    (
        "circulatory_load",
        "the circulatory load (cardiac output times the partial gradient)",
        "cardiac_output * partial_gradient",
    ),
    (
        "crossed",
        "the alveolar tension divided by the SUM of the blood solubility and cardiac output, then scaled by the partial gradient, not two independent terms (a wrong grouping)",
        "alveolar_tension / (blood_solubility + cardiac_output) * partial_gradient",
    ),
    (
        "swapped",
        "the alveolar tension MULTIPLIED by the blood solubility plus the cardiac output DIVIDED by the partial gradient, the operations swapped (a wrong grouping)",
        "alveolar_tension * blood_solubility + cardiac_output / partial_gradient",
    ),
]
QUERIED = ["uptake_index", "partition_quotient", "circulatory_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(alveolar_tension, blood_solubility, cardiac_output, partial_gradient):
    # Operation order mirrors the ADJ programs exactly (the divide and the multiply bind before the add, per precedence),
    # so the Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match
    # tolerance).
    return {
        "uptake_index": alveolar_tension / blood_solubility + cardiac_output * partial_gradient,
        "partition_quotient": alveolar_tension / blood_solubility,
        "circulatory_load": cardiac_output * partial_gradient,
        "crossed": alveolar_tension / (blood_solubility + cardiac_output) * partial_gradient,
        "swapped": alveolar_tension * blood_solubility + cardiac_output / partial_gradient,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for alveolar_tension, blood_solubility, cardiac_output, partial_gradient in TABLES:
        assert (
            alveolar_tension > 0
            and blood_solubility > 0
            and cardiac_output > 0
            and partial_gradient > 0
        ), (alveolar_tension, blood_solubility, cardiac_output, partial_gradient)
        # Blood solubility exceeds one so the partition quotient stays below the alveolar tension (and the crossed
        # variant, which enlarges the denominator, diverges), and partial gradient exceeds one so the product term and
        # the swapped quotient (cardiac_output/partial_gradient) differ. Every family member is a positive
        # quotient/product/sum of positive quantities, so positivity is automatic.
        assert blood_solubility > 1, (alveolar_tension, blood_solubility, cardiac_output, partial_gradient)
        assert partial_gradient > 1, (alveolar_tension, blood_solubility, cardiac_output, partial_gradient)
        fv = family_values(alveolar_tension, blood_solubility, cardiac_output, partial_gradient)
        for key, v in fv.items():
            assert v > 0, (key, alveolar_tension, blood_solubility, cardiac_output, partial_gradient, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    alveolar_tension,
                    blood_solubility,
                    cardiac_output,
                    partial_gradient,
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
                alveolar_tension,
                blood_solubility,
                cardiac_output,
                partial_gradient,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r77gas-{idx + 1:02d}",
                "qtype": "gas_uptake",
                "stem": (
                    f"A gas-uptake assessment records an alveolar tension of {num(alveolar_tension)}, a blood "
                    f"solubility of {num(blood_solubility)} to divide by, a cardiac output of {num(cardiac_output)} and "
                    f"a partial gradient of {num(partial_gradient)} to scale it by. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe alveolar_tension({num(alveolar_tension)})\n"
                    f"observe blood_solubility({num(blood_solubility)})\n"
                    f"observe cardiac_output({num(cardiac_output)})\n"
                    f"observe partial_gradient({num(partial_gradient)})\n"
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
            "ADJ-LADDER rung 77 — anesthesia gas-uptake index from four stated quantities (a NEW panel: anesthesia / "
            "gas-uptake). From an alveolar tension to divide, a blood solubility to divide by, a cardiac output to "
            "multiply, and a partial gradient to scale by, compute the gas-uptake index "
            "(alveolar_tension/blood_solubility + cardiac_output*partial_gradient), the partition quotient "
            "(alveolar_tension/blood_solubility), or the circulatory load (cardiac_output*partial_gradient). Each item "
            "is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine "
            "carries the arithmetic — a NEW shape, QUOTIENT PLUS A PRODUCT a/b+c*d (two INDEPENDENT binary terms — a "
            "pure quotient and a pure product — summed, divide/multiply before add; contrast rungs 69-74 which chained "
            "the +/- and */÷ through a SHARED operand; here the two sides of the + are disjoint 2-operand terms, so "
            "a/b+c*d = (a/b)+(c*d), not a/(b+c)*d and not a*b+c/d) — and the harness matches the scalar to the printed "
            "options. Contamination-safe: every index is built only from the four observed quantities via /, *, and + — "
            "no constant leaks, and neither the partition quotient, the circulatory load, nor any uptake figure ever "
            "appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. The five options are a family over the same four quantities, so the "
            "distractors are exactly the slips students make: ADDING the cardiac output INTO the denominator "
            "(a/(b+c)*d, a wrong grouping) and SWAPPING the multiply and divide between the two pairs (a*b+c/d, a wrong "
            "grouping). The core confusion tested is that a/b+c*d is (a/b)+(c*d), not a/(b+c)*d and not a*b+c/d."
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
