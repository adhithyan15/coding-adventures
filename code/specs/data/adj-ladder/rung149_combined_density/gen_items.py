"""Generate rung-149 (combined density / a THREE-TERM sum over a TWO-TERM sum — the first FIVE-quantity rung) items.json.

Rung 149 opens the **FIVE-QUANTITY** rungs. Rungs 145-148 kept a three-term numerator over a LONE denominator (four observed quantities in
all). rung-149 gives the DENOMINATOR a second term too — a three-term sum over a two-term sum, `(a+b+c)/(d+e)` — so the panel now observes
FIVE quantities. It is the combined-density shape, `(a+b+c)/(d+e)`, the first rung whose fraction has multiple terms on BOTH sides of the bar.

`(a+b+c)/(d+e)` sums THREE parts `a+b+c` over the sum of TWO spans `d+e`. Both sides are totals that must be formed BEFORE the division: all
three parts pool into the numerator total, both spans pool into the denominator total, and only then is one total divided by the other. With
terms on both sides, the new canonical slip is **dropping a term** — and now it can happen on EITHER side: dropping a part from the top,
`(a+b)/(d+e)`, or dropping a span from the bottom, `(a+b+c)/d`. These two symmetric term-drops are the mistakes the four-quantity rungs
(with only one denominator term) could not test.

The setup: three parts `part_one`, `part_two`, `part_three` are pooled (a part total `part_one + part_two + part_three`) and spread across a
span formed from two stretches `span_one`, `span_two` (a span total `span_one + span_two`). The figures are:

  COMBINED DENSITY  (part_one + part_two + part_three) / (span_one + span_two)  [ THREE-TERM sum OVER a TWO-TERM sum: part total / span total ]
  PART TOTAL        part_one + part_two + part_three                          [ the three-term numerator total (divided by the span total) ]
  SPAN TOTAL        span_one + span_two                                       [ the two-term denominator total (the part total is divided by) ]

The **combined density** is the headline; the **part total** (all three parts) and the **span total** (both spans) ride alongside as
component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline rungs 47-148
shipped. The part total and the span total are the two honest sub-totals; the distractors are the two ways to drop a term while forming them.

Each figure is a `compute_dimensioned` program (`observe` the five quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the two additions to pool the parts, the addition to pool the spans, then the division of the part total by the span total to
form the compound figure (so (a+b+c)/(d+e) evaluates as ((a+b+c)/(d+e))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../147/148. This rung exercises the engine across a
**three-term sum divided by a two-term sum** — the fact that `(a+b+c)/(d+e)` pools ALL parts over ALL spans and is NOT `(a+b)/(d+e)` and NOT
`(a+b+c)/d` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way
rungs 100/.../147/148 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the five observed quantities via `+` and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the part total, the span total, nor the combined density is ever a
literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`part_one`, `part_two`,
`part_three`, `span_one`, `span_two`) so no numeral hides inside a variable name. (The `_one/_two/_three` suffixes are English words, not
digits.)

The five options are a tight family over the same five quantities: the three real readouts plus the two term-drop slips —

  DROPPED_PART  (part_one + part_two) / (span_one + span_two)  pool only TWO of the three parts, dropping a part from the numerator, and
  DROPPED_SPAN  (part_one + part_two + part_three) / span_one  divide by only ONE of the two spans, dropping a span from the denominator,

which are exactly the mistakes a student makes pooling parts over spans with terms on both sides (dropping a numerator term, or dropping a
denominator term). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `+` and `/` over positive quantities, so **every figure is automatically positive** (no
subtraction, no product) — no positivity guards are needed. Every observed quantity is `>= 2`. Every family member is asserted `> 0` at
build time. The seven tables give distinct combined densities, distinct part totals, and distinct span totals so all three queried readouts
vary across the panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PART_ONE, PART_TWO, PART_THREE, SPAN_ONE, SPAN_TWO) — three parts pooled (part_one + part_two + part_three) over the sum of two spans
# (span_one + span_two), giving the combined density as a three-term sum over a two-term sum (a+b+c)/(d+e). This is the FIRST five-quantity
# rung. This rung uses only + and / over positive quantities, so every figure is automatically positive; no positivity guards are needed. The
# seven tables give distinct part totals (a+b+c), distinct span totals (d+e), and distinct combined densities ((a+b+c)/(d+e)); the five family
# values are asserted pairwise-distinct below.
TABLES = [
    (2, 4, 6, 2, 3),      # part = 12, span = 5,  density = 2.4
    (3, 5, 12, 4, 4),     # part = 20, span = 8,  density = 2.5
    (4, 6, 20, 5, 5),     # part = 30, span = 10, density = 3.0
    (3, 9, 12, 5, 2),     # part = 24, span = 7,  density = 24/7
    (5, 9, 21, 5, 4),     # part = 35, span = 9,  density = 35/9
    (6, 10, 32, 4, 2),    # part = 48, span = 6,  density = 8.0
    (7, 11, 18, 5, 6),    # part = 36, span = 11, density = 36/11
]

# The option family (5 members), all built from the five observed quantities via + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "combined_density",
        "combined density (the part total divided by the span total)",
        "(part_one + part_two + part_three) / (span_one + span_two)",
    ),
    (
        "part_total",
        "the part total (all three parts added, the numerator that is divided by the span total)",
        "part_one + part_two + part_three",
    ),
    (
        "span_total",
        "the span total (the two spans added, the denominator the part total is divided by)",
        "span_one + span_two",
    ),
    (
        "dropped_part",
        "the first two parts divided by the span total, pooling only two of the three parts and dropping a part from the numerator (a wrong operation)",
        "(part_one + part_two) / (span_one + span_two)",
    ),
    (
        "dropped_span",
        "the part total divided by the first span only, dividing by one of the two spans and dropping a span from the denominator (a wrong operation)",
        "(part_one + part_two + part_three) / span_one",
    ),
]
QUERIED = ["combined_density", "part_total", "span_total"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(part_one, part_two, part_three, span_one, span_two):
    # Operation order mirrors the ADJ programs exactly (the additions pool the parts and the spans, then the part total is divided by the
    # span total to form the compound figure, so (a+b+c)/(d+e) evaluates as ((a+b+c)/(d+e))), so the Python option value and the engine
    # result are the same IEEE-double (well within the 1e-9 tolerance).
    part = part_one + part_two + part_three
    span = span_one + span_two
    return {
        "combined_density": part / span,
        "part_total": part,
        "span_total": span,
        "dropped_part": (part_one + part_two) / span,
        "dropped_span": part / span_one,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for part_one, part_two, part_three, span_one, span_two in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only + and / over positive quantities, so positivity is
        # automatic — no positivity guards are needed.
        assert (
            part_one >= 2
            and part_two >= 2
            and part_three >= 2
            and span_one >= 2
            and span_two >= 2
        ), (part_one, part_two, part_three, span_one, span_two)
        fv = family_values(part_one, part_two, part_three, span_one, span_two)
        for key, v in fv.items():
            assert v > 0, (key, part_one, part_two, part_three, span_one, span_two, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    part_one,
                    part_two,
                    part_three,
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
                part_three,
                span_one,
                span_two,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r149cda-{idx + 1:02d}",
                "qtype": "combined_density",
                "stem": (
                    f"A coverage study records three parts of {num(part_one)}, {num(part_two)}, and "
                    f"{num(part_three)} pooled across two spans of {num(span_one)} and {num(span_two)}. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe part_one({num(part_one)})\n"
                    f"observe part_two({num(part_two)})\n"
                    f"observe part_three({num(part_three)})\n"
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
            "ADJ-LADDER rung 149 — combined density from FIVE stated quantities (the FIRST five-quantity rung). Rungs 145-148 kept a "
            "three-term numerator over a LONE denominator (four quantities). rung-149 gives the denominator a second term too — a "
            "three-term sum over a two-term sum (a+b+c)/(d+e) — so the panel now observes FIVE quantities. From a part total (part_one + "
            "part_two + part_three) divided by a span total (span_one + span_two), compute the combined density "
            "((part_one+part_two+part_three)/(span_one+span_two)), the part total (part_one+part_two+part_three), or the span total "
            "(span_one+span_two). Each item is a compute_dimensioned program (observe the five quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — a THREE-TERM SUM OVER A TWO-TERM SUM (a+b+c)/(d+e) (pool all three parts and both spans FIRST, "
            "then divide the part total by the span total). With terms on both sides, the new slip is dropping a term on EITHER side — "
            "dropping a part from the top ((a+b)/(d+e)) or dropping a span from the bottom ((a+b+c)/d) — the mistakes the four-quantity "
            "rungs (one denominator term) could not test. The harness matches the scalar to the printed options. Contamination-safe: every "
            "figure is built only from the five observed quantities via + and / — no constant leaks, and neither the part total, the span "
            "total, nor the combined density ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. This rung uses only + and / over positive quantities, so every figure "
            "is automatically positive — no positivity guards are needed — and the five family values are kept pairwise distinct with all "
            "three queried readouts varying across the panel, all asserted strictly positive at build time."
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
