"""Generate rung-140 (yield ratio / a QUOTIENT-numerator over a DIFFERENCE — divide a per-unit rate by a gap) items.json.

Rung 140 **closes the OVER-A-DIFFERENCE column** and, with it, completes the numerator-op x denominator-shape matrix the ladder has been
walking since rung-130. rung-137 put a PRODUCT over a difference, `(a*b)/(c-d)`; rung-138 a SUM, `(a+b)/(c-d)`; rung-139 a DIFFERENCE,
`(a-b)/(c-d)`; rung-140 puts a QUOTIENT over a difference, `(a/b)/(c-d)`. Across rungs 130-140 the numerator op walks quotient / sum /
difference / product over each of the three two-part denominators — a rate `c/d`, a sum `c+d`, and a difference `c-d`.

`(a/b)/(c-d)` is a QUOTIENT `a/b` (a per-unit rate) divided by a DIFFERENCE `c-d` (a gap): dividing a fraction by a fraction, so
`(a/b)/(c-d) = a/(b*(c-d))`. The rate `a/b` binds and stays grouped over the bar, and the run `c-d` is ONE gap the whole rate is divided
by. As on rungs 137-139, a difference denominator sends the divide-across (`x/c - x/d`) and lost-grouping (`x/c - d`) errors NEGATIVE when
`c > d` (the regime a positive gap requires), so those are not the clean in-range confusions. The two canonical divide-by-a-gap slips that
stay in-range are: using the WRONG denominator operation, summing the two marks instead of gapping them (`(a/b)/(c+d)` — a total instead of
a gap), and INVERTING the ratio, dividing the gap by the rate instead of the rate by the gap (`(c-d)/(a/b)` — the reciprocal).

The setup: a `harvest` split over a `dose_split` (a unit yield `harvest / dose_split`), read against a window formed from a `close_mark`
minus an `open_mark` (a window gap `close_mark - open_mark`). The figures are:

  YIELD RATIO   (harvest / dose_split) / (close_mark - open_mark)  [ quotient-numerator OVER a difference: unit yield / window gap ]
  UNIT YIELD    harvest / dose_split                              [ the quotient numerator (divided by the window gap) ]
  WINDOW GAP    close_mark - open_mark                            [ the difference the unit yield is divided by ]

The **yield ratio** is the ladder's first **(a quotient) over (a difference) as a headline** — a ratio (how much unit yield rides on each
unit of the window gap), framed as a *ratio* to keep it dimensionless-clean, the same discipline rungs 100/.../138/139 used for their
ratios, spans, concentrations, densities, indices, and slopes. (The unit yield `a/b` and the window gap `c-d` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-139 shipped their component figures beside the headline. The two
components anchor the "form the unit yield FIRST, gap the marks, then divide the yield by the gap" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the division to form the unit yield, the subtraction to form the window gap, then the division of the unit yield by the window
gap to form the compound figure (so (a/b)/(c-d) evaluates as ((a/b)/(c-d)) = a/(b*(c-d))) — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../138/139. This rung exercises the engine across
a **quotient divided by a difference** — the fact that `(a/b)/(c-d)` is one rate over one gap and NOT `(a/b)/(c+d)` and NOT `(c-d)/(a/b)`
made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../138/139 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `-`, and `+` — **no structural
constants** — so no numeric literal appears in any program, and neither the unit yield, the window gap, nor the yield ratio is ever a
literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`harvest`, `dose_split`,
`close_mark`, `open_mark`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SUMMED     (harvest / dose_split) / (close_mark + open_mark)  divide the unit yield by the SUM of the marks instead of their gap, using
                                                                a total where a gap belongs (the wrong denominator operation), and
  INVERTED   (close_mark - open_mark) / (harvest / dose_split)  divide the window gap BY the unit yield, the ratio upside down (the
                                                                reciprocal of the yield ratio, the wrong direction),

which are exactly the mistakes a student makes with a gap denominator (mis-reading the difference as a total, or inverting the ratio). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung has a SUBTRACTION in the denominator, so it needs a **positivity guard** — the window gap is guarded
(`close_mark - open_mark >= 2`) so the denominator (and the headline) stay positive. The unit yield `harvest / dose_split` is a quotient of
positive quantities, and the tables use `dose_split` values that divide `harvest` evenly so the unit yield is a clean integer. With the
guard and positive quantities, every family member is positive (the summed and inverted distractors are quotients of positive quantities).
Every observed quantity is `>= 2`. Every family member is asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give
distinct yield ratios, distinct unit yields, and distinct window gaps so all three queried readouts vary across the panel; the five family
values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (HARVEST, DOSE_SPLIT, CLOSE_MARK, OPEN_MARK) — a unit yield (harvest / dose_split) divided by a window gap (close_mark - open_mark), giving
# the yield ratio as a quotient over a difference (a/b)/(c-d). This rung has a SUBTRACTION in the denominator, so the window gap is guarded
# (close_mark - open_mark >= 2) to keep the denominator positive; dose_split divides harvest evenly so the unit yield is a clean integer, and
# with positive quantities every figure is positive. The seven tables give distinct unit yields (a/b), distinct window gaps (c-d), and distinct
# yield ratios ((a/b)/(c-d)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (12, 2, 7, 2),     # yield = 6,  gap = 5,  ratio = 1.2
    (24, 3, 6, 3),     # yield = 8,  gap = 3,  ratio = 2.666...
    (20, 2, 8, 4),     # yield = 10, gap = 4,  ratio = 2.5
    (36, 3, 10, 4),    # yield = 12, gap = 6,  ratio = 2.0
    (28, 2, 11, 2),    # yield = 14, gap = 9,  ratio = 1.555...
    (27, 3, 9, 2),     # yield = 9,  gap = 7,  ratio = 1.285...
    (32, 2, 13, 3),    # yield = 16, gap = 10, ratio = 1.6
]

# The option family (5 members), all built from the four observed quantities via /, -, and +. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "yield_ratio",
        "yield ratio (the unit yield divided by the window gap)",
        "(harvest / dose_split) / (close_mark - open_mark)",
    ),
    (
        "unit_yield",
        "the unit yield (the harvest split over the dose split, the numerator that is divided by the window gap)",
        "harvest / dose_split",
    ),
    (
        "window_gap",
        "the window gap (the close mark minus the open mark, the difference the unit yield is divided by)",
        "close_mark - open_mark",
    ),
    (
        "summed",
        "the unit yield divided by the close mark plus the open mark, using the sum of the marks instead of their gap as the divisor (a wrong operation)",
        "(harvest / dose_split) / (close_mark + open_mark)",
    ),
    (
        "inverted",
        "the window gap divided by the unit yield, the ratio upside down instead of the unit yield over the window gap (a wrong operation)",
        "(close_mark - open_mark) / (harvest / dose_split)",
    ),
]
QUERIED = ["yield_ratio", "unit_yield", "window_gap"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(harvest, dose_split, close_mark, open_mark):
    # Operation order mirrors the ADJ programs exactly (the division forms the unit yield, the subtraction forms the window gap, then the
    # unit yield is divided by the window gap to form the compound figure, so (a/b)/(c-d) evaluates as ((a/b)/(c-d)) = a/(b*(c-d))), so the
    # Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    yld = harvest / dose_split
    gap = close_mark - open_mark
    return {
        "yield_ratio": yld / gap,
        "unit_yield": yld,
        "window_gap": gap,
        "summed": yld / (close_mark + open_mark),
        "inverted": gap / yld,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for harvest, dose_split, close_mark, open_mark in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung has a subtraction in the denominator, so the window gap is
        # guarded (close_mark - open_mark >= 2) to keep the denominator positive; dose_split divides harvest evenly (unit yield is a clean
        # integer).
        assert (
            harvest >= 2
            and dose_split >= 2
            and close_mark >= 2
            and open_mark >= 2
        ), (harvest, dose_split, close_mark, open_mark)
        assert close_mark - open_mark >= 2, (close_mark, open_mark)
        assert harvest % dose_split == 0, (harvest, dose_split)
        fv = family_values(harvest, dose_split, close_mark, open_mark)
        for key, v in fv.items():
            assert v > 0, (key, harvest, dose_split, close_mark, open_mark, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    harvest,
                    dose_split,
                    close_mark,
                    open_mark,
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
                harvest,
                dose_split,
                close_mark,
                open_mark,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r140yra-{idx + 1:02d}",
                "qtype": "yield_ratio",
                "stem": (
                    f"A yield study records a harvest of {num(harvest)} split over a dose split of "
                    f"{num(dose_split)}, read against a close mark of {num(close_mark)} and an open mark of "
                    f"{num(open_mark)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe harvest({num(harvest)})\n"
                    f"observe dose_split({num(dose_split)})\n"
                    f"observe close_mark({num(close_mark)})\n"
                    f"observe open_mark({num(open_mark)})\n"
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
            "ADJ-LADDER rung 140 — yield ratio from four stated quantities (CLOSING the OVER-A-DIFFERENCE column and completing the "
            "numerator-op x denominator-shape matrix). rung-137 put a product over a difference (a*b)/(c-d); rung-138 a sum (a+b)/(c-d); "
            "rung-139 a difference (a-b)/(c-d); rung-140 puts a QUOTIENT over a difference (a/b)/(c-d) — across rungs 130-140 the numerator "
            "op walks quotient/sum/difference/product over each two-part denominator (rate c/d, sum c+d, difference c-d). From a unit yield "
            "(harvest / dose_split) divided by a window gap (close_mark - open_mark), compute the yield ratio "
            "((harvest/dose_split)/(close_mark-open_mark)), the unit yield (harvest/dose_split), or the window gap "
            "(close_mark-open_mark). Each item is a compute_dimensioned program (observe the four quantities, let answer = formula); the "
            "ADJ engine carries the arithmetic — a QUOTIENT NUMERATOR OVER A DIFFERENCE (a/b)/(c-d) (form the unit yield, gap the marks, "
            "then divide the yield by the gap, so (a/b)/(c-d) = a/(b*(c-d)) — the two-part denominator is ONE gap, not two divisors). As on "
            "rungs 137-139, the divide-across and lost-grouping errors go negative when c>d, so the two in-range canonical slips are used "
            "as distractors. The harness matches the scalar to the printed options. The yield ratio is a ratio (how much unit yield rides "
            "on each unit of the window gap), framed as a RATIO so the dimensionless value stays honest. Contamination-safe: every figure "
            "is built only from the four observed quantities via /, -, and + — no constant leaks, and neither the unit yield, the window "
            "gap, nor the yield ratio ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the same four quantities, so the "
            "distractors are exactly the slips students make with a gap denominator: dividing by the SUM of the marks instead of their gap "
            "((a/b)/(c+d), a total where a gap belongs, a wrong operation) and INVERTING the ratio ((c-d)/(a/b), the gap over the yield, "
            "the reciprocal, a wrong operation). The core confusion tested is that (a/b)/(c-d) is one rate over one gap, not (a/b)/(c+d) "
            "and not (c-d)/(a/b). This rung has a subtraction in the denominator, so the window gap is guarded (close_mark - open_mark >= "
            "2) to keep the denominator positive, and dose_split divides harvest evenly so the unit yield is a clean integer; the five "
            "family values are kept pairwise distinct with all three queried readouts varying across the panel, all asserted strictly "
            "positive at build time."
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
