"""Generate rung-81 (cardiology cardiac-output perfusion index) items.json for the ADJ-LADDER.

Rung 81 opens the **cardiology / cardiac-output** panel on the quantitative band — the arithmetic of a net perfusion
index. Two inflow volumes, a `forward_volume` and a `collateral_volume`, are POOLED (summed) and spread over a
`circulation_time` — a **grouped-sum quotient** `(forward_volume + collateral_volume) / circulation_time` (the combined
flow rate) — and then a steady `baseline_drain` is SUBTRACTED from that rate. A grouped sum over a divisor, minus a fourth
term, introduces a genuinely NEW arithmetic shape on the ladder: a **grouped-sum quotient MINUS a term** —
`(a+b)/c-d`, i.e. `((a+b)/c) - d`.

This is the trailing-subtraction sibling of rung-76's `(a+b)/c*d` (a grouped-sum quotient TIMES a fourth term): rung-76
multiplied the quotient by `d`, rung-81 SUBTRACTS `d` from it. The grouping matters at TWO points: the sum
`forward_volume + collateral_volume` is formed FIRST and divided as a whole (`(a+b)/c`, not `a + b/c`), and the `-d` is
applied to the quotient AFTER the division (`((a+b)/c) - d`, not `(a+b)/(c-d)` and not with the minus folded inside the
sum). The operation order is `((a+b)/c) - d` by precedence (the parenthesised sum divides, then the subtraction), NOT
`(a+b)/(c-d)` (subtracting `d` INSIDE the denominator) and NOT `(a-b)/c - d` (a MINUS inside the group instead of the
plus) — the two distractors exploit exactly those confusions.

The setup: a `forward_volume`, a `collateral_volume`, a `circulation_time`, and a `baseline_drain`. The perfusion index
is:

  PERFUSION INDEX   (forward_volume + collateral_volume) / circulation_time - baseline_drain   [ grouped-sum quotient minus a term ]
  COMBINED FLOW     (forward_volume + collateral_volume) / circulation_time                    [ the quotient term, before -d ]
  COMBINED VOLUME    forward_volume + collateral_volume                                        [ the summed numerator ]

The **perfusion index** is what makes this rung distinctive — it is the ladder's first **grouped-sum quotient MINUS a
term**. (The combined flow `(a+b)/c` and the combined volume `a+b` ride alongside as component readouts, so the panel
teaches the whole calculation — exactly as rungs 47-80 shipped their component sums/products/differences/ratios beside
the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the summation of the two inflow volumes, the division of that sum by the circulation time, and
the subtraction of the baseline drain from the quotient (form the grouped sum, divide, then subtract) — and the harness
reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../79/80. This rung exercises the engine across **a grouped-sum quotient minus a term** — the fact that
`(a+b)/c-d` is `((a+b)/c) - d` and NOT `(a+b)/(c-d)` and NOT `(a-b)/c - d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `/`, and `-`
— **no structural constants** — so no numeric literal appears in any program, and neither the combined flow, the combined
volume, nor any index figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`forward_volume`, `collateral_volume`, `circulation_time`, `baseline_drain`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (forward_volume + collateral_volume) / (circulation_time - baseline_drain)   subtract the baseline drain
                                                                                          INSIDE the denominator instead
                                                                                          of after the quotient (the
                                                                                          classic `(a+b)/c-d` vs
                                                                                          `(a+b)/(c-d)` error), and
  SWAPPED    (forward_volume - collateral_volume) / circulation_time - baseline_drain      SUBTRACT the collateral volume
                                                                                          inside the group instead of
                                                                                          adding it (`(a-b)/c-d` instead
                                                                                          of `(a+b)/c-d`, a wrong
                                                                                          grouping),

which are exactly the mistakes a student makes (folding the trailing term into the denominator, or flipping the sum into
a difference). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: the tables are chosen so all guards hold — `circulation_time > baseline_drain` (and
`circulation_time - baseline_drain >= 2`, so the crossed denominator is positive AND crossed never coincides with the
combined volume), `forward_volume + collateral_volume > circulation_time * baseline_drain` (perfusion index positive),
and `forward_volume - collateral_volume > circulation_time * baseline_drain` (swapped positive, the binding constraint,
which also forces the collateral volume below the forward volume) — so every family member, including the headline
perfusion index `(a+b)/c-d`, is strictly positive; the five family values are pairwise distinct with a comfortable
margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (FORWARD_VOLUME, COLLATERAL_VOLUME, CIRCULATION_TIME, BASELINE_DRAIN) — two inflow volumes to pool, a circulation time
# to divide the pooled sum by, and a baseline drain to subtract from the quotient, all plain positive numbers. The
# tables satisfy every guard: circulation_time > baseline_drain (and circulation_time - baseline_drain >= 2, keeping the
# crossed denominator positive and crossed distinct from the combined volume), forward_volume + collateral_volume >
# circulation_time * baseline_drain (perfusion index > 0), and forward_volume - collateral_volume > circulation_time *
# baseline_drain (swapped > 0, the binding constraint). The five family values are asserted pairwise-distinct below.
TABLES = [
    (30, 6, 4, 2),
    (40, 8, 6, 3),
    (24, 4, 5, 2),
    (50, 10, 8, 3),
    (36, 6, 6, 2),
    (44, 8, 5, 2),
    (60, 12, 8, 4),
]

# The option family (5 members), all built from the four observed quantities via +, /, and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "perfusion_index",
        "net perfusion index (the combined flow minus the baseline drain)",
        "(forward_volume + collateral_volume) / circulation_time - baseline_drain",
    ),
    (
        "combined_flow",
        "the combined flow (pooled inflow volume over the circulation time)",
        "(forward_volume + collateral_volume) / circulation_time",
    ),
    (
        "combined_volume",
        "the combined volume (forward volume plus the collateral volume)",
        "forward_volume + collateral_volume",
    ),
    (
        "crossed",
        "the pooled inflow volume divided by the circulation time MINUS the baseline drain in the denominator, not after the quotient (a wrong grouping)",
        "(forward_volume + collateral_volume) / (circulation_time - baseline_drain)",
    ),
    (
        "swapped",
        "the forward volume MINUS the collateral volume over the circulation time, minus the baseline drain, the sum flipped to a difference (a wrong grouping)",
        "(forward_volume - collateral_volume) / circulation_time - baseline_drain",
    ),
]
QUERIED = ["perfusion_index", "combined_flow", "combined_volume"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(forward_volume, collateral_volume, circulation_time, baseline_drain):
    # Operation order mirrors the ADJ programs exactly (the parenthesised sum divides, then the subtraction applies to
    # the quotient), so the Python option value and the engine result are the same IEEE-double (well within the
    # harness's 1e-9 match tolerance).
    return {
        "perfusion_index": (forward_volume + collateral_volume) / circulation_time - baseline_drain,
        "combined_flow": (forward_volume + collateral_volume) / circulation_time,
        "combined_volume": forward_volume + collateral_volume,
        "crossed": (forward_volume + collateral_volume) / (circulation_time - baseline_drain),
        "swapped": (forward_volume - collateral_volume) / circulation_time - baseline_drain,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for forward_volume, collateral_volume, circulation_time, baseline_drain in TABLES:
        assert (
            forward_volume > 0
            and collateral_volume > 0
            and circulation_time > 0
            and baseline_drain > 0
        ), (forward_volume, collateral_volume, circulation_time, baseline_drain)
        fv = family_values(forward_volume, collateral_volume, circulation_time, baseline_drain)
        # The tables satisfy all guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, forward_volume, collateral_volume, circulation_time, baseline_drain, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    forward_volume,
                    collateral_volume,
                    circulation_time,
                    baseline_drain,
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
                forward_volume,
                collateral_volume,
                circulation_time,
                baseline_drain,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r81cpi-{idx + 1:02d}",
                "qtype": "cardiac_perfusion_index",
                "stem": (
                    f"Cardiac output pools a forward volume of {num(forward_volume)} and a collateral volume of "
                    f"{num(collateral_volume)} over a circulation time of {num(circulation_time)}, then subtracts a "
                    f"baseline drain of {num(baseline_drain)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe forward_volume({num(forward_volume)})\n"
                    f"observe collateral_volume({num(collateral_volume)})\n"
                    f"observe circulation_time({num(circulation_time)})\n"
                    f"observe baseline_drain({num(baseline_drain)})\n"
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
            "ADJ-LADDER rung 81 — cardiology cardiac-output perfusion index from four stated quantities (a NEW panel: "
            "cardiology / cardiac-output). From two inflow volumes to pool, a circulation time to divide the pooled sum "
            "by, and a baseline drain to subtract, compute the perfusion index "
            "((forward_volume+collateral_volume)/circulation_time - baseline_drain), the combined flow "
            "((forward_volume+collateral_volume)/circulation_time), or the combined volume "
            "(forward_volume+collateral_volume). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, GROUPED-SUM "
            "QUOTIENT MINUS A TERM (a+b)/c-d (form the parenthesised sum, divide by c, then subtract d, so (a+b)/c-d = "
            "((a+b)/c)-d; the trailing-subtraction sibling of rung-76 (a+b)/c*d which multiplied the quotient by d) — "
            "and the harness matches the scalar to the printed options. Contamination-safe: every index is built only "
            "from the four observed quantities via +, /, and - — no constant leaks, and neither the combined flow, the "
            "combined volume, nor any index figure ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same four quantities, so the distractors are exactly the slips students make: subtracting "
            "the baseline drain INSIDE the denominator ((a+b)/(c-d), a wrong grouping) and flipping the pooled SUM to a "
            "difference ((a-b)/c-d, a wrong grouping). The core confusion tested is that (a+b)/c-d is ((a+b)/c)-d, not "
            "(a+b)/(c-d) and not (a-b)/c-d."
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
