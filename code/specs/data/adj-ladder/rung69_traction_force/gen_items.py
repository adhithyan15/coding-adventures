"""Generate rung-69 (skeletal traction force) items.json for the ADJ-LADDER.

Rung 69 opens the **orthopedics / skeletal traction** panel on the quantitative band — the arithmetic of a traction
force. A skeletal traction rig hangs a weight over a pulley to pull a fractured limb straight: the applied weight is
multiplied by a mechanical `pulley_factor`, normalised to the `limb_length` being pulled, and added to a resting
`baseline_tension` already in the line. Multiplying two quantities into a PRODUCT, dividing that product by a normaliser,
and then ADDING a baseline term introduces a genuinely NEW arithmetic shape on the ladder: a **product-over-a-divisor
plus a term** — `a*b/c+d`, i.e. `((a*b)/c)+d`.

This is the first rung to put a **product** (not a sum or a difference) over the divisor and then add a trailing term.
Contrast the neighbours already on the ladder: rung-65 was `(a-b)/c+d` (a DIFFERENCE over the divisor, plus a term),
rung-66 was `(a+b)/c-d` (a SUM over the divisor, minus a term), and rung-68 was `(a+b)*c/d` (a sum scaled by a factor
then divided, with NO trailing term). Here a product is normalised AND offset.

The setup: an `applied_weight`, a `pulley_factor`, a `limb_length`, and a `baseline_tension`. The traction force is:

  TRACTION FORCE   applied_weight * pulley_factor / limb_length + baseline_tension   [ geared load per unit limb, offset ]
  LEVERAGE         applied_weight * pulley_factor                                    [ the raw product ]
  DISTRIBUTED      applied_weight * pulley_factor / limb_length                      [ the product over the limb, before offsetting ]

The **traction force** is what makes this rung distinctive — it is the ladder's first **product over a divisor, then a
term added**. (The leverage `applied_weight * pulley_factor` and the distributed load `applied_weight * pulley_factor /
limb_length` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-68
shipped their component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the product, the division by limb length, and the addition of the baseline tension — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../67/68. This rung exercises the engine across **a product over a divisor, then offset** — the fact that
`a*b/c+d` is NOT `(a*b+d)/c` and NOT `a*b/(c+d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `/`, and `+`
— **no structural constants** — so no numeric literal appears in any program, and neither the leverage, the distributed
load, nor any traction figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`applied_weight`, `pulley_factor`, `limb_length`, `baseline_tension`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (applied_weight * pulley_factor + baseline_tension) / limb_length   ADD the baseline BEFORE dividing instead
                                                                                 of after (the classic `a*b/c+d` vs
                                                                                 `(a*b+d)/c` error), and
  SWAPPED    applied_weight * pulley_factor / (limb_length + baseline_tension)    ADD the baseline into the DENOMINATOR
                                                                                 instead of after the division
                                                                                 (`a*b/(c+d)` instead of `a*b/c+d`),

which are exactly the mistakes a student makes (folding the baseline into the numerator before dividing, or into the
denominator). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness: all four observed quantities are strictly positive (so every family member is positive); the limb length
exceeds one (so the raw leverage `a*b` differs from the distributed load `a*b/c`); the five family values are pairwise
distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (APPLIED_WEIGHT, PULLEY_FACTOR, LIMB_LENGTH, BASELINE_TENSION) — a weight and a pulley factor whose product is the
# leverage, a limb length to normalise by, and a baseline tension to add, all plain positive numbers with limb_length > 1
# (so leverage != distributed). The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (12, 5, 3, 4),
    (10, 6, 4, 5),
    (8, 9, 6, 3),
    (15, 4, 5, 6),
    (9, 8, 4, 2),
    (14, 5, 7, 3),
    (6, 10, 4, 8),
]

# The option family (5 members), all built from the four observed quantities via *, /, and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "traction_force",
        "traction force (leverage distributed over the limb length, plus the baseline tension)",
        "applied_weight * pulley_factor / limb_length + baseline_tension",
    ),
    (
        "leverage",
        "the leverage (applied weight times the pulley factor)",
        "applied_weight * pulley_factor",
    ),
    (
        "distributed",
        "the distributed load before adding the baseline tension (leverage over the limb length)",
        "applied_weight * pulley_factor / limb_length",
    ),
    (
        "crossed",
        "the baseline tension added to the leverage BEFORE dividing by the limb length, not after (a wrong offset)",
        "(applied_weight * pulley_factor + baseline_tension) / limb_length",
    ),
    (
        "swapped",
        "the leverage divided by the limb length PLUS the baseline tension in the denominator, the baseline mis-placed (a wrong divisor)",
        "applied_weight * pulley_factor / (limb_length + baseline_tension)",
    ),
]
QUERIED = ["traction_force", "leverage", "distributed"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(applied_weight, pulley_factor, limb_length, baseline_tension):
    # Operation order mirrors the ADJ programs exactly (the product formed first, then the left-to-right divide, then the
    # trailing add), so the Python option value and the engine result are the same IEEE-double (well within the harness's
    # 1e-9 match tolerance).
    return {
        "traction_force": applied_weight * pulley_factor / limb_length + baseline_tension,
        "leverage": applied_weight * pulley_factor,
        "distributed": applied_weight * pulley_factor / limb_length,
        "crossed": (applied_weight * pulley_factor + baseline_tension) / limb_length,
        "swapped": applied_weight * pulley_factor / (limb_length + baseline_tension),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for applied_weight, pulley_factor, limb_length, baseline_tension in TABLES:
        assert (
            applied_weight > 0
            and pulley_factor > 0
            and limb_length > 0
            and baseline_tension > 0
        ), (applied_weight, pulley_factor, limb_length, baseline_tension)
        # Limb length exceeds one so the raw leverage (a*b) differs from the distributed load (a*b/c); all four
        # quantities are positive so every family member is positive.
        assert limb_length > 1, (applied_weight, pulley_factor, limb_length, baseline_tension)
        fv = family_values(applied_weight, pulley_factor, limb_length, baseline_tension)
        for key, v in fv.items():
            assert v > 0, (key, applied_weight, pulley_factor, limb_length, baseline_tension, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    applied_weight,
                    pulley_factor,
                    limb_length,
                    baseline_tension,
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
                applied_weight,
                pulley_factor,
                limb_length,
                baseline_tension,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r69trac-{idx + 1:02d}",
                "qtype": "traction_force",
                "stem": (
                    f"A skeletal traction rig applies a weight of {num(applied_weight)} through a pulley factor of "
                    f"{num(pulley_factor)}, normalised to a limb length of {num(limb_length)} and added to a baseline "
                    f"tension of {num(baseline_tension)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe applied_weight({num(applied_weight)})\n"
                    f"observe pulley_factor({num(pulley_factor)})\n"
                    f"observe limb_length({num(limb_length)})\n"
                    f"observe baseline_tension({num(baseline_tension)})\n"
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
            "ADJ-LADDER rung 69 — skeletal traction force from four stated quantities (a NEW panel: orthopedics / "
            "skeletal traction). From an applied weight and a pulley factor (their product is the leverage), a limb "
            "length to normalise by, and a baseline tension to add, compute the traction force "
            "(applied_weight*pulley_factor/limb_length+baseline_tension), the leverage (applied_weight*pulley_factor), "
            "or the distributed load (applied_weight*pulley_factor/limb_length). Each item is a compute_dimensioned "
            "program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW "
            "shape, PRODUCT OVER A DIVISOR PLUS A TERM a*b/c+d, the first on the ladder to put a product (not a sum or "
            "difference) over the divisor and then add a term (distinct from rung-65 (a-b)/c+d, rung-66 (a+b)/c-d, and "
            "rung-68 (a+b)*c/d with no trailing term) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via *, /, and + — no "
            "constant leaks, and neither the leverage, the distributed load, nor any traction figure ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same four quantities, so the distractors "
            "are exactly the slips students make: ADDING the baseline BEFORE dividing ((a*b+d)/c, a wrong offset) and "
            "folding the baseline into the DENOMINATOR (a*b/(c+d), a wrong divisor). The core confusion tested is that "
            "a*b/c+d is not (a*b+d)/c and not a*b/(c+d)."
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
