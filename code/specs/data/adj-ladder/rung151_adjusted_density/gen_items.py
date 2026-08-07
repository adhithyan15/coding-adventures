"""Generate rung-151 (adjusted density / a MIXED-OP three-term numerator over a TWO-TERM sum — five-quantity, mixed numerator) items.json.

Rung 151 continues the **FIVE-QUANTITY, two-term-denominator** family. rung-149 put a three-term SUM over a two-term sum, `(a+b+c)/(d+e)`;
rung-150 flipped the DENOMINATOR to a difference, `(a+b+c)/(d−e)`. rung-151 instead flips the NUMERATOR operation — a MIXED-OP three-term
numerator over a two-term SUM, `(a+b−c)/(d+e)`: two parts are added, a deduction is subtracted (a net numerator), and that net is divided by
the sum of two spans. It is the adjusted-density shape, `(a+b−c)/(d+e)`, the first five-quantity rung whose numerator mixes `+` and `−`.

`(a+b−c)/(d+e)` combines THREE numerator terms with mixed signs `a+b−c` over the SUM of two spans `d+e`. Both sides are totals that must be
formed BEFORE the division: the two parts add and the deduction subtracts into the numerator NET, both spans pool into the denominator total,
and only then is the net divided by the span total. The mixed-sign numerator brings a slip the all-`+` numerator rung-149 could not test:
**the numerator sign error** — ADDING the deduction instead of subtracting it, `(a+b+c)/(d+e)` (treating the deduction as a third part). The
other canonical slip carries over from the two-term-denominator family — **dropping a denominator term**, `(a+b−c)/d` (dividing by only one
of the two spans).

The setup: two parts `part_one`, `part_two` are added and a `deduction` is subtracted (a net numerator `part_one + part_two − deduction`),
and that net is spread across a span formed from two stretches `span_one`, `span_two` (a span total `span_one + span_two`). The figures are:

  ADJUSTED DENSITY  (part_one + part_two − deduction) / (span_one + span_two)  [ MIXED-OP numerator OVER a two-term sum: net numerator / span total ]
  NET NUMERATOR     part_one + part_two − deduction                          [ the mixed-op numerator net (divided by the span total) ]
  SPAN TOTAL        span_one + span_two                                      [ the two-term denominator total (the net is divided by) ]

The **adjusted density** is the headline; the **net numerator** (parts minus the deduction) and the **span total** (both spans) ride
alongside as component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline
rungs 47-150 shipped. Critically, the net numerator `(a+b−c)` is the *legitimate* subtract-the-deduction figure, whereas the distractor
`(a+b+c)/(d+e)` is the *slip* of adding the deduction as if it were a part — the panel puts the honest net numerator and the sign-error slip
side by side so the difference is exactly "did you SUBTRACT the deduction, or add it?".

Each figure is a `compute_dimensioned` program (`observe` the five quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the addition and subtraction to net the numerator, the addition to pool the spans, then the division of the net by the span
total to form the compound figure (so (a+b−c)/(d+e) evaluates as ((a+b−c)/(d+e))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../149/150. This rung exercises the engine across a
**mixed-op three-term numerator over a two-term sum** — the fact that `(a+b−c)/(d+e)` subtracts the deduction and is NOT `(a+b+c)/(d+e)` and
NOT `(a+b−c)/d` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same
way rungs 100/.../149/150 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the five observed quantities via `+`, `−`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the net numerator, the span total, nor the adjusted density is ever
a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`part_one`, `part_two`,
`deduction`, `span_one`, `span_two`) so no numeral hides inside a variable name.

The five options are a tight family over the same five quantities: the three real readouts plus the two classic slips —

  ADDED         (part_one + part_two + deduction) / (span_one + span_two)  ADD the deduction instead of subtracting it, treating the
                                                                          deduction as a third part (the numerator sign error), and
  DROPPED_SPAN  (part_one + part_two − deduction) / span_one              divide by only ONE of the two spans, dropping a span from the
                                                                          denominator,

which are exactly the mistakes a student makes on a net numerator over a two-term span (adding the deduction instead of subtracting, or
dropping a denominator term). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: the numerator subtracts the deduction, so — like rung-146 — the net numerator needs a **positivity guard**:
every table is built so `part_one + part_two − deduction >= 2` (asserted at build time), keeping the net numerator, the adjusted density, and
the dropped-span slip all strictly positive (the span total and the added slip are sums of positives, so they are automatically positive;
only the mixed-op numerator can go non-positive). Every observed quantity is `>= 2`. Every family member is asserted `> 0` at build time. The
seven tables give distinct adjusted densities, distinct net numerators, and distinct span totals so all three queried readouts vary across the
panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PART_ONE, PART_TWO, DEDUCTION, SPAN_ONE, SPAN_TWO) — two parts added and a deduction subtracted (part_one + part_two - deduction) over the
# sum of two spans (span_one + span_two), giving the adjusted density as a mixed-op numerator over a two-term sum (a+b-c)/(d+e). The numerator
# subtracts the deduction, so the net numerator needs a positivity guard: every row satisfies part_one + part_two - deduction >= 2 (asserted
# below). The span total and the added slip are sums of positives, so they are automatically positive. The seven tables give distinct net
# numerators (a+b-c), distinct span totals (d+e), and distinct adjusted densities ((a+b-c)/(d+e)); the five family values are asserted
# pairwise-distinct below.
TABLES = [
    (8, 6, 2, 2, 3),      # net = 12, span = 5,  density = 2.4
    (11, 9, 2, 4, 4),     # net = 18, span = 8,  density = 2.25
    (14, 8, 2, 5, 5),     # net = 20, span = 10, density = 2.0
    (10, 6, 2, 2, 2),     # net = 14, span = 4,  density = 3.5
    (12, 9, 2, 5, 4),     # net = 19, span = 9,  density = 19/9
    (16, 8, 2, 4, 2),     # net = 22, span = 6,  density = 22/6
    (13, 5, 2, 6, 5),     # net = 16, span = 11, density = 16/11
]

# The option family (5 members), all built from the five observed quantities via +, -, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "adjusted_density",
        "adjusted density (the net numerator divided by the span total)",
        "(part_one + part_two - deduction) / (span_one + span_two)",
    ),
    (
        "net_numerator",
        "the net numerator (the two parts added and the deduction subtracted, the numerator that is divided by the span total)",
        "part_one + part_two - deduction",
    ),
    (
        "span_total",
        "the span total (the two spans added, the denominator the net numerator is divided by)",
        "span_one + span_two",
    ),
    (
        "added",
        "the two parts and the deduction all added, divided by the span total, adding the deduction instead of subtracting it (a wrong operation)",
        "(part_one + part_two + deduction) / (span_one + span_two)",
    ),
    (
        "dropped_span",
        "the net numerator divided by the first span only, dividing by one of the two spans and dropping a span from the denominator (a wrong operation)",
        "(part_one + part_two - deduction) / span_one",
    ),
]
QUERIED = ["adjusted_density", "net_numerator", "span_total"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(part_one, part_two, deduction, span_one, span_two):
    # Operation order mirrors the ADJ programs exactly (the addition and subtraction net the numerator, the addition pools the spans, then
    # the net numerator is divided by the span total to form the compound figure, so (a+b-c)/(d+e) evaluates as ((a+b-c)/(d+e))), so the
    # Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    net = part_one + part_two - deduction
    span = span_one + span_two
    return {
        "adjusted_density": net / span,
        "net_numerator": net,
        "span_total": span,
        "added": (part_one + part_two + deduction) / span,
        "dropped_span": net / span_one,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for part_one, part_two, deduction, span_one, span_two in TABLES:
        # Every observed quantity is a plain positive number >= 2, AND the mixed-op numerator is guarded positive: the net numerator
        # part_one + part_two - deduction must be >= 2. The span total and the added slip are sums of positives, so they are automatically
        # positive; only the mixed-op numerator can go non-positive, so it is the only guard needed.
        assert (
            part_one >= 2
            and part_two >= 2
            and deduction >= 2
            and span_one >= 2
            and span_two >= 2
        ), (part_one, part_two, deduction, span_one, span_two)
        assert part_one + part_two - deduction >= 2, (part_one, part_two, deduction)
        fv = family_values(part_one, part_two, deduction, span_one, span_two)
        for key, v in fv.items():
            assert v > 0, (key, part_one, part_two, deduction, span_one, span_two, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    part_one,
                    part_two,
                    deduction,
                    span_one,
                    span_two,
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
                part_one,
                part_two,
                deduction,
                span_one,
                span_two,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r151ada-{idx + 1:02d}",
                "qtype": "adjusted_density",
                "stem": (
                    f"A coverage study records two parts of {num(part_one)} and {num(part_two)} with a "
                    f"deduction of {num(deduction)}, spread across two spans of {num(span_one)} and "
                    f"{num(span_two)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe part_one({num(part_one)})\n"
                    f"observe part_two({num(part_two)})\n"
                    f"observe deduction({num(deduction)})\n"
                    f"observe span_one({num(span_one)})\n"
                    f"observe span_two({num(span_two)})\n"
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
            "ADJ-LADDER rung 151 — adjusted density from FIVE stated quantities (CONTINUING the five-quantity, two-term-denominator "
            "family). rung-149 put a three-term SUM over a two-term sum (a+b+c)/(d+e); rung-150 flipped the denominator to a difference "
            "(a+b+c)/(d−e); rung-151 instead flips the NUMERATOR operation — a MIXED-OP three-term numerator over a two-term SUM "
            "(a+b−c)/(d+e). From a net numerator (part_one + part_two − deduction) divided by a span total (span_one + span_two), compute "
            "the adjusted density ((part_one+part_two−deduction)/(span_one+span_two)), the net numerator (part_one+part_two−deduction), or "
            "the span total (span_one+span_two). Each item is a compute_dimensioned program (observe the five quantities, let answer = "
            "formula); the ADJ engine carries the arithmetic — a MIXED-OP NUMERATOR OVER A TWO-TERM SUM (a+b−c)/(d+e) (add the two parts, "
            "subtract the deduction, pool both spans FIRST, then divide the net by the span total). The mixed-sign numerator brings a slip "
            "the all-`+` numerator rung-149 could not test — the NUMERATOR SIGN ERROR, adding the deduction instead of subtracting it "
            "((a+b+c)/(d+e), treating the deduction as a third part) — alongside the carried-over DROPPING a denominator term ((a+b−c)/d). "
            "The panel puts the honest net numerator (a+b−c) beside the sign-error slip ((a+b+c)/(d+e)) so the difference is exactly 'did "
            "you SUBTRACT the deduction, or add it?'. The harness matches the scalar to the printed options. Contamination-safe: every "
            "figure is built only from the five observed quantities via +, −, and / — no constant leaks, and neither the net numerator, the "
            "span total, nor the adjusted density ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. Because the numerator subtracts the deduction, the net "
            "numerator carries a positivity guard (part_one + part_two − deduction >= 2) so every figure stays strictly positive; the span "
            "total and the added slip are sums of positives and so are automatically positive. The five family values are kept pairwise "
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
