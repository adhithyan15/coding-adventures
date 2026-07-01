"""Generate rung-30 (body mass index) items.json for the ADJ-LADDER.

Rung 30 opens the **anthropometric / nutritional** panel on the quantitative band — the arithmetic of the
body mass index (BMI), the bedside index that stratifies underweight / normal / overweight / obese. It uses
the same contamination-safe shape as the serum-protein rung (29), the coagulation rung (28), and the urine
rung (27): a small table of *observed* quantities and a tight family of mutually-confusable formulas built
**only from those observed quantities** (no numeric literal anywhere in any program), so nothing structural
can leak.

The clinical setup is a single anthropometric measurement. Two quantities are measured:

  WEIGHT  body mass       (kg)
  HEIGHT  body height     (m)

BMI falls out as a pure function of the observed quantities — no constant required (metric BMI is exactly
`kg / m^2`). That is what makes this rung distinctive: BMI is a quantity **divided by a SQUARE** — `WEIGHT /
(HEIGHT * HEIGHT)` — a shape not yet seen on the ladder (rung-24 put a difference in the numerator, rung-27
summed inside a difference, rung-29 divided by a difference; this rung divides *by a product of the same
quantity with itself*). The core confusion this rung tests is remembering to **square the height** before
dividing (the classic slip is `WEIGHT / HEIGHT`, forgetting the square):

  BODY MASS INDEX      WEIGHT / (HEIGHT * HEIGHT)  [ =BMI, kg/m^2 ]
  WEIGHT:HEIGHT RATIO  WEIGHT / HEIGHT            [ the slip: divided by height, not height-squared ]
  HEIGHT SQUARED       HEIGHT * HEIGHT            [ the denominator alone, m^2 ]

Each index is a `compute_dimensioned` program (observe the two quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/28/29. This rung exercises the engine across a
PRODUCT in the denominator (`HEIGHT * HEIGHT`) contrasted with a plain division.

Contamination-safe by construction: every formula is built only from the two observed quantities via `*`,
`/` — **no structural constants** — so every program literal is grounded in the stem (`HEIGHT` appears twice
in the BMI denominator, but it is the observed *identifier*, not a numeral). The observed quantities carry
**digit-free identifiers** (`weight`, `height`) so no numeral hides inside a variable name. The five options
are a tight family over the same quantities: the three real indices plus the two classic slips —

  (HEIGHT * HEIGHT) / WEIGHT    the *inverted* BMI (upside-down), and
  WEIGHT * HEIGHT              the weight-times-height product (multiplying instead of dividing),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: BMI is order ~20-40, the weight:height ratio is order ~30-60, height-squared is order ~2-4,
the inverted BMI is order ~0.03, and the weight-times-height product is order ~100 — five very different
magnitudes, so no two family values collide; the tables below are chosen so the five family values are
pairwise distinct — with a comfortable margin — for every item, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (WEIGHT, HEIGHT) observed quantities. WEIGHT in kg, HEIGHT in m. The five index-family values are
# asserted pairwise-distinct (with margin) below.
#   WEIGHT = body mass
#   HEIGHT = body height
TABLES = [
    (72, 1.8),
    (80, 2.0),
    (90, 1.8),
    (60, 1.5),
    (100, 2.0),
    (50, 1.6),
    (77, 1.75),
]

# The option family (5 members), all built from the observed quantities via `*` / `/`. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all
# five always appear as the options.
FAMILY = [
    ("bmi", "body mass index", "weight / (height * height)"),
    ("weight_height_ratio", "weight-to-height ratio", "weight / height"),
    ("height_squared", "height squared", "height * height"),
    ("inverse_bmi", "inverse BMI (height-squared over weight)", "(height * height) / weight"),
    ("weight_height_product", "weight-times-height product", "weight * height"),
]
QUERIED = ["bmi", "weight_height_ratio", "height_squared"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(weight, height):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "bmi": weight / (height * height),
        "weight_height_ratio": weight / height,
        "height_squared": height * height,
        "inverse_bmi": (height * height) / weight,
        "weight_height_product": weight * height,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for weight, height in TABLES:
        assert weight > 0 and height > 0, (weight, height)
        fv = family_values(weight, height)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (weight, height, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, weight, height, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r30bmi-{idx + 1:02d}",
                "qtype": "body_mass_index",
                "stem": (
                    f"A patient weighs {num(weight)} kg and is {num(height)} m tall. What is the patient's "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe weight({num(weight)})\n"
                    f"observe height({num(height)})\n"
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
            "ADJ-LADDER rung 30 — body mass index from a single anthropometric measurement (a NEW panel: "
            "anthropometric / nutritional). From two stated quantities (body weight WEIGHT in kg, height "
            "HEIGHT in m) compute the body mass index (WEIGHT/(HEIGHT*HEIGHT)), the weight-to-height ratio "
            "(WEIGHT/HEIGHT), or height squared (HEIGHT*HEIGHT). Each item is a compute_dimensioned program "
            "(observe the two quantities, let answer = formula); the ADJ engine carries the arithmetic — a "
            "NEW shape, a quantity DIVIDED BY A SQUARE (WEIGHT/(HEIGHT*HEIGHT)), so a PRODUCT in the "
            "denominator is contrasted with a plain division — and the harness matches the scalar to the "
            "printed options. Contamination-safe: every index is built only from the two observed "
            "quantities via * and / — no constant leaks (metric BMI is exactly kg/m^2), and HEIGHT appears "
            "twice only as the observed identifier — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the "
            "same quantities, so the distractors are exactly the slips students make: the inverted BMI "
            "((HEIGHT*HEIGHT)/WEIGHT) and the weight-times-height product (WEIGHT*HEIGHT, multiplying "
            "instead of dividing). The core confusion tested is remembering to square the height before "
            "dividing (the classic WEIGHT/HEIGHT slip)."
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
              "=", round(it["options"][it["gold_letter"]]["value"], 4))
