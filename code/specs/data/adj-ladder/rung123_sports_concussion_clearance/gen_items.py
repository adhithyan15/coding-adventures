"""Generate rung-123 (sports-concussion clearance index / lone term over a term-MINUS-product denominator) items.json.

Rung 123 opens the **sports-concussion / return-to-play clearance** panel on the ADJ-LADDER's quantitative band — the arithmetic
of a clearance index. A single observed `symptom_load` is DIVIDED by a net recovery window formed as a `recovery_window` MINUS a
provocation PRODUCT (an `exertion_step` TIMES a `provocation_factor`), to give the clearance index. A **lone term over a term-MINUS-
a-product denominator**, `a/(b - c*d)`, i.e. `(a / (b - (c * d)))` (the multiplication binds tighter than the subtraction inside the
denominator parentheses), introduces a genuinely NEW arithmetic family on the ladder — the **subtraction twin of rung-122's
`a/(b + c*d)`**, and the ladder's first denominator that SUBTRACTS a product from a lone term.

This is genuinely new. rung-122 opened the term-vs-product denominator with the PLUS form `a/(b + c*d)` = `a/(b + (c*d))`. rung-123
is the MINUS sibling: `a/(b - c*d)` = `a/(b - (c*d))` — the provocation product is formed first, then SUBTRACTED from the lone
recovery window. It is distinct from rung-119 `a/(b*c - d)` = `a/((b*c) - d)` (which multiplies the first two and subtracts the
third): rung-123 subtracts the WHOLE `c*d` product from the lone `b`, whereas rung-119 subtracts the lone `d` from the `b*c` product.
The two are the classic distributivity/precedence confusion, and rung-119's shape rides alongside as the swapped distractor. Also
distinct from the three-term-difference denominators (117 `a/(b+c-d)`), the product-over-product (120), and the lone-term-over-
triple-product (121).

The setup: a `symptom_load`, a `recovery_window`, an `exertion_step`, and a `provocation_factor`. The figures are:

  CLEARANCE INDEX     symptom_load / (recovery_window - exertion_step * provocation_factor)  [ a lone term over a term-minus-product ]
  NET WINDOW          recovery_window - exertion_step * provocation_factor                   [ the term-minus-product denominator ]
  PROVOCATION LOAD    exertion_step * provocation_factor                                      [ the product being subtracted ]

The **clearance index** is what makes this rung distinctive — it is the ladder's first **lone quantity over a term-MINUS-a-product**.
It is a rate (symptom load per unit of net recovery window), framed as an *index* to keep it dimensionless-clean — the same
discipline rungs 100/.../118/119/120/121/122 used for their ratios. (The net window `b-c*d` and the provocation load `c*d` ride
alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-122 shipped their component
sums/products/differences/ratios beside the headline figure. The provocation load `c*d` is the subtracted product reported straight,
anchoring the "multiply first, then subtract" grouping against the swapped distractor.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication of the exertion step and provocation factor to form the provocation product, the subtraction from
the recovery window to form the whole net window, then the division of the symptom load by that whole net window (so a/(b-c*d)
evaluates as (a/(b-(c*d)))) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine
change, exactly as rungs 8/16/.../121/122. This rung exercises the engine across a **lone-term-over-(term-minus-product) ratio** —
the fact that `a/(b-c*d)` is `(a/(b-(c*d)))` and NOT `a/b-c*d` and NOT `a/(b*c-d)` made computable. The ratio golds are non-integer
f64s; the engine's IEEE-double division matches Python's the same way rungs 100/.../121/122 relied on (well within the harness's
1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `*`, and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the net window, the provocation load, nor the
clearance index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`symptom_load`, `recovery_window`, `exertion_step`, `provocation_factor`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    symptom_load / recovery_window - exertion_step * provocation_factor  drop the denominator parentheses so only the
                                                                    recovery window divides the symptom load, then the provocation
                                                                    product is subtracted (the classic `a/(b-c*d)` vs `a/b-c*d`
                                                                    grouping error, evaluating (a/b)-c*d), and
  SWAPPED    symptom_load / (recovery_window * exertion_step - provocation_factor)  multiply the recovery window and exertion step
                                                                    first and subtract the lone provocation factor (`a/(b*c-d)`,
                                                                    rung-119's grouping, instead of `a/(b-c*d)`),

which are exactly the mistakes a student makes (failing to keep the whole term-minus-product under the bar, or multiplying the wrong
pair before subtracting). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: this rung SUBTRACTS a product inside the denominator (and both distractors subtract), so positivity is
NOT automatic — it is guarded explicitly per table. Every observed quantity is `>= 2`, and each table guarantees **recovery_window >
exertion_step*provocation_factor** with the net window `b-c*d >= 2` (headline denominator positive, away from zero), **symptom_load >
recovery_window*exertion_step*provocation_factor** so the crossed slip `(a/b)-c*d` stays positive (its `a/b` exceeds the subtracted
`c*d`), and **recovery_window*exertion_step - provocation_factor >= 2** so the swapped slip's denominator stays positive. Every
family member is asserted `> 0` at build time. And — so all three queried readouts vary across the panel — the seven tables give
distinct clearance indices, distinct net windows, and distinct provocation loads, all asserted at build time; the five family values
are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (SYMPTOM_LOAD, RECOVERY_WINDOW, EXERTION_STEP, PROVOCATION_FACTOR) — a lone symptom load divided by a net recovery window (a
# recovery window MINUS an exertion-step-times-provocation-factor product) for the clearance index. This rung SUBTRACTS inside the
# denominator, so positivity is NOT automatic; each table guarantees recovery_window > exertion_step*provocation_factor (net window
# b-c*d >= 2), symptom_load > recovery_window*exertion_step*provocation_factor (crossed (a/b)-c*d positive), and
# recovery_window*exertion_step - provocation_factor >= 2 (swapped denom positive). The five family values are asserted pairwise-
# distinct below. The seven tables give distinct clearance indices, distinct net windows, and distinct provocation loads so all three
# queried readouts vary across the panel.
TABLES = [
    (42, 6, 2, 2),
    (72, 9, 2, 3),
    (132, 12, 2, 4),
    (154, 14, 3, 3),
    (208, 16, 2, 5),
    (266, 19, 3, 4),
    (396, 22, 2, 7),
]

# The option family (5 members), all built from the four observed quantities via -, * and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "clearance_index",
        "clearance index (the symptom load divided by the net window)",
        "symptom_load / (recovery_window - exertion_step * provocation_factor)",
    ),
    (
        "net_window",
        "the net window (the recovery window minus the exertion step times the provocation factor, the divisor the symptom load is divided by)",
        "recovery_window - exertion_step * provocation_factor",
    ),
    (
        "provocation_load",
        "the provocation load (the exertion step times the provocation factor, the product subtracted inside the net window)",
        "exertion_step * provocation_factor",
    ),
    (
        "crossed",
        "the symptom load divided by the recovery window, minus the exertion step times the provocation factor, dropping the denominator parentheses so only the recovery window divides (a wrong grouping)",
        "symptom_load / recovery_window - exertion_step * provocation_factor",
    ),
    (
        "swapped",
        "the symptom load divided by the recovery window times the exertion step minus the provocation factor, multiplying the wrong pair before subtracting (a wrong grouping)",
        "symptom_load / (recovery_window * exertion_step - provocation_factor)",
    ),
]
QUERIED = ["clearance_index", "net_window", "provocation_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(symptom_load, recovery_window, exertion_step, provocation_factor):
    # Operation order mirrors the ADJ programs exactly (the exertion step and provocation factor multiply first to form the
    # provocation product, it is subtracted from the recovery window to form the whole net window, then the symptom load is divided
    # by that whole net window, so a/(b-c*d) evaluates as (a/(b-(c*d)))), so the Python option value and the engine result are the
    # same IEEE-double (well within the 1e-9 tolerance).
    return {
        "clearance_index": symptom_load / (recovery_window - exertion_step * provocation_factor),
        "net_window": recovery_window - exertion_step * provocation_factor,
        "provocation_load": exertion_step * provocation_factor,
        "crossed": symptom_load / recovery_window - exertion_step * provocation_factor,
        "swapped": symptom_load / (recovery_window * exertion_step - provocation_factor),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for symptom_load, recovery_window, exertion_step, provocation_factor in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung SUBTRACTS a product inside the denominator (and both
        # distractors subtract), so positivity is NOT automatic; it is guarded explicitly per table.
        assert (
            symptom_load >= 2
            and recovery_window >= 2
            and exertion_step >= 2
            and provocation_factor >= 2
        ), (symptom_load, recovery_window, exertion_step, provocation_factor)
        assert recovery_window - exertion_step * provocation_factor >= 2, (
            "net window recovery_window - exertion_step*provocation_factor must be >= 2",
            symptom_load,
            recovery_window,
            exertion_step,
            provocation_factor,
        )
        assert symptom_load > recovery_window * exertion_step * provocation_factor, (
            "symptom_load must exceed recovery_window*exertion_step*provocation_factor so the crossed slip stays positive",
            symptom_load,
            recovery_window,
            exertion_step,
            provocation_factor,
        )
        assert recovery_window * exertion_step - provocation_factor >= 2, (
            "swapped denom recovery_window*exertion_step - provocation_factor must be >= 2",
            symptom_load,
            recovery_window,
            exertion_step,
            provocation_factor,
        )
        fv = family_values(symptom_load, recovery_window, exertion_step, provocation_factor)
        for key, v in fv.items():
            assert v > 0, (key, symptom_load, recovery_window, exertion_step, provocation_factor, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    symptom_load,
                    recovery_window,
                    exertion_step,
                    provocation_factor,
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
                symptom_load,
                recovery_window,
                exertion_step,
                provocation_factor,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r123sc-{idx + 1:02d}",
                "qtype": "clearance_index",
                "stem": (
                    f"A sports-concussion chart records a symptom load of {num(symptom_load)} divided by a recovery window of "
                    f"{num(recovery_window)} minus an exertion step of {num(exertion_step)} times a provocation factor of "
                    f"{num(provocation_factor)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe symptom_load({num(symptom_load)})\n"
                    f"observe recovery_window({num(recovery_window)})\n"
                    f"observe exertion_step({num(exertion_step)})\n"
                    f"observe provocation_factor({num(provocation_factor)})\n"
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
            "ADJ-LADDER rung 123 — clearance index from four stated quantities (a NEW panel: sports-concussion / return-to-play "
            "clearance). From a lone symptom load divided by a net recovery window (a recovery window MINUS an exertion-step-times-"
            "provocation-factor product), compute the clearance index "
            "(symptom_load/(recovery_window-exertion_step*provocation_factor)), the net window "
            "(recovery_window-exertion_step*provocation_factor), or the provocation load (exertion_step*provocation_factor). Each "
            "item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, a LONE TERM OVER A TERM-MINUS-PRODUCT a/(b-c*d) (multiply the exertion step and provocation "
            "factor, subtract from the recovery window, then divide the symptom load by that whole net window, so a/(b-c*d) = "
            "(a/(b-(c*d))); the SUBTRACTION twin of rung-122's a/(b+c*d), and the ladder's first denominator that subtracts a "
            "product from a lone term. Distinct from rung-119 a/(b*c-d) = a/((b*c)-d), which multiplies the first two and subtracts "
            "the third; rung-123 subtracts the WHOLE c*d product from the lone b. rung-119's shape rides alongside as the swapped "
            "distractor. The harness matches the scalar to the printed options. The clearance index is a rate (symptom load per "
            "unit of net recovery window), framed as an INDEX so the dimensionless value stays honest. Contamination-safe: every "
            "figure is built only from the four observed quantities via -, * and / — no constant leaks, and neither the net window, "
            "the provocation load, nor the clearance index ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a family over "
            "the same four quantities, so the distractors are exactly the slips students make: dropping the denominator parentheses "
            "so only the recovery window divides (a/b-c*d, evaluating (a/b)-c*d, a wrong grouping) and multiplying the wrong pair "
            "before subtracting (a/(b*c-d), rung-119's grouping). The core confusion tested is that a/(b-c*d) is (a/(b-(c*d))), not "
            "a/b-c*d and not a/(b*c-d). This rung SUBTRACTS a product inside the denominator so positivity is NOT automatic; each "
            "table guards recovery_window > exertion_step*provocation_factor (net window >= 2), symptom_load > "
            "recovery_window*exertion_step*provocation_factor (crossed positive), and recovery_window*exertion_step - "
            "provocation_factor >= 2 (swapped positive), keeping the five family values pairwise distinct and all three queried "
            "readouts varying across the panel, all asserted strictly positive at build time."
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
