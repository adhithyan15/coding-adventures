"""Generate rung-141 (grid density / a SUM-numerator over a PRODUCT — divide a total by an area) items.json.

Rung 141 **opens the fourth two-part-denominator family: OVER A PRODUCT**. The ladder walked the numerator-op x denominator-shape matrix
across rungs 130-140 for three denominators — a rate `c/d`, a sum `c+d`, and a difference `c-d`. The fourth elementary two-part denominator
is a PRODUCT `c*d`, and rung-141 opens that column with a SUM numerator, `(a+b)/(c*d)`.

`(a+b)/(c*d)` is a SUM `a+b` divided by a PRODUCT `c*d` (an area). The sum `a+b` binds and stays grouped over the bar, and the two-part
denominator `c*d` is ONE area the whole numerator is divided by. A product denominator has its own two canonical slips, and they are NOT the
sum/difference/rate slips: dividing by two factors in turn IS the same as dividing by their product (`x/c/d = x/(c*d)`), so that is not a
wrong distractor. The two canonical divide-by-a-product slips that a student actually makes are: using the WRONG denominator operation,
ADDING the two dimensions instead of multiplying them (`(a+b)/(c+d)` — a perimeter-style total instead of an area), and INVERTING the ratio,
dividing the area by the total instead of the total by the area (`(c*d)/(a+b)` — the reciprocal, the ratio upside down).

The setup: a `north_count` combined with a `south_count` (a combined count `north_count + south_count`), spread over a grid formed from
`grid_rows` times `grid_cols` (a grid area `grid_rows * grid_cols`). The figures are:

  GRID DENSITY   (north_count + south_count) / (grid_rows * grid_cols)  [ sum-numerator OVER a product: combined count / grid area ]
  COMBINED COUNT north_count + south_count                            [ the sum numerator (divided by the grid area) ]
  GRID AREA      grid_rows * grid_cols                                [ the product the combined count is divided by ]

The **grid density** is the ladder's first **(a sum) over (a product) as a headline** — a density (how much combined count sits in each cell
of the grid area), framed as a *density* to keep it dimensionless-clean, the same discipline rungs 100/.../139/140 used for their ratios,
spans, concentrations, densities, indices, slopes. (The combined count `a+b` and the grid area `c*d` ride alongside as component readouts,
so the panel teaches the whole calculation — exactly as rungs 47-140 shipped their component figures beside the headline. The two components
anchor the "add the counts FIRST, multiply out the area, then divide the count by the area" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the addition to form the combined count, the multiplication to form the grid area, then the division of the combined count by
the grid area to form the compound figure (so (a+b)/(c*d) evaluates as ((a+b)/(c*d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../139/140. This rung exercises the engine across a
**sum divided by a product** — the fact that `(a+b)/(c*d)` is one sum over one area and NOT `(a+b)/(c+d)` and NOT `(c*d)/(a+b)` made
computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../139/140 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `/`, and `*` — **no structural
constants** — so no numeric literal appears in any program, and neither the combined count, the grid area, nor the grid density is ever a
literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`north_count`,
`south_count`, `grid_rows`, `grid_cols`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  ADDED      (north_count + south_count) / (grid_rows + grid_cols)  divide the combined count by the SUM of the dimensions instead of their
                                                                product, using a perimeter-style total where an area belongs (the wrong
                                                                denominator operation), and
  INVERTED   (grid_rows * grid_cols) / (north_count + south_count)  divide the grid area BY the combined count, the ratio upside down (the
                                                                reciprocal of the grid density, the wrong direction),

which are exactly the mistakes a student makes with a product denominator (adding the dimensions instead of multiplying, or inverting the
ratio). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `+`, `*`, and `/` over positive quantities, so **every figure is automatically positive**
(no subtraction anywhere) — like rungs 128/130/131/132/134/135, no positivity guards are needed. Every observed quantity is `>= 2`. Every
family member is asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give distinct grid densities, distinct
combined counts, and distinct grid areas so all three queried readouts vary across the panel; the five family values are pairwise distinct
with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (NORTH_COUNT, SOUTH_COUNT, GRID_ROWS, GRID_COLS) — a combined count (north_count + south_count) divided by a grid area (grid_rows *
# grid_cols), giving the grid density as a sum over a product (a+b)/(c*d). This rung uses only +, *, and / over positive quantities, so every
# figure is automatically positive; no positivity guards are needed. The seven tables give distinct combined counts (a+b), distinct grid
# areas (c*d), and distinct grid densities ((a+b)/(c*d)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (4, 2, 2, 5),      # count = 6,  area = 10, density = 0.6
    (5, 3, 2, 3),      # count = 8,  area = 6,  density = 1.333...
    (6, 4, 2, 4),      # count = 10, area = 8,  density = 1.25
    (7, 5, 3, 5),      # count = 12, area = 15, density = 0.8
    (9, 5, 4, 5),      # count = 14, area = 20, density = 0.7
    (5, 4, 3, 4),      # count = 9,  area = 12, density = 0.75
    (9, 7, 2, 7),      # count = 16, area = 14, density = 1.142...
]

# The option family (5 members), all built from the four observed quantities via +, *, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "grid_density",
        "grid density (the combined count divided by the grid area)",
        "(north_count + south_count) / (grid_rows * grid_cols)",
    ),
    (
        "combined_count",
        "the combined count (the north count plus the south count, the numerator that is divided by the grid area)",
        "north_count + south_count",
    ),
    (
        "grid_area",
        "the grid area (the grid rows times the grid cols, the product the combined count is divided by)",
        "grid_rows * grid_cols",
    ),
    (
        "added",
        "the combined count divided by the grid rows plus the grid cols, using the sum of the dimensions instead of their product as the divisor (a wrong operation)",
        "(north_count + south_count) / (grid_rows + grid_cols)",
    ),
    (
        "inverted",
        "the grid area divided by the combined count, the ratio upside down instead of the combined count over the grid area (a wrong operation)",
        "(grid_rows * grid_cols) / (north_count + south_count)",
    ),
]
QUERIED = ["grid_density", "combined_count", "grid_area"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(north_count, south_count, grid_rows, grid_cols):
    # Operation order mirrors the ADJ programs exactly (the addition forms the combined count, the multiplication forms the grid area, then
    # the combined count is divided by the grid area to form the compound figure, so (a+b)/(c*d) evaluates as ((a+b)/(c*d))), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    count = north_count + south_count
    area = grid_rows * grid_cols
    return {
        "grid_density": count / area,
        "combined_count": count,
        "grid_area": area,
        "added": count / (grid_rows + grid_cols),
        "inverted": area / count,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for north_count, south_count, grid_rows, grid_cols in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only +, *, and / over positive quantities, so positivity is
        # automatic — no positivity guards are needed.
        assert (
            north_count >= 2
            and south_count >= 2
            and grid_rows >= 2
            and grid_cols >= 2
        ), (north_count, south_count, grid_rows, grid_cols)
        fv = family_values(north_count, south_count, grid_rows, grid_cols)
        for key, v in fv.items():
            assert v > 0, (key, north_count, south_count, grid_rows, grid_cols, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    north_count,
                    south_count,
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
                north_count,
                south_count,
                grid_rows,
                grid_cols,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r141gda-{idx + 1:02d}",
                "qtype": "grid_density",
                "stem": (
                    f"A grid study records a north count of {num(north_count)} combined with a south count of "
                    f"{num(south_count)}, spread over a grid of {num(grid_rows)} rows by {num(grid_cols)} cols. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe north_count({num(north_count)})\n"
                    f"observe south_count({num(south_count)})\n"
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
            "ADJ-LADDER rung 141 — grid density from four stated quantities (a NEW panel: grid, OPENING the fourth two-part-denominator "
            "family, OVER A PRODUCT). The ladder walked the numerator-op x denominator-shape matrix (rungs 130-140) for a rate (c/d), a "
            "sum (c+d), and a difference (c-d); the fourth elementary two-part denominator is a PRODUCT (c*d), and rung-141 opens that "
            "column with a SUM numerator (a+b)/(c*d). From a combined count (north_count + south_count) divided by a grid area (grid_rows "
            "* grid_cols), compute the grid density ((north_count+south_count)/(grid_rows*grid_cols)), the combined count "
            "(north_count+south_count), or the grid area (grid_rows*grid_cols). Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a SUM NUMERATOR OVER A PRODUCT (a+b)/(c*d) "
            "(add the counts, multiply out the area, then divide the count by the area — the two-part denominator is ONE area, not two "
            "divisors). A product denominator has its own slips: dividing by two factors in turn equals dividing by their product "
            "(x/c/d = x/(c*d)), so that is not a wrong distractor; the two canonical slips are used instead. The harness matches the "
            "scalar to the printed options. The grid density is a density (how much combined count sits in each cell of the grid area), "
            "framed as a DENSITY so the dimensionless value stays honest. Contamination-safe: every figure is built only from the four "
            "observed quantities via +, *, and / — no constant leaks, and neither the combined count, the grid area, nor the grid density "
            "ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make with a product denominator: dividing by the SUM of the dimensions instead of their product ((a+b)/(c+d), a "
            "perimeter-style total where an area belongs, a wrong operation) and INVERTING the ratio ((c*d)/(a+b), the area over the "
            "count, the reciprocal, a wrong operation). The core confusion tested is that (a+b)/(c*d) is one sum over one area, not "
            "(a+b)/(c+d) and not (c*d)/(a+b). This rung uses only +, *, and / over positive quantities, so every figure is automatically "
            "positive — no positivity guards are needed — and the five family values are kept pairwise distinct with all three queried "
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
