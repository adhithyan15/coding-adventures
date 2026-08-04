"""Generate rung-139 (slope ratio / a DIFFERENCE-numerator over a DIFFERENCE — a rise over a run) items.json.

Rung 139 continues the **OVER-A-DIFFERENCE** column. rung-137 put a PRODUCT over a difference, `(a*b)/(c-d)`; rung-138 a SUM,
`(a+b)/(c-d)`; rung-139 puts a DIFFERENCE over a difference, `(a-b)/(c-d)`. This is the classic **slope** shape — a rise (a difference)
divided by a run (a difference) — and it is the first ladder figure with a difference in BOTH the numerator and the denominator.

`(a-b)/(c-d)` is a DIFFERENCE `a-b` (a rise) divided by a DIFFERENCE `c-d` (a run/gap). The rise `a-b` binds and stays grouped over the bar,
and the run `c-d` is ONE gap the whole rise is divided by. As on rungs 137-138, a difference denominator sends the divide-across
(`x/c - x/d`) and lost-grouping (`x/c - d`) errors NEGATIVE when `c > d` (the regime a positive run requires), so those are not the clean
in-range confusions. The two canonical divide-by-a-gap slips that stay in-range are: using the WRONG denominator operation, summing the two
run marks instead of gapping them (`(a-b)/(c+d)` — a total instead of a gap), and INVERTING the ratio, dividing the run by the rise instead
of the rise by the run (`(c-d)/(a-b)` — the reciprocal, rise-over-run flipped to run-over-rise).

The setup: a rise formed from a `rise_top` minus a `rise_base` (a net rise `rise_top - rise_base`), read against a run formed from a
`run_right` minus a `run_left` (a net run `run_right - run_left`). The figures are:

  SLOPE RATIO   (rise_top - rise_base) / (run_right - run_left)  [ difference-numerator OVER a difference: net rise / net run ]
  NET RISE      rise_top - rise_base                            [ the difference numerator (divided by the net run) ]
  NET RUN       run_right - run_left                            [ the difference the net rise is divided by ]

The **slope ratio** is the ladder's first **(a difference) over (a difference) as a headline** — a slope (how much net rise rides on each
unit of the net run, rise over run), framed as a *ratio* to keep it dimensionless-clean, the same discipline rungs 100/.../137/138 used for
their ratios, spans, concentrations, densities, indices. (The net rise `a-b` and the net run `c-d` ride alongside as component readouts, so
the panel teaches the whole calculation — exactly as rungs 47-138 shipped their component figures beside the headline. The two components
anchor the "subtract the rise FIRST, subtract the run, then divide the rise by the run" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the subtraction to form the net rise, the subtraction to form the net run, then the division of the net rise by the net run to
form the compound figure (so (a-b)/(c-d) evaluates as ((a-b)/(c-d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../137/138. This rung exercises the engine across a
**difference divided by a difference** — the fact that `(a-b)/(c-d)` is one rise over one run and NOT `(a-b)/(c+d)` and NOT `(c-d)/(a-b)`
made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../137/138 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `/`, and `+` — **no structural
constants** — so no numeric literal appears in any program, and neither the net rise, the net run, nor the slope ratio is ever a literal
(each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`rise_top`, `rise_base`,
`run_right`, `run_left`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SUMMED     (rise_top - rise_base) / (run_right + run_left)  divide the net rise by the SUM of the run marks instead of their gap, using
                                                                a total where a gap belongs (the wrong denominator operation), and
  INVERTED   (run_right - run_left) / (rise_top - rise_base)  divide the net run BY the net rise, run over rise instead of rise over run
                                                                (the reciprocal of the slope, the wrong direction),

which are exactly the mistakes a student makes with a gap denominator (mis-reading the run difference as a total, or inverting rise/run).
Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung has a SUBTRACTION in BOTH the numerator and the denominator, so it needs **two positivity guards** —
the net rise is guarded (`rise_top - rise_base >= 2`) and the net run is guarded (`run_right - run_left >= 2`), so both the numerator and
the denominator (and the headline) stay positive. With those guards and positive quantities, every family member is positive (the summed
and inverted distractors are quotients of positive quantities). Every observed quantity is `>= 2`. Every family member is asserted `> 0` at
build time as a belt-and-suspenders check. The seven tables give distinct slope ratios, distinct net rises, and distinct net runs so all
three queried readouts vary across the panel; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (RISE_TOP, RISE_BASE, RUN_RIGHT, RUN_LEFT) — a net rise (rise_top - rise_base) divided by a net run (run_right - run_left), giving the slope
# ratio as a difference over a difference (a-b)/(c-d). This rung has a SUBTRACTION in BOTH the numerator and the denominator, so it needs two
# positivity guards: rise_top - rise_base >= 2 and run_right - run_left >= 2, so both stay positive. The seven tables give distinct net rises
# (a-b), distinct net runs (c-d), and distinct slope ratios ((a-b)/(c-d)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (8, 2, 7, 2),      # rise = 6,  run = 5,  slope = 1.2
    (11, 3, 6, 3),     # rise = 8,  run = 3,  slope = 2.666...
    (14, 4, 8, 4),     # rise = 10, run = 4,  slope = 2.5
    (16, 4, 10, 4),    # rise = 12, run = 6,  slope = 2.0
    (19, 5, 11, 2),    # rise = 14, run = 9,  slope = 1.555...
    (12, 3, 9, 2),     # rise = 9,  run = 7,  slope = 1.285...
    (22, 6, 13, 3),    # rise = 16, run = 10, slope = 1.6
]

# The option family (5 members), all built from the four observed quantities via -, /, and +. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "slope_ratio",
        "slope ratio (the net rise divided by the net run)",
        "(rise_top - rise_base) / (run_right - run_left)",
    ),
    (
        "net_rise",
        "the net rise (the rise top minus the rise base, the numerator that is divided by the net run)",
        "rise_top - rise_base",
    ),
    (
        "net_run",
        "the net run (the run right minus the run left, the difference the net rise is divided by)",
        "run_right - run_left",
    ),
    (
        "summed",
        "the net rise divided by the run right plus the run left, using the sum of the run marks instead of their gap as the divisor (a wrong operation)",
        "(rise_top - rise_base) / (run_right + run_left)",
    ),
    (
        "inverted",
        "the net run divided by the net rise, run over rise instead of rise over run (a wrong operation)",
        "(run_right - run_left) / (rise_top - rise_base)",
    ),
]
QUERIED = ["slope_ratio", "net_rise", "net_run"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(rise_top, rise_base, run_right, run_left):
    # Operation order mirrors the ADJ programs exactly (the subtraction forms the net rise, the subtraction forms the net run, then the net
    # rise is divided by the net run to form the compound figure, so (a-b)/(c-d) evaluates as ((a-b)/(c-d))), so the Python option value and
    # the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    rise = rise_top - rise_base
    run = run_right - run_left
    return {
        "slope_ratio": rise / run,
        "net_rise": rise,
        "net_run": run,
        "summed": rise / (run_right + run_left),
        "inverted": run / rise,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for rise_top, rise_base, run_right, run_left in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung has a subtraction in BOTH the numerator and the denominator, so
        # both differences are guarded (rise_top - rise_base >= 2 and run_right - run_left >= 2) to keep them positive.
        assert (
            rise_top >= 2
            and rise_base >= 2
            and run_right >= 2
            and run_left >= 2
        ), (rise_top, rise_base, run_right, run_left)
        assert rise_top - rise_base >= 2, (rise_top, rise_base)
        assert run_right - run_left >= 2, (run_right, run_left)
        fv = family_values(rise_top, rise_base, run_right, run_left)
        for key, v in fv.items():
            assert v > 0, (key, rise_top, rise_base, run_right, run_left, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    rise_top,
                    rise_base,
                    run_right,
                    run_left,
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
                rise_top,
                rise_base,
                run_right,
                run_left,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r139sra-{idx + 1:02d}",
                "qtype": "slope_ratio",
                "stem": (
                    f"A slope study records a rise from a rise base of {num(rise_base)} up to a rise top of "
                    f"{num(rise_top)}, over a run from a run left of {num(run_left)} to a run right of "
                    f"{num(run_right)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe rise_top({num(rise_top)})\n"
                    f"observe rise_base({num(rise_base)})\n"
                    f"observe run_right({num(run_right)})\n"
                    f"observe run_left({num(run_left)})\n"
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
            "ADJ-LADDER rung 139 — slope ratio from four stated quantities (CONTINUING the OVER-A-DIFFERENCE column). rung-137 put a "
            "product over a difference (a*b)/(c-d); rung-138 a sum (a+b)/(c-d); rung-139 puts a DIFFERENCE over a difference (a-b)/(c-d) — "
            "the classic slope shape, a rise over a run, the first ladder figure with a difference in BOTH the numerator and the "
            "denominator. From a net rise (rise_top - rise_base) divided by a net run (run_right - run_left), compute the slope ratio "
            "((rise_top-rise_base)/(run_right-run_left)), the net rise (rise_top-rise_base), or the net run (run_right-run_left). Each item "
            "is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — "
            "a DIFFERENCE NUMERATOR OVER A DIFFERENCE (a-b)/(c-d) (subtract the rise, subtract the run, then divide the rise by the run — "
            "the two-part denominator is ONE run, not two divisors). As on rungs 137-138, the divide-across and lost-grouping errors go "
            "negative when c>d, so the two in-range canonical slips are used as distractors. The harness matches the scalar to the printed "
            "options. The slope ratio is a slope (how much net rise rides on each unit of the net run, rise over run), framed as a RATIO so "
            "the dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed quantities via -, "
            "/, and + — no constant leaks, and neither the net rise, the net run, nor the slope ratio ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same four quantities, so the distractors are exactly the slips students make with a gap "
            "denominator: dividing by the SUM of the run marks instead of their gap ((a-b)/(c+d), a total where a gap belongs, a wrong "
            "operation) and INVERTING the ratio ((c-d)/(a-b), run over rise, the reciprocal, a wrong operation). The core confusion tested "
            "is that (a-b)/(c-d) is one rise over one run, not (a-b)/(c+d) and not (c-d)/(a-b). This rung has a subtraction in BOTH the "
            "numerator and the denominator, so both differences are guarded (rise_top - rise_base >= 2 and run_right - run_left >= 2) to "
            "keep them positive; the five family values are kept pairwise distinct with all three queried readouts varying across the "
            "panel, all asserted strictly positive at build time."
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
