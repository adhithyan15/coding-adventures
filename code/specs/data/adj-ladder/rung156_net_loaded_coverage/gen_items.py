"""Generate rung-156 (net loaded coverage / a PRODUCT-MINUS-TERM numerator over a TWO-TERM PRODUCT — five-quantity, product denominator, precedence + subtraction) items.json.

Rung 156 closes the four numerator shapes over the two-term PRODUCT denominator. Over the product denominator `(d*e)`, rung-153 divided a
three-term SUM `(a+b+c)`, rung-154 a mixed-op sum/difference `(a+b-c)`, rung-155 a product-plus-term `(a*b+c)`; rung-156 is the last of the
four — a PRODUCT-MINUS-TERM numerator, `(a*b-c)/(d*e)`: a batch quantity formed by multiplying two factors, then a spoilage taken off, divided by
a grid AREA. It combines OPERATOR PRECEDENCE (the multiplication `a*b` binds tighter than the `-c`, so the numerator is `(a*b)-c` and NOT
`a*(b-c)`) with a SUBTRACTION (so this rung reintroduces the numerator net guard). This mirrors rung-148's product-minus-term numerator, now over
a product (area) denominator instead of a lone divisor — the four-numerator family (145-148 over a lone divisor) is now mirrored in full over the
product denominator (153-156).

`(a*b-c)/(d*e)` forms a net loaded numerator `a*b-c` (two factors multiplied, then a spoilage subtracted) over the PRODUCT of two dimensions `d*e`
(an area). Both sides are totals that must be formed BEFORE the division: the two factors multiply and the spoilage is subtracted (multiplication
FIRST, by precedence) to form the net loaded numerator, the two dimensions multiply into the denominator AREA, and only then is the net loaded
total divided by the area. As on the over-a-product rungs (141-144, 153-155), a product denominator has its own canonical slip that is NOT the
sum/difference slip: dividing by two factors in turn IS the same as dividing by their product (`x/d/e = x/(d*e)`), so that is not a wrong
distractor. Two slips are put on the panel: the WRONG DENOMINATOR OPERATION — ADDING the two dimensions instead of multiplying them,
`(a*b-c)/(d+e)` (a perimeter-style total where an area belongs) — and the NUMERATOR SIGN ERROR carried in by the product-minus-term numerator —
ADDING the spoilage instead of subtracting it, `(a*b+c)/(d*e)`. The rung thus sits at the crossing of its two questions: "did you MULTIPLY the
grid dimensions, or add them?" and "did you SUBTRACT the spoilage, or add it?".

The setup: two factors `factor_one`, `factor_two` are multiplied and a `spoilage` is taken off (a net loaded total `factor_one * factor_two -
spoilage`) and spread across a grid formed from `grid_rows` times `grid_cols` (a grid area `grid_rows * grid_cols`). The figures are:

  NET LOADED COVERAGE  (factor_one * factor_two - spoilage) / (grid_rows * grid_cols)  [ PRODUCT-MINUS-TERM numerator OVER a two-term product: net loaded total / grid area ]
  NET LOADED TOTAL     factor_one * factor_two - spoilage                            [ the product-minus-term numerator total (divided by the grid area) ]
  GRID AREA            grid_rows * grid_cols                                          [ the two-term denominator product (the net loaded total is divided by) ]

The **net loaded coverage** is the headline; the **net loaded total** (product minus spoilage) and the **grid area** (rows times cols) ride
alongside as component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline rungs
47-155 shipped. Critically, the grid area `(d*e)` is the *legitimate* multiply-the-dimensions figure, whereas the distractor `(a*b-c)/(d+e)` is
the *slip* of adding the dimensions where their product belongs, and `(a*b+c)/(d*e)` is the *slip* of adding the spoilage where it should be
subtracted — the panel puts the honest net loaded coverage and both wrong-operation slips side by side.

Each figure is a `compute_dimensioned` program (`observe` the five quantities + `let answer = formula`); the ADJ engine carries the arithmetic —
the multiplication of the two factors, the subtraction of the spoilage (multiplication first by precedence), the multiplication to form the grid
area, then the division of the net loaded total by the grid area to form the compound figure (so (a*b-c)/(d*e) evaluates as ((a*b-c)/(d*e))) —
and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../154/155.
This rung exercises the engine across a **product-minus-term numerator divided by a two-term product** — the fact that `(a*b-c)/(d*e)` multiplies
BEFORE it subtracts and is NOT `(a*(b-c))/(d*e)` and NOT `(a*b+c)/(d*e)` and NOT `(a*b-c)/(d+e)` made computable. The golds are exact rationals
rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs 100/.../154/155 relied on (well within the harness's 1e-9
tolerance).

Contamination-safe by construction: every formula is built ONLY from the five observed quantities via `+`, `-`, `*`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the net loaded total, the grid area, nor the net loaded coverage is ever
a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`factor_one`, `factor_two`,
`spoilage`, `grid_rows`, `grid_cols`) so no numeral hides inside a variable name.

The five options are a tight family over the same five quantities: the three real readouts plus the two classic slips —

  SUMMED  (factor_one * factor_two - spoilage) / (grid_rows + grid_cols)  ADD the two dimensions instead of multiplying them, the wrong
                                                                          denominator operation (a perimeter-style total where an area belongs), and
  ADDED   (factor_one * factor_two + spoilage) / (grid_rows * grid_cols)  ADD the spoilage instead of subtracting it, the numerator sign error,

which are exactly the mistakes a student makes loading a net batch over a grid area (adding the dimensions instead of multiplying, or adding the
spoilage instead of subtracting). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: the numerator carries a subtraction (`factor_one * factor_two - spoilage`), so — exactly as on the difference rungs
— one positivity guard is needed: the net loaded numerator is kept `>= 2` at build time (so the net loaded total, and therefore the net loaded
coverage, are strictly positive). The denominator uses only `*` over positive quantities (automatically positive), and the two distractors'
numerators (`a*b-c` for SUMMED, `a*b+c` for ADDED) are both positive given the net guard. Every observed quantity is `>= 2`. Every family member
is asserted `> 0` at build time. The seven tables give distinct net loaded coverages, distinct net loaded totals, and distinct grid areas so all
three queried readouts vary across the panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (FACTOR_ONE, FACTOR_TWO, SPOILAGE, GRID_ROWS, GRID_COLS) — two factors multiplied and a spoilage taken off (factor_one * factor_two -
# spoilage) over a grid area (grid_rows * grid_cols), giving the net loaded coverage as a product-minus-term numerator over a two-term product
# (a*b-c)/(d*e). The numerator carries a subtraction, so the net loaded numerator is guarded >= 2 below (keeping the net loaded total and the net
# loaded coverage strictly positive); the denominator uses only * over positives (automatically positive). The seven tables give distinct net
# loaded totals (a*b-c), distinct grid areas (d*e), and distinct net loaded coverages ((a*b-c)/(d*e)); the five family values are asserted
# pairwise-distinct below.
TABLES = [
    (4, 4, 2, 2, 5),      # net = 14, area = 10, coverage = 1.4
    (5, 4, 2, 2, 3),      # net = 18, area = 6,  coverage = 3.0
    (4, 6, 4, 2, 4),      # net = 20, area = 8,  coverage = 2.5
    (6, 4, 2, 3, 5),      # net = 22, area = 15, coverage = 22/15
    (5, 6, 4, 4, 5),      # net = 26, area = 20, coverage = 1.3
    (6, 5, 6, 3, 4),      # net = 24, area = 12, coverage = 2.0
    (5, 7, 3, 2, 7),      # net = 32, area = 14, coverage = 32/14
]

# The option family (5 members), all built from the five observed quantities via +, -, *, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "net_loaded_coverage",
        "net loaded coverage (the net loaded total divided by the grid area)",
        "(factor_one * factor_two - spoilage) / (grid_rows * grid_cols)",
    ),
    (
        "net_loaded_total",
        "the net loaded total (the two factors multiplied then the spoilage subtracted, the numerator that is divided by the grid area)",
        "factor_one * factor_two - spoilage",
    ),
    (
        "grid_area",
        "the grid area (the grid rows times the grid cols, the denominator the net loaded total is divided by)",
        "grid_rows * grid_cols",
    ),
    (
        "summed",
        "the net loaded total divided by the grid rows plus the grid cols, adding the two dimensions instead of multiplying them (a wrong operation)",
        "(factor_one * factor_two - spoilage) / (grid_rows + grid_cols)",
    ),
    (
        "added",
        "the two factors multiplied plus the spoilage divided by the grid area, adding the spoilage instead of subtracting it (a wrong operation)",
        "(factor_one * factor_two + spoilage) / (grid_rows * grid_cols)",
    ),
]
QUERIED = ["net_loaded_coverage", "net_loaded_total", "grid_area"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(factor_one, factor_two, spoilage, grid_rows, grid_cols):
    # Operation order mirrors the ADJ programs exactly (the two factors multiply, the spoilage is subtracted — multiplication FIRST by
    # precedence — to form the net loaded total, the multiplication forms the grid area, then the net loaded total is divided by the grid area to
    # form the compound figure, so (a*b-c)/(d*e) evaluates as ((a*b-c)/(d*e))), so the Python option value and the engine result are the same
    # IEEE-double (well within the 1e-9 tolerance).
    net = factor_one * factor_two - spoilage
    area = grid_rows * grid_cols
    return {
        "net_loaded_coverage": net / area,
        "net_loaded_total": net,
        "grid_area": area,
        "summed": net / (grid_rows + grid_cols),
        "added": (factor_one * factor_two + spoilage) / area,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for factor_one, factor_two, spoilage, grid_rows, grid_cols in TABLES:
        # Every observed quantity is a plain positive number >= 2.
        assert (
            factor_one >= 2
            and factor_two >= 2
            and spoilage >= 2
            and grid_rows >= 2
            and grid_cols >= 2
        ), (factor_one, factor_two, spoilage, grid_rows, grid_cols)
        # The numerator carries a subtraction, so guard the net loaded numerator >= 2 (keeping the net loaded total and the net loaded coverage
        # strictly positive).
        net = factor_one * factor_two - spoilage
        assert net >= 2, (factor_one, factor_two, spoilage, net)
        fv = family_values(factor_one, factor_two, spoilage, grid_rows, grid_cols)
        for key, v in fv.items():
            assert v > 0, (key, factor_one, factor_two, spoilage, grid_rows, grid_cols, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    factor_one,
                    factor_two,
                    spoilage,
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
                factor_one,
                factor_two,
                spoilage,
                grid_rows,
                grid_cols,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r156nlc-{idx + 1:02d}",
                "qtype": "net_loaded_coverage",
                "stem": (
                    f"A coverage study records a batch of {num(factor_one)} by {num(factor_two)} with a "
                    f"spoilage of {num(spoilage)} spread across a grid of {num(grid_rows)} rows by {num(grid_cols)} cols. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe factor_one({num(factor_one)})\n"
                    f"observe factor_two({num(factor_two)})\n"
                    f"observe spoilage({num(spoilage)})\n"
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
            "ADJ-LADDER rung 156 — net loaded coverage from FIVE stated quantities (a PRODUCT-MINUS-TERM numerator over a two-term PRODUCT, "
            "precedence + subtraction). This closes the four numerator shapes over the two-term product denominator: rungs 153/154/155 divided a "
            "three-term SUM, a mixed-op numerator, and a product-plus-term by a two-term product; rung-156 divides a PRODUCT-MINUS-TERM — two "
            "factors multiplied and a spoilage taken off, a net loaded total (factor_one * factor_two - spoilage) divided by an AREA "
            "(grid_rows * grid_cols): a product-minus-term numerator over a two-term product (a*b-c)/(d*e). The multiplication binds tighter than "
            "the subtraction, so the numerator is (a*b)-c and NOT a*(b-c). From a net loaded total (factor_one * factor_two - spoilage) divided "
            "by a grid area (grid_rows * grid_cols), compute the net loaded coverage ((factor_one*factor_two-spoilage)/(grid_rows*grid_cols)), "
            "the net loaded total (factor_one*factor_two-spoilage), or the grid area (grid_rows*grid_cols). Each item is a compute_dimensioned "
            "program (observe the five quantities, let answer = formula); the ADJ engine carries the arithmetic — a PRODUCT-MINUS-TERM NUMERATOR "
            "OVER A TWO-TERM PRODUCT (a*b-c)/(d*e) (multiply the two factors FIRST, subtract the spoilage, multiply the two dimensions into an "
            "area, then divide the net loaded total by the area). As on the over-a-product rungs, dividing by two factors in turn equals dividing "
            "by their product (x/d/e = x/(d*e)), so that is not a wrong distractor; the two slips on the panel are the WRONG DENOMINATOR "
            "OPERATION — adding the two dimensions instead of multiplying ((a*b-c)/(d+e), a perimeter-style total where an area belongs) — and "
            "the NUMERATOR SIGN ERROR — adding the spoilage instead of subtracting it ((a*b+c)/(d*e)). The rung sits at the crossing of its two "
            "questions: 'did you MULTIPLY the grid dimensions, or add them?' and 'did you SUBTRACT the spoilage, or add it?'. The harness matches "
            "the scalar to the printed options. Contamination-safe: every figure is built only from the five observed quantities via +, -, *, "
            "and / — no constant leaks, and neither the net loaded total, the grid area, nor the net loaded coverage ever appears as a literal "
            "(each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The "
            "numerator carries a subtraction, so the net loaded numerator is guarded >= 2 (keeping the net loaded total and the net loaded "
            "coverage strictly positive); the denominator uses only * over positives. The five family values are kept pairwise distinct with all "
            "three queried readouts varying across the panel, all asserted strictly positive at build time."
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
