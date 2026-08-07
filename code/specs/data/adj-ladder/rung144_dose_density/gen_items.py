"""Generate rung-144 (dose density / a QUOTIENT-numerator over a PRODUCT — divide a rate by an area) items.json.

Rung 144 **CLOSES the OVER-A-PRODUCT column** and, with it, the whole four-numerator x four-denominator compound-fraction matrix
(rungs 130-144). rung-141 put a SUM over a product, `(a+b)/(c*d)`; rung-142 a PRODUCT over a product, `(a*b)/(c*d)`; rung-143 a DIFFERENCE
over a product, `(a−b)/(c*d)`; rung-144 puts a **QUOTIENT** over a product, `(a/b)/(c*d)` — a per-split rate `a/b` divided by an area
product `c*d`. This is the quotient-numerator member of the over-a-product family — the numerator is itself a division, `(a/b)`, and the
denominator is still ONE area `c*d`.

`(a/b)/(c*d)` is a QUOTIENT `a/b` divided by a PRODUCT `c*d` (an area). The quotient `a/b` binds and stays grouped over the bar (the
per-split rate the division produces), and the two-part denominator `c*d` is ONE area the whole rate is divided by. As on rungs 141/142/143,
a product denominator has its own two canonical slips, and they are NOT the sum/difference/rate slips: dividing by two factors in turn IS the
same as dividing by their product (`x/c/d = x/(c*d)`), so that is not a wrong distractor. The two canonical divide-by-a-product slips that a
student actually makes are: using the WRONG denominator operation, ADDING the two dimensions instead of multiplying them (`(a/b)/(c+d)` — a
perimeter-style total instead of an area), and INVERTING the ratio, dividing the area by the rate instead of the rate by the area
(`(c*d)/(a/b)` — the reciprocal).

The setup: a `total_dose` is split evenly into `dose_split` portions (a per-split dose `total_dose / dose_split`), and that per-split dose is
spread over a grid formed from a `grid_rows` times a `grid_cols` (a grid area `grid_rows * grid_cols`). The figures are:

  DOSE DENSITY     (total_dose / dose_split) / (grid_rows * grid_cols)  [ quotient-numerator OVER a product: per-split dose / grid area ]
  PER-SPLIT DOSE   total_dose / dose_split                            [ the quotient numerator (divided by the grid area) ]
  GRID AREA        grid_rows * grid_cols                              [ the product the per-split dose is divided by ]

The **dose density** is a **(a quotient) over (a product) as a headline** — a density (how much per-split dose rides on each cell of the grid
area), framed as a *density* to keep it dimensionless-clean, the same discipline rungs 100/.../142/143 used for their ratios, spans,
concentrations, densities, indices, slopes. (The per-split dose `a/b` and the grid area `c*d` ride alongside as component readouts, so the
panel teaches the whole calculation — exactly as rungs 47-143 shipped their component figures beside the headline. The two components anchor
the "divide to the per-split dose FIRST, multiply out the area, then divide the rate by the area" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the division to form the per-split dose, the multiplication to form the grid area, then the division of the per-split dose by
the grid area to form the compound figure (so (a/b)/(c*d) evaluates as ((a/b)/(c*d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../142/143. This rung exercises the engine across a
**quotient divided by a product** — the fact that `(a/b)/(c*d)` is one rate over one area and NOT `(a/b)/(c+d)` and NOT `(c*d)/(a/b)`
made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../142/143 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `*`, and `+` — **no structural
constants** — so no numeric literal appears in any program, and neither the per-split dose, the grid area, nor the dose density is ever a
literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`total_dose`,
`dose_split`, `grid_rows`, `grid_cols`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  ADDED      (total_dose / dose_split) / (grid_rows + grid_cols)  divide the per-split dose by the SUM of the dimensions instead of their
                                                                  product, using a perimeter-style total where an area belongs (the wrong
                                                                  denominator operation), and
  INVERTED   (grid_rows * grid_cols) / (total_dose / dose_split)  divide the grid area BY the per-split dose, the ratio upside down (the
                                                                  reciprocal of the dose density, the wrong direction),

which are exactly the mistakes a student makes with a product denominator (adding the dimensions instead of multiplying, or inverting the
ratio). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: the numerator is a division, so — as on rungs 130/140 (the other quotient-numerator rungs) — every table is
built so the per-split dose divides EXACTLY: `total_dose % dose_split == 0` (asserted at build time), keeping the readouts clean integers /
simple rationals. This rung uses only `/`, `*`, and `+` over positive quantities, so **every figure is automatically positive** (no
subtraction anywhere) — like rungs 130/141/142, no positivity guards are needed; the exact-division check is a cleanliness constraint, not a
positivity one. Every observed quantity is `>= 2`. Every family member is asserted `> 0` at build time. The seven tables give distinct
per-split doses, distinct grid areas, and distinct dose densities so all three queried readouts vary across the panel; the five family
values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TOTAL_DOSE, DOSE_SPLIT, GRID_ROWS, GRID_COLS) — a per-split dose (total_dose / dose_split) divided by a grid area (grid_rows * grid_cols),
# giving the dose density as a quotient over a product (a/b)/(c*d). The numerator is a division, so every row is built so total_dose divides
# EXACTLY by dose_split (total_dose % dose_split == 0, asserted below), keeping the readouts clean. This rung uses only /, *, and + over
# positive quantities, so every figure is automatically positive; no positivity guards are needed. The seven tables give distinct per-split
# doses (a/b), distinct grid areas (c*d), and distinct dose densities ((a/b)/(c*d)); the five family values are asserted pairwise-distinct
# below.
TABLES = [
    (12, 2, 2, 5),     # per = 6,  area = 10, density = 0.6
    (24, 3, 2, 3),     # per = 8,  area = 6,  density = 1.333...
    (20, 2, 2, 4),     # per = 10, area = 8,  density = 1.25
    (36, 3, 3, 5),     # per = 12, area = 15, density = 0.8
    (28, 2, 4, 5),     # per = 14, area = 20, density = 0.7
    (27, 3, 3, 4),     # per = 9,  area = 12, density = 0.75
    (32, 2, 2, 7),     # per = 16, area = 14, density = 1.142...
]

# The option family (5 members), all built from the four observed quantities via /, *, and +. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "dose_density",
        "dose density (the per-split dose divided by the grid area)",
        "(total_dose / dose_split) / (grid_rows * grid_cols)",
    ),
    (
        "per_split_dose",
        "the per-split dose (the total dose divided by the dose split, the numerator that is divided by the grid area)",
        "total_dose / dose_split",
    ),
    (
        "grid_area",
        "the grid area (the grid rows times the grid cols, the product the per-split dose is divided by)",
        "grid_rows * grid_cols",
    ),
    (
        "added",
        "the per-split dose divided by the grid rows plus the grid cols, using the sum of the dimensions instead of their product as the divisor (a wrong operation)",
        "(total_dose / dose_split) / (grid_rows + grid_cols)",
    ),
    (
        "inverted",
        "the grid area divided by the per-split dose, the ratio upside down instead of the per-split dose over the grid area (a wrong operation)",
        "(grid_rows * grid_cols) / (total_dose / dose_split)",
    ),
]
QUERIED = ["dose_density", "per_split_dose", "grid_area"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(total_dose, dose_split, grid_rows, grid_cols):
    # Operation order mirrors the ADJ programs exactly (the division forms the per-split dose, the multiplication forms the grid area, then
    # the per-split dose is divided by the grid area to form the compound figure, so (a/b)/(c*d) evaluates as ((a/b)/(c*d))), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    per = total_dose / dose_split
    area = grid_rows * grid_cols
    return {
        "dose_density": per / area,
        "per_split_dose": per,
        "grid_area": area,
        "added": per / (grid_rows + grid_cols),
        "inverted": area / per,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for total_dose, dose_split, grid_rows, grid_cols in TABLES:
        # Every observed quantity is a plain positive number >= 2, AND the quotient numerator divides EXACTLY: total_dose % dose_split == 0,
        # keeping the per-split dose (and every readout built on it) a clean integer / simple rational. This rung uses only /, *, and + over
        # positive quantities, so positivity is automatic — no positivity guards are needed; the exact-division check is a cleanliness
        # constraint, not a positivity one.
        assert (
            total_dose >= 2
            and dose_split >= 2
            and grid_rows >= 2
            and grid_cols >= 2
        ), (total_dose, dose_split, grid_rows, grid_cols)
        assert total_dose % dose_split == 0, (total_dose, dose_split)
        fv = family_values(total_dose, dose_split, grid_rows, grid_cols)
        for key, v in fv.items():
            assert v > 0, (key, total_dose, dose_split, grid_rows, grid_cols, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    total_dose,
                    dose_split,
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
                total_dose,
                dose_split,
                grid_rows,
                grid_cols,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r144dda-{idx + 1:02d}",
                "qtype": "dose_density",
                "stem": (
                    f"A dosing study records a total dose of {num(total_dose)} split into "
                    f"{num(dose_split)} portions, spread over a grid of {num(grid_rows)} rows by "
                    f"{num(grid_cols)} cols. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe total_dose({num(total_dose)})\n"
                    f"observe dose_split({num(dose_split)})\n"
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
            "ADJ-LADDER rung 144 — dose density from four stated quantities (CLOSES the OVER-A-PRODUCT column and completes the "
            "four-numerator x four-denominator compound-fraction matrix, rungs 130-144). rung-141 put a sum over a product (a+b)/(c*d); "
            "rung-142 a product over a product (a*b)/(c*d); rung-143 a difference over a product (a−b)/(c*d); rung-144 puts a QUOTIENT over "
            "a product (a/b)/(c*d) — a per-split rate divided by an area product. From a per-split dose (total_dose / dose_split) divided "
            "by a grid area (grid_rows * grid_cols), compute the dose density ((total_dose/dose_split)/(grid_rows*grid_cols)), the per-split "
            "dose (total_dose/dose_split), or the grid area (grid_rows*grid_cols). Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a QUOTIENT NUMERATOR OVER A PRODUCT (a/b)/(c*d) "
            "(divide to the per-split dose, multiply out the area, then divide the rate by the area — the two-part denominator is ONE area, "
            "not two divisors). As on rungs 141/142/143, dividing by two factors in turn equals dividing by their product (x/c/d = "
            "x/(c*d)), so that is not a wrong distractor; the two canonical slips are used instead. The harness matches the scalar to the "
            "printed options. The dose density is a density (how much per-split dose rides on each cell of the grid area), framed as a "
            "DENSITY so the dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via /, *, and + — no constant leaks, and neither the per-split dose, the grid area, nor the dose density ever "
            "appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside "
            "a variable name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make with a product denominator: dividing by the SUM of the dimensions instead of their product ((a/b)/(c+d), a "
            "perimeter-style total where an area belongs, a wrong operation) and INVERTING the ratio ((c*d)/(a/b), the area over the rate, "
            "the reciprocal, a wrong operation). The core confusion tested is that (a/b)/(c*d) is one rate over one area, not (a/b)/(c+d) "
            "and not (c*d)/(a/b). The numerator is a division built to divide EXACTLY (total_dose % dose_split == 0) for clean readouts; "
            "this rung uses only /, *, and + over positive quantities, so every figure is automatically positive — no positivity guards are "
            "needed. The five family values are kept pairwise distinct with all three queried readouts varying across the panel, all "
            "asserted strictly positive at build time."
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
