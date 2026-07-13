"""Generate rung-142 (packing ratio / a PRODUCT-numerator over a PRODUCT — divide a total by an area) items.json.

Rung 142 continues the **OVER-A-PRODUCT** column. rung-141 put a SUM over a product, `(a+b)/(c*d)`; rung-142 puts a PRODUCT over a product,
`(a*b)/(c*d)`. This is the product-numerator twin of the over-a-product family — a total product `a*b` divided by an area product `c*d`,
`(a*b)/(c*d) = (a*b)/(c*d)`, a ratio of two products.

`(a*b)/(c*d)` is a PRODUCT `a*b` divided by a PRODUCT `c*d` (an area). The product `a*b` binds and stays grouped over the bar, and the
two-part denominator `c*d` is ONE area the whole numerator is divided by. As on rung-141, a product denominator has its own two canonical
slips, and they are NOT the sum/difference/rate slips: dividing by two factors in turn IS the same as dividing by their product
(`x/c/d = x/(c*d)`), so that is not a wrong distractor. The two canonical divide-by-a-product slips that a student actually makes are: using
the WRONG denominator operation, ADDING the two dimensions instead of multiplying them (`(a*b)/(c+d)` — a perimeter-style total instead of
an area), and INVERTING the ratio, dividing the area by the total instead of the total by the area (`(c*d)/(a*b)` — the reciprocal).

The setup: a `pack_count` of packs each of `pack_size` (a total pack `pack_count * pack_size`), stored in a bay formed from a `tier_span`
times a `bay_span` (a store area `tier_span * bay_span`). The figures are:

  PACKING RATIO  (pack_count * pack_size) / (tier_span * bay_span)  [ product-numerator OVER a product: total pack / store area ]
  TOTAL PACK     pack_count * pack_size                            [ the product numerator (divided by the store area) ]
  STORE AREA     tier_span * bay_span                              [ the product the total pack is divided by ]

The **packing ratio** is the ladder's first **(a product) over (a product) as a headline** — a ratio (how much total pack rides on each
cell of the store area), framed as a *ratio* to keep it dimensionless-clean, the same discipline rungs 100/.../140/141 used for their
ratios, spans, concentrations, densities, indices, slopes. (The total pack `a*b` and the store area `c*d` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-141 shipped their component figures beside the headline. The two
components anchor the "multiply out the pack FIRST, multiply out the area, then divide the pack by the area" structure against both
distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication to form the total pack, the multiplication to form the store area, then the division of the total pack by
the store area to form the compound figure (so (a*b)/(c*d) evaluates as ((a*b)/(c*d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../140/141. This rung exercises the engine across a
**product divided by a product** — the fact that `(a*b)/(c*d)` is one product over one area and NOT `(a*b)/(c+d)` and NOT `(c*d)/(a*b)`
made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../140/141 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `/`, and `+` — **no structural
constants** — so no numeric literal appears in any program, and neither the total pack, the store area, nor the packing ratio is ever a
literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`pack_count`,
`pack_size`, `tier_span`, `bay_span`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  ADDED      (pack_count * pack_size) / (tier_span + bay_span)  divide the total pack by the SUM of the dimensions instead of their
                                                                product, using a perimeter-style total where an area belongs (the wrong
                                                                denominator operation), and
  INVERTED   (tier_span * bay_span) / (pack_count * pack_size)  divide the store area BY the total pack, the ratio upside down (the
                                                                reciprocal of the packing ratio, the wrong direction),

which are exactly the mistakes a student makes with a product denominator (adding the dimensions instead of multiplying, or inverting the
ratio). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `*`, `/`, and `+` over positive quantities, so **every figure is automatically positive**
(no subtraction anywhere) — like rungs 128/130/131/132/134/135/141, no positivity guards are needed. Every observed quantity is `>= 2`.
Every family member is asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give distinct packing ratios, distinct
total packs, and distinct store areas so all three queried readouts vary across the panel; the five family values are pairwise distinct with
a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PACK_COUNT, PACK_SIZE, TIER_SPAN, BAY_SPAN) — a total pack (pack_count * pack_size) divided by a store area (tier_span * bay_span), giving
# the packing ratio as a product over a product (a*b)/(c*d). This rung uses only *, /, and + over positive quantities, so every figure is
# automatically positive; no positivity guards are needed. The seven tables give distinct total packs (a*b), distinct store areas (c*d), and
# distinct packing ratios ((a*b)/(c*d)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (2, 3, 2, 5),      # pack = 6,  area = 10, ratio = 0.6
    (2, 4, 2, 3),      # pack = 8,  area = 6,  ratio = 1.333...
    (2, 5, 2, 4),      # pack = 10, area = 8,  ratio = 1.25
    (3, 4, 3, 5),      # pack = 12, area = 15, ratio = 0.8
    (2, 7, 4, 5),      # pack = 14, area = 20, ratio = 0.7
    (3, 3, 3, 4),      # pack = 9,  area = 12, ratio = 0.75
    (2, 8, 2, 7),      # pack = 16, area = 14, ratio = 1.142...
]

# The option family (5 members), all built from the four observed quantities via *, /, and +. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "packing_ratio",
        "packing ratio (the total pack divided by the store area)",
        "(pack_count * pack_size) / (tier_span * bay_span)",
    ),
    (
        "total_pack",
        "the total pack (the pack count times the pack size, the numerator that is divided by the store area)",
        "pack_count * pack_size",
    ),
    (
        "store_area",
        "the store area (the tier span times the bay span, the product the total pack is divided by)",
        "tier_span * bay_span",
    ),
    (
        "added",
        "the total pack divided by the tier span plus the bay span, using the sum of the dimensions instead of their product as the divisor (a wrong operation)",
        "(pack_count * pack_size) / (tier_span + bay_span)",
    ),
    (
        "inverted",
        "the store area divided by the total pack, the ratio upside down instead of the total pack over the store area (a wrong operation)",
        "(tier_span * bay_span) / (pack_count * pack_size)",
    ),
]
QUERIED = ["packing_ratio", "total_pack", "store_area"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(pack_count, pack_size, tier_span, bay_span):
    # Operation order mirrors the ADJ programs exactly (the multiplication forms the total pack, the multiplication forms the store area,
    # then the total pack is divided by the store area to form the compound figure, so (a*b)/(c*d) evaluates as ((a*b)/(c*d))), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    pack = pack_count * pack_size
    area = tier_span * bay_span
    return {
        "packing_ratio": pack / area,
        "total_pack": pack,
        "store_area": area,
        "added": pack / (tier_span + bay_span),
        "inverted": area / pack,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for pack_count, pack_size, tier_span, bay_span in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only *, /, and + over positive quantities, so positivity is
        # automatic — no positivity guards are needed.
        assert (
            pack_count >= 2
            and pack_size >= 2
            and tier_span >= 2
            and bay_span >= 2
        ), (pack_count, pack_size, tier_span, bay_span)
        fv = family_values(pack_count, pack_size, tier_span, bay_span)
        for key, v in fv.items():
            assert v > 0, (key, pack_count, pack_size, tier_span, bay_span, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    pack_count,
                    pack_size,
                    tier_span,
                    bay_span,
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
                pack_count,
                pack_size,
                tier_span,
                bay_span,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r142pka-{idx + 1:02d}",
                "qtype": "packing_ratio",
                "stem": (
                    f"A packing study records a pack count of {num(pack_count)} packs each of pack size "
                    f"{num(pack_size)}, stored in a bay of {num(tier_span)} tiers by {num(bay_span)} bays. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe pack_count({num(pack_count)})\n"
                    f"observe pack_size({num(pack_size)})\n"
                    f"observe tier_span({num(tier_span)})\n"
                    f"observe bay_span({num(bay_span)})\n"
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
            "ADJ-LADDER rung 142 — packing ratio from four stated quantities (CONTINUING the OVER-A-PRODUCT column). rung-141 put a sum "
            "over a product (a+b)/(c*d); rung-142 puts a PRODUCT over a product (a*b)/(c*d) — a total product divided by an area product, "
            "a ratio of two products. From a total pack (pack_count * pack_size) divided by a store area (tier_span * bay_span), compute "
            "the packing ratio ((pack_count*pack_size)/(tier_span*bay_span)), the total pack (pack_count*pack_size), or the store area "
            "(tier_span*bay_span). Each item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — a PRODUCT NUMERATOR OVER A PRODUCT (a*b)/(c*d) (multiply out the pack, multiply out the area, "
            "then divide the pack by the area — the two-part denominator is ONE area, not two divisors). As on rung-141, dividing by two "
            "factors in turn equals dividing by their product (x/c/d = x/(c*d)), so that is not a wrong distractor; the two canonical slips "
            "are used instead. The harness matches the scalar to the printed options. The packing ratio is a ratio (how much total pack "
            "rides on each cell of the store area), framed as a RATIO so the dimensionless value stays honest. Contamination-safe: every "
            "figure is built only from the four observed quantities via *, /, and + — no constant leaks, and neither the total pack, the "
            "store area, nor the packing ratio ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the same four quantities, so the "
            "distractors are exactly the slips students make with a product denominator: dividing by the SUM of the dimensions instead of "
            "their product ((a*b)/(c+d), a perimeter-style total where an area belongs, a wrong operation) and INVERTING the ratio "
            "((c*d)/(a*b), the area over the pack, the reciprocal, a wrong operation). The core confusion tested is that (a*b)/(c*d) is one "
            "product over one area, not (a*b)/(c+d) and not (c*d)/(a*b). This rung uses only *, /, and + over positive quantities, so every "
            "figure is automatically positive — no positivity guards are needed — and the five family values are kept pairwise distinct "
            "with all three queried readouts varying across the panel, all asserted strictly positive at build time."
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
