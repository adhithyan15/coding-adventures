"""Generate rung-154 (net coverage / a MIXED-OP three-term numerator over a TWO-TERM PRODUCT — five-quantity, product denominator) items.json.

Rung 154 sits at the intersection of the two axes rung-153 opened. Rung-153 divided a THREE-TERM SUM by a two-term PRODUCT, `(a+b+c)/(d*e)`; rung-154
keeps the two-term product denominator but makes the numerator MIXED-OP — two credits pooled and a deduction taken off, `(a+b-c)/(d*e)`: a net
part total divided by an AREA formed from a rows-times-cols product. It is the net-coverage shape, `(a+b-c)/(d*e)`, the smallest step up from
rung-153 (flip one numerator sign) — a subtraction now lives in the numerator, so this rung reintroduces the numerator net guard while keeping
the product denominator.

`(a+b-c)/(d*e)` nets THREE terms `a+b-c` (two credits pooled, a deduction taken off) over the PRODUCT of two dimensions `d*e` (an area). Both
sides are totals that must be formed BEFORE the division: the two credits pool and the deduction is subtracted to form the net numerator, the
two dimensions multiply into the denominator AREA, and only then is the net total divided by the area. As on the over-a-product rungs (141-144,
153), a product denominator has its own canonical slip that is NOT the sum/difference slip: dividing by two factors in turn IS the same as
dividing by their product (`x/d/e = x/(d*e)`), so that is not a wrong distractor. Two slips are put on the panel: the WRONG DENOMINATOR
OPERATION — ADDING the two dimensions instead of multiplying them, `(a+b-c)/(d+e)` (a perimeter-style total where an area belongs) — and the
NUMERATOR SIGN ERROR carried in by the mixed-op numerator — ADDING the deduction instead of subtracting it, `(a+b+c)/(d*e)`. The rung thus sits
exactly at the crossing of its two questions: "did you MULTIPLY the grid dimensions, or add them?" and "did you SUBTRACT the deduction, or add
it?".

The setup: two credits `credit_one`, `credit_two` are pooled and a `deduction` is taken off (a net part total `credit_one + credit_two -
deduction`) and spread across a grid formed from `grid_rows` times `grid_cols` (a grid area `grid_rows * grid_cols`). The figures are:

  NET COVERAGE  (credit_one + credit_two - deduction) / (grid_rows * grid_cols)  [ MIXED-OP numerator OVER a two-term product: net part / grid area ]
  NET PART      credit_one + credit_two - deduction                            [ the mixed-op numerator total (divided by the grid area) ]
  GRID AREA     grid_rows * grid_cols                                          [ the two-term denominator product (the net part is divided by) ]

The **net coverage** is the headline; the **net part** (two credits minus the deduction) and the **grid area** (rows times cols) ride alongside
as component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline rungs 47-153
shipped. Critically, the grid area `(d*e)` is the *legitimate* multiply-the-dimensions figure, whereas the distractor `(a+b-c)/(d+e)` is the
*slip* of adding the dimensions where their product belongs, and `(a+b+c)/(d*e)` is the *slip* of adding the deduction where it should be
subtracted — the panel puts the honest net coverage and both wrong-operation slips side by side.

Each figure is a `compute_dimensioned` program (`observe` the five quantities + `let answer = formula`); the ADJ engine carries the arithmetic
— the addition to pool the two credits, the subtraction to take off the deduction, the multiplication to form the grid area, then the division
of the net part by the grid area to form the compound figure (so (a+b-c)/(d*e) evaluates as ((a+b-c)/(d*e))) — and the harness reads the scalar
via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../152/153. This rung exercises the engine
across a **mixed-op three-term numerator divided by a two-term product** — the fact that `(a+b-c)/(d*e)` divides by an AREA and is NOT
`(a+b-c)/(d+e)` and NOT `(a+b+c)/(d*e)` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division
matches Python's the same way rungs 100/.../152/153 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the five observed quantities via `+`, `-`, `*`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the net part, the grid area, nor the net coverage is ever a literal
(each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`credit_one`, `credit_two`,
`deduction`, `grid_rows`, `grid_cols`) so no numeral hides inside a variable name.

The five options are a tight family over the same five quantities: the three real readouts plus the two classic slips —

  SUMMED  (credit_one + credit_two - deduction) / (grid_rows + grid_cols)  ADD the two dimensions instead of multiplying them, the wrong
                                                                          denominator operation (a perimeter-style total where an area
                                                                          belongs), and
  ADDED   (credit_one + credit_two + deduction) / (grid_rows * grid_cols)  ADD the deduction instead of subtracting it, the numerator sign
                                                                          error,

which are exactly the mistakes a student makes netting credits over a grid area (adding the dimensions instead of multiplying, or adding the
deduction instead of subtracting). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: the numerator carries a subtraction (`credit_one + credit_two - deduction`), so — exactly as on the difference
rungs — one positivity guard is needed: the net numerator is kept `>= 2` at build time (so the net part, and therefore the net coverage, are
strictly positive). The denominator uses only `*` over positive quantities (automatically positive), and the two distractors' numerators
(`a+b-c` for SUMMED, `a+b+c` for ADDED) are both positive given the net guard. Every observed quantity is `>= 2`. Every family member is
asserted `> 0` at build time. The seven tables give distinct net coverages, distinct net parts, and distinct grid areas so all three queried
readouts vary across the panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CREDIT_ONE, CREDIT_TWO, DEDUCTION, GRID_ROWS, GRID_COLS) — two credits pooled and a deduction taken off (credit_one + credit_two -
# deduction) over a grid area (grid_rows * grid_cols), giving the net coverage as a mixed-op three-term numerator over a two-term product
# (a+b-c)/(d*e). The numerator carries a subtraction, so the net numerator is guarded >= 2 below (keeping the net part and the net coverage
# strictly positive); the denominator uses only * over positives (automatically positive). The seven tables give distinct net parts (a+b-c),
# distinct grid areas (d*e), and distinct net coverages ((a+b-c)/(d*e)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (8, 6, 2, 2, 5),      # net = 12, area = 10, coverage = 1.2
    (11, 9, 2, 2, 3),     # net = 18, area = 6,  coverage = 3.0
    (14, 8, 2, 2, 4),     # net = 20, area = 8,  coverage = 2.5
    (12, 6, 2, 3, 5),     # net = 16, area = 15, coverage = 16/15
    (15, 9, 2, 4, 5),     # net = 22, area = 20, coverage = 1.1
    (18, 10, 4, 3, 4),    # net = 24, area = 12, coverage = 2.0
    (15, 13, 2, 2, 7),    # net = 26, area = 14, coverage = 26/14
]

# The option family (5 members), all built from the five observed quantities via +, -, *, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "net_coverage",
        "net coverage (the net part divided by the grid area)",
        "(credit_one + credit_two - deduction) / (grid_rows * grid_cols)",
    ),
    (
        "net_part",
        "the net part (the two credits pooled minus the deduction, the numerator that is divided by the grid area)",
        "credit_one + credit_two - deduction",
    ),
    (
        "grid_area",
        "the grid area (the grid rows times the grid cols, the denominator the net part is divided by)",
        "grid_rows * grid_cols",
    ),
    (
        "summed",
        "the net part divided by the grid rows plus the grid cols, adding the two dimensions instead of multiplying them (a wrong operation)",
        "(credit_one + credit_two - deduction) / (grid_rows + grid_cols)",
    ),
    (
        "added",
        "the two credits plus the deduction divided by the grid area, adding the deduction instead of subtracting it (a wrong operation)",
        "(credit_one + credit_two + deduction) / (grid_rows * grid_cols)",
    ),
]
QUERIED = ["net_coverage", "net_part", "grid_area"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(credit_one, credit_two, deduction, grid_rows, grid_cols):
    # Operation order mirrors the ADJ programs exactly (the addition pools the two credits, the subtraction takes off the deduction, the
    # multiplication forms the grid area, then the net part is divided by the grid area to form the compound figure, so (a+b-c)/(d*e) evaluates
    # as ((a+b-c)/(d*e))), so the Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    net = credit_one + credit_two - deduction
    area = grid_rows * grid_cols
    return {
        "net_coverage": net / area,
        "net_part": net,
        "grid_area": area,
        "summed": net / (grid_rows + grid_cols),
        "added": (credit_one + credit_two + deduction) / area,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for credit_one, credit_two, deduction, grid_rows, grid_cols in TABLES:
        # Every observed quantity is a plain positive number >= 2.
        assert (
            credit_one >= 2
            and credit_two >= 2
            and deduction >= 2
            and grid_rows >= 2
            and grid_cols >= 2
        ), (credit_one, credit_two, deduction, grid_rows, grid_cols)
        # The numerator carries a subtraction, so guard the net numerator >= 2 (keeping the net part and the net coverage strictly positive).
        net = credit_one + credit_two - deduction
        assert net >= 2, (credit_one, credit_two, deduction, net)
        fv = family_values(credit_one, credit_two, deduction, grid_rows, grid_cols)
        for key, v in fv.items():
            assert v > 0, (key, credit_one, credit_two, deduction, grid_rows, grid_cols, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    credit_one,
                    credit_two,
                    deduction,
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
                credit_one,
                credit_two,
                deduction,
                grid_rows,
                grid_cols,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r154nca-{idx + 1:02d}",
                "qtype": "net_coverage",
                "stem": (
                    f"A coverage study records two credits of {num(credit_one)} and {num(credit_two)} with a "
                    f"deduction of {num(deduction)} spread across a grid of {num(grid_rows)} rows by {num(grid_cols)} cols. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe credit_one({num(credit_one)})\n"
                    f"observe credit_two({num(credit_two)})\n"
                    f"observe deduction({num(deduction)})\n"
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
            "ADJ-LADDER rung 154 — net coverage from FIVE stated quantities (a MIXED-OP numerator over a two-term PRODUCT). Rung-153 divided a "
            "three-term SUM by a two-term product (a+b+c)/(d*e); rung-154 keeps the two-term product denominator but makes the numerator "
            "MIXED-OP — two credits pooled and a deduction taken off, a net part total (credit_one + credit_two - deduction) divided by an AREA "
            "(grid_rows * grid_cols): a mixed-op three-term numerator over a two-term product (a+b-c)/(d*e). From a net part "
            "(credit_one + credit_two - deduction) divided by a grid area (grid_rows * grid_cols), compute the net coverage "
            "((credit_one+credit_two-deduction)/(grid_rows*grid_cols)), the net part (credit_one+credit_two-deduction), or the grid area "
            "(grid_rows*grid_cols). Each item is a compute_dimensioned program (observe the five quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — a MIXED-OP THREE-TERM NUMERATOR OVER A TWO-TERM PRODUCT (a+b-c)/(d*e) (pool the two credits, "
            "subtract the deduction, multiply the two dimensions into an area FIRST, then divide the net part by the area). As on the "
            "over-a-product rungs, dividing by two factors in turn equals dividing by their product (x/d/e = x/(d*e)), so that is not a wrong "
            "distractor; the two slips on the panel are the WRONG DENOMINATOR OPERATION — adding the two dimensions instead of multiplying "
            "((a+b-c)/(d+e), a perimeter-style total where an area belongs) — and the NUMERATOR SIGN ERROR — adding the deduction instead of "
            "subtracting it ((a+b+c)/(d*e)). The rung sits at the crossing of its two questions: 'did you MULTIPLY the grid dimensions, or add "
            "them?' and 'did you SUBTRACT the deduction, or add it?'. The harness matches the scalar to the printed options. Contamination-safe: "
            "every figure is built only from the five observed quantities via +, -, *, and / — no constant leaks, and neither the net part, the "
            "grid area, nor the net coverage ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The numerator carries a subtraction, so the net numerator is guarded >= 2 "
            "(keeping the net part and the net coverage strictly positive); the denominator uses only * over positives. The five family values "
            "are kept pairwise distinct with all three queried readouts varying across the panel, all asserted strictly positive at build time."
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
