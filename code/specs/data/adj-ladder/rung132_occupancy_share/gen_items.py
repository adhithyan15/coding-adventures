"""Generate rung-132 (receptor-occupancy share / a QUOTIENT over a SUM — divide a ratio by a pooled total) items.json.

Rung 132 opens the **receptor-occupancy** panel and is the **transpose of rung-131**. rung-131 put a SUM over a quotient, `(a+b)/(c/d)`
(a pooled total divided by a rate); rung-132 flips it to a quotient over a SUM, `(a/b)/(c+d)` (a rate divided by a pooled total). Where
rung-131 tested inverting a rate you divide BY, rung-132 tests keeping a SUM grouped as the single divisor.

This is genuinely new. `(a/b)/(c+d)` is a ratio `a/b` divided by a pooled total `c+d`. The sum `c+d` must be formed FIRST and divided as a
WHOLE, so `(a/b)/(c+d) = a/(b*(c+d))`. The core confusions this rung tests are the two canonical divide-by-a-sum slips: DISTRIBUTING the
division over the sum — `(a/b)/(c+d)` is NOT `(a/b)/c + (a/b)/d` (the classic `1/(c+d) != 1/c + 1/d` error, the same trap behind harmonic
means and parallel resistances) — and dropping the sum grouping so only the first term divides and the second is ADDED — `(a/b)/(c+d)` is
NOT `(a/b)/c + d`.

The setup: a `ligand_amount` on a `receptor_units` count (a binding ratio `ligand_amount/receptor_units`), spread across a pooled capacity
formed from a `primary_pool` plus a `reserve_pool` (a pooled capacity `primary_pool + reserve_pool`). The figures are:

  OCCUPANCY SHARE   (ligand_amount / receptor_units) / (primary_pool + reserve_pool)   [ quotient OVER a sum: binding ratio / pooled cap ]
  BINDING RATIO     ligand_amount / receptor_units                                     [ the numerator ratio (divided by the pooled cap) ]
  POOLED CAPACITY   primary_pool + reserve_pool                                        [ the summed denominator (the single divisor) ]

The **occupancy share** is the ladder's first **(a quotient) over (a sum) as a headline** — a share (how much binding ratio falls on each
unit of pooled capacity), framed as a *share* to keep it dimensionless-clean, the same discipline rungs 100/.../130/131 used for their
ratios. (The binding ratio `a/b` and the pooled capacity `c+d` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 47-131 shipped their component figures beside the headline. The two components anchor the "form the ratio,
pool the capacity FIRST, then divide the ratio by the whole pool" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the division to form the binding ratio, the sum to form the pooled capacity, then the division of the binding ratio by the
whole pooled capacity (so (a/b)/(c+d) evaluates as ((a/b)/(c+d)) = a/(b*(c+d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../130/131. This rung exercises the engine across a
**quotient divided by a sum** — the fact that `(a/b)/(c+d)` is `a/(b*(c+d))` and NOT `(a/b)/c + (a/b)/d` and NOT `(a/b)/c + d` made
computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../130/131 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/` and `+` — **no structural
constants** — so no numeric literal appears in any program, and neither the binding ratio, the pooled capacity, nor the occupancy share
is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers**
(`ligand_amount`, `receptor_units`, `primary_pool`, `reserve_pool`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SPLIT       (ligand_amount / receptor_units) / primary_pool + (ligand_amount / receptor_units) / reserve_pool   distribute the division
                                                                  over the pooled sum, dividing the binding ratio by each pool separately
                                                                  and adding (the classic `1/(c+d) != 1/c + 1/d` error, evaluating
                                                                  `(a/b)/c + (a/b)/d`), and
  UNGROUPED   (ligand_amount / receptor_units) / primary_pool + reserve_pool   drop the sum grouping so only the primary pool divides the
                                                                  binding ratio and the reserve pool is ADDED (`(a/b)/c + d`, ignoring
                                                                  that the pooled capacity is a single divisor),

which are exactly the mistakes a student makes (distributing division over a sum, or dividing by only the first term and adding the rest).
Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `/` and `+` over positive quantities, so **every figure is automatically positive** (no
subtraction anywhere) — like rungs 128/130/131, no positivity guards are needed. Every observed quantity is `>= 2`. Every family member is
asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give distinct occupancy shares, distinct binding ratios, and
distinct pooled capacities so all three queried readouts vary across the panel; the five family values are pairwise distinct with a
comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (LIGAND_AMOUNT, RECEPTOR_UNITS, PRIMARY_POOL, RESERVE_POOL) — a binding ratio (ligand_amount/receptor_units) divided by a pooled
# capacity (primary_pool + reserve_pool), giving the occupancy share as a quotient over a sum (a/b)/(c+d) = a/(b*(c+d)). This rung uses
# only / and + over positive quantities, so every figure is automatically positive; no positivity guards are needed. The seven tables
# give distinct binding ratios (a/b), distinct pooled capacities (c+d), and distinct occupancy shares ((a/b)/(c+d)); the five family
# values are asserted pairwise-distinct below.
TABLES = [
    (6, 2, 2, 4),     # a/b = 3.0,  c+d = 6,  share = 0.5
    (8, 2, 3, 7),     # a/b = 4.0,  c+d = 10, share = 0.4
    (10, 2, 4, 5),    # a/b = 5.0,  c+d = 9,  share = 0.555...
    (12, 2, 5, 3),    # a/b = 6.0,  c+d = 8,  share = 0.75
    (14, 2, 4, 7),    # a/b = 7.0,  c+d = 11, share = 0.636...
    (16, 2, 6, 7),    # a/b = 8.0,  c+d = 13, share = 0.615...
    (20, 2, 3, 4),    # a/b = 10.0, c+d = 7,  share = 1.428...
]

# The option family (5 members), all built from the four observed quantities via / and +. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "occupancy_share",
        "occupancy share (the binding ratio divided by the pooled capacity)",
        "(ligand_amount / receptor_units) / (primary_pool + reserve_pool)",
    ),
    (
        "binding_ratio",
        "the binding ratio (the ligand amount per receptor unit, the numerator that is divided by the pooled capacity)",
        "ligand_amount / receptor_units",
    ),
    (
        "pooled_capacity",
        "the pooled capacity (the primary pool plus the reserve pool, the single divisor the binding ratio is divided by)",
        "primary_pool + reserve_pool",
    ),
    (
        "split",
        "the binding ratio over the primary pool plus the binding ratio over the reserve pool, distributing the division over the pooled sum instead of dividing by the whole pool (a wrong operation)",
        "(ligand_amount / receptor_units) / primary_pool + (ligand_amount / receptor_units) / reserve_pool",
    ),
    (
        "ungrouped",
        "the binding ratio over the primary pool plus the reserve pool, dropping the sum grouping so only the primary pool divides and the reserve pool is added (a wrong operation)",
        "(ligand_amount / receptor_units) / primary_pool + reserve_pool",
    ),
]
QUERIED = ["occupancy_share", "binding_ratio", "pooled_capacity"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(ligand_amount, receptor_units, primary_pool, reserve_pool):
    # Operation order mirrors the ADJ programs exactly (the division forms the binding ratio, the sum forms the pooled capacity, then the
    # binding ratio is divided by the whole pooled capacity, so (a/b)/(c+d) evaluates as ((a/b)/(c+d)) = a/(b*(c+d))), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    ratio = ligand_amount / receptor_units
    return {
        "occupancy_share": ratio / (primary_pool + reserve_pool),
        "binding_ratio": ratio,
        "pooled_capacity": primary_pool + reserve_pool,
        "split": ratio / primary_pool + ratio / reserve_pool,
        "ungrouped": ratio / primary_pool + reserve_pool,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for ligand_amount, receptor_units, primary_pool, reserve_pool in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only / and + over positive quantities, so positivity is
        # automatic — no positivity guards are needed.
        assert (
            ligand_amount >= 2
            and receptor_units >= 2
            and primary_pool >= 2
            and reserve_pool >= 2
        ), (ligand_amount, receptor_units, primary_pool, reserve_pool)
        fv = family_values(ligand_amount, receptor_units, primary_pool, reserve_pool)
        for key, v in fv.items():
            assert v > 0, (key, ligand_amount, receptor_units, primary_pool, reserve_pool, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    ligand_amount,
                    receptor_units,
                    primary_pool,
                    reserve_pool,
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
                ligand_amount,
                receptor_units,
                primary_pool,
                reserve_pool,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r132osa-{idx + 1:02d}",
                "qtype": "occupancy_share",
                "stem": (
                    f"A receptor-occupancy study records a ligand amount of {num(ligand_amount)} on {num(receptor_units)} receptor "
                    f"units, spread across a pooled capacity of a primary pool of {num(primary_pool)} plus a reserve pool of "
                    f"{num(reserve_pool)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe ligand_amount({num(ligand_amount)})\n"
                    f"observe receptor_units({num(receptor_units)})\n"
                    f"observe primary_pool({num(primary_pool)})\n"
                    f"observe reserve_pool({num(reserve_pool)})\n"
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
            "ADJ-LADDER rung 132 — receptor-occupancy share from four stated quantities (a NEW panel: receptor occupancy, the TRANSPOSE "
            "of rung-131). rung-131 put a sum over a quotient (a+b)/(c/d); rung-132 flips it to a quotient over a SUM (a/b)/(c+d). From "
            "a binding ratio (ligand_amount/receptor_units) divided by a pooled capacity (primary_pool + reserve_pool), compute the "
            "occupancy share ((ligand_amount/receptor_units)/(primary_pool+reserve_pool)), the binding ratio "
            "(ligand_amount/receptor_units), or the pooled capacity (primary_pool+reserve_pool). Each item is a compute_dimensioned "
            "program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a "
            "QUOTIENT OVER A SUM (a/b)/(c+d) (form the ratio, pool the capacity FIRST, then divide the ratio by the whole pool, so "
            "(a/b)/(c+d) = a/(b*(c+d)) — the sum is a single divisor). The divide-by-a-sum slips ride alongside as distractors. The "
            "harness matches the scalar to the printed options. The occupancy share is a share (how much binding ratio falls on each "
            "unit of pooled capacity), framed as a SHARE so the dimensionless value stays honest. Contamination-safe: every figure is "
            "built only from the four observed quantities via / and + — no constant leaks, and neither the binding ratio, the pooled "
            "capacity, nor the occupancy share ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the same four "
            "quantities, so the distractors are exactly the slips students make: distributing the division over the pooled sum "
            "((a/b)/c + (a/b)/d, the classic 1/(c+d) != 1/c + 1/d error, a wrong operation) and dropping the sum grouping so only the "
            "primary pool divides and the reserve pool is added ((a/b)/c + d, a wrong operation). The core confusion tested is that "
            "(a/b)/(c+d) is a/(b*(c+d)), not (a/b)/c + (a/b)/d and not (a/b)/c + d. This rung uses only / and + over positive "
            "quantities, so every figure is automatically positive — no positivity guards are needed — and the five family values are "
            "kept pairwise distinct with all three queried readouts varying across the panel, all asserted strictly positive at build "
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
