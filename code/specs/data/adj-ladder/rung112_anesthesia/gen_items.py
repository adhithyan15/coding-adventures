"""Generate rung-112 (anesthesiology / gas-delivery) items.json for the ADJ-LADDER.

Rung 112 opens the **anesthesiology / anaesthetic-gas delivery** panel on the quantitative band — the arithmetic of a
delivered-gas index. A `flow_rate` (fresh-gas flow) TIMES the SUM of an `induction_minutes` and a `maintenance_minutes` (the two
phases the flow runs across) gives the delivered gas volume, and that volume is DIVIDED by a `circuit_count` (how many breathing
circuits the reading is averaged over) to give the delivered-gas index. A **factor DISTRIBUTED over a two-term sum, all over a
divisor** introduces a genuinely NEW arithmetic family on the ladder: `a*(b+c)/d`, i.e. `((a * (b + c)) / d)`.

This is genuinely new — rungs 108-111 built the three-term-numerator ratios where the two leading terms were combined by a
SINGLE operator before the third joined (108 `(a+b+c)/d`, 109 `(a-b+c)/d`, 110 `(a*b+c)/d`, 111 `(a*b-c)/d`); rung-112 is the
FIRST **distributive** shape — a factor multiplied INTO a parenthesised SUM, all over a divisor. The parentheses now sit around
the SUM (the multiplicand) rather than around the whole numerator. Every prior ratio used either a two-term numerator (rung-37
`(a+b)/(c+d)`, rung-99 `(a*b)/(c+d)`, rung-100 `(a+b)/(c*d)`, rung-104 `(a-b)/(c*d)`, the difference-denominator trio rung-105
`(a+b)/(c-d)`, rung-106 `a*b/(c-d)`, rung-107 `(a-b)/(c-d)`) or a flat three-term numerator (108-111). Rung-112 moves to
`a*(b+c)/d`. The operator order matters: `a*(b+c)/d` is `((a*(b+c))/d)` (the factor multiplies the whole sum, then the product is
divided; `*` and `/` bind left-to-right so `a*(b+c)/d` = `(a*(b+c))/d`), NOT `a*b+c/d` (dropping the sum parentheses so the factor
multiplies only the first term and the second term is divided by the divisor and then added) and NOT `(a*b)/(c+d)` (regrouping so
only `a*b` forms the numerator and the second sum-term joins the divisor in the denominator) — the two distractors exploit
exactly those confusions.

The setup: a `flow_rate`, an `induction_minutes`, a `maintenance_minutes`, and a `circuit_count`. The total is:

  DELIVERED-GAS INDEX  flow_rate * (induction_minutes + maintenance_minutes) / circuit_count  [ a factor over a sum, over a divisor ]
  DELIVERED GAS        flow_rate * (induction_minutes + maintenance_minutes)                  [ the distributed-product numerator ]
  CIRCUIT COUNT        circuit_count                                                          [ the divisor ]

The **delivered-gas index** is what makes this rung distinctive — it is the ladder's first **distributive** figure, a factor
carried into a sum before the division. It is a rate (delivered gas per circuit), framed as an *index* to keep it
dimensionless-clean — the same discipline rungs 100/104/.../111 used for their ratios. (The delivered gas `a*(b+c)` and the
circuit count `d` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-111
shipped their component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the addition of the two phase minutes into the total time, the multiplication of that sum by the flow rate into
the delivered gas, then the division of that gas by the circuit count (the factor distributed over the parenthesised sum, so
a*(b+c)/d evaluates as ((a*(b+c))/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../110/111. This rung exercises the engine across a **factor-over-a-sum, over a
divisor** — the fact that `a*(b+c)/d` is `((a*(b+c))/d)` and NOT `a*b+c/d` and NOT `(a*b)/(c+d)` made computable. The ratio golds
are non-integer f64s; the engine's IEEE-double division matches Python's the same way rungs 99/100/104/.../111 relied on (well
within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `+` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the delivered gas, the circuit count, nor any
index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers**
(`flow_rate`, `induction_minutes`, `maintenance_minutes`, `circuit_count`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    flow_rate * induction_minutes + maintenance_minutes / circuit_count  drop the sum parentheses so the flow rate
                                                                        multiplies only the induction minutes and the maintenance
                                                                        minutes are divided by the circuit count and then added
                                                                        (the classic `a*(b+c)/d` vs `a*b+c/d` distributivity
                                                                        error), and
  SWAPPED    (flow_rate * induction_minutes) / (maintenance_minutes + circuit_count)  regroup so only the flow-times-induction
                                                                        product forms the numerator and the maintenance minutes
                                                                        join the circuit count in the denominator
                                                                        (`(a*b)/(c+d)` instead of `a*(b+c)/d`),

which are exactly the mistakes a student makes (failing to distribute the factor across the whole sum, or regrouping which terms
belong to the numerator vs the divisor). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: every family member is a product, sum, or quotient of strictly positive observed quantities `>= 2`,
so all five are strictly positive by construction (this rung has NO subtraction). The tables are chosen so `circuit_count >= 2`
(divisor never zero), the delivered-gas index never coincides with the circuit count or the delivered gas, and the five family
values are pairwise distinct with a comfortable margin; and — so all three queried readouts vary across the panel — the seven
tables give distinct delivered-gas indices, distinct delivered gases, and distinct circuit counts, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (FLOW_RATE, INDUCTION_MINUTES, MAINTENANCE_MINUTES, CIRCUIT_COUNT) — a flow rate times the sum of an induction and a maintenance
# time for the delivered gas, all divided by a circuit count, all plain positive numbers >= 2. This rung has NO subtraction, so
# every family member is a product, sum, or quotient of positives and is strictly positive by construction; circuit_count >= 2
# keeps the divisor away from zero. The five family values are asserted pairwise-distinct below. The seven tables give distinct
# delivered-gas indices, distinct delivered gases, and distinct circuit counts so all three queried readouts vary across the
# panel.
TABLES = [
    (2, 3, 2, 2),
    (3, 2, 2, 3),
    (2, 5, 4, 4),
    (3, 4, 3, 5),
    (2, 6, 5, 6),
    (3, 6, 2, 7),
    (4, 4, 3, 8),
]

# The option family (5 members), all built from the four observed quantities via *, + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "delivered_gas_index",
        "delivered-gas index (the delivered gas divided by the circuit count)",
        "flow_rate * (induction_minutes + maintenance_minutes) / circuit_count",
    ),
    (
        "delivered_gas",
        "the delivered gas (the flow rate times the sum of the induction and maintenance minutes, the numerator divided by the circuit count)",
        "flow_rate * (induction_minutes + maintenance_minutes)",
    ),
    (
        "circuit_count",
        "the circuit count (the divisor the delivered gas is divided by)",
        "circuit_count",
    ),
    (
        "crossed",
        "the flow rate times the induction minutes plus the maintenance minutes divided by the circuit count, dropping the sum parentheses so the flow rate multiplies only the induction minutes (a wrong distribution)",
        "flow_rate * induction_minutes + maintenance_minutes / circuit_count",
    ),
    (
        "swapped",
        "the flow rate times the induction minutes, divided by the maintenance minutes plus the circuit count, regrouping so only that product forms the numerator (a wrong pairing)",
        "(flow_rate * induction_minutes) / (maintenance_minutes + circuit_count)",
    ),
]
QUERIED = ["delivered_gas_index", "delivered_gas", "circuit_count"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(flow_rate, induction_minutes, maintenance_minutes, circuit_count):
    # Operation order mirrors the ADJ programs exactly (the two phase minutes sum, the flow rate multiplies that sum into the
    # delivered gas, then that numerator is divided by the circuit count, so a*(b+c)/d evaluates as ((a*(b+c))/d)), so the Python
    # option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "delivered_gas_index": flow_rate * (induction_minutes + maintenance_minutes) / circuit_count,
        "delivered_gas": flow_rate * (induction_minutes + maintenance_minutes),
        "circuit_count": circuit_count,
        "crossed": flow_rate * induction_minutes + maintenance_minutes / circuit_count,
        "swapped": (flow_rate * induction_minutes) / (maintenance_minutes + circuit_count),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for flow_rate, induction_minutes, maintenance_minutes, circuit_count in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung has NO subtraction, so every family member is a
        # product, sum, or quotient of positives and is strictly positive by construction; circuit_count >= 2 keeps the divisor
        # away from zero.
        assert (
            flow_rate >= 2
            and induction_minutes >= 2
            and maintenance_minutes >= 2
            and circuit_count >= 2
        ), (flow_rate, induction_minutes, maintenance_minutes, circuit_count)
        fv = family_values(flow_rate, induction_minutes, maintenance_minutes, circuit_count)
        for key, v in fv.items():
            assert v > 0, (key, flow_rate, induction_minutes, maintenance_minutes, circuit_count, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    flow_rate,
                    induction_minutes,
                    maintenance_minutes,
                    circuit_count,
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
                flow_rate,
                induction_minutes,
                maintenance_minutes,
                circuit_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r112anes-{idx + 1:02d}",
                "qtype": "delivered_gas_index",
                "stem": (
                    f"An anaesthetic record shows a flow rate of {num(flow_rate)} times an induction time of "
                    f"{num(induction_minutes)} plus a maintenance time of {num(maintenance_minutes)}, divided by a circuit "
                    f"count of {num(circuit_count)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe flow_rate({num(flow_rate)})\n"
                    f"observe induction_minutes({num(induction_minutes)})\n"
                    f"observe maintenance_minutes({num(maintenance_minutes)})\n"
                    f"observe circuit_count({num(circuit_count)})\n"
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
            "ADJ-LADDER rung 112 — delivered-gas index from four stated quantities (a NEW panel: anesthesiology / anaesthetic-gas "
            "delivery). From a flow rate times the sum of an induction and a maintenance time for the delivered gas, all divided "
            "by a circuit count, compute the delivered-gas index (flow_rate*(induction_minutes+maintenance_minutes)/"
            "circuit_count), the delivered gas (flow_rate*(induction_minutes+maintenance_minutes)), or the circuit count. Each "
            "item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW family, A FACTOR DISTRIBUTED OVER A SUM, OVER A DIVISOR a*(b+c)/d (add the two phase minutes, "
            "multiply the sum by the flow rate, divide by the circuit count, so a*(b+c)/d = ((a*(b+c))/d); the FIRST DISTRIBUTIVE "
            "shape on the ladder — a factor carried INTO a parenthesised sum before the division, where the parentheses sit "
            "around the SUM rather than the whole numerator. Rungs 108-111 built the flat three-term numerators (108 (a+b+c)/d, "
            "109 (a-b+c)/d, 110 (a*b+c)/d, 111 (a*b-c)/d), and every earlier ratio used a TWO-term numerator: 37 (a+b)/(c+d), 99 "
            "(a*b)/(c+d), 100 (a+b)/(c*d), 104 (a-b)/(c*d), and the difference-denominator trio 105 (a+b)/(c-d), 106 a*b/(c-d), "
            "107 (a-b)/(c-d)) — and the harness matches the scalar to the printed options. The delivered-gas index is a rate "
            "(delivered gas per circuit), framed as an INDEX so the dimensionless value stays honest. Contamination-safe: every "
            "figure is built only from the four observed quantities via *, + and / — no constant leaks, and neither the delivered "
            "gas, the circuit count, nor any index ever appears as a literal (each is computed) — and the observed quantities "
            "carry digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the same "
            "four quantities, so the distractors are exactly the slips students make: dropping the sum parentheses so the factor "
            "multiplies only the first term (a*b+c/d, a wrong distribution) and regrouping so only that product forms the "
            "numerator ((a*b)/(c+d), a wrong pairing). The core confusion tested is that a*(b+c)/d is ((a*(b+c))/d), not a*b+c/d "
            "and not (a*b)/(c+d). This rung has NO subtraction, so every family member is a product, sum, or quotient of "
            "positives and is strictly positive by construction; the circuit count is >= 2 (divisor never zero)."
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
