"""Generate rung-102 (sports-medicine / resistance-training rep accounting) items.json for the ADJ-LADDER.

Rung 102 opens the **sports-medicine / resistance-training** panel on the quantitative band — the arithmetic of a lifting
session's total reps performed. A `planned_reps` count MINUS a `missed_reps` count gives the scheduled reps actually
completed (the reps the athlete got through from the prescribed plan), a `set_count` TIMES an `added_per_set` count gives
the bonus reps (extra reps tacked on beyond the plan, the same number added across each set), and the bonus reps are ADDED
to the scheduled reps completed to give the total reps performed. A **difference plus a product** introduces a genuinely
NEW arithmetic family on the ladder: `a-b+c*d`, i.e. `((a-b) + (c*d))`.

This is genuinely new — the first time the ladder adds a bare PRODUCT to a bare DIFFERENCE. It is the **plus-sibling of
rung-101** `a+b-c*d` (a sum minus a product): rung-101 subtracted the product from a *sum*, rung-102 adds the product to a
*difference*. No prior rung took a difference plus a product: rung-91 `a+b+c*d` added a product to a SUM, rung-34
`a*b+c*d` summed two PRODUCTS, rung-35 `a*b-c*d` subtracted two products, rung-31 subtracted two differences, and rungs
79/80 attach a `c/d` division rather than a `c*d` product to the `a*b` term. The operator order matters: `a-b+c*d` is
`((a-b) + (c*d))` (the difference forms, the product forms, then the product is added to the difference — multiplication
binds tighter than the addition, and the leading subtraction and the trailing addition are the low-precedence joins), NOT
`(a-b+c)*d` (folding the `+c` inside so the set count is added to the scheduled-completed reps *before* multiplying by the
added-per-set count) and NOT `(a*b)+(c-d)` (multiplying the first pair and differencing the second pair, mispairing which
pair is the product and which is the difference) — the two distractors exploit exactly those confusions.

The setup: a `planned_reps`, a `missed_reps`, a `set_count`, and an `added_per_set`. The total is:

  TOTAL REPS           (planned_reps - missed_reps) + (set_count * added_per_set)  [ a difference plus a product ]
  SCHEDULED COMPLETED  planned_reps - missed_reps                                  [ the difference, added to ]
  BONUS REPS           set_count * added_per_set                                   [ the product, added ]

The **total reps** is what makes this rung distinctive — it is the ladder's first **bare DIFFERENCE plus a bare PRODUCT**.
(The scheduled-completed reps `a-b` and the bonus reps `c*d` ride alongside as component readouts, so the panel teaches the
whole calculation — exactly as rungs 47-101 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the missed reps from the planned reps into the scheduled-completed reps, the
multiplication of the set count by the added-per-set count into the bonus reps, then the addition of the bonus reps to the
scheduled-completed reps (the product forming before it is added, so a-b+c*d evaluates as ((a-b)+(c*d))) — and the harness
reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../100/101. This rung exercises the engine across a **difference plus a product** — the fact that `a-b+c*d` is
`((a-b)+(c*d))` and NOT `(a-b+c)*d` and NOT `(a*b)+(c-d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-`, and `*` —
**no structural constants** — so no numeric literal appears in any program, and neither the scheduled-completed reps, the
bonus reps, nor any total figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`planned_reps`, `missed_reps`, `set_count`, `added_per_set`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (planned_reps - missed_reps + set_count) * added_per_set  fold the `+ set_count` inside the parentheses so the
                                                                       set count is added to the scheduled-completed reps
                                                                       *before* multiplying by the added-per-set count (the
                                                                       classic `a-b+c*d` vs `(a-b+c)*d` precedence error),
                                                                       and
  SWAPPED    (planned_reps * missed_reps) + (set_count - added_per_set)  multiply the first pair and difference the second
                                                                       pair, mispairing which pair is the product and which
                                                                       is the difference (`(a*b)+(c-d)` instead of
                                                                       `(a-b)+(c*d)`),

which are exactly the mistakes a student makes (folding the addition inside the parentheses before multiplying, or
mispairing which pair is a difference and which is a product). Gold rotates A-E by index. QUERIED (used as gold) = the three
real readouts; all five always appear as options.

Distinctness and positivity: the tables are chosen so `planned_reps > missed_reps` (scheduled-completed reps strictly
positive — the athlete completes at least some of the plan) and `planned_reps * missed_reps + set_count > added_per_set`
(the swapped figure strictly positive), so no family member is ever zero or negative; every observed quantity is >= 2. The
bonus reps (a product of positives) and the total reps (a positive difference plus a positive product) are trivially
positive, and the crossed figure `(a-b+c)*d` is positive because `a-b > 0` so `a-b+c > 0`. The tables are chosen so the five
family values are pairwise distinct with a comfortable margin, and — so all three queried readouts vary across the panel —
the seven tables give distinct total reps, distinct scheduled-completed reps, and distinct bonus reps, all asserted at build
time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PLANNED_REPS, MISSED_REPS, SET_COUNT, ADDED_PER_SET) — a planned-reps count minus a missed-reps count for the
# scheduled-completed reps, a set count times an added-per-set count for the bonus reps, all plain positive numbers >= 2.
# Each table satisfies planned_reps > missed_reps (scheduled completed > 0) and planned_reps * missed_reps + set_count >
# added_per_set (swapped > 0), so every family member is strictly positive (no negatives anywhere); the five family values
# are asserted pairwise-distinct below. The seven tables give distinct total reps, distinct scheduled-completed reps, and
# distinct bonus reps so all three queried readouts vary across the panel.
TABLES = [
    (4, 2, 2, 3),
    (6, 3, 3, 4),
    (8, 4, 4, 2),
    (10, 5, 5, 6),
    (12, 6, 6, 7),
    (14, 7, 7, 5),
    (13, 5, 8, 8),
]

# The option family (5 members), all built from the four observed quantities via +, -, and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "total_reps",
        "total reps performed (the scheduled-completed reps plus the bonus reps)",
        "(planned_reps - missed_reps) + (set_count * added_per_set)",
    ),
    (
        "scheduled_completed",
        "the scheduled-completed reps (the planned reps minus the missed reps, the difference the bonus reps are added to)",
        "planned_reps - missed_reps",
    ),
    (
        "bonus_reps",
        "the bonus reps (the set count times the added-per-set count, the product added to the scheduled-completed reps)",
        "set_count * added_per_set",
    ),
    (
        "crossed",
        "the planned reps minus the missed reps plus the set count, all multiplied by the added-per-set count, folding the addition inside the parentheses so the set count is added before multiplying (a wrong grouping)",
        "(planned_reps - missed_reps + set_count) * added_per_set",
    ),
    (
        "swapped",
        "the planned reps times the missed reps, plus the set count minus the added-per-set count, multiplying the first pair and differencing the second pair instead (a wrong pairing)",
        "(planned_reps * missed_reps) + (set_count - added_per_set)",
    ),
]
QUERIED = ["total_reps", "scheduled_completed", "bonus_reps"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(planned_reps, missed_reps, set_count, added_per_set):
    # Operation order mirrors the ADJ programs exactly (the difference forms, the product forms, then the product is added
    # to the difference, so a-b+c*d evaluates as ((a-b)+(c*d))), so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "total_reps": (planned_reps - missed_reps) + (set_count * added_per_set),
        "scheduled_completed": planned_reps - missed_reps,
        "bonus_reps": set_count * added_per_set,
        "crossed": (planned_reps - missed_reps + set_count) * added_per_set,
        "swapped": (planned_reps * missed_reps) + (set_count - added_per_set),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for planned_reps, missed_reps, set_count, added_per_set in TABLES:
        # Every observed quantity is a plain positive number >= 2, and the tables guarantee planned_reps > missed_reps
        # (scheduled completed > 0) and planned_reps * missed_reps + set_count > added_per_set (swapped > 0), so every
        # family member is strictly positive with no negative anywhere.
        assert (
            planned_reps >= 2
            and missed_reps >= 2
            and set_count >= 2
            and added_per_set >= 2
        ), (planned_reps, missed_reps, set_count, added_per_set)
        assert planned_reps > missed_reps, (
            planned_reps, missed_reps, set_count, added_per_set,
        )
        assert planned_reps * missed_reps + set_count > added_per_set, (
            planned_reps, missed_reps, set_count, added_per_set,
        )
        fv = family_values(planned_reps, missed_reps, set_count, added_per_set)
        for key, v in fv.items():
            assert v > 0, (key, planned_reps, missed_reps, set_count, added_per_set, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    planned_reps,
                    missed_reps,
                    set_count,
                    added_per_set,
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
                planned_reps,
                missed_reps,
                set_count,
                added_per_set,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r102smed-{idx + 1:02d}",
                "qtype": "smed_total_reps",
                "stem": (
                    f"A resistance-training log records planned reps of {num(planned_reps)} minus missed reps of "
                    f"{num(missed_reps)}, plus a set count of {num(set_count)} times an added-per-set count of "
                    f"{num(added_per_set)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe planned_reps({num(planned_reps)})\n"
                    f"observe missed_reps({num(missed_reps)})\n"
                    f"observe set_count({num(set_count)})\n"
                    f"observe added_per_set({num(added_per_set)})\n"
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
            "ADJ-LADDER rung 102 — total reps performed from four stated quantities (a NEW panel: sports-medicine / "
            "resistance-training rep accounting). From a planned-reps count minus a missed-reps count for the "
            "scheduled-completed reps, a set count times an added-per-set count for the bonus reps, and the bonus reps added "
            "to the scheduled-completed reps, compute the total reps ((planned_reps-missed_reps)+(set_count*added_per_set)), "
            "the scheduled-completed reps (planned_reps-missed_reps), or the bonus reps (set_count*added_per_set). Each item "
            "is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, A DIFFERENCE PLUS A PRODUCT a-b+c*d (subtract b from a, multiply c and d, add the "
            "product to the difference, so a-b+c*d = ((a-b)+(c*d)); the FIRST time the ladder adds a bare PRODUCT to a bare "
            "DIFFERENCE — the PLUS-SIBLING of rung-101 a+b-c*d which subtracted the product from a sum; rung-91 a+b+c*d added "
            "a product to a sum, rung-34 a*b+c*d summed two products, rung-35 a*b-c*d subtracted two products) — and the "
            "harness matches the scalar to the printed options. Contamination-safe: every figure is built only from the four "
            "observed quantities via +, -, and * — no constant leaks, and neither the scheduled-completed reps, the bonus "
            "reps, nor any total figure ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the same "
            "four quantities, so the distractors are exactly the slips students make: folding the addition inside the "
            "parentheses so the set count is added before multiplying ((a-b+c)*d, a wrong grouping) and multiplying the first "
            "pair while differencing the second pair ((a*b)+(c-d), a wrong pairing). The core confusion tested is that "
            "a-b+c*d is ((a-b)+(c*d)), not (a-b+c)*d and not (a*b)+(c-d). Each table guarantees the planned reps exceed the "
            "missed reps and the planned-times-missed product plus the set count exceeds the added-per-set count, so every "
            "figure stays strictly positive."
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
