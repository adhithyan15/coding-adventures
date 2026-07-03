"""Generate rung-51 (pharmacy dispensing quantity) items.json for the ADJ-LADDER.

Rung 51 opens the **pharmacy dispensing** panel on the quantitative band — the arithmetic of how many tablets a
prescription dispenses over its whole course. The total quantity is the product of THREE stated quantities: how many
tablets the patient takes per dose, how many doses per day, and how many days the supply must cover. Multiplying three
observed quantities together introduces a genuinely NEW arithmetic shape on the ladder: a **triple product** —
`a · b · c` — three quantities multiplied in one chain, distinct from every two- and three-term shape shipped so far
(rungs 48-50 all multiplied a factor by a *parenthesised sum or difference*; none multiplied three lone factors).

The setup: a prescription directs `tablets_per_dose` tablets per dose, `doses_per_day` doses each day, for a supply of
`days_supply` days. The pharmacist dispenses the whole course:

  TOTAL TABLETS   tablets_per_dose · doses_per_day · days_supply   [ tablets — the whole dispensed quantity ]
  DAILY TABLETS   tablets_per_dose · doses_per_day                 [ tablets taken in a single day ]
  TOTAL DOSES     doses_per_day · days_supply                      [ how many doses over the whole course ]

The **total tablets** is what makes this rung distinctive — it is the ladder's first **triple product**: three
observed quantities multiplied in one chain. Contrast the neighbours already on the ladder: rung-49 was a
*distributive product-over-a-sum* `a·(b+c)` and rung-50 a *distributive product-over-a-difference* `a·(b−c)`; both
multiplied a *single* factor by a parenthesised pair. This rung multiplies three bare factors, so its two component
readouts are themselves partial products — the daily tablets `tablets_per_dose · doses_per_day` and the total doses
`doses_per_day · days_supply` — each a two-factor product that the triple product builds on. (The partial products
ride alongside the headline figure exactly as rungs 47-50 shipped their component sums/products/differences beside the
answer, so the panel teaches the whole calculation.)

Each index is a `compute_dimensioned` program (`observe` the three quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including the chained `tablets_per_dose · doses_per_day · days_supply` product — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../49/50. This rung exercises the engine across a **product of three quantities** — associative multiplication
`a·b·c` made computable.

Contamination-safe by construction: every formula is built ONLY from the three observed quantities via `·` and `+` —
**no structural constants** — so no numeric literal appears in any program, and neither partial product nor any total
is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`tablets_per_dose`, `doses_per_day`, `days_supply`) so no numeral hides inside a variable name.

The five options are a tight family over the same three quantities: the three real readouts plus the two classic
slips —

  SUM VERSION   tablets_per_dose + doses_per_day + days_supply     ADD the three quantities instead of multiplying
                                                                   them (a quantity is a product, not a sum), and
  MIXED         tablets_per_dose · doses_per_day + days_supply      multiply the first two but ADD the day count
                                                                   (forgetting the third factor multiplies too),

which are exactly the mistakes a student makes (adding quantities that should be multiplied, or dropping a factor out
of the product). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear
as options.

Distinctness: all three observed quantities are positive integers with `tablets_per_dose > 1` and `tablets_per_dose ≠
days_supply` (so the total exceeds either partial product and the two partials differ), so every product and sum is a
distinct positive number; the tables below are chosen so the five family values are pairwise distinct with a
comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TABLETS_PER_DOSE, DOSES_PER_DAY, DAYS_SUPPLY) — tablets per dose, doses per day, days the supply covers, all plain
# positive integers with TABLETS_PER_DOSE > 1 and TABLETS_PER_DOSE != DAYS_SUPPLY. The five family values are asserted
# pairwise-distinct (with margin) below.
TABLES = [
    (2, 3, 10),
    (3, 2, 7),
    (2, 4, 5),
    (3, 3, 6),
    (4, 2, 9),
    (2, 5, 8),
    (5, 2, 6),
]

# The option family (5 members), all built from the three observed quantities via * and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "total_tablets",
        "total tablets dispensed over the whole course (per-dose times doses-per-day times days)",
        "tablets_per_dose * doses_per_day * days_supply",
    ),
    (
        "daily_tablets",
        "the tablets taken in a single day (per-dose times doses-per-day)",
        "tablets_per_dose * doses_per_day",
    ),
    (
        "total_doses",
        "the total number of doses over the course (doses-per-day times days)",
        "doses_per_day * days_supply",
    ),
    (
        "sum_version",
        "the three quantities ADDED instead of multiplied (a quantity is a product, not a sum)",
        "tablets_per_dose + doses_per_day + days_supply",
    ),
    (
        "mixed",
        "per-dose times doses-per-day but the day count ADDED, dropping the third factor from the product",
        "tablets_per_dose * doses_per_day + days_supply",
    ),
]
QUERIED = ["total_tablets", "daily_tablets", "total_doses"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(tablets_per_dose, doses_per_day, days_supply):
    # Operation order mirrors the ADJ programs exactly (a left-folded triple product for the total), so the Python
    # option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    daily = tablets_per_dose * doses_per_day
    return {
        "total_tablets": daily * days_supply,
        "daily_tablets": daily,
        "total_doses": doses_per_day * days_supply,
        "sum_version": tablets_per_dose + doses_per_day + days_supply,
        "mixed": daily + days_supply,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for tablets_per_dose, doses_per_day, days_supply in TABLES:
        assert (
            tablets_per_dose > 1
            and doses_per_day > 0
            and days_supply > 0
            and tablets_per_dose != days_supply
        ), (tablets_per_dose, doses_per_day, days_supply)
        fv = family_values(tablets_per_dose, doses_per_day, days_supply)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    tablets_per_dose,
                    doses_per_day,
                    days_supply,
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
                tablets_per_dose,
                doses_per_day,
                days_supply,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r51disp-{idx + 1:02d}",
                "qtype": "dispensing_quantity",
                "stem": (
                    f"A prescription directs {num(tablets_per_dose)} tablets per dose, {num(doses_per_day)} doses per "
                    f"day, for a {num(days_supply)}-day supply. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe tablets_per_dose({num(tablets_per_dose)})\n"
                    f"observe doses_per_day({num(doses_per_day)})\n"
                    f"observe days_supply({num(days_supply)})\n"
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
            "ADJ-LADDER rung 51 — pharmacy dispensing quantity from three stated quantities (a NEW panel: pharmacy "
            "dispensing). From tablets-per-dose, doses-per-day and a days-supply compute the total tablets dispensed "
            "(tablets_per_dose*doses_per_day*days_supply), the daily tablets (tablets_per_dose*doses_per_day), or the "
            "total doses (doses_per_day*days_supply). Each item is a compute_dimensioned program (observe the three "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, TRIPLE PRODUCT "
            "a*b*c, the first product on the ladder to multiply three bare factors in one chain (distinct from rung-49 "
            "distributive product-over-a-sum a*(b+c) and rung-50 distributive product-over-a-difference a*(b-c)) — and "
            "the harness matches the scalar to the printed options. Contamination-safe: every index is built only from "
            "the three observed quantities via * and + — no constant leaks, and neither partial product (the daily "
            "tablets or the total doses) nor any total ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same three quantities, so the distractors are exactly the slips students make: ADDING the "
            "three quantities instead of multiplying them, and dropping a factor by adding the day count instead of "
            "multiplying by it (a*b+c, not a*b*c). The core confusion tested is multiplying three quantities into a "
            "single dispensed total."
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
