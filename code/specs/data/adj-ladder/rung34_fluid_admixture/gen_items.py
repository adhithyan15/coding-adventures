"""Generate rung-34 (IV-fluid admixture total solute) items.json for the ADJ-LADDER.

Rung 34 opens the **fluid & electrolyte / IV-therapy** panel on the quantitative band — the arithmetic of how
much solute you get when you combine two intravenous fluids. Each fluid carries an *amount* of solute equal to
its **volume times its concentration** (amount = volume × concentration), and when two fluids are mixed the
total solute is simply the two amounts **added together**. It uses the same contamination-safe shape as the
stroke-work rung (33), the respiratory rung (32), and the Starling rung (31): a small table of *observed*
volumes and concentrations and a tight family of mutually-confusable formulas built **only from those observed
quantities** (no numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a single admixture of two fluids. FOUR quantities are measured — two volumes (L) and two
concentrations (mEq/L):

  VOLUME_FIRST           V1   volume of the first fluid
  CONCENTRATION_FIRST    C1   solute concentration of the first fluid
  VOLUME_SECOND          V2   volume of the second fluid
  CONCENTRATION_SECOND   C2   solute concentration of the second fluid

The total solute is the **first fluid's amount plus the second fluid's amount** — a *sum of two products* —
`(VOLUME_FIRST * CONCENTRATION_FIRST) + (VOLUME_SECOND * CONCENTRATION_SECOND)`. That is what makes this rung
distinctive: it is a NEW arithmetic shape on the ladder — a sum whose TWO addends are each their own product.
This continues the two-operand-composition series: rung-31 subtracted one difference from another, rung-32
divided one difference by another, rung-33 multiplied one difference by another, and rung-34 ADDS one product
to another. The core confusion this rung tests is pairing the right volume with the right concentration inside
each product (a fluid's own volume times its own concentration), rather than crossing them:

  TOTAL SOLUTE               (V1 * C1) + (V2 * C2)   [ first amount + second amount = total solute ]
  FIRST-COMPARTMENT SOLUTE   V1 * C1                 [ the first fluid's amount, one addend ]
  SECOND-COMPARTMENT SOLUTE  V2 * C2                 [ the second fluid's amount, the other addend ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/32/33. This rung exercises the engine across an
ADDITION of two parenthesised PRODUCTS.

Contamination-safe by construction: every formula is built only from the four observed quantities via `*`, `+`,
`-` — **no structural constants** — so every program literal is grounded in the stem. Neither compartment's
amount ever appears as a literal (each is computed from the observed volume and concentration). The observed
quantities carry **digit-free identifiers** (`volume_first`, `concentration_first`, `volume_second`,
`concentration_second`) so no numeral hides inside a variable name. The five options are a tight family over the
same quantities: the three real indices plus the two classic slips —

  CROSSED PRODUCTS SUM   (V1 * C2) + (V2 * C1)   each volume paired with the OTHER fluid's concentration, and
  DIFFERENCE OF PRODUCTS (V1 * C1) - (V2 * C2)   the two amounts SUBTRACTED instead of added,

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the total solute is the largest value (sum of both amounts), the two compartment amounts are
smaller (and unequal), the crossed-products sum is close to but never equal to the total, and the difference of
products is the smallest positive value; the tables below are chosen so the five family values are pairwise
distinct — with a comfortable margin — for every item, asserted at build time (the first amount exceeds the
second so the difference is strictly positive, and no two family values collide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (VOLUME_FIRST, CONCENTRATION_FIRST, VOLUME_SECOND, CONCENTRATION_SECOND) observed per admixture. Volumes in L,
# concentrations in mEq/L, so each product is an amount in mEq. The first fluid's amount (V1*C1) exceeds the
# second's (V2*C2) on every row, so the difference of products is strictly positive. The five family values are
# asserted pairwise-distinct (with margin) below.
#   V1 = volume of the first fluid            C1 = its concentration
#   V2 = volume of the second fluid           C2 = its concentration
TABLES = [
    (2, 140, 1, 75),
    (3, 120, 2, 85),
    (3, 150, 2, 80),
    (2, 130, 3, 70),
    (3, 100, 1, 85),
    (2, 110, 1, 95),
    (3, 160, 2, 95),
]

# The option family (5 members), all built from the observed quantities via `*` / `+` / `-`. Every identifier
# is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all
# five always appear as the options.
FAMILY = [
    (
        "total_solute",
        "total solute in the combined fluid",
        "(volume_first * concentration_first) + (volume_second * concentration_second)",
    ),
    (
        "first_compartment_solute",
        "solute contributed by the first fluid",
        "volume_first * concentration_first",
    ),
    (
        "second_compartment_solute",
        "solute contributed by the second fluid",
        "volume_second * concentration_second",
    ),
    (
        "crossed_products_sum",
        "crossed-products sum (each volume with the other fluid's concentration)",
        "(volume_first * concentration_second) + (volume_second * concentration_first)",
    ),
    (
        "difference_of_products",
        "difference of the two amounts (first minus second)",
        "(volume_first * concentration_first) - (volume_second * concentration_second)",
    ),
]
QUERIED = ["total_solute", "first_compartment_solute", "second_compartment_solute"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(v1, c1, v2, c2):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    first_amount = v1 * c1
    second_amount = v2 * c2
    return {
        "total_solute": first_amount + second_amount,
        "first_compartment_solute": first_amount,
        "second_compartment_solute": second_amount,
        "crossed_products_sum": (v1 * c2) + (v2 * c1),
        "difference_of_products": first_amount - second_amount,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for v1, c1, v2, c2 in TABLES:
        first_amount = v1 * c1
        second_amount = v2 * c2
        assert first_amount > 0 and second_amount > 0, (v1, c1, v2, c2)
        assert first_amount > second_amount, (v1, c1, v2, c2)  # difference strictly positive
        fv = family_values(v1, c1, v2, c2)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (v1, c1, v2, c2, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, v1, c1, v2, c2, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r34admix-{idx + 1:02d}",
                "qtype": "fluid_admixture",
                "stem": (
                    f"Two intravenous fluids are combined: the first is {num(v1)} L at a solute concentration "
                    f"of {num(c1)} mEq/L, and the second is {num(v2)} L at {num(c2)} mEq/L. What is the "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe volume_first({num(v1)})\n"
                    f"observe concentration_first({num(c1)})\n"
                    f"observe volume_second({num(v2)})\n"
                    f"observe concentration_second({num(c2)})\n"
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
            "ADJ-LADDER rung 34 — total solute of an IV-fluid admixture from two volumes and two concentrations "
            "(a NEW panel: fluid & electrolyte / IV therapy). From four stated quantities (first volume V1, "
            "first concentration C1, second volume V2, second concentration C2) compute the total solute "
            "((V1*C1)+(V2*C2)), the first-compartment solute (V1*C1), or the second-compartment solute (V2*C2). "
            "Each item is a compute_dimensioned program (observe the four quantities, let answer = formula); the "
            "ADJ engine carries the arithmetic — a NEW shape, a SUM OF TWO PRODUCTS ((V1*C1)+(V2*C2)), so one "
            "parenthesised product is added to another — and the harness matches the scalar to the printed "
            "options. Contamination-safe: every index is built only from the four observed quantities via *, + "
            "and - — no constant leaks (each amount is a pure volume*concentration product), and neither "
            "compartment's amount ever appears as a literal (each is computed from the observed volume and "
            "concentration) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same quantities, so the distractors "
            "are exactly the slips students make: the crossed-products sum ((V1*C2)+(V2*C1), each volume with "
            "the other fluid's concentration) and the difference of products ((V1*C1)-(V2*C2), the two amounts "
            "subtracted instead of added). The core confusion tested is adding a volume*concentration product "
            "to another with each pairing correct."
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
              "=", round(it["options"][it["gold_letter"]]["value"], 4))
