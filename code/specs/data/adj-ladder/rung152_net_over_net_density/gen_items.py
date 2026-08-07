"""Generate rung-152 (net-over-net density / a MIXED-OP numerator over a TWO-TERM DIFFERENCE — five-quantity, both sides netted) items.json.

Rung 152 **CLOSES the five-quantity two-term-denominator mini-matrix**. That matrix flips one operator at a time from the (a+b+c)/(d+e)
baseline: rung-149 kept both sides all-sum; rung-150 flipped the DENOMINATOR to a difference, (a+b+c)/(d−e); rung-151 flipped the NUMERATOR
to mixed-op, (a+b−c)/(d+e). rung-152 combines BOTH flips at once — a MIXED-OP numerator over a two-term DIFFERENCE, `(a+b−c)/(d−e)`: two
credits are added and a deduction subtracted (a net numerator), divided by a gross span minus a loss (a net denominator). It is the
net-over-net-density shape, `(a+b−c)/(d−e)`, the first five-quantity rung whose numerator AND denominator are both nets.

`(a+b−c)/(d−e)` combines a net numerator `a+b−c` over a net denominator `d−e`. BOTH sides subtract before the division: the two credits add
and the deduction subtracts into the numerator NET, the loss subtracts from the gross span into the denominator NET, and only then is one net
divided by the other. With a subtraction on BOTH sides, the rung inherits BOTH the numerator-side and denominator-side operation slips at
once: the **numerator sign error** — ADDING the deduction instead of subtracting it, `(a+b+c)/(d−e)` (treating the deduction as a third
credit) — and the **denominator wrong operation** — ADDING the loss to the span instead of subtracting, `(a+b−c)/(d+e)` (using the
gross-plus-loss where the net belongs).

The setup: two credits `credit_one`, `credit_two` are added and a `deduction` is subtracted (a net numerator `credit_one + credit_two −
deduction`), and that net is divided by a NET span — a `span_gross` minus a `span_loss` (a net span `span_gross − span_loss`). The figures
are:

  NET-OVER-NET DENSITY  (credit_one + credit_two − deduction) / (span_gross − span_loss)  [ net numerator OVER a net denominator ]
  NET NUMERATOR         credit_one + credit_two − deduction                            [ the mixed-op numerator net (divided by the net span) ]
  NET SPAN              span_gross − span_loss                                          [ the two-term denominator net (the net numerator is divided by) ]

The **net-over-net density** is the headline; the **net numerator** (credits minus the deduction) and the **net span** (gross minus loss)
ride alongside as component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline"
discipline rungs 47-151 shipped. Critically, the net numerator and the net span are the two honest subtract-first figures; the distractors
are the two ways to flip a subtraction into an addition (on the numerator, or on the denominator).

Each figure is a `compute_dimensioned` program (`observe` the five quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the addition and subtraction to net the numerator, the subtraction to net the span, then the division of the numerator net by
the span net to form the compound figure (so (a+b−c)/(d−e) evaluates as ((a+b−c)/(d−e))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../150/151. This rung exercises the engine across a
**mixed-op numerator over a two-term difference** — the fact that `(a+b−c)/(d−e)` nets BOTH sides and is NOT `(a+b+c)/(d−e)` and NOT
`(a+b−c)/(d+e)` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same
way rungs 100/.../150/151 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the five observed quantities via `+`, `−`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the net numerator, the net span, nor the net-over-net density is
ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`credit_one`,
`credit_two`, `deduction`, `span_gross`, `span_loss`) so no numeral hides inside a variable name.

The five options are a tight family over the same five quantities: the three real readouts plus the two operation slips —

  NUM_ADDED   (credit_one + credit_two + deduction) / (span_gross − span_loss)  ADD the deduction instead of subtracting it, the numerator
                                                                               sign error, and
  DEN_ADDED   (credit_one + credit_two − deduction) / (span_gross + span_loss)  ADD the loss to the span instead of subtracting, the
                                                                               denominator wrong operation,

which are exactly the mistakes a student makes with a subtraction on both sides (flipping the numerator sign, or the denominator sign). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: BOTH sides subtract, so — unlike the one-guard rungs 150/151 — this rung needs **two positivity guards**: every
table is built so `credit_one + credit_two − deduction >= 2` AND `span_gross − span_loss >= 2` (both asserted at build time), keeping the net
numerator, the net span, the density, and both slips all strictly positive (a non-positive numerator or denominator would break or flip the
density). Every observed quantity is `>= 2`. Every family member is asserted `> 0` at build time. The seven tables give distinct densities,
distinct net numerators, and distinct net spans so all three queried readouts vary across the panel; the five family values are pairwise
distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CREDIT_ONE, CREDIT_TWO, DEDUCTION, SPAN_GROSS, SPAN_LOSS) — a net numerator (credit_one + credit_two - deduction) over a net span
# (span_gross - span_loss), giving the net-over-net density as a mixed-op numerator over a two-term difference (a+b-c)/(d-e). BOTH sides
# subtract, so TWO positivity guards are needed: credit_one + credit_two - deduction >= 2 AND span_gross - span_loss >= 2 (both asserted
# below). The seven tables give distinct net numerators (a+b-c), distinct net spans (d-e), and distinct densities ((a+b-c)/(d-e)); the five
# family values are asserted pairwise-distinct below.
TABLES = [
    (8, 6, 2, 7, 2),      # net = 12, span = 5,  density = 2.4
    (11, 9, 2, 10, 2),    # net = 18, span = 8,  density = 2.25
    (14, 8, 2, 12, 2),    # net = 20, span = 10, density = 2.0
    (10, 6, 2, 6, 2),     # net = 14, span = 4,  density = 3.5
    (12, 9, 2, 11, 2),    # net = 19, span = 9,  density = 19/9
    (16, 8, 2, 8, 2),     # net = 22, span = 6,  density = 22/6
    (13, 5, 2, 13, 2),    # net = 16, span = 11, density = 16/11
]

# The option family (5 members), all built from the five observed quantities via +, -, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "net_over_net_density",
        "net-over-net density (the net numerator divided by the net span)",
        "(credit_one + credit_two - deduction) / (span_gross - span_loss)",
    ),
    (
        "net_numerator",
        "the net numerator (the two credits added and the deduction subtracted, the numerator that is divided by the net span)",
        "credit_one + credit_two - deduction",
    ),
    (
        "net_span",
        "the net span (the gross span minus the span loss, the denominator the net numerator is divided by)",
        "span_gross - span_loss",
    ),
    (
        "num_added",
        "the two credits and the deduction all added, divided by the net span, adding the deduction instead of subtracting it (a wrong operation)",
        "(credit_one + credit_two + deduction) / (span_gross - span_loss)",
    ),
    (
        "den_added",
        "the net numerator divided by the gross span plus the span loss, adding the loss to the span instead of subtracting it (a wrong operation)",
        "(credit_one + credit_two - deduction) / (span_gross + span_loss)",
    ),
]
QUERIED = ["net_over_net_density", "net_numerator", "net_span"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(credit_one, credit_two, deduction, span_gross, span_loss):
    # Operation order mirrors the ADJ programs exactly (the addition and subtraction net the numerator, the subtraction nets the span, then
    # the numerator net is divided by the span net to form the compound figure, so (a+b-c)/(d-e) evaluates as ((a+b-c)/(d-e))), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    net = credit_one + credit_two - deduction
    span = span_gross - span_loss
    return {
        "net_over_net_density": net / span,
        "net_numerator": net,
        "net_span": span,
        "num_added": (credit_one + credit_two + deduction) / span,
        "den_added": net / (span_gross + span_loss),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for credit_one, credit_two, deduction, span_gross, span_loss in TABLES:
        # Every observed quantity is a plain positive number >= 2, AND BOTH nets are guarded positive: the net numerator
        # credit_one + credit_two - deduction must be >= 2 AND the net span span_gross - span_loss must be >= 2. Both are subtractions that
        # can go non-positive, so both are guarded.
        assert (
            credit_one >= 2
            and credit_two >= 2
            and deduction >= 2
            and span_gross >= 2
            and span_loss >= 2
        ), (credit_one, credit_two, deduction, span_gross, span_loss)
        assert credit_one + credit_two - deduction >= 2, (credit_one, credit_two, deduction)
        assert span_gross - span_loss >= 2, (span_gross, span_loss)
        fv = family_values(credit_one, credit_two, deduction, span_gross, span_loss)
        for key, v in fv.items():
            assert v > 0, (key, credit_one, credit_two, deduction, span_gross, span_loss, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    credit_one,
                    credit_two,
                    deduction,
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
                credit_one,
                credit_two,
                deduction,
                span_gross,
                span_loss,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r152nnd-{idx + 1:02d}",
                "qtype": "net_over_net_density",
                "stem": (
                    f"A ledger study records two credits of {num(credit_one)} and {num(credit_two)} with a "
                    f"deduction of {num(deduction)}, divided over a gross span of {num(span_gross)} with a "
                    f"span loss of {num(span_loss)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe credit_one({num(credit_one)})\n"
                    f"observe credit_two({num(credit_two)})\n"
                    f"observe deduction({num(deduction)})\n"
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
            "ADJ-LADDER rung 152 — net-over-net density from FIVE stated quantities (CLOSES the five-quantity two-term-denominator "
            "mini-matrix). rung-149 kept both sides all-sum (a+b+c)/(d+e); rung-150 flipped the denominator to a difference (a+b+c)/(d−e); "
            "rung-151 flipped the numerator to mixed-op (a+b−c)/(d+e); rung-152 combines BOTH flips — a MIXED-OP numerator over a two-term "
            "DIFFERENCE (a+b−c)/(d−e): two credits added and a deduction subtracted (a net numerator), divided by a gross span minus a loss "
            "(a net denominator). From a net numerator (credit_one + credit_two − deduction) divided by a net span (span_gross − span_loss), "
            "compute the net-over-net density, the net numerator, or the net span. Each item is a compute_dimensioned program (observe the "
            "five quantities, let answer = formula); the ADJ engine carries the arithmetic — a MIXED-OP NUMERATOR OVER A TWO-TERM DIFFERENCE "
            "(a+b−c)/(d−e) (net the numerator by adding the credits and subtracting the deduction, net the span by subtracting the loss "
            "FIRST, then divide net by net). With a subtraction on BOTH sides, it inherits both operation slips at once — the NUMERATOR SIGN "
            "ERROR, adding the deduction instead of subtracting it ((a+b+c)/(d−e)) — and the DENOMINATOR WRONG OPERATION, adding the loss to "
            "the span instead of subtracting ((a+b−c)/(d+e)). The panel puts the two honest nets beside the two ways to flip a subtraction "
            "into an addition. The harness matches the scalar to the printed options. Contamination-safe: every figure is built only from "
            "the five observed quantities via +, −, and / — no constant leaks, and neither the net numerator, the net span, nor the density "
            "ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. Because both sides subtract, this rung needs TWO positivity guards (credit_one + credit_two − deduction "
            ">= 2 AND span_gross − span_loss >= 2) so every figure stays strictly positive. The five family values are kept pairwise distinct "
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
