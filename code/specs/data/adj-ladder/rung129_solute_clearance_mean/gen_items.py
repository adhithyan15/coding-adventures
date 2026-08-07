"""Generate rung-129 (solute-clearance windowed mean / SUBTRACT-A-QUOTIENT NUMERATOR over a lone denominator) items.json.

Rung 129 opens the **solute-clearance** panel and is the **subtract twin of rung-128's add-a-quotient numerator**. rung-128 introduced
the first quotient-bearing NUMERATOR, `(a + b/c)/d` (add a quotient in the numerator, over a lone denominator); rung-129 is its minus
twin, `(a - b/c)/d` (subtract a quotient in the numerator, over a lone denominator) — exactly as rung-127 (`a/(b - c/d)`) mirrored
rung-126 (`a/(b + c/d)`) in the denominator. The add→subtract pairing repeats one level up the fraction bar.

This is genuinely new. `(a - b/c)/d` evaluates as `((a - (b/c)) / d)`: the inner quotient `b/c` still binds BEFORE the `-` (operator
precedence), and the whole `a - b/c` sits over the bar as one grouped numerator (grouping), so it is NOT `a - (b/c)/d` and NOT
`((a - b)/c)/d`. Because rung-129 SUBTRACTS inside the numerator, positivity is NOT automatic (unlike rung-128's all-plus numerator): the
misgrouped denominator's numerator `a - b` must be guarded positive, which is the binding constraint (and comfortably makes the net load
`a - b/c` and the crossed slip `a - b/(c*d)` positive too, since `b/c <= b` and `b/(c*d) <= b/c <= b`).

The setup: a `gross_load`, a `bound_charge`, a `bound_spread`, and a `clearance_windows` count. A per-unit bound component is the bound
charge spread over the bound spread; the net load is the gross load MINUS that per-unit bound component; the windowed clearance is the net
load divided (averaged) over the clearance windows. The figures are:

  WINDOWED CLEARANCE   (gross_load - bound_charge / bound_spread) / clearance_windows   [ subtract-a-quotient numerator / lone denom ]
  NET LOAD             gross_load - bound_charge / bound_spread                          [ the subtract-a-quotient numerator ]
  PER SPREAD           bound_charge / bound_spread                                       [ the bound charge spread over the bound spread ]

The **windowed clearance** is the ladder's first **(difference that subtracts a quotient) over a lone denominator (as a headline)** — a
mean (net load averaged per window), framed as a *mean* to keep it dimensionless-clean, the same discipline rungs 100/.../127/128 used for
their ratios. (The net load `a - b/c` and the per-spread quotient `b/c` ride alongside as component readouts, so the panel teaches the
whole calculation — exactly as rungs 47-128 shipped their component figures beside the headline. The per-spread quotient `b/c` anchors the
"the bound charge is spread over the bound spread FIRST, then subtracted from the gross load" grouping against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the division of the bound charge by the bound spread to form the per-spread quotient, then the subtraction of that quotient
from the gross load to form the whole net-load numerator, then the division of that whole numerator by the clearance windows (so
(a-b/c)/d evaluates as ((a-(b/c))/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../127/128. This rung exercises the engine across a **(subtract-a-quotient three-term
numerator) over a lone denominator ratio** — the fact that `(a-b/c)/d` is `((a-(b/c))/d)` and NOT `a-(b/c)/d` and NOT `((a-b)/c)/d` made
computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs
100/.../127/128 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-` and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the net load, the per-spread quotient, nor the windowed clearance
is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers**
(`gross_load`, `bound_charge`, `bound_spread`, `clearance_windows`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED     gross_load - bound_charge / bound_spread / clearance_windows   drop the numerator parentheses so the clearance windows
                                                                  divide ONLY the per-spread quotient, leaving the gross load
                                                                  un-averaged (the classic `(a-b/c)/d` vs `a-b/c/d` grouping error,
                                                                  evaluating `a - (b/c)/d = a - b/(c*d)`), and
  MISGROUPED  (gross_load - bound_charge) / bound_spread / clearance_windows   subtract the bound charge from the gross load FIRST, then
                                                                  divide by the bound spread and the clearance windows (`(a-b)/c/d` =
                                                                  `(a-b)/(c*d)`, ignoring the precedence that `b/c` binds before the `-`),

which are exactly the mistakes a student makes (failing to keep the whole subtract-a-quotient difference grouped over the bar, or breaking
the `b/c`-binds-first precedence). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: this rung SUBTRACTS a quotient inside the numerator, so positivity is NOT automatic — it is guarded
explicitly per table. Every observed quantity is `>= 2`, and each table guarantees **gross_load - bound_charge >= 2** (so `a-b` is
comfortably positive, making the misgrouped numerator `a-b` positive, and since `b/c <= b` the net load `a - b/c >= a - b >= 2` and the
crossed slip `a - b/(c*d) >= a - b/c >= 2` are positive too). Every family member is asserted `> 0` at build time. The seven tables give
distinct windowed clearances, distinct net loads, and distinct per-spread quotients so all three queried readouts vary across the panel;
the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (GROSS_LOAD, BOUND_CHARGE, BOUND_SPREAD, CLEARANCE_WINDOWS) — a net load (a gross load MINUS a per-unit bound_charge/bound_spread
# quotient) averaged over the clearance windows for the windowed clearance. This rung SUBTRACTS a quotient inside the numerator, so
# positivity is NOT automatic; each table guarantees gross_load - bound_charge >= 2 (a-b comfortably positive, so the misgrouped numerator
# a-b, the net load a-b/c >= a-b, and the crossed slip a-b/(c*d) >= a-b/c are all positive). The seven tables give distinct per-spread
# quotients (b/c), distinct net loads (a - b/c), and distinct windowed clearances ((a-b/c)/d); the five family values are asserted
# pairwise-distinct below.
TABLES = [
    (14, 6, 3, 4),    # b/c = 2.0,  net = 12.0, mean = 3.0
    (18, 5, 2, 3),    # b/c = 2.5,  net = 15.5, mean = 5.166...
    (20, 9, 3, 2),    # b/c = 3.0,  net = 17.0, mean = 8.5
    (12, 3, 2, 5),    # b/c = 1.5,  net = 10.5, mean = 2.1
    (19, 8, 2, 4),    # b/c = 4.0,  net = 15.0, mean = 3.75
    (24, 7, 2, 3),    # b/c = 3.5,  net = 20.5, mean = 6.833...
    (31, 10, 2, 5),   # b/c = 5.0,  net = 26.0, mean = 5.2
]

# The option family (5 members), all built from the four observed quantities via - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "windowed_clearance",
        "windowed clearance (the net load averaged over the clearance windows)",
        "(gross_load - bound_charge / bound_spread) / clearance_windows",
    ),
    (
        "net_load",
        "the net load (the gross load minus the per-spread bound component, the numerator that is averaged over the clearance windows)",
        "gross_load - bound_charge / bound_spread",
    ),
    (
        "per_spread",
        "the per-spread bound component (the bound charge spread over the bound spread, before it is subtracted from the gross load)",
        "bound_charge / bound_spread",
    ),
    (
        "crossed",
        "the gross load minus the bound charge over the bound spread over the clearance windows, dropping the numerator parentheses so the clearance windows divide only the per-spread quotient (a wrong grouping)",
        "gross_load - bound_charge / bound_spread / clearance_windows",
    ),
    (
        "misgrouped",
        "the gross load minus the bound charge, all over the bound spread and the clearance windows, subtracting before forming the per-spread quotient (a wrong grouping)",
        "(gross_load - bound_charge) / bound_spread / clearance_windows",
    ),
]
QUERIED = ["windowed_clearance", "net_load", "per_spread"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(gross_load, bound_charge, bound_spread, clearance_windows):
    # Operation order mirrors the ADJ programs exactly (the bound charge is divided by the bound spread to form the per-spread quotient,
    # then that quotient is subtracted from the gross load to form the whole net-load numerator, then that whole numerator is divided by
    # the clearance windows, so (a-b/c)/d evaluates as ((a-(b/c))/d)), so the Python option value and the engine result are the same
    # IEEE-double (well within the 1e-9 tolerance).
    return {
        "windowed_clearance": (gross_load - bound_charge / bound_spread) / clearance_windows,
        "net_load": gross_load - bound_charge / bound_spread,
        "per_spread": bound_charge / bound_spread,
        "crossed": gross_load - bound_charge / bound_spread / clearance_windows,
        "misgrouped": (gross_load - bound_charge) / bound_spread / clearance_windows,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for gross_load, bound_charge, bound_spread, clearance_windows in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung SUBTRACTS a quotient inside the numerator, so positivity is
        # NOT automatic; it is guarded explicitly per table.
        assert (
            gross_load >= 2
            and bound_charge >= 2
            and bound_spread >= 2
            and clearance_windows >= 2
        ), (gross_load, bound_charge, bound_spread, clearance_windows)
        assert gross_load - bound_charge >= 2, (
            "gross_load - bound_charge must be >= 2 (misgrouped numerator, net load & crossed all positive)",
            gross_load,
            bound_charge,
            bound_spread,
            clearance_windows,
        )
        fv = family_values(gross_load, bound_charge, bound_spread, clearance_windows)
        for key, v in fv.items():
            assert v > 0, (key, gross_load, bound_charge, bound_spread, clearance_windows, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    gross_load,
                    bound_charge,
                    bound_spread,
                    clearance_windows,
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
                gross_load,
                bound_charge,
                bound_spread,
                clearance_windows,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r129wca-{idx + 1:02d}",
                "qtype": "windowed_clearance",
                "stem": (
                    f"A solute-clearance study records a gross load of {num(gross_load)} minus a bound charge of "
                    f"{num(bound_charge)} over a bound spread of {num(bound_spread)}, all averaged over a clearance-window count of "
                    f"{num(clearance_windows)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe gross_load({num(gross_load)})\n"
                    f"observe bound_charge({num(bound_charge)})\n"
                    f"observe bound_spread({num(bound_spread)})\n"
                    f"observe clearance_windows({num(clearance_windows)})\n"
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
            "ADJ-LADDER rung 129 — solute-clearance windowed mean from four stated quantities (a NEW panel: solute clearance, and the "
            "SUBTRACT twin of rung-128's add-a-quotient numerator). From a net load (a gross load MINUS a per-unit "
            "bound_charge/bound_spread quotient) averaged over the clearance windows, compute the windowed clearance "
            "((gross_load-bound_charge/bound_spread)/clearance_windows), the net load (gross_load-bound_charge/bound_spread), or the "
            "per-spread bound component (bound_charge/bound_spread). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a SUBTRACT-A-QUOTIENT THREE-TERM "
            "NUMERATOR OVER A LONE DENOMINATOR (a-b/c)/d (divide the bound charge by the bound spread, subtract that from the gross load, "
            "then divide that whole net-load numerator by the clearance windows, so (a-b/c)/d = ((a-(b/c))/d); the minus twin of "
            "rung-128's (a+b/c)/d, mirroring how rung-127's a/(b-c/d) mirrors rung-126's a/(b+c/d) one level down the bar). The "
            "precedence-and-grouping slips ride alongside as distractors. The harness matches the scalar to the printed options. The "
            "windowed clearance is a mean (net load averaged per window), framed as a MEAN so the dimensionless value stays honest. "
            "Contamination-safe: every figure is built only from the four observed quantities via - and / — no constant leaks, and "
            "neither the net load, the per-spread quotient, nor the windowed clearance ever appears as a literal (each is computed) — and "
            "the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same four quantities, so the distractors are exactly the slips students make: dropping the numerator "
            "parentheses so the clearance windows divide only the per-spread quotient (a-b/c/d, evaluating a-(b/c)/d = a-b/(c*d), a wrong "
            "grouping) and subtracting before forming the quotient ((a-b)/c/d = (a-b)/(c*d), breaking the b/c-binds-first precedence, a "
            "wrong grouping). The core confusion tested is that (a-b/c)/d is ((a-(b/c))/d), not a-b/c/d and not (a-b)/c/d. This rung "
            "SUBTRACTS a quotient inside the numerator so positivity is NOT automatic; each table guards gross_load - bound_charge >= 2 "
            "(keeping the misgrouped numerator a-b, the net load a-b/c, and the crossed slip a-b/(c*d) all positive), keeping the five "
            "family values pairwise distinct with all three queried readouts varying across the panel, all asserted strictly positive at "
            "build time."
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
