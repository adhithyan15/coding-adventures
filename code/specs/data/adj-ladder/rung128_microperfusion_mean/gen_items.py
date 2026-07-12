"""Generate rung-128 (capillary-microperfusion windowed mean / ADD-A-QUOTIENT NUMERATOR over a lone denominator) items.json.

Rung 128 opens the **capillary-microperfusion** panel and moves the quotient the ladder has been threading — for two rungs — OUT of the
denominator and INTO the **numerator**. rung-126 introduced the first quotient-bearing denominator, `a/(b + c/d)` (add a quotient to the
denominator), and rung-127 was its minus twin, `a/(b - c/d)` (subtract a quotient from the denominator). rung-128 is the **numerator
sibling**: a numerator that ADDS a quotient, all divided by a lone denominator — `(a + b/c)/d`, i.e. `((a + (b/c)) / d)`. (Its subtract
twin `(a - b/c)/d` is queued as rung-129, exactly as 126→127 paired add→subtract.)

This is genuinely new. For rungs 126/127 the three-term structure lived UNDER the bar (the quotient was a denominator term); here it lives
ABOVE the bar (the quotient is a numerator term) and a lone term divides the whole sum. The inner quotient `b/c` still binds BEFORE the
`+` (operator precedence), and the whole `a + b/c` sits over the bar as one grouped numerator (grouping), so `(a + b/c)/d` evaluates as
`((a + (b/c)) / d)` — NOT `a + (b/c)/d` and NOT `((a + b)/c)/d`.

The setup: a `base_uptake`, a `surge_charge`, a `surge_spread`, and a `sample_windows` count. A per-unit surge is the surge charge spread
over the surge spread; the loaded uptake is the base uptake PLUS that per-unit surge; the windowed mean is the loaded uptake divided
(averaged) over the sample windows. The figures are:

  WINDOWED MEAN   (base_uptake + surge_charge / surge_spread) / sample_windows   [ add-a-quotient numerator / lone denominator ]
  LOADED UPTAKE   base_uptake + surge_charge / surge_spread                      [ the add-a-quotient numerator ]
  PER SPREAD      surge_charge / surge_spread                                    [ the surge charge spread over the surge spread ]

The **windowed mean** is the ladder's first **(sum that adds a quotient) over a lone denominator (as a headline)** — a mean (loaded
uptake averaged per window), framed as a *mean* to keep it dimensionless-clean, the same discipline rungs 100/.../126/127 used for their
ratios. (The loaded uptake `a + b/c` and the per-spread quotient `b/c` ride alongside as component readouts, so the panel teaches the
whole calculation — exactly as rungs 47-127 shipped their component figures beside the headline. The per-spread quotient `b/c` anchors the
"the surge charge is spread over the surge spread FIRST, then added to the base uptake" grouping against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the division of the surge charge by the surge spread to form the per-spread quotient, then the addition of that quotient to
the base uptake to form the whole loaded-uptake numerator, then the division of that whole numerator by the sample windows (so
(a+b/c)/d evaluates as ((a+(b/c))/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../126/127. This rung exercises the engine across a **(add-a-quotient three-term
numerator) over a lone denominator ratio** — the fact that `(a+b/c)/d` is `((a+(b/c))/d)` and NOT `a+(b/c)/d` and NOT `((a+b)/c)/d` made
computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../126/127 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the loaded uptake, the per-spread quotient, nor the windowed mean
is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers**
(`base_uptake`, `surge_charge`, `surge_spread`, `sample_windows`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED     base_uptake + surge_charge / surge_spread / sample_windows   drop the numerator parentheses so the sample windows divide
                                                                  ONLY the per-spread quotient, leaving the base uptake un-averaged (the
                                                                  classic `(a+b/c)/d` vs `a+b/c/d` grouping error, evaluating
                                                                  `a + (b/c)/d = a + b/(c*d)`), and
  MISGROUPED  (base_uptake + surge_charge) / surge_spread / sample_windows   add the base uptake and surge charge FIRST, then divide by
                                                                  the surge spread and the sample windows (`(a+b)/c/d` = `(a+b)/(c*d)`,
                                                                  ignoring the precedence that `b/c` binds before the `+`),

which are exactly the mistakes a student makes (failing to keep the whole add-a-quotient sum grouped over the bar, or breaking the
`b/c`-binds-first precedence). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: this rung uses only `+` and `/` over positive quantities, so **every figure is automatically positive** (no
subtraction anywhere) — unlike rung-127, no positivity guards are needed. Every observed quantity is `>= 2`. Every family member is
asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give distinct windowed means, distinct loaded uptakes, and
distinct per-spread quotients so all three queried readouts vary across the panel; the five family values are pairwise distinct with a
comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BASE_UPTAKE, SURGE_CHARGE, SURGE_SPREAD, SAMPLE_WINDOWS) — a loaded uptake (a base uptake PLUS a per-unit surge_charge/surge_spread
# quotient) averaged over the sample windows for the windowed mean. This rung uses only + and / over positive quantities, so every figure
# is automatically positive; no positivity guards are needed (unlike rung-127's subtract-in-denominator). The seven tables give distinct
# per-spread quotients (b/c), distinct loaded uptakes (a + b/c), and distinct windowed means ((a+b/c)/d); the five family values are
# asserted pairwise-distinct below.
TABLES = [
    (10, 6, 3, 4),    # b/c = 2.0,  loaded = 12.0, mean = 3.0
    (13, 5, 2, 3),    # b/c = 2.5,  loaded = 15.5, mean = 5.166...
    (14, 9, 3, 2),    # b/c = 3.0,  loaded = 17.0, mean = 8.5
    (8, 3, 2, 5),     # b/c = 1.5,  loaded = 9.5,  mean = 1.9
    (11, 8, 2, 4),    # b/c = 4.0,  loaded = 15.0, mean = 3.75
    (17, 7, 2, 3),    # b/c = 3.5,  loaded = 20.5, mean = 6.833...
    (25, 10, 2, 5),   # b/c = 5.0,  loaded = 30.0, mean = 6.0
]

# The option family (5 members), all built from the four observed quantities via + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "windowed_mean",
        "windowed mean (the loaded uptake averaged over the sample windows)",
        "(base_uptake + surge_charge / surge_spread) / sample_windows",
    ),
    (
        "loaded_uptake",
        "the loaded uptake (the base uptake plus the per-spread surge, the numerator that is averaged over the sample windows)",
        "base_uptake + surge_charge / surge_spread",
    ),
    (
        "per_spread",
        "the per-spread surge (the surge charge spread over the surge spread, before it is added to the base uptake)",
        "surge_charge / surge_spread",
    ),
    (
        "crossed",
        "the base uptake plus the surge charge over the surge spread over the sample windows, dropping the numerator parentheses so the sample windows divide only the per-spread quotient (a wrong grouping)",
        "base_uptake + surge_charge / surge_spread / sample_windows",
    ),
    (
        "misgrouped",
        "the base uptake plus the surge charge, all over the surge spread and the sample windows, adding before forming the per-spread quotient (a wrong grouping)",
        "(base_uptake + surge_charge) / surge_spread / sample_windows",
    ),
]
QUERIED = ["windowed_mean", "loaded_uptake", "per_spread"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(base_uptake, surge_charge, surge_spread, sample_windows):
    # Operation order mirrors the ADJ programs exactly (the surge charge is divided by the surge spread to form the per-spread quotient,
    # then that quotient is added to the base uptake to form the whole loaded-uptake numerator, then that whole numerator is divided by
    # the sample windows, so (a+b/c)/d evaluates as ((a+(b/c))/d)), so the Python option value and the engine result are the same
    # IEEE-double (well within the 1e-9 tolerance).
    return {
        "windowed_mean": (base_uptake + surge_charge / surge_spread) / sample_windows,
        "loaded_uptake": base_uptake + surge_charge / surge_spread,
        "per_spread": surge_charge / surge_spread,
        "crossed": base_uptake + surge_charge / surge_spread / sample_windows,
        "misgrouped": (base_uptake + surge_charge) / surge_spread / sample_windows,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for base_uptake, surge_charge, surge_spread, sample_windows in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only + and / over positive quantities, so positivity is
        # automatic — no positivity guards are needed (unlike rung-127's subtract-in-denominator).
        assert (
            base_uptake >= 2
            and surge_charge >= 2
            and surge_spread >= 2
            and sample_windows >= 2
        ), (base_uptake, surge_charge, surge_spread, sample_windows)
        fv = family_values(base_uptake, surge_charge, surge_spread, sample_windows)
        for key, v in fv.items():
            assert v > 0, (key, base_uptake, surge_charge, surge_spread, sample_windows, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    base_uptake,
                    surge_charge,
                    surge_spread,
                    sample_windows,
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
                base_uptake,
                surge_charge,
                surge_spread,
                sample_windows,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r128wma-{idx + 1:02d}",
                "qtype": "windowed_mean",
                "stem": (
                    f"A capillary-microperfusion study records a base uptake of {num(base_uptake)} plus a surge charge of "
                    f"{num(surge_charge)} over a surge spread of {num(surge_spread)}, all averaged over a sample-window count of "
                    f"{num(sample_windows)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe base_uptake({num(base_uptake)})\n"
                    f"observe surge_charge({num(surge_charge)})\n"
                    f"observe surge_spread({num(surge_spread)})\n"
                    f"observe sample_windows({num(sample_windows)})\n"
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
            "ADJ-LADDER rung 128 — capillary-microperfusion windowed mean from four stated quantities (a NEW panel: capillary "
            "microperfusion, and the NUMERATOR sibling of rungs 126/127 — the quotient moves OUT of the denominator and INTO the "
            "numerator). From a loaded uptake (a base uptake PLUS a per-unit surge_charge/surge_spread quotient) averaged over the "
            "sample windows, compute the windowed mean ((base_uptake+surge_charge/surge_spread)/sample_windows), the loaded uptake "
            "(base_uptake+surge_charge/surge_spread), or the per-spread surge (surge_charge/surge_spread). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a "
            "NEW family, an ADD-A-QUOTIENT THREE-TERM NUMERATOR OVER A LONE DENOMINATOR (a+b/c)/d (divide the surge charge by the surge "
            "spread, add that to the base uptake, then divide that whole loaded-uptake numerator by the sample windows, so (a+b/c)/d = "
            "((a+(b/c))/d); the numerator sibling of rung-126's a/(b+c/d), with its subtract twin (a-b/c)/d queued as rung-129 exactly "
            "as 126->127 paired add->subtract). The precedence-and-grouping slips ride alongside as distractors. The harness matches the "
            "scalar to the printed options. The windowed mean is a mean (loaded uptake averaged per window), framed as a MEAN so the "
            "dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed quantities via + "
            "and / — no constant leaks, and neither the loaded uptake, the per-spread quotient, nor the windowed mean ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the slips students make: "
            "dropping the numerator parentheses so the sample windows divide only the per-spread quotient (a+b/c/d, evaluating "
            "a+(b/c)/d = a+b/(c*d), a wrong grouping) and adding before forming the quotient ((a+b)/c/d = (a+b)/(c*d), breaking the "
            "b/c-binds-first precedence, a wrong grouping). The core confusion tested is that (a+b/c)/d is ((a+(b/c))/d), not a+b/c/d "
            "and not (a+b)/c/d. This rung uses only + and / over positive quantities, so every figure is automatically positive — no "
            "positivity guards are needed (unlike rung-127) — and the five family values are kept pairwise distinct with all three "
            "queried readouts varying across the panel, all asserted strictly positive at build time."
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
