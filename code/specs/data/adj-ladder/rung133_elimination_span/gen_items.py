"""Generate rung-133 (elimination span / a DIFFERENCE-numerator over a RATE — divide a net total by a quotient) items.json.

Rung 133 opens the **elimination** panel and is the **subtract twin of rung-131's sum-numerator over a rate**. rung-131 put a SUM over a
rate, `(a+b)/(c/d)` (a pooled total divided by a rate); rung-133 flips the numerator's `+` to `-`: `(a-b)/(c/d)` (a NET total divided by a
rate) — exactly as rung-129 (`(a-b/c)/d`) mirrored rung-128 (`(a+b/c)/d`), and rung-127 mirrored rung-126. The add->subtract pairing
repeats for the combined-numerator-over-a-rate shape.

This is genuinely new. `(a-b)/(c/d)` is a DIFFERENCE `a-b` divided by a RATE `c/d`. Dividing by a rate is multiplying by its reciprocal,
so `(a-b)/(c/d) = (a-b) * (d/c) = ((a-b)*d)/c` — invert the rate and multiply. The difference `a-b` binds and stays grouped over the bar
(grouping), and the rate `c/d` is one quotient that is inverted as a whole (not two separate divisors). The core confusions this rung
tests are the two canonical divide-by-a-rate slips: multiplying by the rate WITHOUT inverting it (`(a-b) * (c/d) = ((a-b)*c)/d`), and
treating `/(c/d)` as two separate divisions `/c/d` — dropping the reciprocal so the `d` stays in the denominator (`(a-b)/c/d =
(a-b)/(c*d)`).

The setup: a `gross_input` with a `lost_fraction` removed (a net input `gross_input - lost_fraction`), cleared at an elimination rate
formed from `clearance_units` per `clearance_windows` (a rate `clearance_units/clearance_windows`). The figures are:

  ELIMINATION SPAN   (gross_input - lost_fraction) / (clearance_units / clearance_windows)   [ difference-numerator OVER a rate ]
  NET INPUT          gross_input - lost_fraction                                             [ the differenced numerator ]
  CLEARANCE RATE     clearance_units / clearance_windows                                     [ the rate the net input is divided by ]

The **elimination span** is the ladder's first **(a difference) over (a quotient) as a headline** — a span (how much net input clears per
unit of the clearance rate), framed as a *span* to keep it dimensionless-clean, the same discipline rungs 100/.../131/132 used for their
ratios. (The net input `a-b` and the clearance rate `c/d` ride alongside as component readouts, so the panel teaches the whole calculation
— exactly as rungs 47-132 shipped their component figures beside the headline. The two components anchor the "net the input FIRST, form
the rate, then divide the net by the rate" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the subtraction to form the net input, the division to form the clearance rate, then the division of the net input by the
clearance rate to form the compound figure (so (a-b)/(c/d) evaluates as ((a-b)/(c/d)) = ((a-b)*d)/c) — and the harness reads the scalar via
the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../131/132. This rung exercises the engine
across a **difference divided by a quotient** — the fact that `(a-b)/(c/d)` is `((a-b)*d)/c` and NOT `((a-b)*c)/d` and NOT `(a-b)/(c*d)`
made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../131/132 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `/` and `*` — **no structural
constants** — so no numeric literal appears in any program, and neither the net input, the clearance rate, nor the elimination span is
ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`gross_input`,
`lost_fraction`, `clearance_units`, `clearance_windows`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  STRAIGHT   (gross_input - lost_fraction) * (clearance_units / clearance_windows)   multiply the net input BY the rate without inverting
                                                                it (the classic "divide by a fraction = multiply by it" error, evaluating
                                                                `((a-b)*c)/d`), and
  FLAT       (gross_input - lost_fraction) / clearance_units / clearance_windows   divide the net input by the two rate parts separately,
                                                                treating the rate as two divisors instead of one quotient to invert
                                                                (`(a-b)/c/d = (a-b)/(c*d)`, dropping the reciprocal so the clearance
                                                                windows stay in the denominator),

which are exactly the mistakes a student makes (multiplying instead of inverting-and-dividing, or splitting the rate into two divisions and
losing the reciprocal). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung SUBTRACTS in the numerator, so positivity is NOT automatic — it is guarded explicitly per table.
Every observed quantity is `>= 2`, and each table guarantees **gross_input - lost_fraction >= 2** (so the net input `a-b` is comfortably
positive, and since the rate `c/d > 0` the elimination span, straight, and flat are all positive too). Every family member is asserted
`> 0` at build time. The seven tables give distinct elimination spans, distinct net inputs, and distinct clearance rates so all three
queried readouts vary across the panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (GROSS_INPUT, LOST_FRACTION, CLEARANCE_UNITS, CLEARANCE_WINDOWS) — a net input (gross_input - lost_fraction) divided by a clearance rate
# (clearance_units/clearance_windows), giving the elimination span as a difference over a quotient (a-b)/(c/d) = ((a-b)*d)/c. This rung
# SUBTRACTS in the numerator, so positivity is NOT automatic; each table guarantees gross_input - lost_fraction >= 2 (net input
# comfortably positive, so the span, straight, and flat are all positive). The seven tables give distinct net inputs (a-b), distinct
# clearance rates (c/d), and distinct elimination spans ((a-b)/(c/d)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (8, 2, 2, 4),     # net = 6,  rate = 0.5,  span = 12.0
    (11, 3, 6, 2),    # net = 8,  rate = 3.0,  span = 2.666...
    (14, 4, 3, 4),    # net = 10, rate = 0.75, span = 13.333...
    (16, 4, 8, 4),    # net = 12, rate = 2.0,  span = 6.0
    (19, 5, 8, 2),    # net = 14, rate = 4.0,  span = 3.5
    (12, 3, 5, 2),    # net = 9,  rate = 2.5,  span = 3.6
    (22, 6, 3, 2),    # net = 16, rate = 1.5,  span = 10.666...
]

# The option family (5 members), all built from the four observed quantities via -, / and *. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "elimination_span",
        "elimination span (the net input divided by the clearance rate)",
        "(gross_input - lost_fraction) / (clearance_units / clearance_windows)",
    ),
    (
        "net_input",
        "the net input (the gross input minus the lost fraction, the numerator that is divided by the clearance rate)",
        "gross_input - lost_fraction",
    ),
    (
        "clearance_rate",
        "the clearance rate (the clearance units spread over the clearance windows, the rate the net input is divided by)",
        "clearance_units / clearance_windows",
    ),
    (
        "straight",
        "the net input times the clearance rate, multiplying by the rate instead of inverting and dividing (a wrong operation)",
        "(gross_input - lost_fraction) * (clearance_units / clearance_windows)",
    ),
    (
        "flat",
        "the net input divided by the clearance units and then the clearance windows, splitting the rate into two divisors instead of inverting one quotient (a wrong operation)",
        "(gross_input - lost_fraction) / clearance_units / clearance_windows",
    ),
]
QUERIED = ["elimination_span", "net_input", "clearance_rate"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(gross_input, lost_fraction, clearance_units, clearance_windows):
    # Operation order mirrors the ADJ programs exactly (the subtraction forms the net input, the division forms the clearance rate, then
    # the net input is divided by the clearance rate to form the compound figure, so (a-b)/(c/d) evaluates as ((a-b)/(c/d)) =
    # ((a-b)*d)/c), so the Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    net = gross_input - lost_fraction
    rate = clearance_units / clearance_windows
    return {
        "elimination_span": net / rate,
        "net_input": net,
        "clearance_rate": rate,
        "straight": net * rate,
        "flat": net / clearance_units / clearance_windows,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for gross_input, lost_fraction, clearance_units, clearance_windows in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung SUBTRACTS in the numerator, so positivity is NOT automatic;
        # it is guarded explicitly per table.
        assert (
            gross_input >= 2
            and lost_fraction >= 2
            and clearance_units >= 2
            and clearance_windows >= 2
        ), (gross_input, lost_fraction, clearance_units, clearance_windows)
        assert gross_input - lost_fraction >= 2, (
            "gross_input - lost_fraction must be >= 2 (net input, span, straight, flat all positive)",
            gross_input,
            lost_fraction,
            clearance_units,
            clearance_windows,
        )
        fv = family_values(gross_input, lost_fraction, clearance_units, clearance_windows)
        for key, v in fv.items():
            assert v > 0, (key, gross_input, lost_fraction, clearance_units, clearance_windows, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    gross_input,
                    lost_fraction,
                    clearance_units,
                    clearance_windows,
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
                gross_input,
                lost_fraction,
                clearance_units,
                clearance_windows,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r133esa-{idx + 1:02d}",
                "qtype": "elimination_span",
                "stem": (
                    f"An elimination study records a gross input of {num(gross_input)} with a lost fraction of "
                    f"{num(lost_fraction)} removed, cleared at a rate of {num(clearance_units)} units per "
                    f"{num(clearance_windows)} windows. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe gross_input({num(gross_input)})\n"
                    f"observe lost_fraction({num(lost_fraction)})\n"
                    f"observe clearance_units({num(clearance_units)})\n"
                    f"observe clearance_windows({num(clearance_windows)})\n"
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
            "ADJ-LADDER rung 133 — elimination span from four stated quantities (a NEW panel: elimination, and the SUBTRACT twin of "
            "rung-131's sum-numerator over a rate). rung-131 put a sum over a rate (a+b)/(c/d); rung-133 flips the numerator's + to -: "
            "(a-b)/(c/d). From a net input (gross_input - lost_fraction) divided by a clearance rate "
            "(clearance_units/clearance_windows), compute the elimination span "
            "((gross_input-lost_fraction)/(clearance_units/clearance_windows)), the net input (gross_input-lost_fraction), or the "
            "clearance rate (clearance_units/clearance_windows). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a DIFFERENCE NUMERATOR OVER A RATE "
            "(a-b)/(c/d) (net the input, form the rate, then divide the net by the rate, so (a-b)/(c/d) = ((a-b)*d)/c — dividing by a "
            "rate is multiplying by its reciprocal, invert the rate; the minus twin of rung-131's (a+b)/(c/d), mirroring how rung-129's "
            "(a-b/c)/d mirrors rung-128's (a+b/c)/d). The invert-and-multiply slips ride alongside as distractors. The harness matches "
            "the scalar to the printed options. The elimination span is a span (how much net input clears per unit of the clearance "
            "rate), framed as a SPAN so the dimensionless value stays honest. Contamination-safe: every figure is built only from the "
            "four observed quantities via -, / and * — no constant leaks, and neither the net input, the clearance rate, nor the "
            "elimination span ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so "
            "no numeral hides inside a variable name. The five options are a family over the same four quantities, so the distractors "
            "are exactly the slips students make: multiplying the net input by the rate without inverting it ((a-b)*(c/d) = ((a-b)*c)/d, "
            "a wrong operation) and splitting the rate into two divisions so the reciprocal is lost ((a-b)/c/d = (a-b)/(c*d), a wrong "
            "operation). The core confusion tested is that (a-b)/(c/d) is ((a-b)*d)/c, not ((a-b)*c)/d and not (a-b)/(c*d). This rung "
            "SUBTRACTS in the numerator so positivity is NOT automatic; each table guards gross_input - lost_fraction >= 2 (keeping the "
            "net input, span, straight, and flat all positive), keeping the five family values pairwise distinct with all three queried "
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
