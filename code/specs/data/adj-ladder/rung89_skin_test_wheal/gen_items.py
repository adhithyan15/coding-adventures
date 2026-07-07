"""Generate rung-89 (allergy skin-test-wheal reactivity index) items.json for the ADJ-LADDER.

Rung 89 opens the **allergy / skin-test-wheal** panel on the quantitative band — the arithmetic of a skin-test
reactivity index. A `wheal_diameter` and a `flare_rim` are ADDED into a total span, that span is multiplied by a net
reaction — a `peak_reaction` minus a `trough_reaction` — and the result is the reactivity index. A **sum times a
difference** introduces a genuinely NEW arithmetic shape on the ladder: a **product of two binomials** — `(a+b)*(c-d)`,
i.e. `((a+b) * (c-d))`.

This is genuinely new: no shipped shape ever multiplied a SUM by a DIFFERENCE — every prior shape that used a
parenthesised binomial multiplied it by a single observed factor or divided by one (rung-67 `(a-b)*c/d`, rung-68
`(a+b)*c/d`, rung-75 `(a-b)/c*d`, rung-76 `(a+b)/c*d`, rung-81 `(a+b)/c-d`, rung-82 `(a-b)/c+d`, rung-84 `(a+b)*c-d`) —
never a binomial times ANOTHER binomial. `(a+b)*(c-d)` is the ladder's first **product of two binomials**. The operator
order matters: `(a+b)*(c-d)` is `((a+b)*(c-d))` (each parenthesised group evaluates first, then the two groups
multiply), NOT `(a+b)*c-d` (subtracting `d` OUTSIDE the product instead of inside the second factor) and NOT
`(a-b)*(c+d)` (swapping which pair is summed and which is differenced) — the two distractors exploit exactly those
confusions.

The setup: a `wheal_diameter`, a `flare_rim`, a `peak_reaction`, and a `trough_reaction`. The reactivity index is:

  REACTIVITY INDEX  (wheal_diameter + flare_rim) * (peak_reaction - trough_reaction)  [ sum times a difference ]
  SPAN SUM          wheal_diameter + flare_rim                                         [ the total span, before scaling ]
  NET REACTION      peak_reaction - trough_reaction                                    [ the net reaction, before scaling ]

The **reactivity index** is what makes this rung distinctive — it is the ladder's first **product of two binomials**.
(The span sum `a+b` and the net reaction `c-d` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 47-88 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the addition of the wheal diameter and the flare rim into a span, the subtraction of the trough
reaction from the peak reaction into a net reaction, then the multiplication of the two (each binomial group before the
multiply) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine
change, exactly as rungs 8/16/.../87/88. This rung exercises the engine across **a product of two binomials** — the fact
that `(a+b)*(c-d)` is `((a+b)*(c-d))` and NOT `(a+b)*c-d` and NOT `(a-b)*(c+d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-`, and `*`
— **no structural constants** — so no numeric literal appears in any program, and neither the span sum, the net
reaction, nor any reactivity figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`wheal_diameter`, `flare_rim`, `peak_reaction`, `trough_reaction`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (wheal_diameter + flare_rim) * peak_reaction - trough_reaction    subtract the trough reaction OUTSIDE the
                                                                               product instead of inside the second
                                                                               factor (the classic `(a+b)*(c-d)` vs
                                                                               `(a+b)*c-d` error), and
  SWAPPED    (wheal_diameter - flare_rim) * (peak_reaction + trough_reaction)  swap which pair is summed and which is
                                                                               differenced — sum the reactions and take
                                                                               the difference of the spans (`(a-b)*(c+d)`
                                                                               instead of `(a+b)*(c-d)`),

which are exactly the mistakes a student makes (dropping the parenthesis on the difference, or mispairing which group
gets the sum and which gets the difference). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness and positivity: the tables keep the guards — `wheal_diameter > flare_rim >= 2` (so the span difference in
the swapped slip stays positive) and `peak_reaction > trough_reaction >= 2` (so the net reaction is positive) — so every
family member, including the headline reactivity index `(a+b)*(c-d)`, is strictly positive (a positive span times a
positive net reaction, and likewise for the slips); the five family values are pairwise distinct with a comfortable
margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (WHEAL_DIAMETER, FLARE_RIM, PEAK_REACTION, TROUGH_REACTION) — a wheal diameter and a flare rim to add into a total
# span, and a peak reaction and a trough reaction whose difference is the net reaction, all plain positive numbers >= 2.
# The tables satisfy the guards: wheal_diameter > flare_rim >= 2 (so the swapped slip's (a-b) stays positive) and
# peak_reaction > trough_reaction >= 2 (so the net reaction (c-d) is positive). The five family values are asserted
# pairwise-distinct below.
TABLES = [
    (3, 2, 5, 2),
    (4, 2, 5, 2),
    (4, 3, 6, 3),
    (5, 2, 4, 2),
    (5, 3, 4, 2),
    (5, 4, 4, 2),
    (6, 2, 4, 2),
]

# The option family (5 members), all built from the four observed quantities via +, -, and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "reactivity_index",
        "reactivity index (the total span times the net reaction)",
        "(wheal_diameter + flare_rim) * (peak_reaction - trough_reaction)",
    ),
    (
        "span_sum",
        "the span sum (the wheal diameter plus the flare rim, before scaling by the net reaction)",
        "wheal_diameter + flare_rim",
    ),
    (
        "net_reaction",
        "the net reaction (the peak reaction minus the trough reaction, before scaling by the span)",
        "peak_reaction - trough_reaction",
    ),
    (
        "crossed",
        "the total span times the peak reaction, MINUS the trough reaction, with the trough subtracted outside the product instead of inside the second factor (a wrong grouping)",
        "(wheal_diameter + flare_rim) * peak_reaction - trough_reaction",
    ),
    (
        "swapped",
        "the wheal diameter MINUS the flare rim, times the peak reaction PLUS the trough reaction, swapping which pair is summed and which is differenced (a wrong pairing)",
        "(wheal_diameter - flare_rim) * (peak_reaction + trough_reaction)",
    ),
]
QUERIED = ["reactivity_index", "span_sum", "net_reaction"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(wheal_diameter, flare_rim, peak_reaction, trough_reaction):
    # Operation order mirrors the ADJ programs exactly (each parenthesised binomial evaluates first, then the two groups
    # multiply, so (a+b)*(c-d) evaluates as ((a+b)*(c-d))), so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "reactivity_index": (wheal_diameter + flare_rim) * (peak_reaction - trough_reaction),
        "span_sum": wheal_diameter + flare_rim,
        "net_reaction": peak_reaction - trough_reaction,
        "crossed": (wheal_diameter + flare_rim) * peak_reaction - trough_reaction,
        "swapped": (wheal_diameter - flare_rim) * (peak_reaction + trough_reaction),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for wheal_diameter, flare_rim, peak_reaction, trough_reaction in TABLES:
        assert (
            wheal_diameter > 0
            and flare_rim > 0
            and peak_reaction > 0
            and trough_reaction > 0
        ), (wheal_diameter, flare_rim, peak_reaction, trough_reaction)
        fv = family_values(wheal_diameter, flare_rim, peak_reaction, trough_reaction)
        # The tables satisfy the guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, wheal_diameter, flare_rim, peak_reaction, trough_reaction, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    wheal_diameter,
                    flare_rim,
                    peak_reaction,
                    trough_reaction,
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
                wheal_diameter,
                flare_rim,
                peak_reaction,
                trough_reaction,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r89stw-{idx + 1:02d}",
                "qtype": "skin_test_wheal_index",
                "stem": (
                    f"A skin test reads a wheal diameter of {num(wheal_diameter)} plus a flare rim of "
                    f"{num(flare_rim)}, all scaled by a peak reaction of {num(peak_reaction)} minus a trough reaction "
                    f"of {num(trough_reaction)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe wheal_diameter({num(wheal_diameter)})\n"
                    f"observe flare_rim({num(flare_rim)})\n"
                    f"observe peak_reaction({num(peak_reaction)})\n"
                    f"observe trough_reaction({num(trough_reaction)})\n"
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
            "ADJ-LADDER rung 89 — allergy skin-test-wheal reactivity index from four stated quantities (a NEW panel: "
            "allergy / skin-test-wheal). From a wheal diameter and a flare rim to add into a total span and a peak "
            "reaction and a trough reaction whose difference is the net reaction, compute the reactivity index "
            "((wheal_diameter+flare_rim)*(peak_reaction-trough_reaction)), the span sum (wheal_diameter+flare_rim), or "
            "the net reaction (peak_reaction-trough_reaction). Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, PRODUCT OF TWO "
            "BINOMIALS (a+b)*(c-d) (add a and b, subtract d from c, multiply the two groups, so (a+b)*(c-d) = "
            "((a+b)*(c-d)); no prior shape multiplied a SUM by a DIFFERENCE — every earlier binomial shape, e.g. rung-67 "
            "(a-b)*c/d, rung-68 (a+b)*c/d, and rung-84 (a+b)*c-d, multiplied or divided a binomial by a single observed "
            "factor, never by another binomial) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via +, -, and * — no "
            "constant leaks, and neither the span sum, the net reaction, nor any reactivity figure ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same four quantities, so the distractors are "
            "exactly the slips students make: subtracting the trough reaction outside the product instead of inside the "
            "second factor ((a+b)*c-d, a wrong grouping) and swapping which pair is summed and which is differenced "
            "((a-b)*(c+d), a wrong pairing). The core confusion tested is that (a+b)*(c-d) is ((a+b)*(c-d)), not "
            "(a+b)*c-d and not (a-b)*(c+d)."
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
