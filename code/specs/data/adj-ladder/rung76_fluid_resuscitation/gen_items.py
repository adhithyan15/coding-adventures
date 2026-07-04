"""Generate rung-76 (burn-care fluid-resuscitation volume index) items.json for the ADJ-LADDER.

Rung 76 opens the **burn-care / fluid-resuscitation** panel on the quantitative band — the arithmetic of a resuscitation
volume. A burn assessment adds a `burn_area` and a `body_weight` into a combined load, normalises it by a `rate_factor`,
and scales it by a `shift_gain`. Taking a SUM FIRST, then dividing, then scaling introduces a genuinely NEW arithmetic
shape on the ladder: a **sum-quotient scaled by a factor** — `(a+b)/c*d`, i.e. `((a+b)/c)*d`.

This is the deliberate contrast to rung-75's `(a-b)/c*d` (audiology): rung-75 opened with a parenthesised DIFFERENCE
divided and scaled; rung-76 opens with a parenthesised SUM — the plus-numerator counterpart, sharing the same
divide-then-multiply tail. The operation order matters: `(a+b)/c*d` is left-to-right `((a+b)/c)*d = (a+b)*d/c`, NOT
`(a+b)/(c*d)` — the distractor exploits exactly that confusion. Contrast the other neighbours: rung-68 was `(a+b)*c/d`
(a sum scaled THEN divided) and rung-53 was `(a+b+c)/d` (a bare triple sum over a divisor). Here a sum is divided THEN
scaled.

The setup: a `burn_area`, a `body_weight`, a `rate_factor`, and a `shift_gain`. The resuscitation volume is:

  RESUSCITATION VOLUME   (burn_area + body_weight) / rate_factor * shift_gain   [ combined load, normalised, scaled ]
  COMBINED LOAD          burn_area + body_weight                                [ the sum ]
  NORMALIZED LOAD        (burn_area + body_weight) / rate_factor                [ the sum-quotient ]

The **resuscitation volume** is what makes this rung distinctive — it is the ladder's first **sum-quotient scaled by a
factor** (divide the sum, then multiply). (The combined load `burn_area + body_weight` and the normalized load
`(burn_area + body_weight) / rate_factor` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 47-75 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the addition of the burn area and body weight, the division by the rate factor, and the
multiplication by the shift gain (left-to-right) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../74/75. This rung exercises the
engine across **a sum-quotient scaled by a factor** — the fact that `(a+b)/c*d` is NOT `(a+b)/(c*d)` and NOT `(a+b)*c/d`
made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `/`, and `*`
— **no structural constants** — so no numeric literal appears in any program, and neither the combined load, the
normalized load, nor any resuscitation figure is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`burn_area`, `body_weight`, `rate_factor`, `shift_gain`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (burn_area + body_weight) / (rate_factor * shift_gain)   DIVIDE the combined load by the PRODUCT of the
                                                                      rate factor and shift gain, not divide-then-
                                                                      multiply (the classic `(a+b)/c*d` vs `(a+b)/(c*d)`
                                                                      error), and
  SWAPPED    (burn_area + body_weight) * rate_factor / shift_gain     MULTIPLY the combined load by the rate factor and
                                                                      divide by the shift gain — the operations swapped
                                                                      (`(a+b)*c/d` instead of `(a+b)/c*d`),

which are exactly the mistakes a student makes (folding both denominators into one product, or swapping which quantity
divides and which multiplies). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness: all four observed quantities are strictly positive, so the combined load and every family member — a
positive sum times/over positive factors — is positive; the rate factor and the shift gain both exceed one (so the
normalized sum-quotient differs from the combined load) and differ from each other (so the resuscitation value
`(a+b)*d/c` differs from the swapped value `(a+b)*c/d`); the five family values are pairwise distinct with a comfortable
margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BURN_AREA, BODY_WEIGHT, RATE_FACTOR, SHIFT_GAIN) — a burn area, a body weight to add, a rate factor to divide by, and
# a shift gain to scale by, all plain positive numbers with rate_factor > 1, shift_gain > 1, and rate_factor !=
# shift_gain. Because every family value is a positive sum over/times positive factors, positivity is automatic; the
# five family values are asserted pairwise-distinct below.
TABLES = [
    (16, 8, 3, 2),
    (20, 10, 5, 3),
    (14, 10, 4, 3),
    (30, 6, 6, 2),
    (18, 12, 5, 2),
    (10, 6, 2, 3),
    (28, 12, 5, 4),
]

# The option family (5 members), all built from the four observed quantities via +, /, and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "resuscitation_volume",
        "resuscitation volume (combined load normalised by the rate factor, then gain-scaled)",
        "(burn_area + body_weight) / rate_factor * shift_gain",
    ),
    (
        "combined_load",
        "the combined load (burn area plus the body weight)",
        "burn_area + body_weight",
    ),
    (
        "normalized_load",
        "the normalized load before scaling (combined load over the rate factor)",
        "(burn_area + body_weight) / rate_factor",
    ),
    (
        "crossed",
        "the combined load divided by the PRODUCT of the rate factor and shift gain, not divide-then-multiply (a wrong scaling)",
        "(burn_area + body_weight) / (rate_factor * shift_gain)",
    ),
    (
        "swapped",
        "the combined load MULTIPLIED by the rate factor and DIVIDED by the shift gain, the operations swapped (a wrong scaling)",
        "(burn_area + body_weight) * rate_factor / shift_gain",
    ),
]
QUERIED = ["resuscitation_volume", "combined_load", "normalized_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(burn_area, body_weight, rate_factor, shift_gain):
    # Operation order mirrors the ADJ programs exactly (the parenthesised sum is normalised by the rate factor, then the
    # left-to-right multiply by the shift gain), so the Python option value and the engine result are the same
    # IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "resuscitation_volume": (burn_area + body_weight) / rate_factor * shift_gain,
        "combined_load": burn_area + body_weight,
        "normalized_load": (burn_area + body_weight) / rate_factor,
        "crossed": (burn_area + body_weight) / (rate_factor * shift_gain),
        "swapped": (burn_area + body_weight) * rate_factor / shift_gain,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for burn_area, body_weight, rate_factor, shift_gain in TABLES:
        assert (
            burn_area > 0
            and body_weight > 0
            and rate_factor > 0
            and shift_gain > 0
        ), (burn_area, body_weight, rate_factor, shift_gain)
        # Rate factor and shift gain exceed one so the normalized sum-quotient differs from the combined load, and they
        # differ from each other so the resuscitation value ((a+b)*d/c) differs from the swapped value ((a+b)*c/d).
        # Every family member is a positive sum over/times positive factors, so positivity is automatic.
        assert rate_factor > 1, (burn_area, body_weight, rate_factor, shift_gain)
        assert shift_gain > 1, (burn_area, body_weight, rate_factor, shift_gain)
        assert rate_factor != shift_gain, (burn_area, body_weight, rate_factor, shift_gain)
        fv = family_values(burn_area, body_weight, rate_factor, shift_gain)
        for key, v in fv.items():
            assert v > 0, (key, burn_area, body_weight, rate_factor, shift_gain, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    burn_area,
                    body_weight,
                    rate_factor,
                    shift_gain,
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
                burn_area,
                body_weight,
                rate_factor,
                shift_gain,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r76burn-{idx + 1:02d}",
                "qtype": "fluid_resuscitation",
                "stem": (
                    f"A burn assessment records a burn area of {num(burn_area)}, a body weight of "
                    f"{num(body_weight)} to add, a rate factor of {num(rate_factor)} to divide by and a shift gain of "
                    f"{num(shift_gain)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe burn_area({num(burn_area)})\n"
                    f"observe body_weight({num(body_weight)})\n"
                    f"observe rate_factor({num(rate_factor)})\n"
                    f"observe shift_gain({num(shift_gain)})\n"
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
            "ADJ-LADDER rung 76 — burn-care fluid-resuscitation volume index from four stated quantities (a NEW panel: "
            "burn-care / fluid-resuscitation). From a burn area, a body weight to add, a rate factor to divide by, and "
            "a shift gain to scale by, compute the resuscitation volume "
            "((burn_area+body_weight)/rate_factor*shift_gain), the combined load (burn_area+body_weight), or the "
            "normalized load ((burn_area+body_weight)/rate_factor). Each item is a compute_dimensioned program (observe "
            "the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, "
            "SUM-QUOTIENT SCALED BY A FACTOR (a+b)/c*d (add first, then DIVIDE, then scale — contrast rung-68 (a+b)*c/d "
            "which scales THEN divides, and rung-75 (a-b)/c*d which subtracts; the left-to-right (a+b)/c*d = (a+b)*d/c, "
            "not (a+b)/(c*d)) — and the harness matches the scalar to the printed options. Contamination-safe: every "
            "index is built only from the four observed quantities via +, /, and * — no constant leaks, and neither the "
            "combined load, the normalized load, nor any resuscitation figure ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the "
            "slips students make: DIVIDING by the PRODUCT ((a+b)/(c*d), a wrong scaling) and SWAPPING the multiply and "
            "divide ((a+b)*c/d, a wrong scaling). The core confusion tested is that (a+b)/c*d is not (a+b)/(c*d) and "
            "not (a+b)*c/d."
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
