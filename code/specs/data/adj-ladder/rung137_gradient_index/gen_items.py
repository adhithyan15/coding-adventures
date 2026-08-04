"""Generate rung-137 (gradient index / a PRODUCT-numerator over a DIFFERENCE — divide a total by a gap) items.json.

Rung 137 opens the **gradient** panel and **opens the third denominator family: OVER A DIFFERENCE**. The ladder has now filled the
two-part denominator over a RATE (rung-130 quotient, 131 sum, 133 difference, 134 product over `c/d`) and over a SUM (rung-132 quotient,
135 product, 136 difference over `c+d`); rung-137 begins the OVER-A-DIFFERENCE column with a PRODUCT numerator, `(a*b)/(c-d)`.

This is genuinely new. `(a*b)/(c-d)` is a PRODUCT `a*b` divided by a DIFFERENCE `c-d` (a gap). The product `a*b` binds and stays grouped
over the bar (grouping), and the two-part denominator `c-d` is ONE gap the whole numerator is divided by. A difference denominator has its
own two canonical slips, and they are DIFFERENT from the sum/rate slips: the divide-across (`x/c - x/d`) and lost-grouping (`x/c - d`)
errors both go NEGATIVE when `c > d` (which is exactly the regime a positive gap requires), so they are not the clean confusions here.
The two canonical divide-by-a-gap slips that a student actually makes and that stay in-range are: using the WRONG denominator operation,
summing the two marks instead of gapping them (`(a*b)/(c+d)` — a total instead of a gap), and INVERTING the ratio, dividing the gap by the
product instead of the product by the gap (`(c-d)/(a*b)` — the reciprocal, the ratio upside down).

The setup: a `pulse_count` of beats each of `pulse_size` (a total pulse `pulse_count * pulse_size`), read against a band formed from a
`high_mark` minus a `low_mark` (a band gap `high_mark - low_mark`). The figures are:

  GRADIENT INDEX  (pulse_count * pulse_size) / (high_mark - low_mark)  [ product-numerator OVER a difference: total pulse / band gap ]
  TOTAL PULSE     pulse_count * pulse_size                            [ the product numerator (divided by the band gap) ]
  BAND GAP        high_mark - low_mark                                [ the difference the total pulse is divided by ]

The **gradient index** is the ladder's first **(a product) over (a difference) as a headline** — an index (how much total pulse rides on
each unit of the band gap), framed as an *index* to keep it dimensionless-clean, the same discipline rungs 100/.../135/136 used for their
ratios, spans, concentrations, and densities. (The total pulse `a*b` and the band gap `c-d` ride alongside as component readouts, so the
panel teaches the whole calculation — exactly as rungs 47-136 shipped their component figures beside the headline. The two components anchor
the "multiply out the pulse FIRST, gap the marks, then divide the pulse by the gap" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication to form the total pulse, the subtraction to form the band gap, then the division of the total pulse by the
band gap to form the compound figure (so (a*b)/(c-d) evaluates as ((a*b)/(c-d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../135/136. This rung exercises the engine across a
**product divided by a difference** — the fact that `(a*b)/(c-d)` is one product over one gap and NOT `(a*b)/(c+d)` and NOT `(c-d)/(a*b)`
made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../135/136 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `/`, `-`, and `+` — **no
structural constants** — so no numeric literal appears in any program, and neither the total pulse, the band gap, nor the gradient index is
ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`pulse_count`,
`pulse_size`, `high_mark`, `low_mark`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SUMMED     (pulse_count * pulse_size) / (high_mark + low_mark)  divide the total pulse by the SUM of the marks instead of their gap,
                                                                using a total where a gap belongs (the wrong denominator operation), and
  INVERTED   (high_mark - low_mark) / (pulse_count * pulse_size)  divide the band gap BY the total pulse, the ratio upside down (the
                                                                reciprocal of the gradient index, the wrong direction),

which are exactly the mistakes a student makes with a gap denominator (mis-reading the difference as a total, or inverting the ratio). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung has a SUBTRACTION in the denominator, so unlike the pure `* / +` rungs it needs a **positivity
guard** — the band gap is guarded so the denominator (and the headline) stay positive: `high_mark - low_mark >= 2`. With that guard and
positive quantities, every family member is positive (the summed and inverted distractors are quotients of positive quantities). Every
observed quantity is `>= 2`. Every family member is asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give
distinct gradient indices, distinct total pulses, and distinct band gaps so all three queried readouts vary across the panel; the five
family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PULSE_COUNT, PULSE_SIZE, HIGH_MARK, LOW_MARK) — a total pulse (pulse_count * pulse_size) divided by a band gap (high_mark - low_mark),
# giving the gradient index as a product over a difference (a*b)/(c-d). This rung has a SUBTRACTION in the denominator, so the band gap is
# guarded (high_mark - low_mark >= 2) to keep the denominator positive; with positive quantities every figure is positive. The seven tables
# give distinct total pulses (a*b), distinct band gaps (c-d), and distinct gradient indices ((a*b)/(c-d)); the five family values are asserted
# pairwise-distinct below.
TABLES = [
    (2, 3, 7, 2),      # pulse = 6,  gap = 5,  index = 1.2
    (2, 4, 6, 3),      # pulse = 8,  gap = 3,  index = 2.666...
    (2, 5, 8, 4),      # pulse = 10, gap = 4,  index = 2.5
    (3, 4, 10, 4),     # pulse = 12, gap = 6,  index = 2.0
    (2, 7, 11, 2),     # pulse = 14, gap = 9,  index = 1.555...
    (3, 3, 9, 2),      # pulse = 9,  gap = 7,  index = 1.285...
    (2, 8, 13, 3),     # pulse = 16, gap = 10, index = 1.6
]

# The option family (5 members), all built from the four observed quantities via *, /, -, and +. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "gradient_index",
        "gradient index (the total pulse divided by the band gap)",
        "(pulse_count * pulse_size) / (high_mark - low_mark)",
    ),
    (
        "total_pulse",
        "the total pulse (the pulse count times the pulse size, the numerator that is divided by the band gap)",
        "pulse_count * pulse_size",
    ),
    (
        "band_gap",
        "the band gap (the high mark minus the low mark, the difference the total pulse is divided by)",
        "high_mark - low_mark",
    ),
    (
        "summed",
        "the total pulse divided by the high mark plus the low mark, using the sum of the marks instead of their gap as the divisor (a wrong operation)",
        "(pulse_count * pulse_size) / (high_mark + low_mark)",
    ),
    (
        "inverted",
        "the band gap divided by the total pulse, the ratio upside down instead of the total pulse over the band gap (a wrong operation)",
        "(high_mark - low_mark) / (pulse_count * pulse_size)",
    ),
]
QUERIED = ["gradient_index", "total_pulse", "band_gap"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(pulse_count, pulse_size, high_mark, low_mark):
    # Operation order mirrors the ADJ programs exactly (the multiplication forms the total pulse, the subtraction forms the band gap, then
    # the total pulse is divided by the band gap to form the compound figure, so (a*b)/(c-d) evaluates as ((a*b)/(c-d))), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    pulse = pulse_count * pulse_size
    gap = high_mark - low_mark
    return {
        "gradient_index": pulse / gap,
        "total_pulse": pulse,
        "band_gap": gap,
        "summed": pulse / (high_mark + low_mark),
        "inverted": gap / pulse,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for pulse_count, pulse_size, high_mark, low_mark in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung has a subtraction in the denominator, so the band gap is
        # guarded (high_mark - low_mark >= 2) to keep the denominator positive; with positive quantities every figure is positive.
        assert (
            pulse_count >= 2
            and pulse_size >= 2
            and high_mark >= 2
            and low_mark >= 2
        ), (pulse_count, pulse_size, high_mark, low_mark)
        assert high_mark - low_mark >= 2, (high_mark, low_mark)
        fv = family_values(pulse_count, pulse_size, high_mark, low_mark)
        for key, v in fv.items():
            assert v > 0, (key, pulse_count, pulse_size, high_mark, low_mark, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    pulse_count,
                    pulse_size,
                    high_mark,
                    low_mark,
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
                pulse_count,
                pulse_size,
                high_mark,
                low_mark,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r137gia-{idx + 1:02d}",
                "qtype": "gradient_index",
                "stem": (
                    f"A gradient study records a pulse count of {num(pulse_count)} beats each of pulse size "
                    f"{num(pulse_size)}, read against a high mark of {num(high_mark)} and a low mark of "
                    f"{num(low_mark)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe pulse_count({num(pulse_count)})\n"
                    f"observe pulse_size({num(pulse_size)})\n"
                    f"observe high_mark({num(high_mark)})\n"
                    f"observe low_mark({num(low_mark)})\n"
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
            "ADJ-LADDER rung 137 — gradient index from four stated quantities (a NEW panel: gradient, OPENING the third denominator "
            "family, OVER A DIFFERENCE). The ladder filled the two-part denominator over a rate (130 quotient, 131 sum, 133 difference, "
            "134 product over (c/d)) and over a sum (132 quotient, 135 product, 136 difference over (c+d)); rung-137 begins the "
            "OVER-A-DIFFERENCE column with a PRODUCT numerator (a*b)/(c-d). From a total pulse (pulse_count * pulse_size) divided by a "
            "band gap (high_mark - low_mark), compute the gradient index ((pulse_count*pulse_size)/(high_mark-low_mark)), the total pulse "
            "(pulse_count*pulse_size), or the band gap (high_mark-low_mark). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a PRODUCT NUMERATOR OVER A "
            "DIFFERENCE (a*b)/(c-d) (multiply out the pulse, gap the marks, then divide the pulse by the gap — the two-part denominator is "
            "ONE gap, not two divisors). A difference denominator has its own slips: the divide-across and lost-grouping errors both go "
            "negative when c>d (the regime a positive gap requires), so the two in-range canonical slips are used instead as distractors. "
            "The harness matches the scalar to the printed options. The gradient index is an index (how much total pulse rides on each "
            "unit of the band gap), framed as an INDEX so the dimensionless value stays honest. Contamination-safe: every figure is built "
            "only from the four observed quantities via *, /, -, and + — no constant leaks, and neither the total pulse, the band gap, nor "
            "the gradient index ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so "
            "no numeral hides inside a variable name. The five options are a family over the same four quantities, so the distractors are "
            "exactly the slips students make with a gap denominator: dividing by the SUM of the marks instead of their gap ((a*b)/(c+d), a "
            "total where a gap belongs, a wrong operation) and INVERTING the ratio ((c-d)/(a*b), the gap over the pulse, the reciprocal, a "
            "wrong operation). The core confusion tested is that (a*b)/(c-d) is one product over one gap, not (a*b)/(c+d) and not "
            "(c-d)/(a*b). This rung has a subtraction in the denominator, so the band gap is guarded (high_mark - low_mark >= 2) to keep "
            "the denominator positive; the five family values are kept pairwise distinct with all three queried readouts varying across "
            "the panel, all asserted strictly positive at build time."
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
