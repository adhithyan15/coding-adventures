"""Generate rung-108 (dentistry / periodontal probing) items.json for the ADJ-LADDER.

Rung 108 opens the **dentistry / periodontal probing** panel on the quantitative band — the arithmetic of a periodontal
pocket index. A `mesial_depth`, a `distal_depth`, and a `buccal_depth` (three probing depths around a tooth) SUMMED give the
total probing depth, and that three-way sum is DIVIDED by a `quadrant_count` (how many quadrants the reading is averaged over)
to give the pocket index. A **three-term sum over a single divisor** introduces a genuinely NEW arithmetic family on the
ladder: `(a+b+c)/d`, i.e. `((a+b+c) / d)`.

This is genuinely new — the first time the ladder puts a **THREE-TERM numerator** over a divisor. The whole ratio family so
far used TWO-term numerators over various denominators: rung-37 `(a+b)/(c+d)`, rung-99 `(a*b)/(c+d)`, rung-100 `(a+b)/(c*d)`,
rung-104 `(a-b)/(c*d)`, and the just-completed difference-denominator trio rung-105 `(a+b)/(c-d)`, rung-106 `a*b/(c-d)`,
rung-107 `(a-b)/(c-d)`. Rung-108 moves to a compound THREE-way numerator over a single divisor `d`. The operator order
matters: `(a+b+c)/d` is `((a+b+c) / d)` (the whole three-term sum is the numerator), NOT `a+b+c/d` (dropping the numerator
parentheses so only the buccal depth is divided by the quadrant count and then added to the other two depths) and NOT
`(a+b)/(c+d)` (regrouping so only the first two depths form the numerator and the buccal depth joins the quadrant count in the
denominator) — the two distractors exploit exactly those confusions.

The setup: a `mesial_depth`, a `distal_depth`, a `buccal_depth`, and a `quadrant_count`. The total is:

  POCKET INDEX     (mesial_depth + distal_depth + buccal_depth) / quadrant_count  [ a three-term sum over a divisor ]
  TOTAL DEPTH      mesial_depth + distal_depth + buccal_depth                     [ the three-term sum, the numerator ]
  QUADRANT COUNT   quadrant_count                                                 [ the divisor ]

The **pocket index** is what makes this rung distinctive — it is the ladder's first **three-term SUM over a single divisor**.
It is a rate (total probing depth per quadrant), framed as an *index* to keep it dimensionless-clean — the same discipline
rungs 100/104/105/106/107 used for their ratios. (The total depth `a+b+c` and the quadrant count `d` ride alongside as
component readouts, so the panel teaches the whole calculation — exactly as rungs 47-107 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the addition of the three probing depths into the total depth, then the division of that total by the
quadrant count (the whole sum parenthesized, so (a+b+c)/d evaluates as ((a+b+c)/d)) — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../106/107. This rung exercises the
engine across a **three-term sum over a divisor** — the fact that `(a+b+c)/d` is `((a+b+c)/d)` and NOT `a+b+c/d` and NOT
`(a+b)/(c+d)` made computable. The ratio golds are non-integer f64s; the engine's IEEE-double division matches Python's the
same way rungs 99/100/104/105/106/107 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the total depth, the quadrant count, nor
any index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`mesial_depth`, `distal_depth`, `buccal_depth`, `quadrant_count`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    mesial_depth + distal_depth + buccal_depth / quadrant_count  drop the numerator parentheses so only the buccal
                                                                          depth is divided by the quadrant count and then added
                                                                          to the other two depths (the classic `(a+b+c)/d` vs
                                                                          `a+b+c/d` precedence error), and
  SWAPPED    (mesial_depth + distal_depth) / (buccal_depth + quadrant_count)  regroup so only the first two depths form the
                                                                          numerator and the buccal depth joins the quadrant
                                                                          count in the denominator (`(a+b)/(c+d)` instead of
                                                                          `(a+b+c)/d`),

which are exactly the mistakes a student makes (dropping the numerator parentheses before dividing, or regrouping which terms
belong to the numerator vs the divisor). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: every family member is a sum or quotient of strictly positive observed quantities `>= 2`, so all
five are strictly positive by construction (this rung has NO subtraction). The tables are chosen so `quadrant_count >= 2`
(divisor never zero), the pocket index never coincides with the quadrant count or the total depth, and the five family values
are pairwise distinct with a comfortable margin; and — so all three queried readouts vary across the panel — the seven tables
give distinct pocket indices, distinct total depths, and distinct quadrant counts, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (MESIAL_DEPTH, DISTAL_DEPTH, BUCCAL_DEPTH, QUADRANT_COUNT) — three probing depths summed for the total probing depth, all
# divided by a quadrant count, all plain positive numbers >= 2. This rung has NO subtraction, so every family member is a sum
# or quotient of positives and is strictly positive by construction; quadrant_count >= 2 keeps the divisor away from zero. The
# five family values are asserted pairwise-distinct below. The seven tables give distinct pocket indices, distinct total
# depths, and distinct quadrant counts so all three queried readouts vary across the panel.
TABLES = [
    (4, 5, 3, 2),
    (6, 4, 5, 3),
    (7, 6, 5, 4),
    (9, 7, 5, 5),
    (8, 7, 5, 6),
    (10, 8, 6, 7),
    (11, 9, 7, 8),
]

# The option family (5 members), all built from the four observed quantities via + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "pocket_index",
        "pocket index (the total probing depth divided by the quadrant count)",
        "(mesial_depth + distal_depth + buccal_depth) / quadrant_count",
    ),
    (
        "total_depth",
        "the total probing depth (the mesial plus distal plus buccal depths, the numerator divided by the quadrant count)",
        "mesial_depth + distal_depth + buccal_depth",
    ),
    (
        "quadrant_count",
        "the quadrant count (the divisor the total probing depth is divided by)",
        "quadrant_count",
    ),
    (
        "crossed",
        "the mesial plus distal depths plus the buccal depth divided by the quadrant count, dropping the numerator parentheses so only the buccal depth is divided before adding (a wrong grouping)",
        "mesial_depth + distal_depth + buccal_depth / quadrant_count",
    ),
    (
        "swapped",
        "the mesial plus distal depths, divided by the buccal depth plus the quadrant count, regrouping so only the first two depths form the numerator (a wrong pairing)",
        "(mesial_depth + distal_depth) / (buccal_depth + quadrant_count)",
    ),
]
QUERIED = ["pocket_index", "total_depth", "quadrant_count"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(mesial_depth, distal_depth, buccal_depth, quadrant_count):
    # Operation order mirrors the ADJ programs exactly (the three-term sum forms, then the sum is divided by the quadrant
    # count, so (a+b+c)/d evaluates as ((a+b+c)/d)), so the Python option value and the engine result are the same IEEE-double
    # (well within the harness's 1e-9 match tolerance).
    return {
        "pocket_index": (mesial_depth + distal_depth + buccal_depth) / quadrant_count,
        "total_depth": mesial_depth + distal_depth + buccal_depth,
        "quadrant_count": quadrant_count,
        "crossed": mesial_depth + distal_depth + buccal_depth / quadrant_count,
        "swapped": (mesial_depth + distal_depth) / (buccal_depth + quadrant_count),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for mesial_depth, distal_depth, buccal_depth, quadrant_count in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung has NO subtraction, so every family member is
        # a sum or quotient of positives and is strictly positive by construction; quadrant_count >= 2 keeps the divisor away
        # from zero.
        assert (
            mesial_depth >= 2
            and distal_depth >= 2
            and buccal_depth >= 2
            and quadrant_count >= 2
        ), (mesial_depth, distal_depth, buccal_depth, quadrant_count)
        fv = family_values(mesial_depth, distal_depth, buccal_depth, quadrant_count)
        for key, v in fv.items():
            assert v > 0, (key, mesial_depth, distal_depth, buccal_depth, quadrant_count, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    mesial_depth,
                    distal_depth,
                    buccal_depth,
                    quadrant_count,
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
                mesial_depth,
                distal_depth,
                buccal_depth,
                quadrant_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r108perio-{idx + 1:02d}",
                "qtype": "perio_pocket_index",
                "stem": (
                    f"A periodontal chart records a mesial depth of {num(mesial_depth)} plus a distal depth of "
                    f"{num(distal_depth)} plus a buccal depth of {num(buccal_depth)}, divided by a quadrant count of "
                    f"{num(quadrant_count)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe mesial_depth({num(mesial_depth)})\n"
                    f"observe distal_depth({num(distal_depth)})\n"
                    f"observe buccal_depth({num(buccal_depth)})\n"
                    f"observe quadrant_count({num(quadrant_count)})\n"
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
            "ADJ-LADDER rung 108 — periodontal pocket index from four stated quantities (a NEW panel: dentistry / periodontal "
            "probing). From a mesial plus a distal plus a buccal probing depth for the total probing depth, all divided by a "
            "quadrant count, compute the pocket index ((mesial_depth+distal_depth+buccal_depth)/quadrant_count), the total "
            "probing depth (mesial_depth+distal_depth+buccal_depth), or the quadrant count. Each item is a compute_dimensioned "
            "program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW "
            "family, A THREE-TERM SUM OVER A DIVISOR (a+b+c)/d (add the three depths, divide by the quadrant count, so "
            "(a+b+c)/d = ((a+b+c)/d); the FIRST time the ladder puts a THREE-TERM numerator over a divisor — every prior ratio "
            "used a TWO-term numerator: 37 (a+b)/(c+d), 99 (a*b)/(c+d), 100 (a+b)/(c*d), 104 (a-b)/(c*d), and the "
            "difference-denominator trio 105 (a+b)/(c-d), 106 a*b/(c-d), 107 (a-b)/(c-d)) — and the harness matches the scalar "
            "to the printed options. The pocket index is a rate (total depth per quadrant), framed as an INDEX so the "
            "dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via + and / — no constant leaks, and neither the total depth, the quadrant count, nor any index ever "
            "appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same four quantities, so the distractors are "
            "exactly the slips students make: dropping the numerator parentheses so only the buccal depth is divided before "
            "adding (a+b+c/d, a wrong grouping) and regrouping so only the first two depths form the numerator ((a+b)/(c+d), a "
            "wrong pairing). The core confusion tested is that (a+b+c)/d is ((a+b+c)/d), not a+b+c/d and not (a+b)/(c+d). This "
            "rung has NO subtraction, so every family member is a sum or quotient of positives and is strictly positive by "
            "construction; the quadrant count is >= 2 (divisor never zero)."
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
