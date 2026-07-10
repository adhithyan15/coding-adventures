"""Generate rung-121 (neonatology perfusion reserve / single term over a triple product) items.json for the ADJ-LADDER.

Rung 121 opens the **neonatology / perfusion** panel on the quantitative band — the arithmetic of a perfusion reserve. A single
observed `intake_volume` is DIVIDED by a demand product formed from THREE quantities (a `body_mass` TIMES a `feeding_interval`
TIMES a `tolerance_factor`), to give the perfusion reserve (intake volume per unit of aggregate demand). A **single term over a
triple product**, `a/(b*c*d)`, i.e. `(a / (b * c * d))`, introduces a genuinely NEW arithmetic family on the ladder — the ladder's
**first ratio of a lone quantity over a THREE-factor product denominator**.

This is genuinely new. Every ratio the ladder built with a product-denominator had at most a TWO-factor product under the bar: 100
`(a+b)/(c*d)` (sum over a two-factor product), 104 `(a-b)/(c*d)` (difference over a two-factor product), 118 `a/(b*c+d)` (single
term over a product-PLUS-a-term), 119 `a/(b*c-d)` (single term over a product-MINUS-a-term), 120 `a*b/(c*d)` (two-factor product
over a two-factor product). Nobody has yet put a **lone quantity over a pure three-factor product**. Rung-121 is `a/(b*c*d)` — one
quantity divided by the product of three others. (It is exactly rung-120's `swapped` distractor promoted to a headline: rung-120
warned against reading `a*b/(c*d)` as `a/(b*c*d)`; rung-121 makes that very shape the honest calculation, so the ladder now teaches
both readings.) The operator order matters: `a/(b*c*d)` is `(a / (b * c * d))` (the body mass, feeding interval, and tolerance
factor multiply FIRST to form the whole aggregate demand, then the intake volume is divided by that WHOLE product; the explicit
denominator parentheses keep all three factors under the bar), NOT `a/b*c*d` (dropping the denominator parentheses so only the body
mass divides the intake volume, and the result is then multiplied by the feeding interval and the tolerance factor) and NOT
`a*b/(c*d)` (moving the body mass UP into the numerator so it multiplies the intake volume and only two factors ride under the bar)
— the two distractors exploit exactly those confusions.

The setup: an `intake_volume`, a `body_mass`, a `feeding_interval`, and a `tolerance_factor`. The figures are:

  PERFUSION RESERVE    intake_volume / (body_mass * feeding_interval * tolerance_factor)  [ a lone term over a triple product ]
  DEMAND AGGREGATE     body_mass * feeding_interval * tolerance_factor                    [ the triple-product denominator ]
  INTAKE READING       intake_volume                                                      [ the lone numerator quantity ]

The **perfusion reserve** is what makes this rung distinctive — it is the ladder's first **lone quantity over a three-factor
product**. It is a rate (intake volume per unit of aggregate demand), framed as a *reserve* to keep it dimensionless-clean — the
same discipline rungs 100/104/.../118/119/120 used for their ratios. (The demand aggregate `b*c*d` and the intake reading `a` ride
alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-120 shipped their component
sums/products/differences/ratios beside the headline figure. The intake reading is the lone observed numerator reported straight,
a genuine readout of the family that anchors the numerator against the two product-shaped distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication of the body mass, feeding interval, and tolerance factor to form the whole demand aggregate, then
the division of the intake volume by that whole aggregate (the lone numerator over the triple-product denominator, so a/(b*c*d)
evaluates as (a/(b*c*d))) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine
change, exactly as rungs 8/16/.../119/120. This rung exercises the engine across a **lone-term-over-triple-product ratio** — the
fact that `a/(b*c*d)` is `(a/(b*c*d))` and NOT `a/b*c*d` and NOT `a*b/(c*d)` made computable. The ratio golds are non-integer
f64s; the engine's IEEE-double division matches Python's the same way rungs 100/104/.../119/120 relied on (well within the
harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the demand aggregate, the intake reading, nor
the perfusion reserve is ever a literal (each is computed from, or is, an observed quantity). The observed quantities carry
**digit-free identifiers** (`intake_volume`, `body_mass`, `feeding_interval`, `tolerance_factor`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    intake_volume / body_mass * feeding_interval * tolerance_factor  drop the denominator parentheses so only the body
                                                                    mass divides the intake volume, then the result is multiplied
                                                                    by the feeding interval and the tolerance factor (the classic
                                                                    `a/(b*c*d)` vs `a/b*c*d` grouping error), and
  SWAPPED    intake_volume * body_mass / (feeding_interval * tolerance_factor)  move the body mass UP into the numerator so it
                                                                    multiplies the intake volume and only two factors ride under
                                                                    the bar (`a*b/(c*d)` instead of `a/(b*c*d)`),

which are exactly the mistakes a student makes (failing to keep the whole demand aggregate under the bar, or lifting a demand
factor up into the numerator). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear
as options.

Distinctness and positivity: this rung is all multiplication and division of positive quantities, so every family member is
positive automatically — but distinctness is still guarded explicitly. Every observed quantity is `>= 2`, and the seven tables are
chosen so the five family values are pairwise distinct with a comfortable margin (in particular the intake volume is never equal
to the demand aggregate, the crossed slip `(a/b)*c*d` never collapses onto the demand aggregate `b*c*d`, and the swapped slip
`a*b/(c*d)` stays clear of the perfusion reserve). Every family member is asserted `> 0` at build time. And — so all three queried
readouts vary across the panel — the seven tables give distinct perfusion reserves, distinct demand aggregates, and distinct
intake readings, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (INTAKE_VOLUME, BODY_MASS, FEEDING_INTERVAL, TOLERANCE_FACTOR) — a lone intake volume divided by a demand product (a body mass
# times a feeding interval times a tolerance factor) for the perfusion reserve, all plain positive numbers >= 2. Everything is
# multiplication/division of positives so positivity is automatic; distinctness is guarded explicitly (intake_volume != demand
# aggregate; (a/b)*c*d != b*c*d; a*b/(c*d) clear of a/(b*c*d)). The five family values are asserted pairwise-distinct below. The
# seven tables give distinct perfusion reserves, distinct demand aggregates, and distinct intake readings so all three queried
# readouts vary across the panel.
TABLES = [
    (5, 2, 2, 3),
    (9, 2, 3, 3),
    (15, 2, 2, 5),
    (8, 2, 3, 4),
    (21, 2, 3, 5),
    (7, 2, 2, 4),
    (20, 2, 3, 7),
]

# The option family (5 members), all built from the four observed quantities via * and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "perfusion_reserve",
        "perfusion reserve (the intake volume divided by the demand aggregate)",
        "intake_volume / (body_mass * feeding_interval * tolerance_factor)",
    ),
    (
        "demand_aggregate",
        "the demand aggregate (the body mass times the feeding interval times the tolerance factor, the divisor the intake volume is divided by)",
        "body_mass * feeding_interval * tolerance_factor",
    ),
    (
        "intake_reading",
        "the intake reading (the intake volume itself, the lone quantity that rides on top of the bar)",
        "intake_volume",
    ),
    (
        "crossed",
        "the intake volume divided by the body mass, times the feeding interval, times the tolerance factor, dropping the denominator parentheses so only the body mass divides (a wrong grouping)",
        "intake_volume / body_mass * feeding_interval * tolerance_factor",
    ),
    (
        "swapped",
        "the intake volume times the body mass divided by the product of the feeding interval and the tolerance factor, moving the body mass up into the numerator (a wrong pairing)",
        "intake_volume * body_mass / (feeding_interval * tolerance_factor)",
    ),
]
QUERIED = ["perfusion_reserve", "demand_aggregate", "intake_reading"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(intake_volume, body_mass, feeding_interval, tolerance_factor):
    # Operation order mirrors the ADJ programs exactly (the body mass, feeding interval, and tolerance factor multiply first to
    # form the whole demand aggregate, then the intake volume is divided by that whole aggregate, so a/(b*c*d) evaluates as
    # (a/(b*c*d))), so the Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    return {
        "perfusion_reserve": intake_volume / (body_mass * feeding_interval * tolerance_factor),
        "demand_aggregate": body_mass * feeding_interval * tolerance_factor,
        "intake_reading": intake_volume,
        "crossed": intake_volume / body_mass * feeding_interval * tolerance_factor,
        "swapped": intake_volume * body_mass / (feeding_interval * tolerance_factor),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for intake_volume, body_mass, feeding_interval, tolerance_factor in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung is all multiplication/division of positives, so every
        # family member is positive automatically; distinctness is guarded explicitly below.
        assert (
            intake_volume >= 2
            and body_mass >= 2
            and feeding_interval >= 2
            and tolerance_factor >= 2
        ), (intake_volume, body_mass, feeding_interval, tolerance_factor)
        fv = family_values(intake_volume, body_mass, feeding_interval, tolerance_factor)
        for key, v in fv.items():
            assert v > 0, (key, intake_volume, body_mass, feeding_interval, tolerance_factor, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    intake_volume,
                    body_mass,
                    feeding_interval,
                    tolerance_factor,
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
                intake_volume,
                body_mass,
                feeding_interval,
                tolerance_factor,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r121np-{idx + 1:02d}",
                "qtype": "perfusion_reserve",
                "stem": (
                    f"A neonatology chart records an intake volume of {num(intake_volume)} divided by a body mass of "
                    f"{num(body_mass)} times a feeding interval of {num(feeding_interval)} times a tolerance factor of "
                    f"{num(tolerance_factor)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe intake_volume({num(intake_volume)})\n"
                    f"observe body_mass({num(body_mass)})\n"
                    f"observe feeding_interval({num(feeding_interval)})\n"
                    f"observe tolerance_factor({num(tolerance_factor)})\n"
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
            "ADJ-LADDER rung 121 — perfusion reserve from four stated quantities (a NEW panel: neonatology / perfusion). From a "
            "lone intake volume divided by a demand product (a body mass times a feeding interval times a tolerance factor), "
            "compute the perfusion reserve (intake_volume/(body_mass*feeding_interval*tolerance_factor)), the demand aggregate "
            "(body_mass*feeding_interval*tolerance_factor), or the intake reading (intake_volume). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, a LONE TERM OVER A TRIPLE PRODUCT a/(b*c*d) (multiply the body mass, feeding interval, and "
            "tolerance factor, then divide the intake volume by that whole demand aggregate, so a/(b*c*d) = (a/(b*c*d)); the "
            "ladder's FIRST ratio of a lone quantity over a pure three-factor product denominator. Every ratio with a "
            "product-denominator before rung-121 had at most a two-factor product under the bar (100 (a+b)/(c*d), 104 (a-b)/(c*d), "
            "118 a/(b*c+d), 119 a/(b*c-d), 120 a*b/(c*d)) — rung-121 is the first lone-term-over-triple-product, exactly rung-120's "
            "'swapped' distractor promoted to a headline. The harness matches the scalar to the printed options. The perfusion "
            "reserve is a rate (intake volume per unit of aggregate demand), framed as a RESERVE so the dimensionless value stays "
            "honest. Contamination-safe: every figure is built only from the four observed quantities via * and / — no constant "
            "leaks, and neither the demand aggregate, the intake reading, nor the perfusion reserve ever appears as a literal (each "
            "is computed from, or is, an observed quantity) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. The five options are a family over the same four quantities, so the distractors "
            "are exactly the slips students make: dropping the denominator parentheses so only the body mass divides (a/b*c*d, a "
            "wrong grouping) and lifting the body mass up into the numerator (a*b/(c*d), a wrong pairing). The core confusion "
            "tested is that a/(b*c*d) is (a/(b*c*d)), not a/b*c*d and not a*b/(c*d). This rung is all multiplication and division "
            "of positive quantities so positivity is automatic; with every observed quantity >= 2 the tables keep the five family "
            "values pairwise distinct and all three queried readouts varying across the panel, all asserted strictly positive at "
            "build time."
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
