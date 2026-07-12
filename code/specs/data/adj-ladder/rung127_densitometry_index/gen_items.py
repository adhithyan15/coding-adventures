"""Generate rung-127 (bone-densitometry index / lone term over a SUBTRACT-A-QUOTIENT three-term denominator) items.json.

Rung 127 opens the **bone densitometry** panel and is the **subtract sibling of rung-126's add-a-quotient denominator**. A single
observed `mineral_load` is DIVIDED by an effective matrix formed from a `matrix_baseline` with a per-cycle `resorption_pool /
remodel_cycles` quotient SUBTRACTED from it, to give the density index. A **lone term over a subtract-a-quotient three-term
denominator**, `a/(b - c/d)`, i.e. `(a / (b - (c/d)))` — the inner quotient `c/d` binds BEFORE the `-` (operator precedence), and the
whole `b - c/d` sits under the division bar (grouping).

This is genuinely new. rung-126 introduced the first quotient-bearing denominator, `a/(b + c/d)` (ADD a quotient). rung-127 is its
minus twin, `a/(b - c/d)` (SUBTRACT a quotient) — exactly as rung-125 (`b-c+d`) mirrors rung-124 (`b-c-d`). Their swapped distractors
differ only in the sign of the quotient term. Because rung-127 SUBTRACTS inside the denominator, positivity is NOT automatic (unlike
rung-126's all-plus denominator): the effective matrix `b - c/d` and the misgrouped denominator `(b-c)/d` must be guarded positive, and
the crossed slip `(a/b) - c/d` must stay positive too.

The setup: a `mineral_load`, a `matrix_baseline`, a `resorption_pool`, and a `remodel_cycles` count. The figures are:

  DENSITY INDEX     mineral_load / (matrix_baseline - resorption_pool / remodel_cycles)   [ lone term / subtract-a-quotient denom ]
  EFFECTIVE MATRIX  matrix_baseline - resorption_pool / remodel_cycles                    [ the subtract-a-quotient denominator ]
  PER CYCLE         resorption_pool / remodel_cycles                                      [ the resorption pool spread over the cycles ]

The **density index** is the ladder's first **lone quantity over a denominator that SUBTRACTS a quotient (as a headline)**. It is a rate
(mineral load per unit of effective matrix), framed as an *index* to keep it dimensionless-clean — the same discipline rungs
100/.../125/126 used for their ratios. (The effective matrix `b - c/d` and the per-cycle quotient `c/d` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-126 shipped their component figures beside the headline. The
per-cycle quotient `c/d` anchors the "the resorption pool is spread over the remodel cycles FIRST, then subtracted from the matrix
baseline" grouping against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the division of the resorption pool by the remodel cycles to form the per-cycle quotient, then the subtraction of that
quotient from the matrix baseline to form the whole effective matrix, then the division of the mineral load by that whole effective
matrix (so a/(b-c/d) evaluates as (a/(b-(c/d)))) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../125/126. This rung exercises the engine across a **lone-term-over-(subtract-a-quotient
three-term) ratio** — the fact that `a/(b-c/d)` is `(a/(b-(c/d)))` and NOT `(a/b)-c/d` and NOT `a/((b-c)/d)` made computable. The golds
are non-integer f64s; the engine's IEEE-double division matches Python's the same way rungs 100/.../125/126 relied on (well within the
harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-` and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the effective matrix, the per-cycle quotient, nor the density
index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers**
(`mineral_load`, `matrix_baseline`, `resorption_pool`, `remodel_cycles`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED     mineral_load / matrix_baseline - resorption_pool / remodel_cycles   drop the denominator parentheses so only the matrix
                                                                  baseline divides the mineral load, then the per-cycle quotient is
                                                                  subtracted (the classic `a/(b-c/d)` vs `a/b-c/d` grouping error,
                                                                  evaluating `(a/b)-(c/d)`), and
  MISGROUPED  mineral_load / ((matrix_baseline - resorption_pool) / remodel_cycles)   subtract the resorption pool from the matrix
                                                                  baseline FIRST, then divide by the remodel cycles (`a/((b-c)/d)` =
                                                                  `a*d/(b-c)`, ignoring the precedence that `c/d` binds before the `-`),

which are exactly the mistakes a student makes (failing to keep the whole subtract-a-quotient denominator under the bar, or breaking the
`c/d`-binds-first precedence). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: this rung SUBTRACTS a quotient inside the denominator, so positivity is NOT automatic — it is guarded
explicitly per table. Every observed quantity is `>= 2`, and each table guarantees **matrix_baseline - resorption_pool >= 2** (so `b-c`
is comfortably positive, making the misgrouped denominator `(b-c)/d` positive, and since `c/d <= c` the effective matrix `b - c/d >= b-c
>= 2` is positive too) and **mineral_load/matrix_baseline - resorption_pool/remodel_cycles > 0** (crossed `(a/b)-(c/d)` positive). Every
family member is asserted `> 0` at build time. The seven tables give distinct density indices, distinct effective matrices, and distinct
per-cycle quotients so all three queried readouts vary across the panel; the five family values are pairwise distinct with a comfortable
margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (MINERAL_LOAD, MATRIX_BASELINE, RESORPTION_POOL, REMODEL_CYCLES) — a lone mineral load divided by an effective matrix (a matrix
# baseline with a per-cycle resorption_pool/remodel_cycles quotient subtracted from it) for the density index. This rung SUBTRACTS a
# quotient inside the denominator, so positivity is NOT automatic; each table guarantees matrix_baseline - resorption_pool >= 2 (b-c
# comfortably positive, so the effective matrix b-c/d >= b-c and the misgrouped denom (b-c)/d are positive) and
# mineral_load/matrix_baseline - resorption_pool/remodel_cycles > 0 (crossed positive). The seven tables give distinct per-cycle
# quotients (c/d), distinct effective matrices (b - c/d), and distinct density indices (a/(b-c/d)); the five family values are asserted
# pairwise-distinct below.
TABLES = [
    (54, 10, 6, 3),   # c/d = 2.0,  eff = 8.0,  index = 6.75
    (52, 9, 5, 2),    # c/d = 2.5,  eff = 6.5,  index = 8.0
    (45, 13, 9, 3),   # c/d = 3.0,  eff = 10.0, index = 4.5
    (33, 7, 3, 2),    # c/d = 1.5,  eff = 5.5,  index = 6.0
    (77, 11, 8, 2),   # c/d = 4.0,  eff = 7.0,  index = 11.0
    (95, 13, 7, 2),   # c/d = 3.5,  eff = 9.5,  index = 10.0
    (99, 16, 10, 2),  # c/d = 5.0,  eff = 11.0, index = 9.0
]

# The option family (5 members), all built from the four observed quantities via - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "density_index",
        "density index (the mineral load divided by the effective matrix)",
        "mineral_load / (matrix_baseline - resorption_pool / remodel_cycles)",
    ),
    (
        "effective_matrix",
        "the effective matrix (the matrix baseline minus the per-cycle resorption, the divisor the mineral load is divided by)",
        "matrix_baseline - resorption_pool / remodel_cycles",
    ),
    (
        "per_cycle",
        "the per-cycle resorption (the resorption pool spread over the remodel cycles, before it is subtracted from the matrix baseline)",
        "resorption_pool / remodel_cycles",
    ),
    (
        "crossed",
        "the mineral load divided by the matrix baseline, minus the resorption pool over the remodel cycles, dropping the denominator parentheses so only the matrix baseline divides (a wrong grouping)",
        "mineral_load / matrix_baseline - resorption_pool / remodel_cycles",
    ),
    (
        "misgrouped",
        "the mineral load divided by the matrix baseline minus the resorption pool, all over the remodel cycles, subtracting before dividing the resorption (a wrong grouping)",
        "mineral_load / ((matrix_baseline - resorption_pool) / remodel_cycles)",
    ),
]
QUERIED = ["density_index", "effective_matrix", "per_cycle"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(mineral_load, matrix_baseline, resorption_pool, remodel_cycles):
    # Operation order mirrors the ADJ programs exactly (the resorption pool is divided by the remodel cycles to form the per-cycle
    # quotient, then that quotient is subtracted from the matrix baseline to form the whole effective matrix, then the mineral load is
    # divided by that whole effective matrix, so a/(b-c/d) evaluates as (a/(b-(c/d)))), so the Python option value and the engine result
    # are the same IEEE-double (well within the 1e-9 tolerance).
    return {
        "density_index": mineral_load / (matrix_baseline - resorption_pool / remodel_cycles),
        "effective_matrix": matrix_baseline - resorption_pool / remodel_cycles,
        "per_cycle": resorption_pool / remodel_cycles,
        "crossed": mineral_load / matrix_baseline - resorption_pool / remodel_cycles,
        "misgrouped": mineral_load / ((matrix_baseline - resorption_pool) / remodel_cycles),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for mineral_load, matrix_baseline, resorption_pool, remodel_cycles in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung SUBTRACTS a quotient inside the denominator, so positivity
        # is NOT automatic; it is guarded explicitly per table.
        assert (
            mineral_load >= 2
            and matrix_baseline >= 2
            and resorption_pool >= 2
            and remodel_cycles >= 2
        ), (mineral_load, matrix_baseline, resorption_pool, remodel_cycles)
        assert matrix_baseline - resorption_pool >= 2, (
            "matrix_baseline - resorption_pool must be >= 2 (misgrouped denom & effective matrix positive)",
            mineral_load,
            matrix_baseline,
            resorption_pool,
            remodel_cycles,
        )
        assert mineral_load / matrix_baseline - resorption_pool / remodel_cycles > 0, (
            "crossed (a/b)-(c/d) must be positive",
            mineral_load,
            matrix_baseline,
            resorption_pool,
            remodel_cycles,
        )
        fv = family_values(mineral_load, matrix_baseline, resorption_pool, remodel_cycles)
        for key, v in fv.items():
            assert v > 0, (key, mineral_load, matrix_baseline, resorption_pool, remodel_cycles, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    mineral_load,
                    matrix_baseline,
                    resorption_pool,
                    remodel_cycles,
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
                mineral_load,
                matrix_baseline,
                resorption_pool,
                remodel_cycles,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r127dxa-{idx + 1:02d}",
                "qtype": "density_index",
                "stem": (
                    f"A bone densitometry study records a mineral load of {num(mineral_load)} divided by a matrix baseline of "
                    f"{num(matrix_baseline)} minus a resorption pool of {num(resorption_pool)} over a remodel cycle count of "
                    f"{num(remodel_cycles)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe mineral_load({num(mineral_load)})\n"
                    f"observe matrix_baseline({num(matrix_baseline)})\n"
                    f"observe resorption_pool({num(resorption_pool)})\n"
                    f"observe remodel_cycles({num(remodel_cycles)})\n"
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
            "ADJ-LADDER rung 127 — bone-densitometry index from four stated quantities (a NEW panel: bone densitometry, and the "
            "SUBTRACT sibling of rung-126's add-a-quotient denominator). From a lone mineral load divided by an effective matrix (a "
            "matrix baseline with a per-cycle resorption_pool/remodel_cycles quotient subtracted from it), compute the density index "
            "(mineral_load/(matrix_baseline-resorption_pool/remodel_cycles)), the effective matrix "
            "(matrix_baseline-resorption_pool/remodel_cycles), or the per-cycle resorption (resorption_pool/remodel_cycles). Each item "
            "is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, a LONE TERM OVER A SUBTRACT-A-QUOTIENT THREE-TERM DENOMINATOR a/(b-c/d) (divide the resorption "
            "pool by the remodel cycles, subtract that from the matrix baseline, then divide the mineral load by that whole effective "
            "matrix, so a/(b-c/d) = (a/(b-(c/d))); the minus twin of rung-126's a/(b+c/d), mirroring how rung-125 (b-c+d) mirrors "
            "rung-124 (b-c-d). The precedence-and-grouping slips ride alongside as distractors. The harness matches the scalar to the "
            "printed options. The density index is a rate (mineral load per unit of effective matrix), framed as an INDEX so the "
            "dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed quantities via - "
            "and / — no constant leaks, and neither the effective matrix, the per-cycle quotient, nor the density index ever appears as "
            "a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a "
            "variable name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make: dropping the denominator parentheses so only the matrix baseline divides (a/b-c/d, evaluating (a/b)-(c/d), a "
            "wrong grouping) and subtracting before dividing the resorption (a/((b-c)/d) = a*d/(b-c), breaking the c/d-binds-first "
            "precedence, a wrong grouping). The core confusion tested is that a/(b-c/d) is (a/(b-(c/d))), not a/b-c/d and not "
            "a/((b-c)/d). This rung SUBTRACTS a quotient inside the denominator so positivity is NOT automatic; each table guards "
            "matrix_baseline - resorption_pool >= 2 (keeping b-c, the effective matrix b-c/d, and the misgrouped denom all positive) "
            "and mineral_load/matrix_baseline - resorption_pool/remodel_cycles > 0 (crossed positive), keeping the five family values "
            "pairwise distinct and all three queried readouts varying across the panel, all asserted strictly positive at build time."
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
