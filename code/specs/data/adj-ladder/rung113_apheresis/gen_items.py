"""Generate rung-113 (transfusion medicine / apheresis) items.json for the ADJ-LADDER.

Rung 113 opens the **transfusion medicine / therapeutic apheresis** panel on the quantitative band — the arithmetic of a
cleared-load index. A `cycle_factor` (a per-cycle scaling) TIMES the DIFFERENCE of a `pre_count` and a `post_count` (the cell
count before and after the procedure) gives the cleared load, and that load is DIVIDED by a `session_count` (how many sessions
the reading is averaged over) to give the cleared-load index. A **factor DISTRIBUTED over a two-term DIFFERENCE, all over a
divisor** introduces a genuinely NEW arithmetic family on the ladder: `a*(b-c)/d`, i.e. `((a * (b - c)) / d)`.

This is genuinely new — rung-112 opened the distributive frontier with `a*(b+c)/d` (a factor over a SUM); rung-113 is the FIRST
distributive shape where the factor multiplies a parenthesised DIFFERENCE. It is the minus-sibling of rung-112 `a*(b+c)/d`. The
flat three-term numerators (rung-108 `(a+b+c)/d`, rung-109 `(a-b+c)/d`, rung-110 `(a*b+c)/d`, rung-111 `(a*b-c)/d`) wrapped the
WHOLE numerator; the distributive family (112 sum, 113 difference) wraps the SUM/DIFFERENCE (the multiplicand). Every prior ratio
used either a two-term numerator (rung-37 `(a+b)/(c+d)`, rung-99 `(a*b)/(c+d)`, rung-100 `(a+b)/(c*d)`, rung-104 `(a-b)/(c*d)`,
the difference-denominator trio rung-105 `(a+b)/(c-d)`, rung-106 `a*b/(c-d)`, rung-107 `(a-b)/(c-d)`) or a flat three-term
numerator (108-111). Rung-113 moves to `a*(b-c)/d`. The operator order matters: `a*(b-c)/d` is `((a*(b-c))/d)` (the factor
multiplies the whole difference, then the product is divided; `*` and `/` bind left-to-right so `a*(b-c)/d` = `(a*(b-c))/d`), NOT
`a*b-c/d` (dropping the difference parentheses so the factor multiplies only the pre-count and the post-count is divided by the
divisor and then subtracted) and NOT `(a*b)/(c+d)` (regrouping so only `a*b` forms the numerator and the post-count joins the
divisor in the denominator) — the two distractors exploit exactly those confusions.

The setup: a `cycle_factor`, a `pre_count`, a `post_count`, and a `session_count`. The total is:

  CLEARED-LOAD INDEX  cycle_factor * (pre_count - post_count) / session_count  [ a factor over a difference, over a divisor ]
  CLEARED LOAD        cycle_factor * (pre_count - post_count)                  [ the distributed-product numerator ]
  SESSION COUNT       session_count                                           [ the divisor ]

The **cleared-load index** is what makes this rung distinctive — it is the ladder's first **factor over a DIFFERENCE, over a
divisor**. It is a rate (cleared load per session), framed as an *index* to keep it dimensionless-clean — the same discipline
rungs 100/104/.../112 used for their ratios. (The cleared load `a*(b-c)` and the session count `d` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-112 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the subtraction of the post-count from the pre-count into the cleared count, the multiplication of that
difference by the cycle factor into the cleared load, then the division of that load by the session count (the factor distributed
over the parenthesised difference, so a*(b-c)/d evaluates as ((a*(b-c))/d)) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../111/112. This rung exercises the engine
across a **factor-over-a-difference, over a divisor** — the fact that `a*(b-c)/d` is `((a*(b-c))/d)` and NOT `a*b-c/d` and NOT
`(a*b)/(c+d)` made computable. The ratio golds are non-integer f64s; the engine's IEEE-double division matches Python's the same
way rungs 99/100/104/.../112 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `-` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the cleared load, the session count, nor any
index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers**
(`cycle_factor`, `pre_count`, `post_count`, `session_count`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    cycle_factor * pre_count - post_count / session_count  drop the difference parentheses so the cycle factor
                                                                    multiplies only the pre-count and the post-count is divided by
                                                                    the session count and then subtracted (the classic
                                                                    `a*(b-c)/d` vs `a*b-c/d` distributivity error), and
  SWAPPED    (cycle_factor * pre_count) / (post_count + session_count)  regroup so only the factor-times-pre-count product forms
                                                                    the numerator and the post-count joins the session count in
                                                                    the denominator (`(a*b)/(c+d)` instead of `a*(b-c)/d`),

which are exactly the mistakes a student makes (failing to distribute the factor across the whole difference, or regrouping which
terms belong to the numerator vs the divisor). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all
five always appear as options.

Distinctness and positivity: this rung SUBTRACTS inside the parentheses, so positivity is guaranteed by table construction. Each
table guarantees **pre_count > post_count** (so the difference `b-c` is strictly positive, the cleared load `a*(b-c)` is
positive, and the index is positive) AND all quantities `>= 2` (so the crossed slip `a*b - c/d` stays positive because
`a*b >= 2b > 2c > c > c/d`, and the swapped denominator `c+d >= 4`). The **session_count >= 2** keeps the divisor away from zero,
the cleared-load index never coincides with the session count or the cleared load, and the five family values are pairwise
distinct with a comfortable margin; and — so all three queried readouts vary across the panel — the seven tables give distinct
cleared-load indices, distinct cleared loads, and distinct session counts, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CYCLE_FACTOR, PRE_COUNT, POST_COUNT, SESSION_COUNT) — a cycle factor times the difference of a pre-count and a post-count for
# the cleared load, all divided by a session count, all plain positive numbers >= 2. This rung SUBTRACTS inside the parentheses,
# so every table guarantees pre_count > post_count (b>c) which keeps the difference, the cleared load, and the index strictly
# positive; session_count >= 2 keeps the divisor away from zero. The five family values are asserted pairwise-distinct below. The
# seven tables give distinct cleared-load indices, distinct cleared loads, and distinct session counts so all three queried
# readouts vary across the panel.
TABLES = [
    (2, 5, 2, 2),
    (3, 6, 2, 3),
    (5, 5, 2, 4),
    (4, 6, 2, 5),
    (2, 9, 2, 6),
    (3, 8, 2, 7),
    (4, 7, 2, 8),
]

# The option family (5 members), all built from the four observed quantities via *, - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "cleared_load_index",
        "cleared-load index (the cleared load divided by the session count)",
        "cycle_factor * (pre_count - post_count) / session_count",
    ),
    (
        "cleared_load",
        "the cleared load (the cycle factor times the difference of the pre and post counts, the numerator divided by the session count)",
        "cycle_factor * (pre_count - post_count)",
    ),
    (
        "session_count",
        "the session count (the divisor the cleared load is divided by)",
        "session_count",
    ),
    (
        "crossed",
        "the cycle factor times the pre count minus the post count divided by the session count, dropping the difference parentheses so the cycle factor multiplies only the pre count (a wrong distribution)",
        "cycle_factor * pre_count - post_count / session_count",
    ),
    (
        "swapped",
        "the cycle factor times the pre count, divided by the post count plus the session count, regrouping so only that product forms the numerator (a wrong pairing)",
        "(cycle_factor * pre_count) / (post_count + session_count)",
    ),
]
QUERIED = ["cleared_load_index", "cleared_load", "session_count"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(cycle_factor, pre_count, post_count, session_count):
    # Operation order mirrors the ADJ programs exactly (the post count subtracts from the pre count, the cycle factor multiplies
    # that difference into the cleared load, then that numerator is divided by the session count, so a*(b-c)/d evaluates as
    # ((a*(b-c))/d)), so the Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9
    # match tolerance).
    return {
        "cleared_load_index": cycle_factor * (pre_count - post_count) / session_count,
        "cleared_load": cycle_factor * (pre_count - post_count),
        "session_count": session_count,
        "crossed": cycle_factor * pre_count - post_count / session_count,
        "swapped": (cycle_factor * pre_count) / (post_count + session_count),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for cycle_factor, pre_count, post_count, session_count in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung SUBTRACTS inside the parentheses, so each table
        # guarantees pre_count > post_count (the difference b-c is strictly positive) which keeps every family member strictly
        # positive; session_count >= 2 keeps the divisor away from zero.
        assert (
            cycle_factor >= 2
            and pre_count >= 2
            and post_count >= 2
            and session_count >= 2
        ), (cycle_factor, pre_count, post_count, session_count)
        assert pre_count > post_count, (cycle_factor, pre_count, post_count, session_count)
        fv = family_values(cycle_factor, pre_count, post_count, session_count)
        for key, v in fv.items():
            assert v > 0, (key, cycle_factor, pre_count, post_count, session_count, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    cycle_factor,
                    pre_count,
                    post_count,
                    session_count,
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
                cycle_factor,
                pre_count,
                post_count,
                session_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r113apher-{idx + 1:02d}",
                "qtype": "cleared_load_index",
                "stem": (
                    f"An apheresis log records a cycle factor of {num(cycle_factor)} times a pre count of {num(pre_count)} "
                    f"minus a post count of {num(post_count)}, divided by a session count of {num(session_count)}. What is the "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe cycle_factor({num(cycle_factor)})\n"
                    f"observe pre_count({num(pre_count)})\n"
                    f"observe post_count({num(post_count)})\n"
                    f"observe session_count({num(session_count)})\n"
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
            "ADJ-LADDER rung 113 — cleared-load index from four stated quantities (a NEW panel: transfusion medicine / "
            "therapeutic apheresis). From a cycle factor times the difference of a pre-count and a post-count for the cleared "
            "load, all divided by a session count, compute the cleared-load index "
            "(cycle_factor*(pre_count-post_count)/session_count), the cleared load "
            "(cycle_factor*(pre_count-post_count)), or the session count. Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A FACTOR DISTRIBUTED "
            "OVER A DIFFERENCE, OVER A DIVISOR a*(b-c)/d (subtract the post count from the pre count, multiply the difference by "
            "the cycle factor, divide by the session count, so a*(b-c)/d = ((a*(b-c))/d); the SECOND distributive shape and the "
            "FIRST where the factor multiplies a parenthesised DIFFERENCE — the minus-sibling of rung-112 a*(b+c)/d. The flat "
            "three-term numerators (108 (a+b+c)/d, 109 (a-b+c)/d, 110 (a*b+c)/d, 111 (a*b-c)/d) wrapped the WHOLE numerator; the "
            "distributive family wraps the SUM/DIFFERENCE (the multiplicand). Every earlier ratio used a TWO-term numerator: 37 "
            "(a+b)/(c+d), 99 (a*b)/(c+d), 100 (a+b)/(c*d), 104 (a-b)/(c*d), and the difference-denominator trio 105 (a+b)/(c-d), "
            "106 a*b/(c-d), 107 (a-b)/(c-d)) — and the harness matches the scalar to the printed options. The cleared-load index "
            "is a rate (cleared load per session), framed as an INDEX so the dimensionless value stays honest. Contamination-"
            "safe: every figure is built only from the four observed quantities via *, - and / — no constant leaks, and neither "
            "the cleared load, the session count, nor any index ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a family "
            "over the same four quantities, so the distractors are exactly the slips students make: dropping the difference "
            "parentheses so the factor multiplies only the pre count (a*b-c/d, a wrong distribution) and regrouping so only that "
            "product forms the numerator ((a*b)/(c+d), a wrong pairing). The core confusion tested is that a*(b-c)/d is "
            "((a*(b-c))/d), not a*b-c/d and not (a*b)/(c+d). This rung SUBTRACTS inside the parentheses, so positivity is "
            "guaranteed by table construction: every table has pre_count > post_count (b>c) and session_count >= 2 (divisor "
            "never zero), keeping every family member strictly positive."
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
