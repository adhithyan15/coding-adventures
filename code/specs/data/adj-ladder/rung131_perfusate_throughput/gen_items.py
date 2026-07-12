"""Generate rung-131 (perfusate-throughput span / a SUM-NUMERATOR over a RATE — divide a pooled total by a quotient) items.json.

Rung 131 opens the **perfusate-throughput** panel and **composes two ideas the ladder already built**: the SUM numerator of rungs 128/129
(`a + b`) and the divide-by-a-quotient of rung-130's complex fraction. rung-130 introduced the ratio of two ratios `(a/b)/(c/d)` (a quotient
over a quotient); rung-131 keeps the "divide by a quotient" but replaces the numerator quotient with a plain SUM: `(a + b)/(c/d)` — a
**pooled total divided by a rate**.

This is genuinely new. `(a + b)/(c/d)` is a SUM `a + b` divided by a RATE `c/d`. Dividing by a rate is multiplying by its reciprocal, so
`(a + b)/(c/d) = (a + b) * (d/c) = ((a+b)*d)/c` — invert the rate and multiply. The sum `a + b` binds and stays grouped over the bar
(grouping), and the rate `c/d` is one quotient that is inverted as a whole (not two separate divisors). The core confusions this rung tests
are the two canonical divide-by-a-rate slips: multiplying by the rate WITHOUT inverting it (`(a+b) * (c/d) = ((a+b)*c)/d`), and treating
`/(c/d)` as two separate divisions `/c/d` — dropping the reciprocal so the `d` stays in the denominator (`(a+b)/c/d = (a+b)/(c*d)`).

The setup: a `base_charge` and a `boost_charge` pooled together (a pooled charge `base_charge + boost_charge`), pushed through at a span
rate formed from `span_units` per `span_windows` (a rate `span_units/span_windows`). The figures are:

  THROUGHPUT SPAN   (base_charge + boost_charge) / (span_units / span_windows)   [ sum-numerator OVER a rate: pooled charge / rate ]
  POOLED CHARGE     base_charge + boost_charge                                   [ the summed numerator (the pooled total) ]
  SPAN RATE         span_units / span_windows                                    [ the rate the pooled charge is divided by ]

The **throughput span** is the ladder's first **(a sum) over (a quotient) as a headline** — a span (how much pooled charge fits per unit
of the span rate), framed as a *span* to keep it dimensionless-clean, the same discipline rungs 100/.../129/130 used for their ratios.
(The pooled charge `a + b` and the span rate `c/d` ride alongside as component readouts, so the panel teaches the whole calculation —
exactly as rungs 47-130 shipped their component figures beside the headline. The two components anchor the "pool the charge FIRST, form
the rate, then divide the pool by the rate" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the sum to form the pooled charge, the division to form the span rate, then the division of the pooled charge by the span
rate to form the compound figure (so (a+b)/(c/d) evaluates as ((a+b)/(c/d)) = ((a+b)*d)/c) — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../129/130. This rung exercises the engine
across a **sum divided by a quotient** — the fact that `(a+b)/(c/d)` is `((a+b)*d)/c` and NOT `((a+b)*c)/d` and NOT `(a+b)/(c*d)` made
computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../129/130 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `/` and `*` — **no structural
constants** — so no numeric literal appears in any program, and neither the pooled charge, the span rate, nor the throughput span is ever
a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`base_charge`,
`boost_charge`, `span_units`, `span_windows`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  STRAIGHT   (base_charge + boost_charge) * (span_units / span_windows)   multiply the pooled charge BY the rate without inverting it (the
                                                                classic "divide by a fraction = multiply by it" error, evaluating
                                                                `((a+b)*c)/d`), and
  FLAT       (base_charge + boost_charge) / span_units / span_windows   divide the pooled charge by the two rate parts separately, treating
                                                                the rate as two divisors instead of one quotient to invert (`(a+b)/c/d =
                                                                (a+b)/(c*d)`, dropping the reciprocal so the span windows stay in the
                                                                denominator),

which are exactly the mistakes a student makes (multiplying instead of inverting-and-dividing, or splitting the rate into two divisions and
losing the reciprocal). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `+`, `/` and `*` over positive quantities, so **every figure is automatically positive**
(no subtraction anywhere) — like rungs 128/130, no positivity guards are needed. Every observed quantity is `>= 2`. Every family member is
asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give distinct throughput spans, distinct pooled charges, and
distinct span rates so all three queried readouts vary across the panel; the five family values are pairwise distinct with a comfortable
margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BASE_CHARGE, BOOST_CHARGE, SPAN_UNITS, SPAN_WINDOWS) — a pooled charge (base_charge + boost_charge) divided by a span rate
# (span_units/span_windows), giving the throughput span as a sum over a quotient (a+b)/(c/d) = ((a+b)*d)/c. This rung uses only +, / and *
# over positive quantities, so every figure is automatically positive; no positivity guards are needed. The seven tables give distinct
# pooled charges (a+b), distinct span rates (c/d), and distinct throughput spans ((a+b)/(c/d)); the five family values are asserted
# pairwise-distinct below.
TABLES = [
    (4, 2, 2, 4),     # pooled = 6,  rate = 0.5,  span = 12.0
    (5, 3, 6, 2),     # pooled = 8,  rate = 3.0,  span = 2.666...
    (6, 4, 3, 4),     # pooled = 10, rate = 0.75, span = 13.333...
    (8, 4, 8, 4),     # pooled = 12, rate = 2.0,  span = 6.0
    (9, 5, 8, 2),     # pooled = 14, rate = 4.0,  span = 3.5
    (7, 2, 5, 2),     # pooled = 9,  rate = 2.5,  span = 3.6
    (10, 6, 3, 2),    # pooled = 16, rate = 1.5,  span = 10.666...
]

# The option family (5 members), all built from the four observed quantities via +, / and *. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "throughput_span",
        "throughput span (the pooled charge divided by the span rate)",
        "(base_charge + boost_charge) / (span_units / span_windows)",
    ),
    (
        "pooled_charge",
        "the pooled charge (the base charge plus the boost charge, the numerator that is divided by the span rate)",
        "base_charge + boost_charge",
    ),
    (
        "span_rate",
        "the span rate (the span units spread over the span windows, the rate the pooled charge is divided by)",
        "span_units / span_windows",
    ),
    (
        "straight",
        "the pooled charge times the span rate, multiplying by the rate instead of inverting and dividing (a wrong operation)",
        "(base_charge + boost_charge) * (span_units / span_windows)",
    ),
    (
        "flat",
        "the pooled charge divided by the span units and then the span windows, splitting the rate into two divisors instead of inverting one quotient (a wrong operation)",
        "(base_charge + boost_charge) / span_units / span_windows",
    ),
]
QUERIED = ["throughput_span", "pooled_charge", "span_rate"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(base_charge, boost_charge, span_units, span_windows):
    # Operation order mirrors the ADJ programs exactly (the sum forms the pooled charge, the division forms the span rate, then the pooled
    # charge is divided by the span rate to form the compound figure, so (a+b)/(c/d) evaluates as ((a+b)/(c/d)) = ((a+b)*d)/c), so the
    # Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    return {
        "throughput_span": (base_charge + boost_charge) / (span_units / span_windows),
        "pooled_charge": base_charge + boost_charge,
        "span_rate": span_units / span_windows,
        "straight": (base_charge + boost_charge) * (span_units / span_windows),
        "flat": (base_charge + boost_charge) / span_units / span_windows,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for base_charge, boost_charge, span_units, span_windows in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only +, / and * over positive quantities, so positivity
        # is automatic — no positivity guards are needed.
        assert (
            base_charge >= 2
            and boost_charge >= 2
            and span_units >= 2
            and span_windows >= 2
        ), (base_charge, boost_charge, span_units, span_windows)
        fv = family_values(base_charge, boost_charge, span_units, span_windows)
        for key, v in fv.items():
            assert v > 0, (key, base_charge, boost_charge, span_units, span_windows, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    base_charge,
                    boost_charge,
                    span_units,
                    span_windows,
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
                base_charge,
                boost_charge,
                span_units,
                span_windows,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r131tsa-{idx + 1:02d}",
                "qtype": "throughput_span",
                "stem": (
                    f"A perfusate-throughput study records a base charge of {num(base_charge)} plus a boost charge of "
                    f"{num(boost_charge)}, pushed through at a span rate of {num(span_units)} units per {num(span_windows)} windows. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe base_charge({num(base_charge)})\n"
                    f"observe boost_charge({num(boost_charge)})\n"
                    f"observe span_units({num(span_units)})\n"
                    f"observe span_windows({num(span_windows)})\n"
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
            "ADJ-LADDER rung 131 — perfusate-throughput span from four stated quantities (a NEW panel: perfusate throughput, COMPOSING "
            "the sum numerator of rungs 128/129 with the divide-by-a-quotient of rung-130). From a pooled charge (base_charge + "
            "boost_charge) divided by a span rate (span_units/span_windows), compute the throughput span "
            "((base_charge+boost_charge)/(span_units/span_windows)), the pooled charge (base_charge+boost_charge), or the span rate "
            "(span_units/span_windows). Each item is a compute_dimensioned program (observe the four quantities, let answer = formula); "
            "the ADJ engine carries the arithmetic — a NEW family, a SUM NUMERATOR OVER A RATE (a+b)/(c/d) (pool the charge, form the "
            "rate, then divide the pool by the rate, so (a+b)/(c/d) = ((a+b)*d)/c — dividing by a rate is multiplying by its reciprocal, "
            "invert the rate; keeps rung-130's divide-by-a-quotient but swaps the numerator quotient for a plain sum). The "
            "invert-and-multiply slips ride alongside as distractors. The harness matches the scalar to the printed options. The "
            "throughput span is a span (how much pooled charge fits per unit of the span rate), framed as a SPAN so the dimensionless "
            "value stays honest. Contamination-safe: every figure is built only from the four observed quantities via +, / and * — no "
            "constant leaks, and neither the pooled charge, the span rate, nor the throughput span ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same four quantities, so the distractors are exactly the slips students make: multiplying the "
            "pooled charge by the rate without inverting it ((a+b)*(c/d) = ((a+b)*c)/d, a wrong operation) and splitting the rate into "
            "two divisions so the reciprocal is lost ((a+b)/c/d = (a+b)/(c*d), a wrong operation). The core confusion tested is that "
            "(a+b)/(c/d) is ((a+b)*d)/c, not ((a+b)*c)/d and not (a+b)/(c*d). This rung uses only +, / and * over positive quantities, so "
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
