"""Generate rung-134 (cumulative-load span / a PRODUCT-numerator over a RATE — divide a total load by a quotient) items.json.

Rung 134 opens the **cumulative-load** panel and **completes the numerator trio over a rate**. rung-131 put a SUM over a rate,
`(a+b)/(c/d)`; rung-133 put a DIFFERENCE over a rate, `(a-b)/(c/d)`; rung-134 puts a PRODUCT over a rate, `(a*b)/(c/d)`. The combining
operation in the numerator walks +, then -, then *, each over the same divide-by-a-rate skeleton.

This is genuinely new. `(a*b)/(c/d)` is a PRODUCT `a*b` divided by a RATE `c/d`. Dividing by a rate is multiplying by its reciprocal, so
`(a*b)/(c/d) = (a*b) * (d/c) = (a*b*d)/c` — invert the rate and multiply. The product `a*b` binds and stays grouped over the bar
(grouping), and the rate `c/d` is one quotient that is inverted as a whole (not two separate divisors). The core confusions this rung
tests are the two canonical divide-by-a-rate slips: multiplying by the rate WITHOUT inverting it (`(a*b) * (c/d) = (a*b*c)/d`), and
treating `/(c/d)` as two separate divisions `/c/d` — dropping the reciprocal so the `d` stays in the denominator (`(a*b)/c/d =
(a*b)/(c*d)`).

The setup: a `unit_count` of items each of `unit_size` (a total load `unit_count * unit_size`), flushed at a rate formed from `flush_units`
per `flush_windows` (a rate `flush_units/flush_windows`). The figures are:

  LOAD SPAN     (unit_count * unit_size) / (flush_units / flush_windows)   [ product-numerator OVER a rate: total load / flush rate ]
  TOTAL LOAD    unit_count * unit_size                                     [ the product numerator (divided by the flush rate) ]
  FLUSH RATE    flush_units / flush_windows                               [ the rate the total load is divided by ]

The **load span** is the ladder's first **(a product) over (a quotient) as a headline** — a span (how much total load clears per unit of
the flush rate), framed as a *span* to keep it dimensionless-clean, the same discipline rungs 100/.../132/133 used for their ratios. (The
total load `a*b` and the flush rate `c/d` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as
rungs 47-133 shipped their component figures beside the headline. The two components anchor the "multiply out the load FIRST, form the
rate, then divide the load by the rate" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication to form the total load, the division to form the flush rate, then the division of the total load by the
flush rate to form the compound figure (so (a*b)/(c/d) evaluates as ((a*b)/(c/d)) = (a*b*d)/c) — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../132/133. This rung exercises the engine
across a **product divided by a quotient** — the fact that `(a*b)/(c/d)` is `(a*b*d)/c` and NOT `(a*b*c)/d` and NOT `(a*b)/(c*d)` made
computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../132/133 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*` and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the total load, the flush rate, nor the load span is ever a
literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`unit_count`,
`unit_size`, `flush_units`, `flush_windows`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  STRAIGHT   (unit_count * unit_size) * (flush_units / flush_windows)   multiply the total load BY the rate without inverting it (the
                                                                classic "divide by a fraction = multiply by it" error, evaluating
                                                                `(a*b*c)/d`), and
  FLAT       (unit_count * unit_size) / flush_units / flush_windows   divide the total load by the two rate parts separately, treating the
                                                                rate as two divisors instead of one quotient to invert (`(a*b)/c/d =
                                                                (a*b)/(c*d)`, dropping the reciprocal so the flush windows stay in the
                                                                denominator),

which are exactly the mistakes a student makes (multiplying instead of inverting-and-dividing, or splitting the rate into two divisions and
losing the reciprocal). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `*` and `/` over positive quantities, so **every figure is automatically positive** (no
subtraction anywhere) — like rungs 128/130/131/132, no positivity guards are needed. Every observed quantity is `>= 2`. Every family
member is asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give distinct load spans, distinct total loads, and
distinct flush rates so all three queried readouts vary across the panel; the five family values are pairwise distinct with a comfortable
margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (UNIT_COUNT, UNIT_SIZE, FLUSH_UNITS, FLUSH_WINDOWS) — a total load (unit_count * unit_size) divided by a flush rate
# (flush_units/flush_windows), giving the load span as a product over a quotient (a*b)/(c/d) = (a*b*d)/c. This rung uses only * and / over
# positive quantities, so every figure is automatically positive; no positivity guards are needed. The seven tables give distinct total
# loads (a*b), distinct flush rates (c/d), and distinct load spans ((a*b)/(c/d)); the five family values are asserted pairwise-distinct
# below.
TABLES = [
    (2, 3, 2, 4),     # load = 6,  rate = 0.5,  span = 12.0
    (2, 4, 6, 2),     # load = 8,  rate = 3.0,  span = 2.666...
    (2, 5, 3, 4),     # load = 10, rate = 0.75, span = 13.333...
    (3, 4, 8, 4),     # load = 12, rate = 2.0,  span = 6.0
    (2, 7, 8, 2),     # load = 14, rate = 4.0,  span = 3.5
    (3, 3, 5, 2),     # load = 9,  rate = 2.5,  span = 3.6
    (2, 8, 3, 2),     # load = 16, rate = 1.5,  span = 10.666...
]

# The option family (5 members), all built from the four observed quantities via * and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "load_span",
        "load span (the total load divided by the flush rate)",
        "(unit_count * unit_size) / (flush_units / flush_windows)",
    ),
    (
        "total_load",
        "the total load (the unit count times the unit size, the numerator that is divided by the flush rate)",
        "unit_count * unit_size",
    ),
    (
        "flush_rate",
        "the flush rate (the flush units spread over the flush windows, the rate the total load is divided by)",
        "flush_units / flush_windows",
    ),
    (
        "straight",
        "the total load times the flush rate, multiplying by the rate instead of inverting and dividing (a wrong operation)",
        "(unit_count * unit_size) * (flush_units / flush_windows)",
    ),
    (
        "flat",
        "the total load divided by the flush units and then the flush windows, splitting the rate into two divisors instead of inverting one quotient (a wrong operation)",
        "(unit_count * unit_size) / flush_units / flush_windows",
    ),
]
QUERIED = ["load_span", "total_load", "flush_rate"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(unit_count, unit_size, flush_units, flush_windows):
    # Operation order mirrors the ADJ programs exactly (the multiplication forms the total load, the division forms the flush rate, then
    # the total load is divided by the flush rate to form the compound figure, so (a*b)/(c/d) evaluates as ((a*b)/(c/d)) = (a*b*d)/c), so
    # the Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    load = unit_count * unit_size
    rate = flush_units / flush_windows
    return {
        "load_span": load / rate,
        "total_load": load,
        "flush_rate": rate,
        "straight": load * rate,
        "flat": load / flush_units / flush_windows,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for unit_count, unit_size, flush_units, flush_windows in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only * and / over positive quantities, so positivity is
        # automatic — no positivity guards are needed.
        assert (
            unit_count >= 2
            and unit_size >= 2
            and flush_units >= 2
            and flush_windows >= 2
        ), (unit_count, unit_size, flush_units, flush_windows)
        fv = family_values(unit_count, unit_size, flush_units, flush_windows)
        for key, v in fv.items():
            assert v > 0, (key, unit_count, unit_size, flush_units, flush_windows, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    unit_count,
                    unit_size,
                    flush_units,
                    flush_windows,
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
                unit_count,
                unit_size,
                flush_units,
                flush_windows,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r134lsa-{idx + 1:02d}",
                "qtype": "load_span",
                "stem": (
                    f"A cumulative-load study records a unit count of {num(unit_count)} items each of unit size "
                    f"{num(unit_size)}, flushed at a rate of {num(flush_units)} units per {num(flush_windows)} windows. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe unit_count({num(unit_count)})\n"
                    f"observe unit_size({num(unit_size)})\n"
                    f"observe flush_units({num(flush_units)})\n"
                    f"observe flush_windows({num(flush_windows)})\n"
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
            "ADJ-LADDER rung 134 — cumulative-load span from four stated quantities (a NEW panel: cumulative load, COMPLETING the "
            "numerator trio over a rate). rung-131 put a sum over a rate (a+b)/(c/d); rung-133 put a difference over a rate "
            "(a-b)/(c/d); rung-134 puts a PRODUCT over a rate (a*b)/(c/d) — the combining op walks +, -, * over the same divide-by-a-rate "
            "skeleton. From a total load (unit_count * unit_size) divided by a flush rate (flush_units/flush_windows), compute the load "
            "span ((unit_count*unit_size)/(flush_units/flush_windows)), the total load (unit_count*unit_size), or the flush rate "
            "(flush_units/flush_windows). Each item is a compute_dimensioned program (observe the four quantities, let answer = "
            "formula); the ADJ engine carries the arithmetic — a NEW family, a PRODUCT NUMERATOR OVER A RATE (a*b)/(c/d) (multiply out "
            "the load, form the rate, then divide the load by the rate, so (a*b)/(c/d) = (a*b*d)/c — dividing by a rate is multiplying by "
            "its reciprocal, invert the rate). The invert-and-multiply slips ride alongside as distractors. The harness matches the "
            "scalar to the printed options. The load span is a span (how much total load clears per unit of the flush rate), framed as a "
            "SPAN so the dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via * and / — no constant leaks, and neither the total load, the flush rate, nor the load span ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the slips students make: "
            "multiplying the total load by the rate without inverting it ((a*b)*(c/d) = (a*b*c)/d, a wrong operation) and splitting the "
            "rate into two divisions so the reciprocal is lost ((a*b)/c/d = (a*b)/(c*d), a wrong operation). The core confusion tested is "
            "that (a*b)/(c/d) is (a*b*d)/c, not (a*b*c)/d and not (a*b)/(c*d). This rung uses only * and / over positive quantities, so "
            "every figure is automatically positive — no positivity guards are needed — and the five family values are kept pairwise "
            "distinct with all three queried readouts varying across the panel, all asserted strictly positive at build time."
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
