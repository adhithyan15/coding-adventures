"""Generate rung-79 (hematology transfusion-load index) items.json for the ADJ-LADDER.

Rung 79 opens the **hematology / transfusion** panel on the quantitative band — the arithmetic of a total transfusion
load. A transfusion pools a `bag_count` of blood-product bags each of a `bag_volume` (a pure PRODUCT: `bag_count *
bag_volume`, the pooled bag volume) and, independently, dilutes an `additive_volume` down by a `dilution_factor` (a pure
QUOTIENT: `additive_volume / dilution_factor`, the diluted additive contribution), then ADDS the diluted additive to the
pooled bag volume. Two INDEPENDENT binary terms — one a pure product, one a pure quotient — with the quotient added to
the product introduces a genuinely NEW arithmetic shape on the ladder: a **product PLUS a quotient** — `a*b+c/d`, i.e.
`(a*b)+(c/d)`.

This is the deliberate operand-order MIRROR of rung-77's `a/b+c*d` (a quotient plus a product): here the FIRST pair
MULTIPLIES and the SECOND pair DIVIDES, exactly the reverse of rung-77. Like rung-77 (and rung-78's `a/b-c*d`) — and
unlike rungs 69-74, which chained the `+`/`-` and the `*`/`/` through a SHARED operand — the two sides of the `+` are
DISJOINT two-operand terms: `a*b` uses only the first pair, `c/d` only the second, so the shape is a sum of two
independent binary sub-results. The operation order matters: `a*b+c/d` is `(a*b)+(c/d)` by precedence (multiply and
divide bind before add), NOT `(a*b+c)/d` (dividing the WHOLE sum by the last operand) and NOT `a/b+c*d` (swapping which
pair multiplies and which divides — rung-77's shape) — the two distractors exploit exactly those confusions.

The setup: a `bag_count`, a `bag_volume`, an `additive_volume`, and a `dilution_factor`. The total load is:

  TOTAL LOAD          bag_count * bag_volume + additive_volume / dilution_factor   [ product plus quotient ]
  POOLED COMPONENT    bag_count * bag_volume                                       [ the product term ]
  ADDITIVE COMPONENT  additive_volume / dilution_factor                            [ the quotient term ]

The **total load** is what makes this rung distinctive — it is the ladder's first **product PLUS a quotient** (a sum of
two disjoint binary terms). (The pooled component `bag_count * bag_volume` and the additive component `additive_volume /
dilution_factor` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs
47-78 shipped their component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the bag count by the bag volume, the division of the additive volume by
the dilution factor, and the addition of the two independent terms (multiply/divide before add) — and the harness reads
the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../77/78.
This rung exercises the engine across **a product plus a quotient** — the fact that `a*b+c/d` is `(a*b)+(c/d)` and NOT
`(a*b+c)/d` and NOT `a/b+c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `/`, and `+`
— **no structural constants** — so no numeric literal appears in any program, and neither the pooled component, the
additive component, nor any total figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`bag_count`, `bag_volume`, `additive_volume`, `dilution_factor`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (bag_count * bag_volume + additive_volume) / dilution_factor   DIVIDE the WHOLE sum (pooled volume plus
                                                                            additive volume) by the dilution factor
                                                                            instead of dividing only the additive volume
                                                                            (the classic `a*b+c/d` vs `(a*b+c)/d`
                                                                            error), and
  SWAPPED    bag_count / bag_volume + additive_volume * dilution_factor     DIVIDE the first pair and MULTIPLY the second
                                                                            — swapping which pair multiplies and which
                                                                            divides (`a/b+c*d` instead of `a*b+c/d`,
                                                                            rung-77's shape),

which are exactly the mistakes a student makes (folding the additive into the numerator and dividing the whole sum, or
swapping the multiply and the divide between the two pairs). Gold rotates A-E by index. QUERIED (used as gold) = the
three real readouts; all five always appear as options.

Distinctness: all four observed quantities are strictly positive, so every family member — including the headline total
load `a*b+c/d` — is automatically positive; the five family values are pairwise distinct with a comfortable margin,
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BAG_COUNT, BAG_VOLUME, ADDITIVE_VOLUME, DILUTION_FACTOR) — a count of bags to pool, a per-bag volume to multiply by,
# an additive volume to divide, and a dilution factor to divide it by, all plain positive numbers. Since every quantity
# is positive, every family member (product, quotient, their sum, and both distractors) is positive with no extra
# ordering constraint needed. The five family values are asserted pairwise-distinct below.
TABLES = [
    (3, 8, 12, 4),
    (2, 9, 20, 5),
    (4, 6, 15, 3),
    (5, 4, 18, 6),
    (6, 5, 14, 7),
    (2, 7, 24, 8),
    (3, 6, 16, 4),
]

# The option family (5 members), all built from the four observed quantities via *, /, and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "total_load",
        "total transfusion load (the pooled component plus the additive component)",
        "bag_count * bag_volume + additive_volume / dilution_factor",
    ),
    (
        "pooled_component",
        "the pooled component (bag count times the bag volume)",
        "bag_count * bag_volume",
    ),
    (
        "additive_component",
        "the additive component (additive volume over the dilution factor)",
        "additive_volume / dilution_factor",
    ),
    (
        "crossed",
        "the SUM of the pooled volume and the additive volume, all divided by the dilution factor, not two independent terms (a wrong grouping)",
        "(bag_count * bag_volume + additive_volume) / dilution_factor",
    ),
    (
        "swapped",
        "the bag count DIVIDED by the bag volume plus the additive volume MULTIPLIED by the dilution factor, the operations swapped (a wrong grouping)",
        "bag_count / bag_volume + additive_volume * dilution_factor",
    ),
]
QUERIED = ["total_load", "pooled_component", "additive_component"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(bag_count, bag_volume, additive_volume, dilution_factor):
    # Operation order mirrors the ADJ programs exactly (the multiply and the divide bind before the add, per
    # precedence), so the Python option value and the engine result are the same IEEE-double (well within the harness's
    # 1e-9 match tolerance).
    return {
        "total_load": bag_count * bag_volume + additive_volume / dilution_factor,
        "pooled_component": bag_count * bag_volume,
        "additive_component": additive_volume / dilution_factor,
        "crossed": (bag_count * bag_volume + additive_volume) / dilution_factor,
        "swapped": bag_count / bag_volume + additive_volume * dilution_factor,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for bag_count, bag_volume, additive_volume, dilution_factor in TABLES:
        assert (
            bag_count > 0
            and bag_volume > 0
            and additive_volume > 0
            and dilution_factor > 0
        ), (bag_count, bag_volume, additive_volume, dilution_factor)
        fv = family_values(bag_count, bag_volume, additive_volume, dilution_factor)
        # Every quantity is positive, so every family member is positive with no extra ordering constraint.
        for key, v in fv.items():
            assert v > 0, (key, bag_count, bag_volume, additive_volume, dilution_factor, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    bag_count,
                    bag_volume,
                    additive_volume,
                    dilution_factor,
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
                bag_count,
                bag_volume,
                additive_volume,
                dilution_factor,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r79txf-{idx + 1:02d}",
                "qtype": "transfusion_load",
                "stem": (
                    f"A transfusion pools a bag count of {num(bag_count)} bags each of a bag volume of "
                    f"{num(bag_volume)}, plus an additive volume of {num(additive_volume)} diluted by a dilution factor "
                    f"of {num(dilution_factor)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe bag_count({num(bag_count)})\n"
                    f"observe bag_volume({num(bag_volume)})\n"
                    f"observe additive_volume({num(additive_volume)})\n"
                    f"observe dilution_factor({num(dilution_factor)})\n"
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
            "ADJ-LADDER rung 79 — hematology transfusion-load index from four stated quantities (a NEW panel: "
            "hematology / transfusion). From a bag count to pool, a bag volume to multiply by, an additive volume to "
            "divide, and a dilution factor to divide it by, compute the total load "
            "(bag_count*bag_volume + additive_volume/dilution_factor), the pooled component "
            "(bag_count*bag_volume), or the additive component (additive_volume/dilution_factor). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW shape, PRODUCT PLUS A QUOTIENT a*b+c/d (two INDEPENDENT binary terms — a pure "
            "product and a pure quotient — with the quotient added to the product, multiply/divide before add; the "
            "operand-order mirror of rung-77 a/b+c*d, here the FIRST pair multiplies and the SECOND divides; contrast "
            "rungs 69-74 which chained the +/- and */÷ through a SHARED operand; here the two sides of the + are "
            "disjoint 2-operand terms, so a*b+c/d = (a*b)+(c/d), not (a*b+c)/d and not a/b+c*d) — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every index is built only from the four "
            "observed quantities via *, /, and + — no constant leaks, and neither the pooled component, the additive "
            "component, nor any total figure ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same four quantities, so the distractors are exactly the slips students make: DIVIDING "
            "the WHOLE sum (pooled plus additive) by the dilution factor ((a*b+c)/d, a wrong grouping) and SWAPPING the "
            "multiply and divide between the two pairs (a/b+c*d, a wrong grouping). The core confusion tested is that "
            "a*b+c/d is (a*b)+(c/d), not (a*b+c)/d and not a/b+c*d."
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
