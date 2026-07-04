"""Generate rung-72 (obstetrics amniotic-fluid index) items.json for the ADJ-LADDER.

Rung 72 opens the **obstetrics / amniotic-fluid** panel on the quantitative band — the arithmetic of a corrected
amniotic-fluid index. A sonographer reads a `pocket_depth`, divides it by a `transducer_factor` to normalise it, scales
the result by a `quadrant_gain`, and subtracts a `probe_offset`. Dividing FIRST, then scaling, then subtracting a term
introduces a genuinely NEW arithmetic shape on the ladder: a **quotient-times-factor minus a term** — `a/b*c-d`, i.e.
`((a/b)*c)-d`.

This is the deliberate contrast to rung-71's `a/b*c+d` (optometry corrected acuity), which ADDS the trailing term;
rung-72 SUBTRACTS it. It is likewise the divide-first mirror of rung-70's `a*b/c-d` (phototherapy), which MULTIPLIES
first then divides before subtracting. The operation order matters: `a/b*c` is left-to-right `((a/b)*c) = a*c/b`, NOT
`a/(b*c)` — the distractor exploits exactly that confusion. Contrast the other neighbours: rung-53 was `(a+b+c)/d`
(a bare triple sum over a divisor) and rung-68 was `(a+b)*c/d` (a sum scaled then divided). Here a quotient is scaled
AND a term subtracted.

The setup: a `pocket_depth`, a `transducer_factor`, a `quadrant_gain`, and a `probe_offset`. The amniotic index is:

  AMNIOTIC INDEX   pocket_depth / transducer_factor * quadrant_gain - probe_offset   [ normalised, gain-scaled, offset ]
  DEPTH RATIO      pocket_depth / transducer_factor                                  [ the quotient ]
  SCALED POCKET    pocket_depth / transducer_factor * quadrant_gain                  [ the quotient scaled, before the offset ]

The **amniotic index** is what makes this rung distinctive — it is the ladder's first **quotient scaled by a factor,
then a term subtracted** (divide-first). (The depth ratio `pocket_depth / transducer_factor` and the scaled pocket
`pocket_depth / transducer_factor * quadrant_gain` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 47-71 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the division by the transducer factor, the multiplication by the quadrant gain (left-to-right),
and the subtraction of the probe offset — and the harness reads the scalar via the existing `compute_dimensioned`
extractor. No harness/engine change, exactly as rungs 8/16/.../70/71. This rung exercises the engine across **a quotient
scaled then a term subtracted** — the fact that `a/b*c-d` is NOT `a/(b*c)-d` and NOT `a*b/c-d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `*`, and `-`
— **no structural constants** — so no numeric literal appears in any program, and neither the depth ratio, the scaled
pocket, nor any amniotic figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`pocket_depth`, `transducer_factor`, `quadrant_gain`, `probe_offset`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    pocket_depth / (transducer_factor * quadrant_gain) - probe_offset   DIVIDE by the PRODUCT of the transducer
                                                                                 factor and quadrant gain, not divide-
                                                                                 then-multiply (the classic `a/b*c-d`
                                                                                 vs `a/(b*c)-d` error), and
  SWAPPED    pocket_depth * transducer_factor / quadrant_gain - probe_offset     MULTIPLY by the transducer factor and
                                                                                 divide by the quadrant gain — the
                                                                                 operations swapped (`a*b/c-d` instead
                                                                                 of `a/b*c-d`),

which are exactly the mistakes a student makes (folding both denominators into one product, or swapping which quantity
divides and which multiplies). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness: all four observed quantities are strictly positive; the transducer factor and the quadrant gain both
exceed one (so the depth-ratio quotient differs from the scaled pocket) and differ from each other (so the amniotic
value `a*c/b` differs from the swapped value `a*b/c`); the tables are chosen so that even the crossed value stays
positive (`pocket_depth / (transducer_factor * quadrant_gain) > probe_offset`), hence every family member is positive;
the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (POCKET_DEPTH, TRANSDUCER_FACTOR, QUADRANT_GAIN, PROBE_OFFSET) — a pocket depth to normalise, a transducer factor to
# divide by, a quadrant gain to scale by, and a probe offset to subtract, all plain positive numbers with
# transducer_factor > 1, quadrant_gain > 1, transducer_factor != quadrant_gain, and pocket_depth/(tf*qg) > probe_offset
# so the crossed distractor also stays positive. The five family values are asserted pairwise-distinct below.
TABLES = [
    (24, 4, 3, 1),
    (30, 5, 2, 2),
    (36, 6, 4, 1),
    (40, 4, 5, 1),
    (28, 7, 3, 1),
    (45, 5, 4, 2),
    (48, 6, 3, 2),
]

# The option family (5 members), all built from the four observed quantities via /, *, and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "amniotic_index",
        "corrected amniotic-fluid index (pocket depth normalised, gain-scaled, then offset)",
        "pocket_depth / transducer_factor * quadrant_gain - probe_offset",
    ),
    (
        "depth_ratio",
        "the depth ratio (pocket depth over the transducer factor)",
        "pocket_depth / transducer_factor",
    ),
    (
        "scaled_pocket",
        "the scaled pocket before subtracting the probe offset (depth ratio times the quadrant gain)",
        "pocket_depth / transducer_factor * quadrant_gain",
    ),
    (
        "crossed",
        "the pocket depth divided by the PRODUCT of the transducer factor and quadrant gain, not divide-then-multiply (a wrong scaling)",
        "pocket_depth / (transducer_factor * quadrant_gain) - probe_offset",
    ),
    (
        "swapped",
        "the pocket depth MULTIPLIED by the transducer factor and DIVIDED by the quadrant gain, the operations swapped (a wrong scaling)",
        "pocket_depth * transducer_factor / quadrant_gain - probe_offset",
    ),
]
QUERIED = ["amniotic_index", "depth_ratio", "scaled_pocket"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(pocket_depth, transducer_factor, quadrant_gain, probe_offset):
    # Operation order mirrors the ADJ programs exactly (the left-to-right divide-then-multiply forms the quotient scaled
    # by the gain, then the trailing subtract), so the Python option value and the engine result are the same IEEE-double
    # (well within the harness's 1e-9 match tolerance).
    return {
        "amniotic_index": pocket_depth / transducer_factor * quadrant_gain - probe_offset,
        "depth_ratio": pocket_depth / transducer_factor,
        "scaled_pocket": pocket_depth / transducer_factor * quadrant_gain,
        "crossed": pocket_depth / (transducer_factor * quadrant_gain) - probe_offset,
        "swapped": pocket_depth * transducer_factor / quadrant_gain - probe_offset,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for pocket_depth, transducer_factor, quadrant_gain, probe_offset in TABLES:
        assert (
            pocket_depth > 0
            and transducer_factor > 0
            and quadrant_gain > 0
            and probe_offset > 0
        ), (pocket_depth, transducer_factor, quadrant_gain, probe_offset)
        # Transducer factor and quadrant gain exceed one so the depth-ratio quotient differs from the scaled pocket, and
        # they differ from each other so the amniotic value (a*c/b) differs from the swapped value (a*b/c). All four
        # quantities are positive and the crossed value pocket_depth/(tf*qg) exceeds the probe offset, so every family
        # member is positive.
        assert transducer_factor > 1, (pocket_depth, transducer_factor, quadrant_gain, probe_offset)
        assert quadrant_gain > 1, (pocket_depth, transducer_factor, quadrant_gain, probe_offset)
        assert transducer_factor != quadrant_gain, (pocket_depth, transducer_factor, quadrant_gain, probe_offset)
        fv = family_values(pocket_depth, transducer_factor, quadrant_gain, probe_offset)
        for key, v in fv.items():
            assert v > 0, (key, pocket_depth, transducer_factor, quadrant_gain, probe_offset, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    pocket_depth,
                    transducer_factor,
                    quadrant_gain,
                    probe_offset,
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
                pocket_depth,
                transducer_factor,
                quadrant_gain,
                probe_offset,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r72amni-{idx + 1:02d}",
                "qtype": "amniotic_index",
                "stem": (
                    f"A sonographer records a pocket depth of {num(pocket_depth)}, normalised by a transducer factor of "
                    f"{num(transducer_factor)}, scaled by a quadrant gain of {num(quadrant_gain)} and offset by a probe "
                    f"baseline of {num(probe_offset)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe pocket_depth({num(pocket_depth)})\n"
                    f"observe transducer_factor({num(transducer_factor)})\n"
                    f"observe quadrant_gain({num(quadrant_gain)})\n"
                    f"observe probe_offset({num(probe_offset)})\n"
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
            "ADJ-LADDER rung 72 — obstetrics amniotic-fluid index from four stated quantities (a NEW panel: obstetrics "
            "/ amniotic-fluid). From a pocket depth, a transducer factor to divide by, a quadrant gain to scale by, and "
            "a probe offset to subtract, compute the amniotic index "
            "(pocket_depth/transducer_factor*quadrant_gain-probe_offset), the depth ratio "
            "(pocket_depth/transducer_factor), or the scaled pocket (pocket_depth/transducer_factor*quadrant_gain). "
            "Each item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — a NEW shape, QUOTIENT-TIMES-FACTOR MINUS A TERM a/b*c-d (DIVIDE first, "
            "then scale, then subtract — contrast rung-71 a/b*c+d which adds the term, and rung-70 a*b/c-d which "
            "multiplies first; the left-to-right a/b*c = a*c/b, not a/(b*c)) — and the harness matches the scalar to "
            "the printed options. Contamination-safe: every index is built only from the four observed quantities via "
            "/, *, and - — no constant leaks, and neither the depth ratio, the scaled pocket, nor any amniotic figure "
            "ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so "
            "no numeral hides inside a variable name. The five options are a family over the same four quantities, so "
            "the distractors are exactly the slips students make: DIVIDING by the PRODUCT (a/(b*c)-d, a wrong scaling) "
            "and SWAPPING the multiply and divide (a*b/c-d, a wrong scaling). The core confusion tested is that "
            "a/b*c-d is not a/(b*c)-d and not a*b/c-d."
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
