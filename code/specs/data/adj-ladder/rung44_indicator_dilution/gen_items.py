"""Generate rung-44 (indicator-dilution mixed concentration) items.json for the ADJ-LADDER.

Rung 44 opens the **indicator / tracer dilution** panel on the quantitative band — the arithmetic of what a
tracer's concentration becomes once a known bolus of it distributes through a combined blood volume. A bolus
carries a fixed amount of indicator (`indicator_volume * indicator_concentration` of dye); when that amount
spreads through the sum of two compartments (`central_volume + peripheral_volume`), the resulting mixed
concentration is the bolus amount divided by the total distribution volume. This rung introduces a genuinely NEW
arithmetic shape on the ladder: **product-over-sum** — `(a * b) / (c + d)` — a product in the numerator divided by
a sum in the denominator.

The setup: an indicator bolus of `indicator_volume` (mL) at `indicator_concentration` (mg/mL) is injected and
distributes through a `central_volume` (L) plus a `peripheral_volume` (L). The mixed concentration is the injected
amount over the volume it distributes through:

  MIXED CONCENTRATION   (indicator_volume * indicator_concentration) / (central_volume + peripheral_volume)   [ the concentration after distribution ]
  INDICATOR MASS        indicator_volume * indicator_concentration                                            [ the numerator: injected amount ]
  DISTRIBUTION VOLUME   central_volume + peripheral_volume                                                    [ the denominator: total volume ]

The **mixed concentration** is what makes this rung distinctive — it is the ladder's first **product-over-sum**: a
parenthesised product divided by a parenthesised sum. Contrast the neighbours already on the ladder: rung-37 was a
*ratio of two sums* `(a+b)/(c+d)`, rung-41 a *difference-over-sum* `(a-b)/(a+b)`, rung-42 a *sum-over-difference*
`(a+b)/(c-d)`; none divided a PRODUCT by a SUM. (The indicator mass `indicator_volume * indicator_concentration`
and the distribution volume `central_volume + peripheral_volume` ride alongside as the two component quantities, so
the panel teaches the whole calculation — exactly as rung-42 shipped its total-solute and remaining-volume beside
the headline concentration.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — including the inner `(indicator_volume * indicator_concentration)` product and the
`(central_volume + peripheral_volume)` sum — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../42/43. This rung exercises the
engine across a **product divided by a sum**.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `+` and
`/` — **no structural constants** — so no numeric literal appears in any program, and neither the injected mass,
the distribution volume, nor any concentration is ever a literal (each is computed from the observed quantities).
The observed quantities carry **digit-free identifiers** (`indicator_volume`, `indicator_concentration`,
`central_volume`, `peripheral_volume`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic
slips —

  CENTRAL ONLY       (indicator_volume * indicator_concentration) / central_volume        divide by the CENTRAL
                                                                                            compartment alone,
                                                                                            forgetting the
                                                                                            peripheral one, and
  SUMMED NUMERATOR   (indicator_volume + indicator_concentration) / (central_volume + peripheral_volume)   ADD the
                                                                                            two indicator quantities
                                                                                            instead of multiplying
                                                                                            them,

which are exactly the mistakes a student makes (dropping a compartment from the denominator, or adding the bolus's
volume and concentration instead of forming their product). Gold rotates A-E by index. QUERIED (used as gold) = the
three real readouts; all five always appear as options.

Distinctness: all four observed quantities are positive, so every product and sum is positive and every denominator
is strictly positive (no division by zero); the tables below are chosen so the five family values are pairwise
distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (INDICATOR_VOLUME, INDICATOR_CONCENTRATION, CENTRAL_VOLUME, PERIPHERAL_VOLUME) — bolus volume in mL, bolus
# concentration in mg/mL, compartment volumes in L, all strictly positive. The five family values are asserted
# pairwise-distinct (with margin) below.
TABLES = [
    (2, 10, 3, 2),
    (4, 5, 3, 5),
    (2, 20, 4, 4),
    (5, 4, 3, 2),
    (3, 10, 2, 4),
    (2, 25, 5, 3),
    (4, 10, 6, 2),
]

# The option family (5 members), all built from the four observed quantities via *, + and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "mixed_concentration",
        "mixed concentration after distribution (injected amount over the total volume)",
        "(indicator_volume * indicator_concentration) / (central_volume + peripheral_volume)",
    ),
    (
        "indicator_mass",
        "the injected indicator amount (bolus volume times its concentration)",
        "indicator_volume * indicator_concentration",
    ),
    (
        "distribution_volume",
        "the total distribution volume (central plus peripheral)",
        "central_volume + peripheral_volume",
    ),
    (
        "central_only",
        "injected amount over the CENTRAL compartment alone, forgetting the peripheral one (a wrong denominator)",
        "(indicator_volume * indicator_concentration) / central_volume",
    ),
    (
        "summed_numerator",
        "the two indicator quantities ADDED instead of multiplied, over the total volume (a wrong numerator)",
        "(indicator_volume + indicator_concentration) / (central_volume + peripheral_volume)",
    ),
]
QUERIED = ["mixed_concentration", "indicator_mass", "distribution_volume"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(indicator_volume, indicator_concentration, central_volume, peripheral_volume):
    # Operation order mirrors the ADJ programs exactly (product binds tighter than +, division over the
    # parenthesised sum), so the Python option value and the engine result are the same IEEE-double (well within
    # the harness's 1e-9 match tolerance).
    mass = indicator_volume * indicator_concentration
    total = central_volume + peripheral_volume
    return {
        "mixed_concentration": mass / total,
        "indicator_mass": mass,
        "distribution_volume": total,
        "central_only": mass / central_volume,
        "summed_numerator": (indicator_volume + indicator_concentration) / total,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for indicator_volume, indicator_concentration, central_volume, peripheral_volume in TABLES:
        assert all(
            q > 0
            for q in (indicator_volume, indicator_concentration, central_volume, peripheral_volume)
        ), (indicator_volume, indicator_concentration, central_volume, peripheral_volume)
        fv = family_values(indicator_volume, indicator_concentration, central_volume, peripheral_volume)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    indicator_volume,
                    indicator_concentration,
                    central_volume,
                    peripheral_volume,
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
                indicator_volume,
                indicator_concentration,
                central_volume,
                peripheral_volume,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r44dil-{idx + 1:02d}",
                "qtype": "indicator_dilution",
                "stem": (
                    f"A tracer bolus of {num(indicator_volume)} mL at {num(indicator_concentration)} mg/mL is "
                    f"injected and distributes through a central volume of {num(central_volume)} L plus a "
                    f"peripheral volume of {num(peripheral_volume)} L. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe indicator_volume({num(indicator_volume)})\n"
                    f"observe indicator_concentration({num(indicator_concentration)})\n"
                    f"observe central_volume({num(central_volume)})\n"
                    f"observe peripheral_volume({num(peripheral_volume)})\n"
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
            "ADJ-LADDER rung 44 — indicator-dilution mixed concentration from four stated quantities (a NEW panel: "
            "indicator / tracer dilution). From a tracer bolus (its volume and concentration) and two distribution "
            "compartments compute the mixed concentration ((indicator_volume*indicator_concentration)/"
            "(central_volume+peripheral_volume)), the injected indicator mass (indicator_volume*"
            "indicator_concentration), or the total distribution volume (central_volume+peripheral_volume). Each "
            "item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — a NEW shape, PRODUCT-OVER-SUM (a*b)/(c+d), the first quotient on the "
            "ladder to divide a parenthesised product by a parenthesised sum (distinct from rung-37 ratio-of-two-"
            "sums (a+b)/(c+d), rung-41 difference-over-sum (a-b)/(a+b), and rung-42 sum-over-difference (a+b)/(c-d)) "
            "— and the harness matches the scalar to the printed options. Contamination-safe: every index is built "
            "only from the four observed quantities via *, + and / — no constant leaks, and neither the injected "
            "mass, the distribution volume, nor any concentration ever appears as a literal (each is computed) — and "
            "the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The "
            "five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make: dividing by the central compartment alone (dropping the peripheral one), and adding the "
            "bolus's volume and concentration instead of multiplying them. The core confusion tested is dividing the "
            "injected product by the pooled (summed) distribution volume."
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
