"""Generate rung-55 (anesthesia fresh-gas volume) items.json for the ADJ-LADDER.

Rung 55 opens the **anesthesia / fresh-gas-delivery** panel on the quantitative band — the arithmetic of how much fresh
gas an anesthesia machine delivers over a whole case. The gas comes from two sources (oxygen and air) whose flows add
to a combined fresh-gas flow, and the case runs through two phases (induction and maintenance) whose minutes add to the
total time; the total gas delivered is the combined flow times the total time. Multiplying a sum of two by ANOTHER sum
of two introduces a genuinely NEW arithmetic shape on the ladder: a **product of two sums** — `(a + b) · (c + d)` — the
classic FOIL, two parenthesised sums multiplied.

The setup: an anesthesia machine runs `oxygen_flow` of oxygen and `air_flow` of air (litres per minute), for an
induction phase of `induction_minutes` and a maintenance phase of `maintenance_minutes`. The total fresh gas is the
combined flow times the total time:

  TOTAL GAS       (oxygen_flow + air_flow) · (induction_minutes + maintenance_minutes)   [ litres — the whole delivery ]
  COMBINED FLOW   oxygen_flow + air_flow                                                 [ one sum: the fresh-gas flow ]
  TOTAL TIME      induction_minutes + maintenance_minutes                                [ the other sum: the case time ]

The **total gas** is what makes this rung distinctive — it is the ladder's first **product of two sums**: two
parenthesised sums multiplied. Contrast the neighbours already on the ladder: rung-48 was a *sum times a difference*
`(a+b)·(c−d)` and rung-37 a *ratio of two sums* `(a+b)/(c+d)`; neither multiplied a sum by a SUM. (The combined flow
`oxygen_flow + air_flow` and the total time `induction_minutes + maintenance_minutes` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-54 shipped their component
sums/products/differences beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including both parenthesised sums and their product — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../53/54. This rung exercises
the engine across a **product of two parenthesised sums** — the distributive law `(a+b)·(c+d) = ac+ad+bc+bd` made
computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `·` —
**no structural constants** — so no numeric literal appears in any program, and neither the combined flow, the total
time, nor any total-gas figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`oxygen_flow`, `air_flow`, `induction_minutes`, `maintenance_minutes`) so
no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SUM VERSION   (oxygen_flow + air_flow) + (induction_minutes + maintenance_minutes)   ADD the two sums instead of
                                                                                       multiplying them, and
  MISGROUPED    (oxygen_flow + air_flow) · induction_minutes + maintenance_minutes      multiply the flow by only the
                                                                                       INDUCTION minutes and then add the
                                                                                       maintenance minutes on
                                                                                       (`… · induction + maintenance`,
                                                                                       not `… · (induction +
                                                                                       maintenance)`),

which are exactly the mistakes a student makes (adding two totals that should be multiplied, or breaking the grouping so
the flow multiplies only the first phase). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts;
all five always appear as options.

Distinctness: all four observed quantities are strictly positive, so every sum and product is positive; the tables
below are chosen so the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (OXYGEN_FLOW, AIR_FLOW, INDUCTION_MINUTES, MAINTENANCE_MINUTES) — two gas-source flows (L/min) and two case phases
# (min), all plain positive numbers. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (2, 3, 10, 20),
    (1, 4, 15, 25),
    (3, 2, 8, 32),
    (2, 2, 12, 18),
    (4, 1, 20, 10),
    (1, 2, 25, 35),
    (3, 3, 9, 21),
]

# The option family (5 members), all built from the four observed quantities via + and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "total_gas",
        "total fresh gas delivered (the combined flow times the total case time)",
        "(oxygen_flow + air_flow) * (induction_minutes + maintenance_minutes)",
    ),
    (
        "combined_flow",
        "the combined fresh-gas flow (oxygen plus air)",
        "oxygen_flow + air_flow",
    ),
    (
        "total_time",
        "the total case time (induction plus maintenance minutes)",
        "induction_minutes + maintenance_minutes",
    ),
    (
        "sum_version",
        "the combined flow and the total time ADDED, not multiplied (a wrong total)",
        "(oxygen_flow + air_flow) + (induction_minutes + maintenance_minutes)",
    ),
    (
        "misgrouped",
        "the flow times only the INDUCTION minutes, with maintenance minutes added on (broken grouping)",
        "(oxygen_flow + air_flow) * induction_minutes + maintenance_minutes",
    ),
]
QUERIED = ["total_gas", "combined_flow", "total_time"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(oxygen_flow, air_flow, induction_minutes, maintenance_minutes):
    # Operation order mirrors the ADJ programs exactly (a product of two parenthesised sums; and, for the misgrouped
    # slip, the flow-times-induction product binds tighter than the trailing addition), so the Python option value and
    # the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    flow = oxygen_flow + air_flow
    time = induction_minutes + maintenance_minutes
    return {
        "total_gas": flow * time,
        "combined_flow": flow,
        "total_time": time,
        "sum_version": flow + time,
        "misgrouped": flow * induction_minutes + maintenance_minutes,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for oxygen_flow, air_flow, induction_minutes, maintenance_minutes in TABLES:
        assert (
            oxygen_flow > 0
            and air_flow > 0
            and induction_minutes > 0
            and maintenance_minutes > 0
        ), (oxygen_flow, air_flow, induction_minutes, maintenance_minutes)
        fv = family_values(oxygen_flow, air_flow, induction_minutes, maintenance_minutes)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    oxygen_flow,
                    air_flow,
                    induction_minutes,
                    maintenance_minutes,
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
                oxygen_flow,
                air_flow,
                induction_minutes,
                maintenance_minutes,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r55gas-{idx + 1:02d}",
                "qtype": "fresh_gas_volume",
                "stem": (
                    f"An anesthesia machine runs {num(oxygen_flow)} L/min of oxygen and {num(air_flow)} L/min of air, "
                    f"for a {num(induction_minutes)}-minute induction and a {num(maintenance_minutes)}-minute "
                    f"maintenance phase. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe oxygen_flow({num(oxygen_flow)})\n"
                    f"observe air_flow({num(air_flow)})\n"
                    f"observe induction_minutes({num(induction_minutes)})\n"
                    f"observe maintenance_minutes({num(maintenance_minutes)})\n"
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
            "ADJ-LADDER rung 55 — anesthesia fresh-gas volume from four stated quantities (a NEW panel: anesthesia / "
            "fresh-gas-delivery). From two gas-source flows (oxygen, air) and two case phases (induction, maintenance) "
            "compute the total fresh gas ((oxygen_flow+air_flow)*(induction_minutes+maintenance_minutes)), the combined "
            "flow (oxygen_flow+air_flow), or the total time (induction_minutes+maintenance_minutes). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW shape, PRODUCT OF TWO SUMS (a+b)*(c+d), the first on the ladder to multiply a sum "
            "by another sum (distinct from rung-48 sum-times-difference (a+b)*(c-d) and rung-37 ratio-of-two-sums "
            "(a+b)/(c+d)) — and the harness matches the scalar to the printed options. Contamination-safe: every index "
            "is built only from the four observed quantities via + and * — no constant leaks, and neither the combined "
            "flow, the total time, nor any total-gas figure ever appears as a literal (each is computed) — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same four quantities, so the distractors are exactly the slips students "
            "make: ADDING the two sums instead of multiplying them, and breaking the grouping so the flow multiplies "
            "only the induction phase ((a+b)*c+d, not (a+b)*(c+d)). The core confusion tested is multiplying two "
            "grouped sums."
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
