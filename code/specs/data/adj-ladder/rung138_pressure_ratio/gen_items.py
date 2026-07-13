"""Generate rung-138 (pressure ratio / a SUM-numerator over a DIFFERENCE — divide a total by a gap) items.json.

Rung 138 continues the **OVER-A-DIFFERENCE** column. rung-137 put a PRODUCT over a difference, `(a*b)/(c-d)`; rung-138 puts a SUM over a
difference, `(a+b)/(c-d)`. Read the other way, it is the difference-denominator twin of the sum-numerator rungs — rung-131 put a SUM over a
rate `(a+b)/(c/d)`, rung-37 a SUM over a sum `(a+b)/(c+d)`; rung-138 keeps the sum numerator and swaps the denominator for a gap, `(a+b)/(c-d)`.

This is a difference denominator. `(a+b)/(c-d)` is a SUM `a+b` divided by a DIFFERENCE `c-d` (a gap). The sum `a+b` binds and stays grouped
over the bar (grouping), and the two-part denominator `c-d` is ONE gap the whole numerator is divided by. As on rung-137, the divide-across
(`x/c - x/d`) and lost-grouping (`x/c - d`) errors go NEGATIVE when `c > d` (the regime a positive gap requires), so they are not the clean
in-range confusions. The two canonical divide-by-a-gap slips that stay in-range are: using the WRONG denominator operation, summing the two
readings instead of gapping them (`(a+b)/(c+d)` — a total instead of a gap), and INVERTING the ratio, dividing the gap by the sum instead of
the sum by the gap (`(c-d)/(a+b)` — the reciprocal, the ratio upside down).

The setup: an `inflow` combined with a `backflow` (a combined flow `inflow + backflow`), read against a pressure band formed from a
`peak_reading` minus a `trough_reading` (a pressure gap `peak_reading - trough_reading`). The figures are:

  PRESSURE RATIO  (inflow + backflow) / (peak_reading - trough_reading)  [ sum-numerator OVER a difference: combined flow / pressure gap ]
  COMBINED FLOW   inflow + backflow                                      [ the sum numerator (divided by the pressure gap) ]
  PRESSURE GAP    peak_reading - trough_reading                         [ the difference the combined flow is divided by ]

The **pressure ratio** is the ladder's first **(a sum) over (a difference) as a headline** — a ratio (how much combined flow rides on each
unit of the pressure gap), framed as a *ratio* to keep it dimensionless-clean, the same discipline rungs 100/.../136/137 used for their
ratios, spans, concentrations, densities, and indices. (The combined flow `a+b` and the pressure gap `c-d` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-137 shipped their component figures beside the headline. The two
components anchor the "add the flows FIRST, gap the readings, then divide the flow by the gap" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the addition to form the combined flow, the subtraction to form the pressure gap, then the division of the combined flow by
the pressure gap to form the compound figure (so (a+b)/(c-d) evaluates as ((a+b)/(c-d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../136/137. This rung exercises the engine across a
**sum divided by a difference** — the fact that `(a+b)/(c-d)` is one sum over one gap and NOT `(a+b)/(c+d)` and NOT `(c-d)/(a+b)` made
computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../136/137 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `/`, and `-` — **no structural
constants** — so no numeric literal appears in any program, and neither the combined flow, the pressure gap, nor the pressure ratio is ever
a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`inflow`, `backflow`,
`peak_reading`, `trough_reading`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SUMMED     (inflow + backflow) / (peak_reading + trough_reading)  divide the combined flow by the SUM of the readings instead of their
                                                                gap, using a total where a gap belongs (the wrong denominator operation), and
  INVERTED   (peak_reading - trough_reading) / (inflow + backflow)  divide the pressure gap BY the combined flow, the ratio upside down (the
                                                                reciprocal of the pressure ratio, the wrong direction),

which are exactly the mistakes a student makes with a gap denominator (mis-reading the difference as a total, or inverting the ratio). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung has a SUBTRACTION in the denominator, so unlike the pure `+ /` rungs it needs a **positivity guard**
— the pressure gap is guarded so the denominator (and the headline) stay positive: `peak_reading - trough_reading >= 2`. With that guard and
positive quantities, every family member is positive (the summed and inverted distractors are quotients of positive quantities). Every
observed quantity is `>= 2`. Every family member is asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give
distinct pressure ratios, distinct combined flows, and distinct pressure gaps so all three queried readouts vary across the panel; the five
family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (INFLOW, BACKFLOW, PEAK_READING, TROUGH_READING) — a combined flow (inflow + backflow) divided by a pressure gap (peak_reading -
# trough_reading), giving the pressure ratio as a sum over a difference (a+b)/(c-d). This rung has a SUBTRACTION in the denominator, so the
# pressure gap is guarded (peak_reading - trough_reading >= 2) to keep the denominator positive; with positive quantities every figure is
# positive. The seven tables give distinct combined flows (a+b), distinct pressure gaps (c-d), and distinct pressure ratios ((a+b)/(c-d)); the
# five family values are asserted pairwise-distinct below.
TABLES = [
    (2, 4, 7, 2),      # flow = 6,  gap = 5,  ratio = 1.2
    (3, 5, 6, 3),      # flow = 8,  gap = 3,  ratio = 2.666...
    (4, 6, 8, 4),      # flow = 10, gap = 4,  ratio = 2.5
    (5, 7, 10, 4),     # flow = 12, gap = 6,  ratio = 2.0
    (6, 8, 11, 2),     # flow = 14, gap = 9,  ratio = 1.555...
    (4, 5, 9, 2),      # flow = 9,  gap = 7,  ratio = 1.285...
    (7, 9, 13, 3),     # flow = 16, gap = 10, ratio = 1.6
]

# The option family (5 members), all built from the four observed quantities via +, /, and -. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "pressure_ratio",
        "pressure ratio (the combined flow divided by the pressure gap)",
        "(inflow + backflow) / (peak_reading - trough_reading)",
    ),
    (
        "combined_flow",
        "the combined flow (the inflow plus the backflow, the numerator that is divided by the pressure gap)",
        "inflow + backflow",
    ),
    (
        "pressure_gap",
        "the pressure gap (the peak reading minus the trough reading, the difference the combined flow is divided by)",
        "peak_reading - trough_reading",
    ),
    (
        "summed",
        "the combined flow divided by the peak reading plus the trough reading, using the sum of the readings instead of their gap as the divisor (a wrong operation)",
        "(inflow + backflow) / (peak_reading + trough_reading)",
    ),
    (
        "inverted",
        "the pressure gap divided by the combined flow, the ratio upside down instead of the combined flow over the pressure gap (a wrong operation)",
        "(peak_reading - trough_reading) / (inflow + backflow)",
    ),
]
QUERIED = ["pressure_ratio", "combined_flow", "pressure_gap"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(inflow, backflow, peak_reading, trough_reading):
    # Operation order mirrors the ADJ programs exactly (the addition forms the combined flow, the subtraction forms the pressure gap, then
    # the combined flow is divided by the pressure gap to form the compound figure, so (a+b)/(c-d) evaluates as ((a+b)/(c-d))), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    flow = inflow + backflow
    gap = peak_reading - trough_reading
    return {
        "pressure_ratio": flow / gap,
        "combined_flow": flow,
        "pressure_gap": gap,
        "summed": flow / (peak_reading + trough_reading),
        "inverted": gap / flow,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for inflow, backflow, peak_reading, trough_reading in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung has a subtraction in the denominator, so the pressure gap is
        # guarded (peak_reading - trough_reading >= 2) to keep the denominator positive; with positive quantities every figure is positive.
        assert (
            inflow >= 2
            and backflow >= 2
            and peak_reading >= 2
            and trough_reading >= 2
        ), (inflow, backflow, peak_reading, trough_reading)
        assert peak_reading - trough_reading >= 2, (peak_reading, trough_reading)
        fv = family_values(inflow, backflow, peak_reading, trough_reading)
        for key, v in fv.items():
            assert v > 0, (key, inflow, backflow, peak_reading, trough_reading, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    inflow,
                    backflow,
                    peak_reading,
                    trough_reading,
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
                inflow,
                backflow,
                peak_reading,
                trough_reading,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r138pra-{idx + 1:02d}",
                "qtype": "pressure_ratio",
                "stem": (
                    f"A pressure study records an inflow of {num(inflow)} combined with a backflow of "
                    f"{num(backflow)}, read against a peak reading of {num(peak_reading)} and a trough reading of "
                    f"{num(trough_reading)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe inflow({num(inflow)})\n"
                    f"observe backflow({num(backflow)})\n"
                    f"observe peak_reading({num(peak_reading)})\n"
                    f"observe trough_reading({num(trough_reading)})\n"
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
            "ADJ-LADDER rung 138 — pressure ratio from four stated quantities (CONTINUING the OVER-A-DIFFERENCE column). rung-137 put a "
            "product over a difference (a*b)/(c-d); rung-138 puts a SUM over a difference (a+b)/(c-d) — the difference-denominator twin of "
            "the sum-numerator rungs (131 sum over a rate, 37 sum over a sum). From a combined flow (inflow + backflow) divided by a "
            "pressure gap (peak_reading - trough_reading), compute the pressure ratio ((inflow+backflow)/(peak_reading-trough_reading)), "
            "the combined flow (inflow+backflow), or the pressure gap (peak_reading-trough_reading). Each item is a compute_dimensioned "
            "program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a SUM NUMERATOR OVER A "
            "DIFFERENCE (a+b)/(c-d) (add the flows, gap the readings, then divide the flow by the gap — the two-part denominator is ONE "
            "gap, not two divisors). As on rung-137, the divide-across and lost-grouping errors go negative when c>d, so the two in-range "
            "canonical slips are used as distractors. The harness matches the scalar to the printed options. The pressure ratio is a ratio "
            "(how much combined flow rides on each unit of the pressure gap), framed as a RATIO so the dimensionless value stays honest. "
            "Contamination-safe: every figure is built only from the four observed quantities via +, /, and - — no constant leaks, and "
            "neither the combined flow, the pressure gap, nor the pressure ratio ever appears as a literal (each is computed) — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a family "
            "over the same four quantities, so the distractors are exactly the slips students make with a gap denominator: dividing by the "
            "SUM of the readings instead of their gap ((a+b)/(c+d), a total where a gap belongs, a wrong operation) and INVERTING the "
            "ratio ((c-d)/(a+b), the gap over the flow, the reciprocal, a wrong operation). The core confusion tested is that (a+b)/(c-d) "
            "is one sum over one gap, not (a+b)/(c+d) and not (c-d)/(a+b). This rung has a subtraction in the denominator, so the pressure "
            "gap is guarded (peak_reading - trough_reading >= 2) to keep the denominator positive; the five family values are kept "
            "pairwise distinct with all three queried readouts varying across the panel, all asserted strictly positive at build time."
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
