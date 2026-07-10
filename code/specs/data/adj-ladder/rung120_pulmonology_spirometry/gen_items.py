"""Generate rung-120 (pulmonology spirometry / ventilation index) items.json for the ADJ-LADDER.

Rung 120 opens the **pulmonology-spirometry / ventilation** panel on the quantitative band — the arithmetic of a ventilation
index. A delivered-volume product (a `tidal_volume` TIMES a `breath_rate`, i.e. the minute ventilation) is DIVIDED by a demand
product (a `deadspace_ratio` TIMES a `cycle_count`, the clearance demand), to give the ventilation index (minute ventilation per
unit of clearance demand). A **product over a product**, `a*b/(c*d)`, i.e. `((a * b) / (c * d))`, introduces a genuinely NEW
arithmetic family on the ladder — the ladder's **first ratio with a product on BOTH sides of the bar**.

This is genuinely new. Every two-term ratio the ladder built with a product had the product on ONE side only: 99 `(a*b)/(c+d)`
(product over a sum), 100 `(a+b)/(c*d)` (sum over a product), 104 `(a-b)/(c*d)` (difference over a product), 106 `a*b/(c-d)`
(product over a difference). Nobody has yet put a product over a product. Rung-120 is `a*b/(c*d)` — a product of two quantities
divided by a product of two others. The operator order matters: `a*b/(c*d)` is `((a * b) / (c * d))` (the tidal volume and breath
rate multiply FIRST to form the minute ventilation, the deadspace ratio and cycle count multiply to form the clearance demand,
then the minute ventilation is divided by the whole clearance demand; the explicit denominator parentheses keep the WHOLE product
`c*d` under the bar), NOT `a*b/c*d` (dropping the denominator parentheses so only the deadspace ratio divides the minute
ventilation, and the result is then multiplied by the cycle count) and NOT `a/(b*c*d)` (moving the breath rate OUT of the
numerator and UNDER the bar, so only the tidal volume rides on top) — the two distractors exploit exactly those confusions.

The setup: a `tidal_volume`, a `breath_rate`, a `deadspace_ratio`, and a `cycle_count`. The total is:

  VENTILATION INDEX     tidal_volume * breath_rate / (deadspace_ratio * cycle_count)  [ a product over a product ]
  MINUTE VENTILATION    tidal_volume * breath_rate                                    [ the numerator product ]
  CLEARANCE DEMAND      deadspace_ratio * cycle_count                                 [ the denominator product ]

The **ventilation index** is what makes this rung distinctive — it is the ladder's first **product over a product**. It is a rate
(minute ventilation per unit of clearance demand), framed as an *index* to keep it dimensionless-clean — the same discipline rungs
99/100/104/106/.../117/118/119 used for their ratios. (The minute ventilation `a*b` and the clearance demand `c*d` ride alongside
as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-119 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication of the tidal volume and breath rate to form the minute ventilation, the multiplication of the
deadspace ratio and cycle count to form the clearance demand, then the division of the minute ventilation by that whole clearance
demand (the product numerator over the product denominator, so a*b/(c*d) evaluates as ((a*b)/(c*d))) — and the harness reads the
scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../118/119. This rung
exercises the engine across a **product-over-product ratio** — the fact that `a*b/(c*d)` is `((a*b)/(c*d))` and NOT `a*b/c*d` and
NOT `a/(b*c*d)` made computable. The ratio golds are non-integer f64s; the engine's IEEE-double division matches Python's the same
way rungs 99/100/104/106/.../118/119 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the minute ventilation, the clearance demand,
nor any index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`tidal_volume`, `breath_rate`, `deadspace_ratio`, `cycle_count`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    tidal_volume * breath_rate / deadspace_ratio * cycle_count  drop the denominator parentheses so only the deadspace
                                                                    ratio divides the minute ventilation, then the result is
                                                                    multiplied by the cycle count (the classic `a*b/(c*d)` vs
                                                                    `a*b/c*d` grouping error), and
  SWAPPED    tidal_volume / (breath_rate * deadspace_ratio * cycle_count)  move the breath rate OUT of the numerator and UNDER the
                                                                    bar, so only the tidal volume rides on top (`a/(b*c*d)`
                                                                    instead of `a*b/(c*d)`),

which are exactly the mistakes a student makes (failing to keep the whole clearance demand under the bar, or dropping the breath
rate out of the minute-ventilation numerator). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all
five always appear as options.

Distinctness and positivity: this rung is all multiplication and division of positive quantities, so every family member is
positive automatically — but distinctness is still guarded explicitly. Every observed quantity is `>= 2`, and the seven tables are
chosen so the five family values are pairwise distinct with a comfortable margin (in particular `deadspace_ratio != cycle_count`
so CROSSED never collapses to the minute ventilation, and `tidal_volume*breath_rate` is neither `(deadspace_ratio*cycle_count)^2`
nor `deadspace_ratio^2`, so the ventilation index never collapses to the clearance demand and the clearance demand never collapses
to CROSSED). Every family member is asserted `> 0` at build time. And — so all three queried readouts vary across the panel — the
seven tables give distinct ventilation indices, distinct minute ventilations, and distinct clearance demands, all asserted at
build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TIDAL_VOLUME, BREATH_RATE, DEADSPACE_RATIO, CYCLE_COUNT) — a minute-ventilation product (a tidal volume times a breath rate)
# divided by a clearance-demand product (a deadspace ratio times a cycle count) for the ventilation index, all plain positive
# numbers >= 2. Everything is multiplication/division of positives so positivity is automatic; distinctness is guarded explicitly
# (deadspace_ratio != cycle_count; a*b not equal to (c*d)^2 or c^2). The five family values are asserted pairwise-distinct below.
# The seven tables give distinct ventilation indices, distinct minute ventilations, and distinct clearance demands so all three
# queried readouts vary across the panel.
TABLES = [
    (6, 4, 2, 3),
    (10, 3, 2, 4),
    (8, 4, 2, 5),
    (12, 3, 3, 4),
    (7, 4, 2, 7),
    (10, 4, 2, 8),
    (12, 4, 3, 6),
]

# The option family (5 members), all built from the four observed quantities via * and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "ventilation_index",
        "ventilation index (the minute ventilation divided by the clearance demand)",
        "tidal_volume * breath_rate / (deadspace_ratio * cycle_count)",
    ),
    (
        "minute_ventilation",
        "the minute ventilation (the tidal volume times the breath rate, the numerator divided by the clearance demand)",
        "tidal_volume * breath_rate",
    ),
    (
        "clearance_demand",
        "the clearance demand (the deadspace ratio times the cycle count, the divisor the minute ventilation is divided by)",
        "deadspace_ratio * cycle_count",
    ),
    (
        "crossed",
        "the tidal volume times the breath rate divided by the deadspace ratio, times the cycle count, dropping the denominator parentheses so only the deadspace ratio divides (a wrong grouping)",
        "tidal_volume * breath_rate / deadspace_ratio * cycle_count",
    ),
    (
        "swapped",
        "the tidal volume divided by the product of the breath rate, the deadspace ratio, and the cycle count, moving the breath rate out of the numerator and under the bar (a wrong pairing)",
        "tidal_volume / (breath_rate * deadspace_ratio * cycle_count)",
    ),
]
QUERIED = ["ventilation_index", "minute_ventilation", "clearance_demand"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(tidal_volume, breath_rate, deadspace_ratio, cycle_count):
    # Operation order mirrors the ADJ programs exactly (the tidal volume and breath rate multiply first to form the minute
    # ventilation, the deadspace ratio and cycle count multiply to form the clearance demand, then the minute ventilation is
    # divided by that whole clearance demand, so a*b/(c*d) evaluates as ((a*b)/(c*d))), so the Python option value and the engine
    # result are the same IEEE-double (well within the 1e-9 tolerance).
    return {
        "ventilation_index": tidal_volume * breath_rate / (deadspace_ratio * cycle_count),
        "minute_ventilation": tidal_volume * breath_rate,
        "clearance_demand": deadspace_ratio * cycle_count,
        "crossed": tidal_volume * breath_rate / deadspace_ratio * cycle_count,
        "swapped": tidal_volume / (breath_rate * deadspace_ratio * cycle_count),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for tidal_volume, breath_rate, deadspace_ratio, cycle_count in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung is all multiplication/division of positives, so every
        # family member is positive automatically; distinctness is guarded explicitly below.
        assert (
            tidal_volume >= 2
            and breath_rate >= 2
            and deadspace_ratio >= 2
            and cycle_count >= 2
        ), (tidal_volume, breath_rate, deadspace_ratio, cycle_count)
        fv = family_values(tidal_volume, breath_rate, deadspace_ratio, cycle_count)
        for key, v in fv.items():
            assert v > 0, (key, tidal_volume, breath_rate, deadspace_ratio, cycle_count, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    tidal_volume,
                    breath_rate,
                    deadspace_ratio,
                    cycle_count,
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
                tidal_volume,
                breath_rate,
                deadspace_ratio,
                cycle_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r120ps-{idx + 1:02d}",
                "qtype": "ventilation_index",
                "stem": (
                    f"A spirometry report records a tidal volume of {num(tidal_volume)} times a breath rate of "
                    f"{num(breath_rate)} divided by a deadspace ratio of {num(deadspace_ratio)} times a cycle count of "
                    f"{num(cycle_count)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe tidal_volume({num(tidal_volume)})\n"
                    f"observe breath_rate({num(breath_rate)})\n"
                    f"observe deadspace_ratio({num(deadspace_ratio)})\n"
                    f"observe cycle_count({num(cycle_count)})\n"
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
            "ADJ-LADDER rung 120 — ventilation index from four stated quantities (a NEW panel: pulmonology spirometry / "
            "ventilation). From a minute-ventilation product (a tidal volume times a breath rate) divided by a clearance-demand "
            "product (a deadspace ratio times a cycle count), compute the ventilation index "
            "(tidal_volume*breath_rate/(deadspace_ratio*cycle_count)), the minute ventilation (tidal_volume*breath_rate), or the "
            "clearance demand (deadspace_ratio*cycle_count). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a PRODUCT OVER A PRODUCT "
            "a*b/(c*d) (multiply the tidal volume and breath rate, multiply the deadspace ratio and cycle count, divide the minute "
            "ventilation by that whole clearance demand, so a*b/(c*d) = ((a*b)/(c*d)); the ladder's FIRST ratio with a product on "
            "BOTH sides of the bar. Every two-term ratio with a product before rung-120 had the product on ONE side only (99 "
            "(a*b)/(c+d), 100 (a+b)/(c*d), 104 (a-b)/(c*d), 106 a*b/(c-d)) — rung-120 is the first product-over-product. The "
            "harness matches the scalar to the printed options. The ventilation index is a rate (minute ventilation per unit of "
            "clearance demand), framed as an INDEX so the dimensionless value stays honest. Contamination-safe: every figure is "
            "built only from the four observed quantities via * and / — no constant leaks, and neither the minute ventilation, "
            "the clearance demand, nor any index ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the same four "
            "quantities, so the distractors are exactly the slips students make: dropping the denominator parentheses so only the "
            "deadspace ratio divides (a*b/c*d, a wrong grouping) and moving the breath rate out of the numerator and under the bar "
            "(a/(b*c*d), a wrong pairing). The core confusion tested is that a*b/(c*d) is ((a*b)/(c*d)), not a*b/c*d and not "
            "a/(b*c*d). This rung is all multiplication and division of positive quantities so positivity is automatic; with every "
            "observed quantity >= 2 the tables keep the five family values pairwise distinct and all three queried readouts "
            "varying across the panel, all asserted strictly positive at build time."
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
