"""Generate rung-82 (nephrology GFR-estimation index) items.json for the ADJ-LADDER.

Rung 82 opens the **nephrology / GFR-estimation** panel on the quantitative band — the arithmetic of an estimated
glomerular filtration rate. A `filtration_rate` has a `reabsorption_rate` taken back from it (a DIFFERENCE), the net is
spread over a `transit_time` — a **grouped-difference quotient** `(filtration_rate - reabsorption_rate) / transit_time`
(the net clearance) — and then a steady `secretion_offset` is ADDED to that rate. A grouped difference over a divisor,
plus a fourth term, introduces a genuinely NEW arithmetic shape on the ladder: a **grouped-difference quotient PLUS a
term** — `(a-b)/c+d`, i.e. `((a-b)/c) + d`.

This is the deliberate MIRROR of rung-81's `(a+b)/c-d` (a grouped-sum quotient minus a term): there the group was a SUM
and the trailing op a MINUS; here the group is a DIFFERENCE and the trailing op is a PLUS. The grouping matters at TWO
points: the difference `filtration_rate - reabsorption_rate` is formed FIRST and divided as a whole (`(a-b)/c`, not
`a - b/c`), and the `+d` is applied to the quotient AFTER the division (`((a-b)/c) + d`, not `(a-b)/(c+d)` and not with
the plus folded inside the group). The operation order is `((a-b)/c) + d` by precedence (the parenthesised difference
divides, then the addition), NOT `(a-b)/(c+d)` (adding `d` INSIDE the denominator) and NOT `(a+b)/c + d` (a PLUS inside
the group instead of the minus) — the two distractors exploit exactly those confusions.

The setup: a `filtration_rate`, a `reabsorption_rate`, a `transit_time`, and a `secretion_offset`. The estimated GFR is:

  ESTIMATED GFR   (filtration_rate - reabsorption_rate) / transit_time + secretion_offset   [ grouped-difference quotient plus a term ]
  NET CLEARANCE   (filtration_rate - reabsorption_rate) / transit_time                      [ the quotient term, before +d ]
  NET FILTERED     filtration_rate - reabsorption_rate                                       [ the differenced numerator ]

The **estimated GFR** is what makes this rung distinctive — it is the ladder's first **grouped-difference quotient PLUS
a term**. (The net clearance `(a-b)/c` and the net filtered `a-b` ride alongside as component readouts, so the panel
teaches the whole calculation — exactly as rungs 47-81 shipped their component sums/products/differences/ratios beside
the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the reabsorption rate from the filtration rate, the division of that
difference by the transit time, and the addition of the secretion offset to the quotient (form the grouped difference,
divide, then add) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine
change, exactly as rungs 8/16/.../80/81. This rung exercises the engine across **a grouped-difference quotient plus a
term** — the fact that `(a-b)/c+d` is `((a-b)/c) + d` and NOT `(a-b)/(c+d)` and NOT `(a+b)/c + d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `/`, and `+`
— **no structural constants** — so no numeric literal appears in any program, and neither the net clearance, the net
filtered, nor any GFR figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`filtration_rate`, `reabsorption_rate`, `transit_time`, `secretion_offset`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (filtration_rate - reabsorption_rate) / (transit_time + secretion_offset)   add the secretion offset INSIDE
                                                                                         the denominator instead of after
                                                                                         the quotient (the classic
                                                                                         `(a-b)/c+d` vs `(a-b)/(c+d)`
                                                                                         error), and
  SWAPPED    (filtration_rate + reabsorption_rate) / transit_time + secretion_offset      ADD the reabsorption rate
                                                                                         inside the group instead of
                                                                                         subtracting it (`(a+b)/c+d`
                                                                                         instead of `(a-b)/c+d`, a wrong
                                                                                         grouping),

which are exactly the mistakes a student makes (folding the trailing term into the denominator, or flipping the
difference into a sum). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always
appear as options.

Distinctness and positivity: the tables keep `filtration_rate > reabsorption_rate` (so the differenced numerator, and
hence every family member, is strictly positive — `+d` and `(a-b)/c` are positive whenever `a>b`) and `transit_time >= 2`
(so the net clearance never coincides with the net filtered); the five family values are pairwise distinct with a
comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (FILTRATION_RATE, REABSORPTION_RATE, TRANSIT_TIME, SECRETION_OFFSET) — a filtration rate, a reabsorption rate to
# subtract from it, a transit time to divide the net by, and a secretion offset to add to the quotient, all plain
# positive numbers. The tables keep filtration_rate > reabsorption_rate (so the difference and every family member is
# positive) and transit_time >= 2. The five family values are asserted pairwise-distinct below.
TABLES = [
    (30, 6, 4, 2),
    (40, 10, 5, 3),
    (48, 12, 6, 2),
    (24, 4, 5, 4),
    (54, 6, 8, 3),
    (36, 12, 4, 5),
    (60, 15, 9, 2),
]

# The option family (5 members), all built from the four observed quantities via -, /, and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "estimated_gfr",
        "estimated GFR (the net clearance plus the secretion offset)",
        "(filtration_rate - reabsorption_rate) / transit_time + secretion_offset",
    ),
    (
        "net_clearance",
        "the net clearance (net filtered load over the transit time)",
        "(filtration_rate - reabsorption_rate) / transit_time",
    ),
    (
        "net_filtered",
        "the net filtered load (filtration rate minus the reabsorption rate)",
        "filtration_rate - reabsorption_rate",
    ),
    (
        "crossed",
        "the net filtered load divided by the transit time PLUS the secretion offset in the denominator, not after the quotient (a wrong grouping)",
        "(filtration_rate - reabsorption_rate) / (transit_time + secretion_offset)",
    ),
    (
        "swapped",
        "the filtration rate PLUS the reabsorption rate over the transit time, plus the secretion offset, the difference flipped to a sum (a wrong grouping)",
        "(filtration_rate + reabsorption_rate) / transit_time + secretion_offset",
    ),
]
QUERIED = ["estimated_gfr", "net_clearance", "net_filtered"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(filtration_rate, reabsorption_rate, transit_time, secretion_offset):
    # Operation order mirrors the ADJ programs exactly (the parenthesised difference divides, then the addition applies
    # to the quotient), so the Python option value and the engine result are the same IEEE-double (well within the
    # harness's 1e-9 match tolerance).
    return {
        "estimated_gfr": (filtration_rate - reabsorption_rate) / transit_time + secretion_offset,
        "net_clearance": (filtration_rate - reabsorption_rate) / transit_time,
        "net_filtered": filtration_rate - reabsorption_rate,
        "crossed": (filtration_rate - reabsorption_rate) / (transit_time + secretion_offset),
        "swapped": (filtration_rate + reabsorption_rate) / transit_time + secretion_offset,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for filtration_rate, reabsorption_rate, transit_time, secretion_offset in TABLES:
        assert (
            filtration_rate > 0
            and reabsorption_rate > 0
            and transit_time > 0
            and secretion_offset > 0
        ), (filtration_rate, reabsorption_rate, transit_time, secretion_offset)
        # filtration_rate > reabsorption_rate keeps the difference (and every family member) strictly positive.
        assert filtration_rate > reabsorption_rate, (filtration_rate, reabsorption_rate)
        fv = family_values(filtration_rate, reabsorption_rate, transit_time, secretion_offset)
        for key, v in fv.items():
            assert v > 0, (key, filtration_rate, reabsorption_rate, transit_time, secretion_offset, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    filtration_rate,
                    reabsorption_rate,
                    transit_time,
                    secretion_offset,
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
                filtration_rate,
                reabsorption_rate,
                transit_time,
                secretion_offset,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r82gfr-{idx + 1:02d}",
                "qtype": "gfr_estimation_index",
                "stem": (
                    f"A kidney filters at a filtration rate of {num(filtration_rate)}, reabsorbs at a reabsorption "
                    f"rate of {num(reabsorption_rate)} over a transit time of {num(transit_time)}, then adds a "
                    f"secretion offset of {num(secretion_offset)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe filtration_rate({num(filtration_rate)})\n"
                    f"observe reabsorption_rate({num(reabsorption_rate)})\n"
                    f"observe transit_time({num(transit_time)})\n"
                    f"observe secretion_offset({num(secretion_offset)})\n"
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
            "ADJ-LADDER rung 82 — nephrology GFR-estimation index from four stated quantities (a NEW panel: nephrology "
            "/ GFR-estimation). From a filtration rate, a reabsorption rate to subtract, a transit time to divide the "
            "net by, and a secretion offset to add, compute the estimated GFR "
            "((filtration_rate-reabsorption_rate)/transit_time + secretion_offset), the net clearance "
            "((filtration_rate-reabsorption_rate)/transit_time), or the net filtered load "
            "(filtration_rate-reabsorption_rate). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, GROUPED-DIFFERENCE "
            "QUOTIENT PLUS A TERM (a-b)/c+d (form the parenthesised difference, divide by c, then add d, so (a-b)/c+d = "
            "((a-b)/c)+d; the mirror of rung-81 (a+b)/c-d which summed the group and subtracted the term) — and the "
            "harness matches the scalar to the printed options. Contamination-safe: every index is built only from the "
            "four observed quantities via -, /, and + — no constant leaks, and neither the net clearance, the net "
            "filtered load, nor any GFR figure ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same four quantities, so the distractors are exactly the slips students make: adding the "
            "secretion offset INSIDE the denominator ((a-b)/(c+d), a wrong grouping) and flipping the net difference to "
            "a SUM ((a+b)/c+d, a wrong grouping). The core confusion tested is that (a-b)/c+d is ((a-b)/c)+d, not "
            "(a-b)/(c+d) and not (a+b)/c+d."
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
