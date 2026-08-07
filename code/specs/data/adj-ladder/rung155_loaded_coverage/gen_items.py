"""Generate rung-155 (loaded coverage / a PRODUCT-PLUS-TERM numerator over a TWO-TERM PRODUCT — five-quantity, product denominator, operator precedence) items.json.

Rung 155 keeps rung-153/154's two-term PRODUCT denominator but changes the numerator to a PRODUCT-PLUS-TERM, `(a*b+c)/(d*e)`: a batch quantity
formed by multiplying two factors, plus an extra addon, divided by a grid AREA. It brings **operator precedence** into the five-quantity
product-denominator family — the multiplication `a*b` binds tighter than the `+c`, so the numerator is `(a*b)+c` and NOT `a*(b+c)`. This mirrors
rung-147's base-plus-product numerator, now with a product (area) denominator instead of a lone divisor.

`(a*b+c)/(d*e)` forms a loaded numerator `a*b+c` (two factors multiplied, then an addon added) over the PRODUCT of two dimensions `d*e` (an area).
Both sides are totals that must be formed BEFORE the division: the two factors multiply and the addon is added (multiplication FIRST, by
precedence) to form the loaded numerator, the two dimensions multiply into the denominator AREA, and only then is the loaded total divided by the
area. As on the over-a-product rungs (141-144, 153, 154), a product denominator has its own canonical slip that is NOT the sum/difference slip:
dividing by two factors in turn IS the same as dividing by their product (`x/d/e = x/(d*e)`), so that is not a wrong distractor. Two slips are put
on the panel: the WRONG DENOMINATOR OPERATION — ADDING the two dimensions instead of multiplying them, `(a*b+c)/(d+e)` (a perimeter-style total
where an area belongs) — and the OPERATOR PRECEDENCE ERROR carried in by the product-plus-term numerator — multiplying the first factor by the
SUM `a*(b+c)` instead of adding the addon after the product, `(a*(b+c))/(d*e)`. The rung thus sits exactly at the crossing of its two questions:
"did you MULTIPLY the grid dimensions, or add them?" and "did you do `a*b` FIRST then add c, or add `b+c` first then multiply?".

The setup: two factors `batch_size`, `batch_count` are multiplied and an `addon` is added (a loaded total `batch_size * batch_count + addon`) and
spread across a grid formed from `grid_rows` times `grid_cols` (a grid area `grid_rows * grid_cols`). The figures are:

  LOADED COVERAGE  (batch_size * batch_count + addon) / (grid_rows * grid_cols)  [ PRODUCT-PLUS-TERM numerator OVER a two-term product: loaded total / grid area ]
  LOADED TOTAL     batch_size * batch_count + addon                            [ the product-plus-term numerator total (divided by the grid area) ]
  GRID AREA        grid_rows * grid_cols                                       [ the two-term denominator product (the loaded total is divided by) ]

The **loaded coverage** is the headline; the **loaded total** (product plus addon) and the **grid area** (rows times cols) ride alongside as
component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline rungs 47-154
shipped. Critically, the loaded total `(a*b+c)` is the *legitimate* multiply-first-then-add figure, whereas the distractor `(a*(b+c))/(d*e)` is
the *slip* of adding first then multiplying (an operator-precedence error), and `(a*b+c)/(d+e)` is the *slip* of adding the dimensions where their
product belongs — the panel puts the honest loaded coverage and both wrong-operation slips side by side.

Each figure is a `compute_dimensioned` program (`observe` the five quantities + `let answer = formula`); the ADJ engine carries the arithmetic —
the multiplication of the two factors, the addition of the addon (multiplication first by precedence), the multiplication to form the grid area,
then the division of the loaded total by the grid area to form the compound figure (so (a*b+c)/(d*e) evaluates as ((a*b+c)/(d*e))) — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../153/154. This rung
exercises the engine across a **product-plus-term numerator divided by a two-term product** — the fact that `(a*b+c)/(d*e)` multiplies BEFORE it
adds and is NOT `(a*(b+c))/(d*e)` and NOT `(a*b+c)/(d+e)` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double
division matches Python's the same way rungs 100/.../153/154 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the five observed quantities via `+`, `*`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the loaded total, the grid area, nor the loaded coverage is ever a literal
(each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`batch_size`, `batch_count`, `addon`,
`grid_rows`, `grid_cols`) so no numeral hides inside a variable name.

The five options are a tight family over the same five quantities: the three real readouts plus the two classic slips —

  SUMMED      (batch_size * batch_count + addon) / (grid_rows + grid_cols)  ADD the two dimensions instead of multiplying them, the wrong
                                                                           denominator operation (a perimeter-style total where an area belongs), and
  PRECEDENCE  (batch_size * (batch_count + addon)) / (grid_rows * grid_cols)  ADD the addon to the count BEFORE multiplying (a*(b+c)) instead of
                                                                             multiplying first then adding ((a*b)+c), the operator-precedence error,

which are exactly the mistakes a student makes loading a batch over a grid area (adding the dimensions instead of multiplying, or adding before
multiplying). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `+`, `*`, and `/` over positive quantities, so **every figure is automatically positive** (no
subtraction anywhere) — no positivity guards are needed. Every observed quantity is `>= 2`, so `batch_size >= 2` keeps the precedence distractor
`a*(b+c)` genuinely distinct from the honest `a*b+c` (they differ by `a*c - c = (a-1)*c > 0`). Every family member is asserted `> 0` at build time.
The seven tables give distinct loaded coverages, distinct loaded totals, and distinct grid areas so all three queried readouts vary across the
panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BATCH_SIZE, BATCH_COUNT, ADDON, GRID_ROWS, GRID_COLS) — two factors multiplied and an addon added (batch_size * batch_count + addon) over a
# grid area (grid_rows * grid_cols), giving the loaded coverage as a product-plus-term numerator over a two-term product (a*b+c)/(d*e). This rung
# uses only +, *, and / over positive quantities, so every figure is automatically positive; no positivity guards are needed. Because every
# quantity is >= 2, the precedence distractor a*(b+c) stays distinct from the honest a*b+c (they differ by (a-1)*c > 0). The seven tables give
# distinct loaded totals (a*b+c), distinct grid areas (d*e), and distinct loaded coverages ((a*b+c)/(d*e)); the five family values are asserted
# pairwise-distinct below.
TABLES = [
    (3, 4, 2, 2, 5),      # loaded = 14, area = 10, coverage = 1.4
    (4, 5, 2, 2, 3),      # loaded = 22, area = 6,  coverage = 22/6
    (3, 6, 2, 2, 4),      # loaded = 20, area = 8,  coverage = 2.5
    (4, 4, 2, 3, 5),      # loaded = 18, area = 15, coverage = 1.2
    (5, 5, 2, 4, 5),      # loaded = 27, area = 20, coverage = 1.35
    (6, 4, 2, 3, 4),      # loaded = 26, area = 12, coverage = 26/12
    (5, 6, 2, 2, 7),      # loaded = 32, area = 14, coverage = 32/14
]

# The option family (5 members), all built from the five observed quantities via +, *, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "loaded_coverage",
        "loaded coverage (the loaded total divided by the grid area)",
        "(batch_size * batch_count + addon) / (grid_rows * grid_cols)",
    ),
    (
        "loaded_total",
        "the loaded total (the two factors multiplied then the addon added, the numerator that is divided by the grid area)",
        "batch_size * batch_count + addon",
    ),
    (
        "grid_area",
        "the grid area (the grid rows times the grid cols, the denominator the loaded total is divided by)",
        "grid_rows * grid_cols",
    ),
    (
        "summed",
        "the loaded total divided by the grid rows plus the grid cols, adding the two dimensions instead of multiplying them (a wrong operation)",
        "(batch_size * batch_count + addon) / (grid_rows + grid_cols)",
    ),
    (
        "precedence",
        "the batch size times the sum of the batch count and the addon over the grid area, adding before multiplying instead of multiplying first then adding (a wrong operation)",
        "(batch_size * (batch_count + addon)) / (grid_rows * grid_cols)",
    ),
]
QUERIED = ["loaded_coverage", "loaded_total", "grid_area"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(batch_size, batch_count, addon, grid_rows, grid_cols):
    # Operation order mirrors the ADJ programs exactly (the two factors multiply, the addon is added — multiplication FIRST by precedence — to
    # form the loaded total, the multiplication forms the grid area, then the loaded total is divided by the grid area to form the compound
    # figure, so (a*b+c)/(d*e) evaluates as ((a*b+c)/(d*e))), so the Python option value and the engine result are the same IEEE-double (well
    # within the 1e-9 tolerance).
    loaded = batch_size * batch_count + addon
    area = grid_rows * grid_cols
    return {
        "loaded_coverage": loaded / area,
        "loaded_total": loaded,
        "grid_area": area,
        "summed": loaded / (grid_rows + grid_cols),
        "precedence": (batch_size * (batch_count + addon)) / area,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for batch_size, batch_count, addon, grid_rows, grid_cols in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only +, *, and / over positive quantities, so positivity is
        # automatic — no positivity guards are needed. batch_size >= 2 keeps the precedence distractor a*(b+c) distinct from the honest a*b+c.
        assert (
            batch_size >= 2
            and batch_count >= 2
            and addon >= 2
            and grid_rows >= 2
            and grid_cols >= 2
        ), (batch_size, batch_count, addon, grid_rows, grid_cols)
        fv = family_values(batch_size, batch_count, addon, grid_rows, grid_cols)
        for key, v in fv.items():
            assert v > 0, (key, batch_size, batch_count, addon, grid_rows, grid_cols, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    batch_size,
                    batch_count,
                    addon,
                    grid_rows,
                    grid_cols,
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
                batch_size,
                batch_count,
                addon,
                grid_rows,
                grid_cols,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r155lca-{idx + 1:02d}",
                "qtype": "loaded_coverage",
                "stem": (
                    f"A coverage study records a batch of {num(batch_size)} by {num(batch_count)} with an "
                    f"addon of {num(addon)} spread across a grid of {num(grid_rows)} rows by {num(grid_cols)} cols. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe batch_size({num(batch_size)})\n"
                    f"observe batch_count({num(batch_count)})\n"
                    f"observe addon({num(addon)})\n"
                    f"observe grid_rows({num(grid_rows)})\n"
                    f"observe grid_cols({num(grid_cols)})\n"
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
            "ADJ-LADDER rung 155 — loaded coverage from FIVE stated quantities (a PRODUCT-PLUS-TERM numerator over a two-term PRODUCT, operator "
            "precedence). Rungs 153/154 divided a three-term SUM and a mixed-op numerator by a two-term product; rung-155 keeps the two-term "
            "product denominator but makes the numerator a PRODUCT-PLUS-TERM — two factors multiplied and an addon added, a loaded total "
            "(batch_size * batch_count + addon) divided by an AREA (grid_rows * grid_cols): a product-plus-term numerator over a two-term product "
            "(a*b+c)/(d*e). The multiplication binds tighter than the addition, so the numerator is (a*b)+c and NOT a*(b+c). From a loaded total "
            "(batch_size * batch_count + addon) divided by a grid area (grid_rows * grid_cols), compute the loaded coverage "
            "((batch_size*batch_count+addon)/(grid_rows*grid_cols)), the loaded total (batch_size*batch_count+addon), or the grid area "
            "(grid_rows*grid_cols). Each item is a compute_dimensioned program (observe the five quantities, let answer = formula); the ADJ engine "
            "carries the arithmetic — a PRODUCT-PLUS-TERM NUMERATOR OVER A TWO-TERM PRODUCT (a*b+c)/(d*e) (multiply the two factors FIRST, add the "
            "addon, multiply the two dimensions into an area, then divide the loaded total by the area). As on the over-a-product rungs, dividing "
            "by two factors in turn equals dividing by their product (x/d/e = x/(d*e)), so that is not a wrong distractor; the two slips on the "
            "panel are the WRONG DENOMINATOR OPERATION — adding the two dimensions instead of multiplying ((a*b+c)/(d+e), a perimeter-style total "
            "where an area belongs) — and the OPERATOR PRECEDENCE ERROR — adding the addon to the count before multiplying ((a*(b+c))/(d*e)) "
            "instead of multiplying first then adding. The rung sits at the crossing of its two questions: 'did you MULTIPLY the grid dimensions, "
            "or add them?' and 'did you do a*b FIRST then add c, or add b+c first then multiply?'. The harness matches the scalar to the printed "
            "options. Contamination-safe: every figure is built only from the five observed quantities via +, *, and / — no constant leaks, and "
            "neither the loaded total, the grid area, nor the loaded coverage ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. This rung uses only +, *, and / over positive "
            "quantities, so every figure is automatically positive — no positivity guards are needed (and batch_size >= 2 keeps the precedence "
            "distractor distinct from the honest numerator) — and the five family values are kept pairwise distinct with all three queried "
            "readouts varying across the panel, all asserted strictly positive at build time."
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
