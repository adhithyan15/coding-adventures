"""Generate rung-150 (net density / a THREE-TERM sum over a TWO-TERM DIFFERENCE — five-quantity, difference denominator) items.json.

Rung 150 continues the **FIVE-QUANTITY, two-term-denominator** family opened by rung-149. rung-149 put a three-term sum over a two-term SUM,
`(a+b+c)/(d+e)`; rung-150 flips the DENOMINATOR operation to a subtraction — a three-term sum over a two-term DIFFERENCE, `(a+b+c)/(d−e)`.
The numerator still pools all three parts; the denominator is now a NET span, one span minus another. It is the net-density shape,
`(a+b+c)/(d−e)`, the first five-quantity rung whose denominator is a difference.

`(a+b+c)/(d−e)` sums THREE parts `a+b+c` over the DIFFERENCE of two spans `d−e`. Both sides are totals that must be formed BEFORE the
division: all three parts pool into the numerator total, the loss is subtracted from the gross span into the denominator NET, and only then
is the part total divided by the net span. The difference denominator brings a slip the SUM-denominator rung-149 could not test: **the wrong
denominator operation** — ADDING the two spans instead of subtracting, `(a+b+c)/(d+e)` (using the gross-plus-loss where the net belongs). The
other canonical slip carries over from rung-149 — **dropping a numerator term**, `(a+b)/(d−e)` (pooling only two of the three parts).

The setup: three parts `part_one`, `part_two`, `part_three` are pooled (a part total `part_one + part_two + part_three`) and spread across a
NET span — a `span_gross` minus a `span_loss` (a net span `span_gross − span_loss`). The figures are:

  NET DENSITY  (part_one + part_two + part_three) / (span_gross − span_loss)  [ THREE-TERM sum OVER a TWO-TERM difference: part total / net span ]
  PART TOTAL   part_one + part_two + part_three                            [ the three-term numerator total (divided by the net span) ]
  NET SPAN     span_gross − span_loss                                      [ the two-term denominator net (the part total is divided by) ]

The **net density** is the headline; the **part total** (all three parts) and the **net span** (gross minus loss) ride alongside as component
readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline rungs 47-149 shipped.
Critically, the net span `(d−e)` is the *legitimate* subtract-first denominator, whereas the distractor `(a+b+c)/(d+e)` is the *slip* of
adding the spans where the net belongs — the panel puts the honest net span and the wrong-operation slip side by side so the difference is
exactly "did you SUBTRACT the loss from the span, or add it?".

Each figure is a `compute_dimensioned` program (`observe` the five quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the two additions to pool the parts, the subtraction to net the span, then the division of the part total by the net span to
form the compound figure (so (a+b+c)/(d−e) evaluates as ((a+b+c)/(d−e))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../148/149. This rung exercises the engine across a
**three-term sum divided by a two-term difference** — the fact that `(a+b+c)/(d−e)` nets the span and is NOT `(a+b+c)/(d+e)` and NOT
`(a+b)/(d−e)` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same
way rungs 100/.../148/149 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the five observed quantities via `+`, `−`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the part total, the net span, nor the net density is ever a literal
(each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`part_one`, `part_two`,
`part_three`, `span_gross`, `span_loss`) so no numeral hides inside a variable name.

The five options are a tight family over the same five quantities: the three real readouts plus the two classic slips —

  SUMMED        (part_one + part_two + part_three) / (span_gross + span_loss)  ADD the two spans instead of subtracting, the wrong denominator
                                                                              operation (gross-plus-loss where the net belongs), and
  DROPPED_PART  (part_one + part_two) / (span_gross − span_loss)              pool only TWO of the three parts, dropping a part from the
                                                                              numerator,

which are exactly the mistakes a student makes pooling parts over a net span (adding the spans instead of subtracting, or dropping a
numerator term). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: the denominator subtracts the loss, so — unlike the all-`+ /` rung-149 — the net span needs a **positivity
guard**: every table is built so `span_gross − span_loss >= 2` (asserted at build time), keeping the net span, the net density, and every
readout strictly positive (the part total and the summed slip are sums of positives, so they are automatically positive; only the difference
denominator can go non-positive, which would also flip the net density's sign). Every observed quantity is `>= 2`. Every family member is
asserted `> 0` at build time. The seven tables give distinct net densities, distinct part totals, and distinct net spans so all three queried
readouts vary across the panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PART_ONE, PART_TWO, PART_THREE, SPAN_GROSS, SPAN_LOSS) — three parts pooled (part_one + part_two + part_three) over a net span (span_gross -
# span_loss), giving the net density as a three-term sum over a two-term difference (a+b+c)/(d-e). The denominator subtracts the loss, so the
# net span needs a positivity guard: every row satisfies span_gross - span_loss >= 2 (asserted below). The part total and the summed slip are
# sums of positives, so they are automatically positive. The seven tables give distinct part totals (a+b+c), distinct net spans (d-e), and
# distinct net densities ((a+b+c)/(d-e)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (2, 4, 6, 7, 2),      # part = 12, net = 5,  density = 2.4
    (3, 5, 12, 10, 2),    # part = 20, net = 8,  density = 2.5
    (4, 6, 20, 12, 2),    # part = 30, net = 10, density = 3.0
    (3, 9, 12, 9, 2),     # part = 24, net = 7,  density = 24/7
    (5, 9, 21, 11, 2),    # part = 35, net = 9,  density = 35/9
    (6, 10, 32, 8, 2),    # part = 48, net = 6,  density = 8.0
    (7, 11, 18, 13, 2),   # part = 36, net = 11, density = 36/11
]

# The option family (5 members), all built from the five observed quantities via +, -, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "net_density",
        "net density (the part total divided by the net span)",
        "(part_one + part_two + part_three) / (span_gross - span_loss)",
    ),
    (
        "part_total",
        "the part total (all three parts added, the numerator that is divided by the net span)",
        "part_one + part_two + part_three",
    ),
    (
        "net_span",
        "the net span (the gross span minus the span loss, the denominator the part total is divided by)",
        "span_gross - span_loss",
    ),
    (
        "summed",
        "the part total divided by the gross span plus the span loss, adding the two spans instead of subtracting them (a wrong operation)",
        "(part_one + part_two + part_three) / (span_gross + span_loss)",
    ),
    (
        "dropped_part",
        "the first two parts divided by the net span, pooling only two of the three parts and dropping a part from the numerator (a wrong operation)",
        "(part_one + part_two) / (span_gross - span_loss)",
    ),
]
QUERIED = ["net_density", "part_total", "net_span"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(part_one, part_two, part_three, span_gross, span_loss):
    # Operation order mirrors the ADJ programs exactly (the additions pool the parts, the subtraction nets the span, then the part total is
    # divided by the net span to form the compound figure, so (a+b+c)/(d-e) evaluates as ((a+b+c)/(d-e))), so the Python option value and the
    # engine result are the same IEEE-double (well within the 1e-9 tolerance).
    part = part_one + part_two + part_three
    net = span_gross - span_loss
    return {
        "net_density": part / net,
        "part_total": part,
        "net_span": net,
        "summed": part / (span_gross + span_loss),
        "dropped_part": (part_one + part_two) / net,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for part_one, part_two, part_three, span_gross, span_loss in TABLES:
        # Every observed quantity is a plain positive number >= 2, AND the difference denominator is guarded positive: the net span
        # span_gross - span_loss must be >= 2. The part total and the summed slip are sums of positives, so they are automatically positive;
        # only the difference denominator can go non-positive, so it is the only guard needed.
        assert (
            part_one >= 2
            and part_two >= 2
            and part_three >= 2
            and span_gross >= 2
            and span_loss >= 2
        ), (part_one, part_two, part_three, span_gross, span_loss)
        assert span_gross - span_loss >= 2, (span_gross, span_loss)
        fv = family_values(part_one, part_two, part_three, span_gross, span_loss)
        for key, v in fv.items():
            assert v > 0, (key, part_one, part_two, part_three, span_gross, span_loss, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    part_one,
                    part_two,
                    part_three,
                    span_gross,
                    span_loss,
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
                span_gross,
                span_loss,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r150nda-{idx + 1:02d}",
                "qtype": "net_density",
                "stem": (
                    f"A coverage study records three parts of {num(part_one)}, {num(part_two)}, and "
                    f"{num(part_three)} spread across a gross span of {num(span_gross)} with a span loss of "
                    f"{num(span_loss)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe part_one({num(part_one)})\n"
                    f"observe part_two({num(part_two)})\n"
                    f"observe part_three({num(part_three)})\n"
                    f"observe span_gross({num(span_gross)})\n"
                    f"observe span_loss({num(span_loss)})\n"
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
            "ADJ-LADDER rung 150 — net density from FIVE stated quantities (CONTINUING the five-quantity, two-term-denominator family). "
            "rung-149 put a three-term sum over a two-term SUM (a+b+c)/(d+e); rung-150 flips the denominator operation to a subtraction — a "
            "three-term sum over a two-term DIFFERENCE (a+b+c)/(d−e). From a part total (part_one + part_two + part_three) divided by a net "
            "span (span_gross − span_loss), compute the net density ((part_one+part_two+part_three)/(span_gross−span_loss)), the part total "
            "(part_one+part_two+part_three), or the net span (span_gross−span_loss). Each item is a compute_dimensioned program (observe the "
            "five quantities, let answer = formula); the ADJ engine carries the arithmetic — a THREE-TERM SUM OVER A TWO-TERM DIFFERENCE "
            "(a+b+c)/(d−e) (pool all three parts, net the span by subtracting the loss FIRST, then divide the part total by the net span). "
            "The difference denominator brings a slip the sum-denominator rung-149 could not test — the WRONG DENOMINATOR OPERATION, adding "
            "the two spans instead of subtracting ((a+b+c)/(d+e), gross-plus-loss where the net belongs) — alongside the carried-over "
            "DROPPING a numerator term ((a+b)/(d−e)). The panel puts the honest net span (d−e) beside the wrong-operation slip ((d+e)) so "
            "the difference is exactly 'did you SUBTRACT the loss from the span, or add it?'. The harness matches the scalar to the printed "
            "options. Contamination-safe: every figure is built only from the five observed quantities via +, −, and / — no constant leaks, "
            "and neither the part total, the net span, nor the net density ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. Because the denominator subtracts the loss, "
            "the net span carries a positivity guard (span_gross − span_loss >= 2) so every figure stays strictly positive; the part total "
            "and the summed slip are sums of positives and so are automatically positive. The five family values are kept pairwise distinct "
            "with all three queried readouts varying across the panel, all asserted strictly positive at build time."
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
