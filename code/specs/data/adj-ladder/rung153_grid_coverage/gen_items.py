"""Generate rung-153 (grid coverage / a THREE-TERM sum over a TWO-TERM PRODUCT — five-quantity, product denominator) items.json.

Rung 153 opens a NEW denominator axis for the five-quantity family. The two-term-denominator mini-matrix (rungs 149-152) explored SUM and
DIFFERENCE denominators; rung-153 makes the two-term denominator a **PRODUCT** — a three-term sum over a two-term product, `(a+b+c)/(d*e)`:
three parts are pooled and divided by an AREA formed from a rows-times-cols product. It is the grid-coverage shape, `(a+b+c)/(d*e)`, the first
five-quantity rung whose denominator is a product (an area).

`(a+b+c)/(d*e)` sums THREE parts `a+b+c` over the PRODUCT of two dimensions `d*e` (an area). Both sides are totals that must be formed BEFORE
the division: all three parts pool into the numerator total, the two dimensions multiply into the denominator AREA, and only then is the part
total divided by the area. As on the over-a-product rungs (141-144), a product denominator has its own canonical slip that is NOT the
sum/difference slip: dividing by two factors in turn IS the same as dividing by their product (`x/d/e = x/(d*e)`), so that is not a wrong
distractor. The product-denominator slip a student actually makes is using the WRONG denominator operation — ADDING the two dimensions
instead of multiplying them, `(a+b+c)/(d+e)` (a perimeter-style total where an area belongs). The other slip carries over from the
five-quantity family — **dropping a numerator term**, `(a+b)/(d*e)` (pooling only two of the three parts).

The setup: three parts `part_one`, `part_two`, `part_three` are pooled (a part total `part_one + part_two + part_three`) and spread across a
grid formed from `grid_rows` times `grid_cols` (a grid area `grid_rows * grid_cols`). The figures are:

  GRID COVERAGE  (part_one + part_two + part_three) / (grid_rows * grid_cols)  [ THREE-TERM sum OVER a two-term product: part total / grid area ]
  PART TOTAL     part_one + part_two + part_three                           [ the three-term numerator total (divided by the grid area) ]
  GRID AREA      grid_rows * grid_cols                                      [ the two-term denominator product (the part total is divided by) ]

The **grid coverage** is the headline; the **part total** (all three parts) and the **grid area** (rows times cols) ride alongside as
component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline rungs 47-152
shipped. Critically, the grid area `(d*e)` is the *legitimate* multiply-the-dimensions figure, whereas the distractor `(a+b+c)/(d+e)` is the
*slip* of adding the dimensions where their product belongs — the panel puts the honest area and the wrong-operation slip side by side so the
difference is exactly "did you MULTIPLY the grid dimensions, or add them?".

Each figure is a `compute_dimensioned` program (`observe` the five quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the two additions to pool the parts, the multiplication to form the grid area, then the division of the part total by the grid
area to form the compound figure (so (a+b+c)/(d*e) evaluates as ((a+b+c)/(d*e))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../151/152. This rung exercises the engine across a
**three-term sum divided by a two-term product** — the fact that `(a+b+c)/(d*e)` divides by an AREA and is NOT `(a+b+c)/(d+e)` and NOT
`(a+b)/(d*e)` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same
way rungs 100/.../151/152 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the five observed quantities via `+`, `*`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the part total, the grid area, nor the grid coverage is ever a
literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`part_one`, `part_two`,
`part_three`, `grid_rows`, `grid_cols`) so no numeral hides inside a variable name.

The five options are a tight family over the same five quantities: the three real readouts plus the two classic slips —

  SUMMED        (part_one + part_two + part_three) / (grid_rows + grid_cols)  ADD the two dimensions instead of multiplying them, the wrong
                                                                             denominator operation (a perimeter-style total where an area
                                                                             belongs), and
  DROPPED_PART  (part_one + part_two) / (grid_rows * grid_cols)              pool only TWO of the three parts, dropping a part from the
                                                                             numerator,

which are exactly the mistakes a student makes pooling parts over a grid area (adding the dimensions instead of multiplying, or dropping a
numerator term). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `+`, `*`, and `/` over positive quantities, so **every figure is automatically positive**
(no subtraction anywhere) — no positivity guards are needed. Every observed quantity is `>= 2`. Every family member is asserted `> 0` at
build time. The seven tables give distinct grid coverages, distinct part totals, and distinct grid areas so all three queried readouts vary
across the panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PART_ONE, PART_TWO, PART_THREE, GRID_ROWS, GRID_COLS) — three parts pooled (part_one + part_two + part_three) over a grid area (grid_rows *
# grid_cols), giving the grid coverage as a three-term sum over a two-term product (a+b+c)/(d*e). This rung uses only +, *, and / over
# positive quantities, so every figure is automatically positive; no positivity guards are needed. The seven tables give distinct part totals
# (a+b+c), distinct grid areas (d*e), and distinct grid coverages ((a+b+c)/(d*e)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (2, 4, 6, 2, 5),      # part = 12, area = 10, coverage = 1.2
    (3, 5, 12, 2, 3),     # part = 20, area = 6,  coverage = 20/6
    (4, 6, 20, 2, 4),     # part = 30, area = 8,  coverage = 3.75
    (3, 9, 12, 3, 5),     # part = 24, area = 15, coverage = 1.6
    (5, 9, 21, 4, 5),     # part = 35, area = 20, coverage = 1.75
    (6, 10, 32, 3, 4),    # part = 48, area = 12, coverage = 4.0
    (7, 11, 18, 2, 7),    # part = 36, area = 14, coverage = 36/14
]

# The option family (5 members), all built from the five observed quantities via +, *, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "grid_coverage",
        "grid coverage (the part total divided by the grid area)",
        "(part_one + part_two + part_three) / (grid_rows * grid_cols)",
    ),
    (
        "part_total",
        "the part total (all three parts added, the numerator that is divided by the grid area)",
        "part_one + part_two + part_three",
    ),
    (
        "grid_area",
        "the grid area (the grid rows times the grid cols, the denominator the part total is divided by)",
        "grid_rows * grid_cols",
    ),
    (
        "summed",
        "the part total divided by the grid rows plus the grid cols, adding the two dimensions instead of multiplying them (a wrong operation)",
        "(part_one + part_two + part_three) / (grid_rows + grid_cols)",
    ),
    (
        "dropped_part",
        "the first two parts divided by the grid area, pooling only two of the three parts and dropping a part from the numerator (a wrong operation)",
        "(part_one + part_two) / (grid_rows * grid_cols)",
    ),
]
QUERIED = ["grid_coverage", "part_total", "grid_area"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(part_one, part_two, part_three, grid_rows, grid_cols):
    # Operation order mirrors the ADJ programs exactly (the additions pool the parts, the multiplication forms the grid area, then the part
    # total is divided by the grid area to form the compound figure, so (a+b+c)/(d*e) evaluates as ((a+b+c)/(d*e))), so the Python option
    # value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    part = part_one + part_two + part_three
    area = grid_rows * grid_cols
    return {
        "grid_coverage": part / area,
        "part_total": part,
        "grid_area": area,
        "summed": part / (grid_rows + grid_cols),
        "dropped_part": (part_one + part_two) / area,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for part_one, part_two, part_three, grid_rows, grid_cols in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only +, *, and / over positive quantities, so positivity is
        # automatic — no positivity guards are needed.
        assert (
            part_one >= 2
            and part_two >= 2
            and part_three >= 2
            and grid_rows >= 2
            and grid_cols >= 2
        ), (part_one, part_two, part_three, grid_rows, grid_cols)
        fv = family_values(part_one, part_two, part_three, grid_rows, grid_cols)
        for key, v in fv.items():
            assert v > 0, (key, part_one, part_two, part_three, grid_rows, grid_cols, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    part_one,
                    part_two,
                    part_three,
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
                part_one,
                part_two,
                part_three,
                grid_rows,
                grid_cols,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r153gca-{idx + 1:02d}",
                "qtype": "grid_coverage",
                "stem": (
                    f"A coverage study records three parts of {num(part_one)}, {num(part_two)}, and "
                    f"{num(part_three)} spread across a grid of {num(grid_rows)} rows by {num(grid_cols)} cols. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe part_one({num(part_one)})\n"
                    f"observe part_two({num(part_two)})\n"
                    f"observe part_three({num(part_three)})\n"
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
            "ADJ-LADDER rung 153 — grid coverage from FIVE stated quantities (a NEW denominator axis: a two-term PRODUCT). The "
            "two-term-denominator mini-matrix (rungs 149-152) explored SUM and DIFFERENCE denominators; rung-153 makes the two-term "
            "denominator a PRODUCT — a three-term sum over a two-term product (a+b+c)/(d*e): three parts pooled and divided by an AREA "
            "(grid_rows * grid_cols). From a part total (part_one + part_two + part_three) divided by a grid area (grid_rows * grid_cols), "
            "compute the grid coverage ((part_one+part_two+part_three)/(grid_rows*grid_cols)), the part total "
            "(part_one+part_two+part_three), or the grid area (grid_rows*grid_cols). Each item is a compute_dimensioned program (observe the "
            "five quantities, let answer = formula); the ADJ engine carries the arithmetic — a THREE-TERM SUM OVER A TWO-TERM PRODUCT "
            "(a+b+c)/(d*e) (pool all three parts, multiply the two dimensions into an area FIRST, then divide the part total by the area). "
            "As on the over-a-product rungs, dividing by two factors in turn equals dividing by their product (x/d/e = x/(d*e)), so that is "
            "not a wrong distractor; the product-denominator slip is the WRONG DENOMINATOR OPERATION — adding the two dimensions instead of "
            "multiplying ((a+b+c)/(d+e), a perimeter-style total where an area belongs) — alongside the carried-over DROPPING a numerator "
            "term ((a+b)/(d*e)). The panel puts the honest grid area (d*e) beside the wrong-operation slip ((d+e)) so the difference is "
            "exactly 'did you MULTIPLY the grid dimensions, or add them?'. The harness matches the scalar to the printed options. "
            "Contamination-safe: every figure is built only from the five observed quantities via +, *, and / — no constant leaks, and "
            "neither the part total, the grid area, nor the grid coverage ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. This rung uses only +, *, and / over "
            "positive quantities, so every figure is automatically positive — no positivity guards are needed — and the five family values "
            "are kept pairwise distinct with all three queried readouts varying across the panel, all asserted strictly positive at build "
            "time."
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
