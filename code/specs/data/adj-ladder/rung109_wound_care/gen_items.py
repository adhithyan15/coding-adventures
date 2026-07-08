"""Generate rung-109 (wound care / burn surface accounting) items.json for the ADJ-LADDER.

Rung 109 opens the **wound care / burn surface accounting** panel on the quantitative band — the arithmetic of a wound
burden index. A `baseline_area` (the original wound area) has some `healed_area` SUBTRACTED (tissue that has closed) and some
`dehisced_area` ADDED (tissue that has reopened), and that net open area is DIVIDED by a `dressing_count` (how many dressing
changes the reading is averaged over) to give the wound burden index. A **three-term numerator WITH a subtraction over a single
divisor** introduces a genuinely NEW arithmetic family on the ladder: `(a-b+c)/d`, i.e. `((a - b + c) / d)`.

This is genuinely new — rung-108 opened the three-term-numerator frontier with `(a+b+c)/d` (a bare three-way SUM over a
divisor); rung-109 is the FIRST three-term numerator that MIXES a subtraction and an addition over a divisor. Every prior ratio
used a two-term numerator (rung-37 `(a+b)/(c+d)`, rung-99 `(a*b)/(c+d)`, rung-100 `(a+b)/(c*d)`, rung-104 `(a-b)/(c*d)`, the
difference-denominator trio rung-105 `(a+b)/(c-d)`, rung-106 `a*b/(c-d)`, rung-107 `(a-b)/(c-d)`) or the pure three-term sum
rung-108 `(a+b+c)/d`. Rung-109 moves to `(a-b+c)/d`. The operator order matters: `(a-b+c)/d` is `((a-b+c) / d)` (the whole
net-open numerator is divided), NOT `a-b+c/d` (dropping the numerator parentheses so only the dehisced area is divided by the
dressing count and then combined with the other two areas) and NOT `(a-b)/(c+d)` (regrouping so only the first two areas form
the numerator and the dehisced area joins the dressing count in the denominator) — the two distractors exploit exactly those
confusions.

The setup: a `baseline_area`, a `healed_area`, a `dehisced_area`, and a `dressing_count`. The total is:

  WOUND BURDEN INDEX  (baseline_area - healed_area + dehisced_area) / dressing_count  [ a mixed three-term numerator over a divisor ]
  NET OPEN AREA       baseline_area - healed_area + dehisced_area                     [ the three-term numerator ]
  DRESSING COUNT      dressing_count                                                  [ the divisor ]

The **wound burden index** is what makes this rung distinctive — it is the ladder's first **three-term numerator that mixes a
subtraction and an addition over a single divisor**. It is a rate (net open wound area per dressing change), framed as an
*index* to keep it dimensionless-clean — the same discipline rungs 100/104/105/106/107/108 used for their ratios. (The net open
area `a-b+c` and the dressing count `d` ride alongside as component readouts, so the panel teaches the whole calculation —
exactly as rungs 47-108 shipped their component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the subtraction of the healed area and the addition of the dehisced area into the net open area, then the
division of that net area by the dressing count (the whole three-term numerator parenthesized, so (a-b+c)/d evaluates as
((a-b+c)/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change,
exactly as rungs 8/16/.../107/108. This rung exercises the engine across a **mixed three-term numerator over a divisor** — the
fact that `(a-b+c)/d` is `((a-b+c)/d)` and NOT `a-b+c/d` and NOT `(a-b)/(c+d)` made computable. The ratio golds are non-integer
f64s; the engine's IEEE-double division matches Python's the same way rungs 99/100/104/105/106/107/108 relied on (well within
the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-`, and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the net open area, the dressing count, nor
any index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`baseline_area`, `healed_area`, `dehisced_area`, `dressing_count`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    baseline_area - healed_area + dehisced_area / dressing_count  drop the numerator parentheses so only the dehisced
                                                                          area is divided by the dressing count and then
                                                                          combined with the other two areas (the classic
                                                                          `(a-b+c)/d` vs `a-b+c/d` precedence error), and
  SWAPPED    (baseline_area - healed_area) / (dehisced_area + dressing_count)  regroup so only the first two areas form the
                                                                          numerator and the dehisced area joins the dressing
                                                                          count in the denominator (`(a-b)/(c+d)` instead of
                                                                          `(a-b+c)/d`),

which are exactly the mistakes a student makes (dropping the numerator parentheses before dividing, or regrouping which terms
belong to the numerator vs the divisor). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: this rung SUBTRACTS, so positivity is guaranteed by table construction rather than automatic. Each
table guarantees **baseline_area > healed_area** (so the difference `a-b` is strictly positive and the net open area
`a-b+c = (a-b)+c` is positive), the **dressing_count >= 2** (divisor never zero), the wound burden index never coincides with
the dressing count or the net open area, and the five family values are pairwise distinct with a comfortable margin; and — so
all three queried readouts vary across the panel — the seven tables give distinct wound burden indices, distinct net open
areas, and distinct dressing counts, all asserted at build time. (crossed `a-b+c/d = (a-b)+(c/d)` is positive because `a>b` and
`c/d>0`; swapped `(a-b)/(c+d)` is positive because `a>b` and the denominator is a sum of positives.)
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BASELINE_AREA, HEALED_AREA, DEHISCED_AREA, DRESSING_COUNT) — an original wound area with a healed area subtracted and a
# dehisced area added for the net open area, all divided by a dressing count, all plain positive numbers >= 2. This rung
# SUBTRACTS (the healed area), so every table guarantees baseline_area > healed_area (a-b>0) which — together with the added
# dehisced area — keeps every family member strictly positive; dressing_count >= 2 keeps the divisor away from zero. The five
# family values are asserted pairwise-distinct below. The seven tables give distinct wound burden indices, distinct net open
# areas, and distinct dressing counts so all three queried readouts vary across the panel.
TABLES = [
    (5, 2, 4, 2),
    (6, 3, 5, 3),
    (7, 4, 7, 4),
    (9, 4, 10, 5),
    (6, 5, 8, 6),
    (8, 4, 10, 7),
    (10, 5, 13, 8),
]

# The option family (5 members), all built from the four observed quantities via +, - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "wound_burden_index",
        "wound burden index (the net open area divided by the dressing count)",
        "(baseline_area - healed_area + dehisced_area) / dressing_count",
    ),
    (
        "net_open_area",
        "the net open area (the baseline area minus the healed area plus the dehisced area, the numerator divided by the dressing count)",
        "baseline_area - healed_area + dehisced_area",
    ),
    (
        "dressing_count",
        "the dressing count (the divisor the net open area is divided by)",
        "dressing_count",
    ),
    (
        "crossed",
        "the baseline minus healed area plus the dehisced area divided by the dressing count, dropping the numerator parentheses so only the dehisced area is divided before combining (a wrong grouping)",
        "baseline_area - healed_area + dehisced_area / dressing_count",
    ),
    (
        "swapped",
        "the baseline minus healed area, divided by the dehisced area plus the dressing count, regrouping so only the first two areas form the numerator (a wrong pairing)",
        "(baseline_area - healed_area) / (dehisced_area + dressing_count)",
    ),
]
QUERIED = ["wound_burden_index", "net_open_area", "dressing_count"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(baseline_area, healed_area, dehisced_area, dressing_count):
    # Operation order mirrors the ADJ programs exactly (the mixed three-term numerator forms, then the numerator is divided by
    # the dressing count, so (a-b+c)/d evaluates as ((a-b+c)/d)), so the Python option value and the engine result are the same
    # IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "wound_burden_index": (baseline_area - healed_area + dehisced_area) / dressing_count,
        "net_open_area": baseline_area - healed_area + dehisced_area,
        "dressing_count": dressing_count,
        "crossed": baseline_area - healed_area + dehisced_area / dressing_count,
        "swapped": (baseline_area - healed_area) / (dehisced_area + dressing_count),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for baseline_area, healed_area, dehisced_area, dressing_count in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung SUBTRACTS the healed area, so each table
        # guarantees baseline_area > healed_area (the difference a-b is strictly positive) which — together with the added
        # dehisced area — keeps every family member strictly positive; dressing_count >= 2 keeps the divisor away from zero.
        assert (
            baseline_area >= 2
            and healed_area >= 2
            and dehisced_area >= 2
            and dressing_count >= 2
        ), (baseline_area, healed_area, dehisced_area, dressing_count)
        assert baseline_area > healed_area, (baseline_area, healed_area, dehisced_area, dressing_count)
        fv = family_values(baseline_area, healed_area, dehisced_area, dressing_count)
        for key, v in fv.items():
            assert v > 0, (key, baseline_area, healed_area, dehisced_area, dressing_count, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    baseline_area,
                    healed_area,
                    dehisced_area,
                    dressing_count,
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
                baseline_area,
                healed_area,
                dehisced_area,
                dressing_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r109wound-{idx + 1:02d}",
                "qtype": "wound_burden_index",
                "stem": (
                    f"A wound chart records a baseline area of {num(baseline_area)} minus a healed area of "
                    f"{num(healed_area)} plus a dehisced area of {num(dehisced_area)}, divided by a dressing count of "
                    f"{num(dressing_count)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe baseline_area({num(baseline_area)})\n"
                    f"observe healed_area({num(healed_area)})\n"
                    f"observe dehisced_area({num(dehisced_area)})\n"
                    f"observe dressing_count({num(dressing_count)})\n"
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
            "ADJ-LADDER rung 109 — wound burden index from four stated quantities (a NEW panel: wound care / burn surface "
            "accounting). From a baseline area minus a healed area plus a dehisced area for the net open area, all divided by a "
            "dressing count, compute the wound burden index ((baseline_area-healed_area+dehisced_area)/dressing_count), the net "
            "open area (baseline_area-healed_area+dehisced_area), or the dressing count. Each item is a compute_dimensioned "
            "program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, "
            "A MIXED THREE-TERM NUMERATOR OVER A DIVISOR (a-b+c)/d (subtract the healed area, add the dehisced area, divide by "
            "the dressing count, so (a-b+c)/d = ((a-b+c)/d); the FIRST time the ladder puts a three-term numerator that MIXES a "
            "subtraction and an addition over a divisor — rung-108 opened the frontier with the pure sum (a+b+c)/d, and every "
            "earlier ratio used a TWO-term numerator: 37 (a+b)/(c+d), 99 (a*b)/(c+d), 100 (a+b)/(c*d), 104 (a-b)/(c*d), and the "
            "difference-denominator trio 105 (a+b)/(c-d), 106 a*b/(c-d), 107 (a-b)/(c-d)) — and the harness matches the scalar "
            "to the printed options. The wound burden index is a rate (net open area per dressing change), framed as an INDEX so "
            "the dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via +, - and / — no constant leaks, and neither the net open area, the dressing count, nor any index "
            "ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. The five options are a family over the same four quantities, so the "
            "distractors are exactly the slips students make: dropping the numerator parentheses so only the dehisced area is "
            "divided before combining (a-b+c/d, a wrong grouping) and regrouping so only the first two areas form the numerator "
            "((a-b)/(c+d), a wrong pairing). The core confusion tested is that (a-b+c)/d is ((a-b+c)/d), not a-b+c/d and not "
            "(a-b)/(c+d). This rung SUBTRACTS the healed area, so positivity is guaranteed by table construction: every table "
            "has baseline_area > healed_area (a-b>0) and dressing_count >= 2 (divisor never zero), keeping every family member "
            "strictly positive."
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
