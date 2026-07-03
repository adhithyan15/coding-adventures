"""Generate rung-62 (urodynamics average flow) items.json for the ADJ-LADDER.

Rung 62 opens the **urology / urodynamics** panel on the quantitative band — the arithmetic of a urodynamic study.
A study voids a `voided_volume` over a total study time made of a `fill_minutes` filling phase PLUS a `void_minutes`
voiding phase; the average flow over the whole study is the voided volume divided by the SUM of the two phase times.
Dividing a lone quantity by the SUM of two others introduces a genuinely NEW arithmetic shape on the ladder: **one
over a sum** — `a/(b+c)` — the mirror of rung-61's one-over-a-difference a/(b-c), with the two denominator terms ADDED
instead of subtracted.

The setup: the study records `voided_volume` mL over `fill_minutes` of filling and `void_minutes` of voiding. The
average flow over the whole study is:

  AVERAGE FLOW    voided_volume / (fill_minutes + void_minutes)   [ volume per minute over the WHOLE study ]
  STUDY MINUTES   fill_minutes + void_minutes                     [ the denominator: total study time ]
  FILL RATE       voided_volume / fill_minutes                    [ flow over the FILL phase only ]

The **average flow** is what makes this rung distinctive — it is the ladder's first **one over a sum**: a lone
quantity divided by a sum of two others. Contrast the neighbour already on the ladder: rung-61 was `a/(b-c)` (one over
a DIFFERENCE); this ADDS the two denominator terms. (The total study time `fill_minutes+void_minutes` and the fill-only
rate `voided_volume/fill_minutes` ride alongside as component readouts, so the panel teaches the whole calculation —
exactly as rungs 47-61 shipped their component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the three quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the numerator, the parenthesised sum, and their quotient — and the harness reads the scalar via
the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../60/61. This rung
exercises the engine across **a division whose divisor is itself a sum** — the fact that `a/(b+c)` is NOT `a/b + a/c`
(division does not distribute over a sum) and NOT `a/(b-c)` made computable.

Contamination-safe by construction: every formula is built ONLY from the three observed quantities via `/` and `+` —
**no structural constants** — so no numeric literal appears in any program, and neither the total study time, the fill
rate, nor any average-flow figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`voided_volume`, `fill_minutes`, `void_minutes`) so no numeral hides inside
a variable name.

The five options are a tight family over the same three quantities: the three real readouts plus the two classic slips —

  DISTRIBUTED   voided_volume / fill_minutes + voided_volume / void_minutes   DISTRIBUTE the division over the sum
                                                                              (the classic `a/(b+c)` vs `a/b + a/c`
                                                                              error — division does NOT distribute), and
  CROSSED       voided_volume / void_minutes                                  divide by the VOID minutes only (the wrong
                                                                              single denominator),

which are exactly the mistakes a student makes (splitting the fraction across the sum, or dividing by only one phase).
Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness: all three observed quantities are strictly positive and every formula uses only `/` and `+`, so every
family member is positive by construction; the five family values are pairwise distinct with a comfortable margin,
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (VOIDED_VOLUME, FILL_MINUTES, VOID_MINUTES) — a voided volume and two timed minutes (the filling and voiding phases),
# all plain positive numbers. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (300, 40, 20),
    (450, 60, 30),
    (240, 30, 50),
    (600, 80, 40),
    (180, 20, 40),
    (360, 90, 30),
    (500, 50, 75),
]

# The option family (5 members), all built from the three observed quantities via / and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "average_flow",
        "average flow over the whole study (voided volume over the total fill-plus-void time)",
        "voided_volume / (fill_minutes + void_minutes)",
    ),
    (
        "study_minutes",
        "the total study time (fill minutes plus void minutes)",
        "fill_minutes + void_minutes",
    ),
    (
        "fill_rate",
        "the flow over the FILL phase only (voided volume per fill minute)",
        "voided_volume / fill_minutes",
    ),
    (
        "distributed",
        "the two phase rates ADDED, as if the division split over the sum (a wrong average)",
        "voided_volume / fill_minutes + voided_volume / void_minutes",
    ),
    (
        "crossed",
        "voided volume divided by the VOID minutes only (wrong single denominator)",
        "voided_volume / void_minutes",
    ),
]
QUERIED = ["average_flow", "study_minutes", "fill_rate"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(voided_volume, fill_minutes, void_minutes):
    # Operation order mirrors the ADJ programs exactly (the parenthesised sum formed first, then the division; and, for
    # the distributed slip, each quotient formed then added), so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "average_flow": voided_volume / (fill_minutes + void_minutes),
        "study_minutes": fill_minutes + void_minutes,
        "fill_rate": voided_volume / fill_minutes,
        "distributed": voided_volume / fill_minutes + voided_volume / void_minutes,
        "crossed": voided_volume / void_minutes,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for voided_volume, fill_minutes, void_minutes in TABLES:
        assert voided_volume > 0 and fill_minutes > 0 and void_minutes > 0, (
            voided_volume,
            fill_minutes,
            void_minutes,
        )
        fv = family_values(voided_volume, fill_minutes, void_minutes)
        # Every family member is positive by construction (only / and + over positive quantities); assert it.
        for key, v in fv.items():
            assert v > 0, (key, voided_volume, fill_minutes, void_minutes, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    voided_volume,
                    fill_minutes,
                    void_minutes,
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
                voided_volume,
                fill_minutes,
                void_minutes,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r62flow-{idx + 1:02d}",
                "qtype": "urodynamics_average_flow",
                "stem": (
                    f"A urodynamic study voids {num(voided_volume)} mL over {num(fill_minutes)} min of filling and "
                    f"{num(void_minutes)} min of voiding. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe voided_volume({num(voided_volume)})\n"
                    f"observe fill_minutes({num(fill_minutes)})\n"
                    f"observe void_minutes({num(void_minutes)})\n"
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
            "ADJ-LADDER rung 62 — average urine flow from three stated quantities (a NEW panel: urology / urodynamics). "
            "From a voided volume, the fill minutes, and the void minutes (the study time is fill plus void), compute "
            "the average flow (voided_volume/(fill_minutes+void_minutes)), the study minutes "
            "(fill_minutes+void_minutes), or the fill-only rate (voided_volume/fill_minutes). Each item is a "
            "compute_dimensioned program (observe the three quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW shape, ONE OVER A SUM a/(b+c), the mirror of rung-61 one-over-a-difference a/(b-c) "
            "with the two denominator terms ADDED — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the three observed quantities via / and + — no constant "
            "leaks, and neither the study time, the fill rate, nor any average-flow figure ever appears as a literal "
            "(each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a "
            "variable name. The five options are a family over the same three quantities, so the distractors are exactly "
            "the slips students make: DISTRIBUTING the division over the sum (a/b + a/c, since division does NOT "
            "distribute) and dividing by the VOID minutes only (a/c). The core confusion tested is that a/(b+c) is not "
            "a/b + a/c and not a/(b-c)."
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
