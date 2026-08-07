"""Generate rung-143 (shelf density / a DIFFERENCE-numerator over a PRODUCT — divide a net by an area) items.json.

Rung 143 continues the **OVER-A-PRODUCT** column. rung-141 put a SUM over a product, `(a+b)/(c*d)`; rung-142 put a PRODUCT over a product,
`(a*b)/(c*d)`; rung-143 puts a **DIFFERENCE** over a product, `(a−b)/(c*d)` — a net remainder `a−b` divided by an area product `c*d`.
This is the difference-numerator member of the over-a-product family — the numerator is now a subtraction, `(a−b)`, but the denominator is
still ONE area `c*d`.

`(a−b)/(c*d)` is a DIFFERENCE `a−b` divided by a PRODUCT `c*d` (an area). The difference `a−b` binds and stays grouped over the bar (the
net that survives the subtraction), and the two-part denominator `c*d` is ONE area the whole net is divided by. As on rungs 141/142, a
product denominator has its own two canonical slips, and they are NOT the sum/difference/rate slips: dividing by two factors in turn IS the
same as dividing by their product (`x/c/d = x/(c*d)`), so that is not a wrong distractor. The two canonical divide-by-a-product slips that a
student actually makes are: using the WRONG denominator operation, ADDING the two dimensions instead of multiplying them (`(a−b)/(c+d)` — a
perimeter-style total instead of an area), and INVERTING the ratio, dividing the area by the net instead of the net by the area
(`(c*d)/(a−b)` — the reciprocal).

The setup: a `gross_stock` arrives, `pulled_stock` is pulled off it (a net stock `gross_stock − pulled_stock`), and what remains is spread
over a shelf formed from a `shelf_rows` times a `shelf_cols` (a shelf area `shelf_rows * shelf_cols`). The figures are:

  SHELF DENSITY  (gross_stock − pulled_stock) / (shelf_rows * shelf_cols)  [ difference-numerator OVER a product: net stock / shelf area ]
  NET STOCK      gross_stock − pulled_stock                              [ the difference numerator (divided by the shelf area) ]
  SHELF AREA     shelf_rows * shelf_cols                                 [ the product the net stock is divided by ]

The **shelf density** is a **(a difference) over (a product) as a headline** — a density (how much net stock rides on each cell of the shelf
area), framed as a *density* to keep it dimensionless-clean, the same discipline rungs 100/.../141/142 used for their ratios, spans,
concentrations, densities, indices, slopes. (The net stock `a−b` and the shelf area `c*d` ride alongside as component readouts, so the panel
teaches the whole calculation — exactly as rungs 47-142 shipped their component figures beside the headline. The two components anchor the
"subtract to the net FIRST, multiply out the area, then divide the net by the area" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the subtraction to form the net stock, the multiplication to form the shelf area, then the division of the net by the shelf
area to form the compound figure (so (a−b)/(c*d) evaluates as ((a−b)/(c*d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../141/142. This rung exercises the engine across a
**difference divided by a product** — the fact that `(a−b)/(c*d)` is one net over one area and NOT `(a−b)/(c+d)` and NOT `(c*d)/(a−b)`
made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../141/142 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `−`, `*`, `/`, and `+` — **no
structural constants** — so no numeric literal appears in any program, and neither the net stock, the shelf area, nor the shelf density is
ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`gross_stock`,
`pulled_stock`, `shelf_rows`, `shelf_cols`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  ADDED      (gross_stock − pulled_stock) / (shelf_rows + shelf_cols)  divide the net stock by the SUM of the dimensions instead of their
                                                                       product, using a perimeter-style total where an area belongs (the
                                                                       wrong denominator operation), and
  INVERTED   (shelf_rows * shelf_cols) / (gross_stock − pulled_stock)  divide the shelf area BY the net stock, the ratio upside down (the
                                                                       reciprocal of the shelf density, the wrong direction),

which are exactly the mistakes a student makes with a product denominator (adding the dimensions instead of multiplying, or inverting the
ratio). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: the numerator is a subtraction, so — unlike the all-`+ * /` rungs 141/142 — the net stock needs a **positivity
guard**: every table is built so `gross_stock − pulled_stock >= 2` (asserted at build time), keeping the net stock, the shelf density, the
added slip, and the inverted slip all strictly positive (the denominator `shelf_rows * shelf_cols` is a product of positives, so it is
automatically positive; only the difference numerator can go non-positive). Every observed quantity is `>= 2`. Every family member is
asserted `> 0` at build time. The seven tables give distinct net stocks, distinct shelf areas, and distinct shelf densities so all three
queried readouts vary across the panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (GROSS_STOCK, PULLED_STOCK, SHELF_ROWS, SHELF_COLS) — a net stock (gross_stock - pulled_stock) divided by a shelf area (shelf_rows *
# shelf_cols), giving the shelf density as a difference over a product (a-b)/(c*d). The numerator is a subtraction, so the net stock needs a
# positivity guard: every row satisfies gross_stock - pulled_stock >= 2 (asserted below). The denominator is a product of positives, so it is
# automatically positive. The seven tables give distinct net stocks (a-b), distinct shelf areas (c*d), and distinct shelf densities
# ((a-b)/(c*d)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (8, 2, 2, 5),      # net = 6,  area = 10, density = 0.6
    (11, 3, 2, 3),     # net = 8,  area = 6,  density = 1.333...
    (14, 4, 2, 4),     # net = 10, area = 8,  density = 1.25
    (16, 4, 3, 5),     # net = 12, area = 15, density = 0.8
    (19, 5, 4, 5),     # net = 14, area = 20, density = 0.7
    (12, 3, 3, 4),     # net = 9,  area = 12, density = 0.75
    (22, 6, 2, 7),     # net = 16, area = 14, density = 1.142...
]

# The option family (5 members), all built from the four observed quantities via -, *, /, and +. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "shelf_density",
        "shelf density (the net stock divided by the shelf area)",
        "(gross_stock - pulled_stock) / (shelf_rows * shelf_cols)",
    ),
    (
        "net_stock",
        "the net stock (the gross stock minus the pulled stock, the numerator that is divided by the shelf area)",
        "gross_stock - pulled_stock",
    ),
    (
        "shelf_area",
        "the shelf area (the shelf rows times the shelf cols, the product the net stock is divided by)",
        "shelf_rows * shelf_cols",
    ),
    (
        "added",
        "the net stock divided by the shelf rows plus the shelf cols, using the sum of the dimensions instead of their product as the divisor (a wrong operation)",
        "(gross_stock - pulled_stock) / (shelf_rows + shelf_cols)",
    ),
    (
        "inverted",
        "the shelf area divided by the net stock, the ratio upside down instead of the net stock over the shelf area (a wrong operation)",
        "(shelf_rows * shelf_cols) / (gross_stock - pulled_stock)",
    ),
]
QUERIED = ["shelf_density", "net_stock", "shelf_area"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(gross_stock, pulled_stock, shelf_rows, shelf_cols):
    # Operation order mirrors the ADJ programs exactly (the subtraction forms the net stock, the multiplication forms the shelf area, then
    # the net stock is divided by the shelf area to form the compound figure, so (a-b)/(c*d) evaluates as ((a-b)/(c*d))), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    net = gross_stock - pulled_stock
    area = shelf_rows * shelf_cols
    return {
        "shelf_density": net / area,
        "net_stock": net,
        "shelf_area": area,
        "added": net / (shelf_rows + shelf_cols),
        "inverted": area / net,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for gross_stock, pulled_stock, shelf_rows, shelf_cols in TABLES:
        # Every observed quantity is a plain positive number >= 2, AND the difference numerator is guarded positive: the net stock
        # gross_stock - pulled_stock must be >= 2. The denominator shelf_rows * shelf_cols is a product of positives, so it is automatically
        # positive; only the difference numerator can go non-positive, so it is the only guard needed.
        assert (
            gross_stock >= 2
            and pulled_stock >= 2
            and shelf_rows >= 2
            and shelf_cols >= 2
        ), (gross_stock, pulled_stock, shelf_rows, shelf_cols)
        assert gross_stock - pulled_stock >= 2, (gross_stock, pulled_stock)
        fv = family_values(gross_stock, pulled_stock, shelf_rows, shelf_cols)
        for key, v in fv.items():
            assert v > 0, (key, gross_stock, pulled_stock, shelf_rows, shelf_cols, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    gross_stock,
                    pulled_stock,
                    shelf_rows,
                    shelf_cols,
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
                gross_stock,
                pulled_stock,
                shelf_rows,
                shelf_cols,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r143sda-{idx + 1:02d}",
                "qtype": "shelf_density",
                "stem": (
                    f"A stocking study records a gross stock of {num(gross_stock)} units with "
                    f"{num(pulled_stock)} units pulled, spread over a shelf of {num(shelf_rows)} rows by "
                    f"{num(shelf_cols)} cols. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe gross_stock({num(gross_stock)})\n"
                    f"observe pulled_stock({num(pulled_stock)})\n"
                    f"observe shelf_rows({num(shelf_rows)})\n"
                    f"observe shelf_cols({num(shelf_cols)})\n"
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
            "ADJ-LADDER rung 143 — shelf density from four stated quantities (CONTINUING the OVER-A-PRODUCT column). rung-141 put a sum "
            "over a product (a+b)/(c*d); rung-142 put a PRODUCT over a product (a*b)/(c*d); rung-143 puts a DIFFERENCE over a product "
            "(a−b)/(c*d) — a net remainder divided by an area product. From a net stock (gross_stock − pulled_stock) divided by a shelf "
            "area (shelf_rows * shelf_cols), compute the shelf density ((gross_stock−pulled_stock)/(shelf_rows*shelf_cols)), the net stock "
            "(gross_stock−pulled_stock), or the shelf area (shelf_rows*shelf_cols). Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a DIFFERENCE NUMERATOR OVER A PRODUCT "
            "(a−b)/(c*d) (subtract to the net, multiply out the area, then divide the net by the area — the two-part denominator is ONE "
            "area, not two divisors). As on rungs 141/142, dividing by two factors in turn equals dividing by their product "
            "(x/c/d = x/(c*d)), so that is not a wrong distractor; the two canonical slips are used instead. The harness matches the scalar "
            "to the printed options. The shelf density is a density (how much net stock rides on each cell of the shelf area), framed as a "
            "DENSITY so the dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via −, *, /, and + — no constant leaks, and neither the net stock, the shelf area, nor the shelf density ever "
            "appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside "
            "a variable name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make with a product denominator: dividing by the SUM of the dimensions instead of their product ((a−b)/(c+d), a "
            "perimeter-style total where an area belongs, a wrong operation) and INVERTING the ratio ((c*d)/(a−b), the area over the net, "
            "the reciprocal, a wrong operation). The core confusion tested is that (a−b)/(c*d) is one net over one area, not (a−b)/(c+d) "
            "and not (c*d)/(a−b). Because the numerator is a subtraction, the net stock carries a positivity guard "
            "(gross_stock − pulled_stock >= 2) so every figure stays strictly positive; the denominator product is automatically positive. "
            "The five family values are kept pairwise distinct with all three queried readouts varying across the panel, all asserted "
            "strictly positive at build time."
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
