"""Generate rung-49 (fractionated radiotherapy total dose) items.json for the ADJ-LADDER.

Rung 49 opens the **radiation-oncology / fractionated-dose** panel on the quantitative band — the arithmetic of the
total radiation dose a course delivers when the same per-fraction dose is given across the fractions of two treatment
phases. The dose delivered in a course is the per-fraction dose multiplied by the TOTAL number of fractions, and the
total fractions is the SUM of the two phases (`fractions_one + fractions_two`). Multiplying the single per-fraction
dose by that parenthesised sum introduces a genuinely NEW arithmetic shape on the ladder: **product-over-a-sum in the
DISTRIBUTIVE sense** — `a · (b + c)` — one quantity multiplying a parenthesised sum of two.

The setup: a course delivers `dose_per_fraction` Gy per fraction, given for `fractions_one` fractions in the first
phase and `fractions_two` fractions in the second. The total dose is the per-fraction dose times the total fractions:

  TOTAL DOSE        dose_per_fraction · (fractions_one + fractions_two)   [ Gy — the whole course ]
  TOTAL FRACTIONS   fractions_one + fractions_two                         [ the summed count ]
  PHASE-ONE DOSE    dose_per_fraction · fractions_one                     [ one distributed term: the first phase ]

The **total dose** is what makes this rung distinctive — it is the ladder's first **distributive product-over-a-sum**:
a single factor multiplying a parenthesised sum. Contrast the neighbours already on the ladder: rung-34 was a *sum of
two products* `a·b + c·d`, rung-43 a *sum of three products*, rung-48 a *sum times a difference* `(a+b)·(c−d)`; none
multiplied a lone factor by a parenthesised SUM. (The total fractions `fractions_one + fractions_two` and the
phase-one dose `dose_per_fraction · fractions_one` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 46/47/48 shipped their component sums/products beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the three quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — including the inner `(fractions_one + fractions_two)` sum — and the harness reads the
scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../47/48.
This rung exercises the engine across a **single factor times a parenthesised sum** — the distributive law
`a·(b+c) = a·b + a·c` made computable.

Contamination-safe by construction: every formula is built ONLY from the three observed quantities via `·`, `+` and
`−` — **no structural constants** — so no numeric literal appears in any program, and neither the total fractions, the
phase-one dose, nor any total-dose figure is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`dose_per_fraction`, `fractions_one`, `fractions_two`) so no
numeral hides inside a variable name.

The five options are a tight family over the same three quantities: the three real readouts plus the two classic
slips —

  DIFF VERSION   dose_per_fraction · (fractions_one − fractions_two)   SUBTRACT the two phases' fractions instead of
                                                                       adding them, and
  MISGROUPED     dose_per_fraction · fractions_one + fractions_two     forget to distribute over the second phase — add
                                                                       the raw second-phase COUNT instead of the dose
                                                                       it delivers (`+ fractions_two`, not
                                                                       `+ dose_per_fraction · fractions_two`),

which are exactly the mistakes a student makes (subtracting quantities that should be added, or breaking the
distributive law by not scaling the second term). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness: all three observed quantities are positive with `fractions_one > fractions_two`, so every sum, product
and difference is positive; the tables below are chosen so the five family values are pairwise distinct with a
comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (DOSE_PER_FRACTION, FRACTIONS_ONE, FRACTIONS_TWO) — dose in Gy per fraction, fractions are plain positive counts.
# All three quantities are strictly positive and FRACTIONS_ONE > FRACTIONS_TWO, so the phase difference is positive.
# The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (3, 15, 10),
    (2, 20, 12),
    (4, 12, 5),
    (5, 8, 6),
    (3, 18, 8),
    (2, 25, 15),
    (6, 9, 4),
]

# The option family (5 members), all built from the three observed quantities via *, + and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "total_dose",
        "total course dose (the per-fraction dose times the total number of fractions)",
        "dose_per_fraction * (fractions_one + fractions_two)",
    ),
    (
        "total_fractions",
        "the total number of fractions (the two phases added)",
        "fractions_one + fractions_two",
    ),
    (
        "phase_one_dose",
        "the first phase's dose (the per-fraction dose times the first phase's fractions)",
        "dose_per_fraction * fractions_one",
    ),
    (
        "diff_version",
        "per-fraction dose times the two phases SUBTRACTED, not added (a wrong total)",
        "dose_per_fraction * (fractions_one - fractions_two)",
    ),
    (
        "misgrouped",
        "first phase's dose plus the raw second-phase COUNT, forgetting to scale it (broken distribution)",
        "dose_per_fraction * fractions_one + fractions_two",
    ),
]
QUERIED = ["total_dose", "total_fractions", "phase_one_dose"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(dose_per_fraction, fractions_one, fractions_two):
    # Operation order mirrors the ADJ programs exactly (a single factor times a parenthesised sum), so the Python
    # option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    total_fractions = fractions_one + fractions_two
    return {
        "total_dose": dose_per_fraction * total_fractions,
        "total_fractions": total_fractions,
        "phase_one_dose": dose_per_fraction * fractions_one,
        "diff_version": dose_per_fraction * (fractions_one - fractions_two),
        "misgrouped": dose_per_fraction * fractions_one + fractions_two,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for dose_per_fraction, fractions_one, fractions_two in TABLES:
        assert (
            dose_per_fraction > 0
            and fractions_one > 0
            and fractions_two > 0
            and fractions_one > fractions_two
        ), (dose_per_fraction, fractions_one, fractions_two)
        fv = family_values(dose_per_fraction, fractions_one, fractions_two)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    dose_per_fraction,
                    fractions_one,
                    fractions_two,
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
                dose_per_fraction,
                fractions_one,
                fractions_two,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r49frx-{idx + 1:02d}",
                "qtype": "fractionated_dose",
                "stem": (
                    f"A radiotherapy course delivers {num(dose_per_fraction)} Gy per fraction, given for "
                    f"{num(fractions_one)} fractions in the first phase and {num(fractions_two)} in the second. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe dose_per_fraction({num(dose_per_fraction)})\n"
                    f"observe fractions_one({num(fractions_one)})\n"
                    f"observe fractions_two({num(fractions_two)})\n"
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
            "ADJ-LADDER rung 49 — fractionated radiotherapy total dose from three stated quantities (a NEW panel: "
            "radiation-oncology / fractionated-dose). From a per-fraction dose plus the fraction counts of two "
            "treatment phases compute the total course dose (dose_per_fraction*(fractions_one+fractions_two)), the "
            "total fractions (fractions_one+fractions_two), or the first phase's dose "
            "(dose_per_fraction*fractions_one). Each item is a compute_dimensioned program (observe the three "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, DISTRIBUTIVE "
            "PRODUCT-OVER-A-SUM a*(b+c), the first product on the ladder to multiply a single factor by a "
            "parenthesised sum (distinct from rung-34 sum-of-two-products a*b+c*d, rung-43 sum-of-three-products, and "
            "rung-48 sum-times-difference (a+b)*(c-d)) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the three observed quantities via *, + and - — no "
            "constant leaks, and neither the total fractions, the phase-one dose, nor any total-dose figure ever "
            "appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. The five options are a family over the same three quantities, so the "
            "distractors are exactly the slips students make: SUBTRACTING the two phases instead of adding them, and "
            "breaking the distributive law by adding the raw second-phase count instead of the dose it delivers "
            "(a*b+c, not a*(b+c)). The core confusion tested is distributing the per-fraction dose over the summed "
            "fraction count."
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
