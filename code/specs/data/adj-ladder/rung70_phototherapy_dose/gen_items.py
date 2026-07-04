"""Generate rung-70 (neonatal phototherapy effective dose) items.json for the ADJ-LADDER.

Rung 70 opens the **neonatology / phototherapy** panel on the quantitative band — the arithmetic of an effective
phototherapy dose. A jaundiced newborn lies under a blue-light lamp: the lamp's `lamp_output` is multiplied by an
`exposure_factor`, normalised to the `skin_distance` from the lamp, and the `ambient_light` already reaching the skin is
subtracted off. Multiplying two quantities into a PRODUCT, dividing that product by a normaliser, and then SUBTRACTING a
baseline term introduces a genuinely NEW arithmetic shape on the ladder: a **product-over-a-divisor minus a term** —
`a*b/c-d`, i.e. `((a*b)/c)-d`.

This is the deliberate MIRROR of rung-69's `a*b/c+d` (skeletal traction force): rung-69 ADDED the trailing term, rung-70
SUBTRACTS it. Contrast the other neighbours already on the ladder: rung-65 was `(a-b)/c+d` (a DIFFERENCE over the
divisor, plus a term), rung-66 was `(a+b)/c-d` (a SUM over the divisor, minus a term), and rung-68 was `(a+b)*c/d` (a sum
scaled by a factor then divided). Here a PRODUCT is normalised and then OFFSET DOWNWARD.

The setup: a `lamp_output`, an `exposure_factor`, a `skin_distance`, and an `ambient_light`. The effective dose is:

  EFFECTIVE DOSE   lamp_output * exposure_factor / skin_distance - ambient_light   [ geared irradiance per unit distance, net of ambient ]
  RADIANT          lamp_output * exposure_factor                                   [ the raw product ]
  DISTRIBUTED      lamp_output * exposure_factor / skin_distance                   [ the product over the distance, before subtracting ambient ]

The **effective dose** is what makes this rung distinctive — it is the ladder's first **product over a divisor, then a
term SUBTRACTED**. (The radiant output `lamp_output * exposure_factor` and the distributed irradiance `lamp_output *
exposure_factor / skin_distance` ride alongside as component readouts, so the panel teaches the whole calculation —
exactly as rungs 47-69 shipped their component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the product, the division by skin distance, and the subtraction of ambient light — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../68/69. This rung exercises the engine across **a product over a divisor, then offset downward** — the fact that
`a*b/c-d` is NOT `(a*b-d)/c` and NOT `a*b/(c-d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `/`, and `-`
— **no structural constants** — so no numeric literal appears in any program, and neither the radiant output, the
distributed irradiance, nor any dose figure is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`lamp_output`, `exposure_factor`, `skin_distance`,
`ambient_light`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (lamp_output * exposure_factor - ambient_light) / skin_distance   SUBTRACT the ambient BEFORE dividing
                                                                               instead of after (the classic `a*b/c-d`
                                                                               vs `(a*b-d)/c` error), and
  SWAPPED    lamp_output * exposure_factor / (skin_distance - ambient_light)    SUBTRACT the ambient inside the
                                                                               DENOMINATOR instead of after the division
                                                                               (`a*b/(c-d)` instead of `a*b/c-d`),

which are exactly the mistakes a student makes (folding the ambient into the numerator before dividing, or into the
denominator). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness: all four observed quantities are strictly positive; the skin distance exceeds one (so the raw radiant
output `a*b` differs from the distributed irradiance `a*b/c`); the skin distance exceeds the ambient light and their
difference is not one (so the swapped denominator `c-d` is positive and `swapped` differs from `radiant`); and the
distributed irradiance exceeds the ambient light (so the headline effective dose stays positive). The five family values
are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (LAMP_OUTPUT, EXPOSURE_FACTOR, SKIN_DISTANCE, AMBIENT_LIGHT) — a lamp output and an exposure factor whose product is
# the radiant output, a skin distance to normalise by, and an ambient light to subtract, all plain positive numbers with
# skin_distance > 1, ambient_light > 1, skin_distance > ambient_light, skin_distance - ambient_light != 1, and
# lamp_output*exposure_factor/skin_distance > ambient_light. The five family values are asserted pairwise-distinct below.
TABLES = [
    (15, 4, 5, 3),
    (20, 3, 6, 4),
    (14, 5, 7, 2),
    (9, 8, 8, 3),
    (16, 4, 8, 2),
    (18, 5, 6, 3),
    (21, 4, 7, 3),
]

# The option family (5 members), all built from the four observed quantities via *, /, and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "effective_dose",
        "effective phototherapy dose (radiant output distributed over the skin distance, net of ambient light)",
        "lamp_output * exposure_factor / skin_distance - ambient_light",
    ),
    (
        "radiant",
        "the radiant output (lamp output times the exposure factor)",
        "lamp_output * exposure_factor",
    ),
    (
        "distributed",
        "the distributed irradiance before subtracting the ambient light (radiant output over the skin distance)",
        "lamp_output * exposure_factor / skin_distance",
    ),
    (
        "crossed",
        "the ambient light subtracted from the radiant output BEFORE dividing by the skin distance, not after (a wrong offset)",
        "(lamp_output * exposure_factor - ambient_light) / skin_distance",
    ),
    (
        "swapped",
        "the radiant output divided by the skin distance LESS the ambient light in the denominator, the ambient mis-placed (a wrong divisor)",
        "lamp_output * exposure_factor / (skin_distance - ambient_light)",
    ),
]
QUERIED = ["effective_dose", "radiant", "distributed"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(lamp_output, exposure_factor, skin_distance, ambient_light):
    # Operation order mirrors the ADJ programs exactly (the product formed first, then the left-to-right divide, then the
    # trailing subtract), so the Python option value and the engine result are the same IEEE-double (well within the
    # harness's 1e-9 match tolerance).
    return {
        "effective_dose": lamp_output * exposure_factor / skin_distance - ambient_light,
        "radiant": lamp_output * exposure_factor,
        "distributed": lamp_output * exposure_factor / skin_distance,
        "crossed": (lamp_output * exposure_factor - ambient_light) / skin_distance,
        "swapped": lamp_output * exposure_factor / (skin_distance - ambient_light),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for lamp_output, exposure_factor, skin_distance, ambient_light in TABLES:
        assert (
            lamp_output > 0
            and exposure_factor > 0
            and skin_distance > 0
            and ambient_light > 0
        ), (lamp_output, exposure_factor, skin_distance, ambient_light)
        # Skin distance exceeds one so the raw radiant output (a*b) differs from the distributed irradiance (a*b/c). Skin
        # distance exceeds ambient light and their difference is not one, so the swapped denominator (c-d) is positive
        # and swapped differs from radiant. The distributed irradiance exceeds the ambient light so the headline dose is
        # positive.
        assert skin_distance > 1, (lamp_output, exposure_factor, skin_distance, ambient_light)
        assert ambient_light > 1, (lamp_output, exposure_factor, skin_distance, ambient_light)
        assert skin_distance > ambient_light, (lamp_output, exposure_factor, skin_distance, ambient_light)
        assert skin_distance - ambient_light != 1, (lamp_output, exposure_factor, skin_distance, ambient_light)
        assert lamp_output * exposure_factor / skin_distance > ambient_light, (
            lamp_output,
            exposure_factor,
            skin_distance,
            ambient_light,
        )
        fv = family_values(lamp_output, exposure_factor, skin_distance, ambient_light)
        for key, v in fv.items():
            assert v > 0, (key, lamp_output, exposure_factor, skin_distance, ambient_light, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    lamp_output,
                    exposure_factor,
                    skin_distance,
                    ambient_light,
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
                lamp_output,
                exposure_factor,
                skin_distance,
                ambient_light,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r70photo-{idx + 1:02d}",
                "qtype": "phototherapy_dose",
                "stem": (
                    f"A phototherapy lamp puts out {num(lamp_output)} through an exposure factor of "
                    f"{num(exposure_factor)}, normalised to a skin distance of {num(skin_distance)} and net of an "
                    f"ambient light of {num(ambient_light)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe lamp_output({num(lamp_output)})\n"
                    f"observe exposure_factor({num(exposure_factor)})\n"
                    f"observe skin_distance({num(skin_distance)})\n"
                    f"observe ambient_light({num(ambient_light)})\n"
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
            "ADJ-LADDER rung 70 — neonatal phototherapy effective dose from four stated quantities (a NEW panel: "
            "neonatology / phototherapy). From a lamp output and an exposure factor (their product is the radiant "
            "output), a skin distance to normalise by, and an ambient light to subtract, compute the effective dose "
            "(lamp_output*exposure_factor/skin_distance-ambient_light), the radiant output "
            "(lamp_output*exposure_factor), or the distributed irradiance "
            "(lamp_output*exposure_factor/skin_distance). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, PRODUCT OVER A "
            "DIVISOR MINUS A TERM a*b/c-d, the mirror of rung-69 a*b/c+d (subtract instead of add the trailing term; "
            "distinct from rung-65 (a-b)/c+d, rung-66 (a+b)/c-d, and rung-68 (a+b)*c/d) — and the harness matches the "
            "scalar to the printed options. Contamination-safe: every index is built only from the four observed "
            "quantities via *, /, and - — no constant leaks, and neither the radiant output, the distributed "
            "irradiance, nor any dose figure ever appears as a literal (each is computed) — and the observed quantities "
            "carry digit-free identifiers so no numeral hides inside a variable name. The five options are a family "
            "over the same four quantities, so the distractors are exactly the slips students make: SUBTRACTING the "
            "ambient BEFORE dividing ((a*b-d)/c, a wrong offset) and folding the ambient into the DENOMINATOR "
            "(a*b/(c-d), a wrong divisor). The core confusion tested is that a*b/c-d is not (a*b-d)/c and not "
            "a*b/(c-d)."
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
