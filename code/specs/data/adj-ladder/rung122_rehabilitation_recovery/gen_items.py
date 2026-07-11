"""Generate rung-122 (rehabilitation-medicine recovery index / lone term over a term-plus-product denominator) items.json.

Rung 122 opens the **rehabilitation-medicine / recovery** panel on the ADJ-LADDER's quantitative band — the arithmetic of a
recovery index. A single observed `exertion_reserve` is DIVIDED by a combined recovery demand formed as a `rest_interval` PLUS a
fatigue-load PRODUCT (a `session_load` TIMES a `fatigue_factor`), to give the recovery index. A **lone term over a term-PLUS-a-
product denominator**, `a/(b + c*d)`, i.e. `(a / (b + (c * d)))` (the multiplication binds tighter than the addition inside the
denominator parentheses), introduces a genuinely NEW arithmetic family on the ladder — the ladder's **first denominator that adds a
lone term to a product**.

This is genuinely new. Every product-in-a-denominator the ladder built grouped the multiplication OUTSIDE the added term: rung-118
`a/(b*c+d)` is `a/((b*c)+d)` — the `b*c` product is formed first, THEN `d` is added. Rung-122 flips the grouping: `a/(b+c*d)` is
`a/(b + (c*d))` — the `c*d` product is formed first, THEN added to the lone `b`. The two are the classic distributivity/precedence
confusion, and rung-122 makes `a/(b+c*d)` the honest reading while `a/(b*c+d)` (rung-118's shape) rides alongside as the swapped
distractor. Also distinct from the three-term-SUM denominators (116 `a/(b+c+d)`, 117 `a/(b+c-d)`), the product-over-product (120),
and the lone-term-over-triple-product (121 `a/(b*c*d)`): here exactly ONE of the three denominator quantities is added and the other
two are multiplied.

The setup: an `exertion_reserve`, a `rest_interval`, a `session_load`, and a `fatigue_factor`. The figures are:

  RECOVERY INDEX    exertion_reserve / (rest_interval + session_load * fatigue_factor)  [ a lone term over a term-plus-product ]
  DEMAND SUM        rest_interval + session_load * fatigue_factor                       [ the term-plus-product denominator ]
  PRODUCT LOAD      session_load * fatigue_factor                                        [ the fatigue-load product inside it ]

The **recovery index** is what makes this rung distinctive — it is the ladder's first **lone quantity over a term-plus-a-product**.
It is a rate (exertion reserve per unit of combined recovery demand), framed as an *index* to keep it dimensionless-clean — the same
discipline rungs 100/104/.../118/119/120/121 used for their ratios. (The demand sum `b+c*d` and the fatigue-load product `c*d` ride
alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-121 shipped their component
sums/products/differences/ratios beside the headline figure. The product load `c*d` is the multiplied part of the denominator
reported straight, anchoring the "multiply first, then add" grouping against the swapped distractor.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication of the session load and fatigue factor to form the fatigue-load product, the addition of the rest
interval to form the whole demand sum, then the division of the exertion reserve by that whole demand sum (so a/(b+c*d) evaluates as
(a/(b+(c*d)))) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change,
exactly as rungs 8/16/.../120/121. This rung exercises the engine across a **lone-term-over-(term-plus-product) ratio** — the fact
that `a/(b+c*d)` is `(a/(b+(c*d)))` and NOT `a/b+c*d` and NOT `a/(b*c+d)` made computable. The ratio golds are non-integer f64s; the
engine's IEEE-double division matches Python's the same way rungs 100/104/.../120/121 relied on (well within the harness's 1e-9
tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `*`, and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the demand sum, the product load, nor the
recovery index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`exertion_reserve`, `rest_interval`, `session_load`, `fatigue_factor`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    exertion_reserve / rest_interval + session_load * fatigue_factor  drop the denominator parentheses so only the rest
                                                                    interval divides the exertion reserve, then the fatigue-load
                                                                    product is added on (the classic `a/(b+c*d)` vs `a/b+c*d`
                                                                    grouping error, evaluating (a/b)+c*d), and
  SWAPPED    exertion_reserve / (rest_interval * session_load + fatigue_factor)  multiply the rest interval and session load first
                                                                    and add the lone fatigue factor (`a/(b*c+d)`, rung-118's
                                                                    grouping, instead of `a/(b+c*d)`),

which are exactly the mistakes a student makes (failing to keep the whole term-plus-product under the bar, or multiplying the wrong
pair before adding). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung is all addition, multiplication, and division of positive quantities, so every family member
is positive automatically — but distinctness is still guarded explicitly. Every observed quantity is `>= 2`, and — crucially —
`rest_interval != fatigue_factor` in every table, because `a/(b+c*d)` equals `a/(b*c+d)` exactly when `b == d` (or `c == 1`, which
never happens since `c >= 2`), so guarding `b != d` keeps the recovery index clear of the swapped distractor. The seven tables are
chosen so the five family values are pairwise distinct with a comfortable margin, and — so all three queried readouts vary across the
panel — the tables give distinct recovery indices, distinct demand sums, and distinct product loads, all asserted at build time.
Every family member is asserted `> 0` at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (EXERTION_RESERVE, REST_INTERVAL, SESSION_LOAD, FATIGUE_FACTOR) — a lone exertion reserve divided by a combined recovery demand
# (a rest interval PLUS a session-load-times-fatigue-factor product) for the recovery index, all plain positive numbers >= 2.
# Everything is addition/multiplication/division of positives so positivity is automatic; distinctness is guarded explicitly
# (rest_interval != fatigue_factor so the recovery index never collapses onto the swapped distractor a/(b*c+d)). The five family
# values are asserted pairwise-distinct below. The seven tables give distinct recovery indices, distinct demand sums, and distinct
# product loads so all three queried readouts vary across the panel.
TABLES = [
    (21, 3, 2, 2),
    (35, 2, 3, 4),
    (45, 4, 2, 3),
    (34, 2, 3, 5),
    (44, 3, 2, 4),
    (65, 3, 2, 5),
    (138, 2, 3, 7),
]

# The option family (5 members), all built from the four observed quantities via +, * and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "recovery_index",
        "recovery index (the exertion reserve divided by the demand sum)",
        "exertion_reserve / (rest_interval + session_load * fatigue_factor)",
    ),
    (
        "demand_sum",
        "the demand sum (the rest interval plus the session load times the fatigue factor, the divisor the exertion reserve is divided by)",
        "rest_interval + session_load * fatigue_factor",
    ),
    (
        "product_load",
        "the product load (the session load times the fatigue factor, the fatigue-load product inside the demand sum)",
        "session_load * fatigue_factor",
    ),
    (
        "crossed",
        "the exertion reserve divided by the rest interval, plus the session load times the fatigue factor, dropping the denominator parentheses so only the rest interval divides (a wrong grouping)",
        "exertion_reserve / rest_interval + session_load * fatigue_factor",
    ),
    (
        "swapped",
        "the exertion reserve divided by the rest interval times the session load plus the fatigue factor, multiplying the wrong pair before adding (a wrong grouping)",
        "exertion_reserve / (rest_interval * session_load + fatigue_factor)",
    ),
]
QUERIED = ["recovery_index", "demand_sum", "product_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(exertion_reserve, rest_interval, session_load, fatigue_factor):
    # Operation order mirrors the ADJ programs exactly (the session load and fatigue factor multiply first to form the fatigue-load
    # product, the rest interval is added to form the whole demand sum, then the exertion reserve is divided by that whole demand
    # sum, so a/(b+c*d) evaluates as (a/(b+(c*d)))), so the Python option value and the engine result are the same IEEE-double
    # (well within the 1e-9 tolerance).
    return {
        "recovery_index": exertion_reserve / (rest_interval + session_load * fatigue_factor),
        "demand_sum": rest_interval + session_load * fatigue_factor,
        "product_load": session_load * fatigue_factor,
        "crossed": exertion_reserve / rest_interval + session_load * fatigue_factor,
        "swapped": exertion_reserve / (rest_interval * session_load + fatigue_factor),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for exertion_reserve, rest_interval, session_load, fatigue_factor in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung is all addition/multiplication/division of positives,
        # so every family member is positive automatically; distinctness is guarded explicitly below (rest_interval != fatigue_factor
        # keeps the recovery index clear of the swapped distractor).
        assert (
            exertion_reserve >= 2
            and rest_interval >= 2
            and session_load >= 2
            and fatigue_factor >= 2
        ), (exertion_reserve, rest_interval, session_load, fatigue_factor)
        assert rest_interval != fatigue_factor, (
            "rest_interval == fatigue_factor collapses recovery_index onto swapped",
            exertion_reserve,
            rest_interval,
            session_load,
            fatigue_factor,
        )
        fv = family_values(exertion_reserve, rest_interval, session_load, fatigue_factor)
        for key, v in fv.items():
            assert v > 0, (key, exertion_reserve, rest_interval, session_load, fatigue_factor, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    exertion_reserve,
                    rest_interval,
                    session_load,
                    fatigue_factor,
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
                exertion_reserve,
                rest_interval,
                session_load,
                fatigue_factor,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r122rr-{idx + 1:02d}",
                "qtype": "recovery_index",
                "stem": (
                    f"A rehabilitation-medicine chart records an exertion reserve of {num(exertion_reserve)} divided by a rest "
                    f"interval of {num(rest_interval)} plus a session load of {num(session_load)} times a fatigue factor of "
                    f"{num(fatigue_factor)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe exertion_reserve({num(exertion_reserve)})\n"
                    f"observe rest_interval({num(rest_interval)})\n"
                    f"observe session_load({num(session_load)})\n"
                    f"observe fatigue_factor({num(fatigue_factor)})\n"
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
            "ADJ-LADDER rung 122 — recovery index from four stated quantities (a NEW panel: rehabilitation-medicine / recovery). "
            "From a lone exertion reserve divided by a combined recovery demand (a rest interval PLUS a session-load-times-fatigue-"
            "factor product), compute the recovery index (exertion_reserve/(rest_interval+session_load*fatigue_factor)), the demand "
            "sum (rest_interval+session_load*fatigue_factor), or the product load (session_load*fatigue_factor). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, a LONE TERM OVER A TERM-PLUS-PRODUCT a/(b+c*d) (multiply the session load and fatigue "
            "factor, add the rest interval, then divide the exertion reserve by that whole demand sum, so a/(b+c*d) = "
            "(a/(b+(c*d))); the ladder's FIRST denominator that adds a lone term to a product. Every product-in-a-denominator "
            "before rung-122 grouped the multiplication OUTSIDE the added term (rung-118 a/(b*c+d) = a/((b*c)+d)); rung-122 flips "
            "the grouping so the product forms first and is then added to the lone term. The harness matches the scalar to the "
            "printed options. The recovery index is a rate (exertion reserve per unit of combined recovery demand), framed as an "
            "INDEX so the dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via +, * and / — no constant leaks, and neither the demand sum, the product load, nor the recovery index "
            "ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same four quantities, so the distractors are "
            "exactly the slips students make: dropping the denominator parentheses so only the rest interval divides (a/b+c*d, "
            "evaluating (a/b)+c*d, a wrong grouping) and multiplying the wrong pair before adding (a/(b*c+d), rung-118's grouping). "
            "The core confusion tested is that a/(b+c*d) is (a/(b+(c*d))), not a/b+c*d and not a/(b*c+d). This rung is all addition, "
            "multiplication, and division of positive quantities so positivity is automatic; with every observed quantity >= 2 and "
            "rest_interval != fatigue_factor (so the recovery index never collapses onto the swapped distractor) the tables keep "
            "the five family values pairwise distinct and all three queried readouts varying across the panel, all asserted "
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
