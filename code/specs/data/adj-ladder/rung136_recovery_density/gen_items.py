"""Generate rung-136 (recovery density / a DIFFERENCE-numerator over a SUM — divide a net amount by a pooled total) items.json.

Rung 136 opens the **recovery** panel and **completes the over-a-sum trio**. rung-132 put a QUOTIENT over a sum, `(a/b)/(c+d)`; rung-135
put a PRODUCT over a sum, `(a*b)/(c+d)`; rung-136 puts a DIFFERENCE over a sum, `(a-b)/(c+d)`. The combining operation in the numerator now
walks quotient, product, difference over the same divide-by-a-sum skeleton — the sum-denominator mirror of the over-a-rate family
(131 sum, 133 difference, 134 product over `c/d`).

This is genuinely new. `(a-b)/(c+d)` is a DIFFERENCE `a-b` divided by a SUM `c+d`. The difference `a-b` binds and stays grouped over the bar
(grouping), and the two-part denominator `c+d` is ONE pooled total that the whole numerator is divided by (not two separate divisors, and
not distributed across). The core confusions this rung tests are the two canonical divide-by-a-sum slips: distributing the division across
the sum (`(a-b)/(c+d)` treated as `(a-b)/c + (a-b)/d`, which is FALSE because `x/(c+d) != x/c + x/d`), and dropping the grouping on the
denominator so only the first part divides and the second is added on (`(a-b)/c + d`).

The setup: an `intake_total` from which a `spillage` is lost (a net recovery `intake_total - spillage`), spread across a pooled capacity
formed from a `holding_span` plus a `surge_span` (a pooled capacity `holding_span + surge_span`). The figures are:

  RECOVERY DENSITY  (intake_total - spillage) / (holding_span + surge_span)  [ difference-numerator OVER a sum: net recovery / pooled capacity ]
  NET RECOVERY      intake_total - spillage                                  [ the difference numerator (divided by the pooled capacity) ]
  POOLED CAPACITY   holding_span + surge_span                               [ the pooled sum the net recovery is divided by ]

The **recovery density** is the ladder's first **(a difference) over (a sum) as a headline** — a density (how much net recovery sits in
each unit of pooled capacity), framed as a *density* to keep it dimensionless-clean, the same discipline rungs 100/.../134/135 used for
their ratios, spans, and concentrations. (The net recovery `a-b` and the pooled capacity `c+d` ride alongside as component readouts, so the
panel teaches the whole calculation — exactly as rungs 47-135 shipped their component figures beside the headline. The two components anchor
the "subtract the loss FIRST, pool the capacity, then divide the net by the pooled capacity" structure against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the subtraction to form the net recovery, the addition to form the pooled capacity, then the division of the net recovery by
the pooled capacity to form the compound figure (so (a-b)/(c+d) evaluates as ((a-b)/(c+d))) — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../134/135. This rung exercises the engine across
a **difference divided by a sum** — the fact that `(a-b)/(c+d)` is one difference over one pooled total and NOT `(a-b)/c + (a-b)/d` and NOT
`(a-b)/c + d` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same
way rungs 100/.../134/135 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `/`, and `+` — **no structural
constants** — so no numeric literal appears in any program, and neither the net recovery, the pooled capacity, nor the recovery density is
ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`intake_total`,
`spillage`, `holding_span`, `surge_span`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SPLIT      (intake_total - spillage) / holding_span + (intake_total - spillage) / surge_span   distribute the division across the sum,
                                                                treating `x/(c+d)` as `x/c + x/d` (FALSE — division does not distribute
                                                                over a sum in the denominator), and
  FLAT       (intake_total - spillage) / holding_span + surge_span   divide the net recovery by the holding span ONLY and then add the surge
                                                                span, dropping the grouping on the denominator so the second part is added
                                                                on instead of pooled into the divisor (`(a-b)/c + d`),

which are exactly the mistakes a student makes (splitting one divisor into two, or losing the parentheses on the pooled denominator). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung has a SUBTRACTION in the numerator, so unlike the pure `* / +` rungs it needs a **positivity guard**
— the binding difference is guarded so the net recovery stays positive: `intake_total - spillage >= 2`. With that guard and positive spans,
every family member is positive (the split and flat distractors are sums/quotients of positive quantities). Every observed quantity is
`>= 2`. Every family member is asserted `> 0` at build time as a belt-and-suspenders check. The seven tables give distinct recovery
densities, distinct net recoveries, and distinct pooled capacities so all three queried readouts vary across the panel; the five family
values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (INTAKE_TOTAL, SPILLAGE, HOLDING_SPAN, SURGE_SPAN) — a net recovery (intake_total - spillage) divided by a pooled capacity
# (holding_span + surge_span), giving the recovery density as a difference over a sum (a-b)/(c+d). This rung has a SUBTRACTION in the
# numerator, so the binding difference is guarded (intake_total - spillage >= 2) to keep the net recovery positive; with positive spans every
# figure is positive. The seven tables give distinct net recoveries (a-b), distinct pooled capacities (c+d), and distinct recovery densities
# ((a-b)/(c+d)); the five family values are asserted pairwise-distinct below.
TABLES = [
    (8, 2, 4, 5),      # net = 6,  pool = 9,  density = 0.666...
    (11, 3, 3, 7),     # net = 8,  pool = 10, density = 0.8
    (14, 4, 4, 7),     # net = 10, pool = 11, density = 0.909...
    (16, 4, 5, 3),     # net = 12, pool = 8,  density = 1.5
    (19, 5, 3, 4),     # net = 14, pool = 7,  density = 2.0
    (12, 3, 4, 9),     # net = 9,  pool = 13, density = 0.692...
    (22, 6, 3, 2),     # net = 16, pool = 5,  density = 3.2
]

# The option family (5 members), all built from the four observed quantities via -, /, and +. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "recovery_density",
        "recovery density (the net recovery divided by the pooled capacity)",
        "(intake_total - spillage) / (holding_span + surge_span)",
    ),
    (
        "net_recovery",
        "the net recovery (the intake total minus the spillage, the numerator that is divided by the pooled capacity)",
        "intake_total - spillage",
    ),
    (
        "pooled_capacity",
        "the pooled capacity (the holding span plus the surge span, the pooled total the net recovery is divided by)",
        "holding_span + surge_span",
    ),
    (
        "split",
        "the net recovery divided by the holding span plus the net recovery divided by the surge span, distributing the division across the sum instead of pooling the capacity first (a wrong operation)",
        "(intake_total - spillage) / holding_span + (intake_total - spillage) / surge_span",
    ),
    (
        "flat",
        "the net recovery divided by the holding span and then the surge span added on, dropping the grouping so the second span is added instead of pooled into the divisor (a wrong operation)",
        "(intake_total - spillage) / holding_span + surge_span",
    ),
]
QUERIED = ["recovery_density", "net_recovery", "pooled_capacity"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(intake_total, spillage, holding_span, surge_span):
    # Operation order mirrors the ADJ programs exactly (the subtraction forms the net recovery, the addition forms the pooled capacity, then
    # the net recovery is divided by the pooled capacity to form the compound figure, so (a-b)/(c+d) evaluates as ((a-b)/(c+d))), so the
    # Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    net = intake_total - spillage
    pool = holding_span + surge_span
    return {
        "recovery_density": net / pool,
        "net_recovery": net,
        "pooled_capacity": pool,
        "split": net / holding_span + net / surge_span,
        "flat": net / holding_span + surge_span,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for intake_total, spillage, holding_span, surge_span in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung has a subtraction in the numerator, so the binding difference
        # is guarded (intake_total - spillage >= 2) to keep the net recovery positive; with positive spans every figure is positive.
        assert (
            intake_total >= 2
            and spillage >= 2
            and holding_span >= 2
            and surge_span >= 2
        ), (intake_total, spillage, holding_span, surge_span)
        assert intake_total - spillage >= 2, (intake_total, spillage)
        fv = family_values(intake_total, spillage, holding_span, surge_span)
        for key, v in fv.items():
            assert v > 0, (key, intake_total, spillage, holding_span, surge_span, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    intake_total,
                    spillage,
                    holding_span,
                    surge_span,
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
                intake_total,
                spillage,
                holding_span,
                surge_span,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r136rda-{idx + 1:02d}",
                "qtype": "recovery_density",
                "stem": (
                    f"A recovery study records an intake total of {num(intake_total)} with a spillage of "
                    f"{num(spillage)}, spread across a holding span of {num(holding_span)} plus a surge span of "
                    f"{num(surge_span)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe intake_total({num(intake_total)})\n"
                    f"observe spillage({num(spillage)})\n"
                    f"observe holding_span({num(holding_span)})\n"
                    f"observe surge_span({num(surge_span)})\n"
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
            "ADJ-LADDER rung 136 — recovery density from four stated quantities (a NEW panel: recovery, COMPLETING the over-a-sum trio). "
            "rung-132 put a quotient over a sum (a/b)/(c+d); rung-135 put a product over a sum (a*b)/(c+d); rung-136 puts a DIFFERENCE "
            "over a sum (a-b)/(c+d) — the combining op walks quotient, product, difference over the same divide-by-a-sum skeleton, the "
            "sum-denominator mirror of the over-a-rate family (131 sum, 133 difference, 134 product over (c/d)). From a net recovery "
            "(intake_total - spillage) divided by a pooled capacity (holding_span + surge_span), compute the recovery density "
            "((intake_total-spillage)/(holding_span+surge_span)), the net recovery (intake_total-spillage), or the pooled capacity "
            "(holding_span+surge_span). Each item is a compute_dimensioned program (observe the four quantities, let answer = formula); "
            "the ADJ engine carries the arithmetic — a NEW family, a DIFFERENCE NUMERATOR OVER A SUM (a-b)/(c+d) (subtract the loss, pool "
            "the capacity, then divide the net by the pooled capacity — the two-part denominator is ONE total, not two divisors). The "
            "divide-by-a-sum slips ride alongside as distractors. The harness matches the scalar to the printed options. The recovery "
            "density is a density (how much net recovery sits in each unit of pooled capacity), framed as a DENSITY so the dimensionless "
            "value stays honest. Contamination-safe: every figure is built only from the four observed quantities via -, /, and + — no "
            "constant leaks, and neither the net recovery, the pooled capacity, nor the recovery density ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same four quantities, so the distractors are exactly the slips students make: distributing the "
            "division across the sum ((a-b)/c + (a-b)/d, FALSE because x/(c+d) != x/c + x/d, a wrong operation) and dropping the grouping "
            "so the second span is added on instead of pooled ((a-b)/c + d, a wrong operation). The core confusion tested is that "
            "(a-b)/(c+d) is one difference over one pooled total, not (a-b)/c + (a-b)/d and not (a-b)/c + d. This rung has a subtraction "
            "in the numerator, so the binding difference is guarded (intake_total - spillage >= 2) to keep the net recovery positive; the "
            "five family values are kept pairwise distinct with all three queried readouts varying across the panel, all asserted strictly "
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
