"""Generate rung-80 (gastroenterology motility net-transit index) items.json for the ADJ-LADDER.

Rung 80 opens the **gastroenterology / motility** panel on the quantitative band — the arithmetic of a net forward
transit. Peristalsis propels a `bolus_size` through a `contraction_count` of successive contractions (a pure PRODUCT:
`bolus_size * contraction_count`, the propelled volume) and, working against it, a `reflux_volume` returns spread over a
`reflux_spacing` between reflux events (a pure QUOTIENT: `reflux_volume / reflux_spacing`, the reflux loss per unit
spacing), and the reflux loss is SUBTRACTED from the propelled volume. Two INDEPENDENT binary terms — one a pure
product, one a pure quotient — with the quotient subtracted from the product introduces a genuinely NEW arithmetic shape
on the ladder: a **product MINUS a quotient** — `a*b-c/d`, i.e. `(a*b)-(c/d)`.

This is the deliberate MINUS-counterpart of rung-79's `a*b+c/d` (a product plus a quotient) and the operand-order MIRROR
of rung-78's `a/b-c*d` (a quotient minus a product): together rungs 77 (`a/b+c*d`), 78 (`a/b-c*d`), 79 (`a*b+c/d`), and 80
(`a*b-c/d`) complete the full 2x2 of {product-first, quotient-first} x {+, -}. Here the FIRST pair MULTIPLIES and the
SECOND pair DIVIDES, and the two terms are joined by a MINUS. Like rungs 77-79 — and unlike rungs 69-74, which chained
the `+`/`-` and the `*`/`/` through a SHARED operand — the two sides of the `-` are DISJOINT two-operand terms: `a*b`
uses only the first pair, `c/d` only the second, so the shape is a difference of two independent binary sub-results. The
operation order matters: `a*b-c/d` is `(a*b)-(c/d)` by precedence (multiply and divide bind before subtract), NOT
`(a*b-c)/d` (subtracting the third operand from the product and then dividing the WHOLE difference by the last operand)
and NOT `a/b-c*d` (swapping which pair multiplies and which divides — rung-78's shape) — the two distractors exploit
exactly those confusions.

The setup: a `bolus_size`, a `contraction_count`, a `reflux_volume`, and a `reflux_spacing`. The net transit is:

  NET TRANSIT          bolus_size * contraction_count - reflux_volume / reflux_spacing   [ product minus quotient ]
  PROPELLED COMPONENT  bolus_size * contraction_count                                    [ the product term ]
  REFLUX COMPONENT     reflux_volume / reflux_spacing                                    [ the quotient term ]

The **net transit** is what makes this rung distinctive — it is the ladder's first **product MINUS a quotient** (a
difference of two disjoint binary terms). (The propelled component `bolus_size * contraction_count` and the reflux
component `reflux_volume / reflux_spacing` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 47-79 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the bolus size by the contraction count, the division of the reflux
volume by the reflux spacing, and the subtraction of the second from the first (multiply/divide before subtract) — and
the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../78/79. This rung exercises the engine across **a product minus a quotient** — the fact that `a*b-c/d` is
`(a*b)-(c/d)` and NOT `(a*b-c)/d` and NOT `a/b-c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `/`, and `-`
— **no structural constants** — so no numeric literal appears in any program, and neither the propelled component, the
reflux component, nor any net figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`bolus_size`, `contraction_count`, `reflux_volume`, `reflux_spacing`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (bolus_size * contraction_count - reflux_volume) / reflux_spacing   SUBTRACT the reflux volume from the
                                                                                 propelled volume and then DIVIDE the
                                                                                 whole difference by the reflux spacing
                                                                                 instead of dividing only the reflux
                                                                                 volume (the classic `a*b-c/d` vs
                                                                                 `(a*b-c)/d` error), and
  SWAPPED    bolus_size / contraction_count - reflux_volume * reflux_spacing     DIVIDE the first pair and MULTIPLY the
                                                                                 second — swapping which pair multiplies
                                                                                 and which divides (`a/b-c*d` instead of
                                                                                 `a*b-c/d`, rung-78's shape),

which are exactly the mistakes a student makes (folding the reflux volume into the numerator and dividing the whole
difference, or swapping the multiply and the divide between the two pairs). Gold rotates A-E by index. QUERIED (used as
gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: the tables are chosen so all three positivity guards hold — `bolus_size *
contraction_count > reflux_volume / reflux_spacing` (net transit positive), `bolus_size * contraction_count >
reflux_volume` (crossed positive), and `bolus_size / contraction_count > reflux_volume * reflux_spacing` (swapped
positive, the binding constraint) — so every family member, including the headline net transit `a*b-c/d`, is strictly
positive; the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BOLUS_SIZE, CONTRACTION_COUNT, REFLUX_VOLUME, REFLUX_SPACING) — a bolus to propel, a count of contractions to
# multiply by, a reflux volume to divide, and a reflux spacing to divide it by, all plain positive numbers. The tables
# satisfy all three positivity guards: bolus_size*contraction_count > reflux_volume/reflux_spacing (net > 0),
# bolus_size*contraction_count > reflux_volume (crossed > 0), and bolus_size/contraction_count > reflux_volume*
# reflux_spacing (swapped > 0, the binding constraint). The five family values are asserted pairwise-distinct below.
TABLES = [
    (48, 2, 3, 4),
    (60, 3, 2, 5),
    (72, 4, 2, 6),
    (54, 2, 5, 3),
    (40, 2, 3, 5),
    (90, 5, 2, 6),
    (66, 3, 4, 4),
]

# The option family (5 members), all built from the four observed quantities via *, /, and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "net_transit",
        "net forward transit (the propelled component minus the reflux component)",
        "bolus_size * contraction_count - reflux_volume / reflux_spacing",
    ),
    (
        "propelled_component",
        "the propelled component (bolus size times the contraction count)",
        "bolus_size * contraction_count",
    ),
    (
        "reflux_component",
        "the reflux component (reflux volume over the reflux spacing)",
        "reflux_volume / reflux_spacing",
    ),
    (
        "crossed",
        "the propelled volume MINUS the reflux volume, all divided by the reflux spacing, not two independent terms (a wrong grouping)",
        "(bolus_size * contraction_count - reflux_volume) / reflux_spacing",
    ),
    (
        "swapped",
        "the bolus size DIVIDED by the contraction count minus the reflux volume MULTIPLIED by the reflux spacing, the operations swapped (a wrong grouping)",
        "bolus_size / contraction_count - reflux_volume * reflux_spacing",
    ),
]
QUERIED = ["net_transit", "propelled_component", "reflux_component"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(bolus_size, contraction_count, reflux_volume, reflux_spacing):
    # Operation order mirrors the ADJ programs exactly (the multiply and the divide bind before the subtract, per
    # precedence), so the Python option value and the engine result are the same IEEE-double (well within the harness's
    # 1e-9 match tolerance).
    return {
        "net_transit": bolus_size * contraction_count - reflux_volume / reflux_spacing,
        "propelled_component": bolus_size * contraction_count,
        "reflux_component": reflux_volume / reflux_spacing,
        "crossed": (bolus_size * contraction_count - reflux_volume) / reflux_spacing,
        "swapped": bolus_size / contraction_count - reflux_volume * reflux_spacing,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for bolus_size, contraction_count, reflux_volume, reflux_spacing in TABLES:
        assert (
            bolus_size > 0
            and contraction_count > 0
            and reflux_volume > 0
            and reflux_spacing > 0
        ), (bolus_size, contraction_count, reflux_volume, reflux_spacing)
        fv = family_values(bolus_size, contraction_count, reflux_volume, reflux_spacing)
        # The tables satisfy all three positivity guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, bolus_size, contraction_count, reflux_volume, reflux_spacing, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    bolus_size,
                    contraction_count,
                    reflux_volume,
                    reflux_spacing,
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
                bolus_size,
                contraction_count,
                reflux_volume,
                reflux_spacing,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r80gim-{idx + 1:02d}",
                "qtype": "gi_motility_transit",
                "stem": (
                    f"Peristalsis propels a bolus size of {num(bolus_size)} through a contraction count of "
                    f"{num(contraction_count)}, against a reflux volume of {num(reflux_volume)} spread over a reflux "
                    f"spacing of {num(reflux_spacing)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe bolus_size({num(bolus_size)})\n"
                    f"observe contraction_count({num(contraction_count)})\n"
                    f"observe reflux_volume({num(reflux_volume)})\n"
                    f"observe reflux_spacing({num(reflux_spacing)})\n"
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
            "ADJ-LADDER rung 80 — gastroenterology motility net-transit index from four stated quantities (a NEW "
            "panel: gastroenterology / motility). From a bolus size to propel, a contraction count to multiply by, a "
            "reflux volume to divide, and a reflux spacing to divide it by, compute the net transit "
            "(bolus_size*contraction_count - reflux_volume/reflux_spacing), the propelled component "
            "(bolus_size*contraction_count), or the reflux component (reflux_volume/reflux_spacing). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW shape, PRODUCT MINUS A QUOTIENT a*b-c/d (two INDEPENDENT binary terms — a pure "
            "product and a pure quotient — with the quotient subtracted from the product, multiply/divide before "
            "subtract; the minus-counterpart of rung-79 a*b+c/d and the operand-order mirror of rung-78 a/b-c*d, "
            "completing the 2x2 of {product-first, quotient-first} x {+,-}; contrast rungs 69-74 which chained the +/- "
            "and */÷ through a SHARED operand; here the two sides of the - are disjoint 2-operand terms, so a*b-c/d = "
            "(a*b)-(c/d), not (a*b-c)/d and not a/b-c*d) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via *, /, and - — no "
            "constant leaks, and neither the propelled component, the reflux component, nor any net figure ever appears "
            "as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same four quantities, so the "
            "distractors are exactly the slips students make: SUBTRACTING the reflux volume from the propelled volume "
            "and then dividing the WHOLE difference by the reflux spacing ((a*b-c)/d, a wrong grouping) and SWAPPING "
            "the multiply and divide between the two pairs (a/b-c*d, a wrong grouping). The core confusion tested is "
            "that a*b-c/d is (a*b)-(c/d), not (a*b-c)/d and not a/b-c*d."
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
