"""Generate rung-107 (optometry / contact-lens fitting) items.json for the ADJ-LADDER.

Rung 107 opens the **optometry / contact-lens fitting** panel on the quantitative band — the arithmetic of a contact-lens
fit index. A `corneal_reading` (the steep corneal curvature) MINUS a `base_curve` (the lens's back curve) gives the sagittal
gap (how much steeper the cornea sits than the lens, the difference), a `lens_diameter` (the lens's overall width) MINUS a
`pupil_diameter` (the pupil width) gives the coverage margin (how far the lens overhangs the pupil, the difference), and the
sagittal gap is DIVIDED by the coverage margin to give the fit index. A **difference over a difference** introduces a
genuinely NEW arithmetic family on the ladder: `(a-b)/(c-d)`, i.e. `((a-b) / (c-d))`.

This is genuinely new — the first time the ladder divides a bare DIFFERENCE by a bare DIFFERENCE. It **completes the
difference-denominator family** that rungs 105-106 opened: rung-105 `(a+b)/(c-d)` divided a SUM by a difference, rung-106
`a*b/(c-d)` a PRODUCT by a difference, and rung-107 divides a DIFFERENCE by a difference — the three numerator shapes (sum,
product, difference) over a difference denominator are now all shipped. No prior rung put a difference over a difference:
rung-104 `(a-b)/(c*d)` divided a difference by a product, rung-37 `(a+b)/(c+d)` a sum by a sum. The operator order matters:
`(a-b)/(c-d)` is `((a-b) / (c-d))` (both sides parenthesized), NOT `a-b/(c-d)` (dropping the numerator parentheses so only the
base curve is divided by the coverage margin and then subtracted from the corneal reading) and NOT `(a-b)/(c+d)` (summing the
denominator pair instead of subtracting, mispairing which pair is the coverage difference) — the two distractors exploit
exactly those confusions.

The setup: a `corneal_reading`, a `base_curve`, a `lens_diameter`, and a `pupil_diameter`. The total is:

  FIT INDEX        (corneal_reading - base_curve) / (lens_diameter - pupil_diameter)  [ a difference over a difference ]
  SAGITTAL GAP     corneal_reading - base_curve                                        [ the numerator difference ]
  COVERAGE MARGIN  lens_diameter - pupil_diameter                                      [ the denominator difference ]

The **fit index** is what makes this rung distinctive — it is the ladder's first **bare DIFFERENCE over a bare DIFFERENCE**.
It is a dimensionless ratio (the sagittal gap per unit of coverage margin), framed as an *index* to keep it honest — the same
discipline rungs 100/104/105/106 used for their ratios. (The sagittal gap `a-b` and the coverage margin `c-d` ride alongside
as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-106 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the base curve from the corneal reading into the sagittal gap, the subtraction of
the pupil diameter from the lens diameter into the coverage margin, then the division of the sagittal gap by the coverage
margin (both parenthesized, so (a-b)/(c-d) evaluates as ((a-b)/(c-d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../105/106. This rung exercises the engine
across a **difference over a difference** — the fact that `(a-b)/(c-d)` is `((a-b)/(c-d))` and NOT `a-b/(c-d)` and NOT
`(a-b)/(c+d)` made computable. The ratio golds are non-integer f64s; the engine's IEEE-double division matches Python's the
same way rungs 99/100/104/105/106 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the sagittal gap, the coverage margin, nor
any index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`corneal_reading`, `base_curve`, `lens_diameter`, `pupil_diameter`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    corneal_reading - base_curve / (lens_diameter - pupil_diameter)  drop the numerator parentheses so only the base
                                                                              curve is divided by the coverage margin and then
                                                                              subtracted from the corneal reading (the classic
                                                                              `(a-b)/(c-d)` vs `a-b/(c-d)` precedence error), and
  SWAPPED    (corneal_reading - base_curve) / (lens_diameter + pupil_diameter)  sum the denominator pair instead of
                                                                              subtracting, mispairing which pair is the
                                                                              coverage difference (`(a-b)/(c+d)` instead of
                                                                              `(a-b)/(c-d)`),

which are exactly the mistakes a student makes (dropping the numerator parentheses before dividing, or summing the
denominator pair that should be a difference). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts;
all five always appear as options.

Distinctness and positivity: the tables are chosen so `corneal_reading > base_curve` (the sagittal gap `a-b` stays strictly
positive) and `lens_diameter > pupil_diameter` with a comfortable margin (the coverage margin `c-d` — the headline denominator
— is strictly positive and the fit index stays finite and clean, never blowing up on a tiny difference), and every observed
quantity is `>= 2`. The two differences are chosen unequal (`a-b != c-d`) so the numerator and denominator readouts never
coincide; the swapped denominator `c+d >= 4` is a sum with no subtraction, never zero; the crossed figure `a - b/(c-d)` is
positive because `b/(c-d) < b <= corneal_reading`. The tables are chosen so the five family values are pairwise distinct with a
comfortable margin, and — so all three queried readouts vary across the panel — the seven tables give distinct fit indices,
distinct sagittal gaps, and distinct coverage margins, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CORNEAL_READING, BASE_CURVE, LENS_DIAMETER, PUPIL_DIAMETER) — a corneal reading minus a base curve for the sagittal gap, a
# lens diameter minus a pupil diameter for the coverage margin, all plain positive numbers >= 2. Each table satisfies
# corneal_reading > base_curve (sagittal gap > 0) and lens_diameter > pupil_diameter with a comfortable margin (coverage
# margin = c-d >= 2 => fit index finite and > 0), and a-b != c-d (numerator and denominator readouts distinct); the swapped
# denominator (c+d) is >= 4 with no subtraction, so nothing is ever zero or undefined. The five family values are asserted
# pairwise-distinct below. The seven tables give distinct fit indices, distinct sagittal gaps, and distinct coverage margins
# so all three queried readouts vary across the panel.
TABLES = [
    (8, 2, 5, 3),
    (10, 3, 7, 4),
    (12, 4, 8, 3),
    (13, 4, 11, 5),
    (14, 4, 9, 2),
    (16, 5, 12, 4),
    (15, 3, 13, 4),
]

# The option family (5 members), all built from the four observed quantities via - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "fit_index",
        "fit index (the sagittal gap divided by the coverage margin)",
        "(corneal_reading - base_curve) / (lens_diameter - pupil_diameter)",
    ),
    (
        "sagittal_gap",
        "the sagittal gap (the corneal reading minus the base curve, the numerator divided by the coverage margin)",
        "corneal_reading - base_curve",
    ),
    (
        "coverage_margin",
        "the coverage margin (the lens diameter minus the pupil diameter, the denominator the sagittal gap is divided by)",
        "lens_diameter - pupil_diameter",
    ),
    (
        "crossed",
        "the corneal reading minus the base curve divided by the coverage margin, dropping the numerator parentheses so only the base curve is divided before subtracting (a wrong grouping)",
        "corneal_reading - base_curve / (lens_diameter - pupil_diameter)",
    ),
    (
        "swapped",
        "the corneal reading minus the base curve, divided by the lens diameter plus the pupil diameter, summing the denominator pair instead of subtracting (a wrong pairing)",
        "(corneal_reading - base_curve) / (lens_diameter + pupil_diameter)",
    ),
]
QUERIED = ["fit_index", "sagittal_gap", "coverage_margin"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(corneal_reading, base_curve, lens_diameter, pupil_diameter):
    # Operation order mirrors the ADJ programs exactly (the numerator difference forms, the denominator difference forms, then
    # the numerator is divided by the denominator, so (a-b)/(c-d) evaluates as ((a-b)/(c-d))), so the Python option value and
    # the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "fit_index": (corneal_reading - base_curve) / (lens_diameter - pupil_diameter),
        "sagittal_gap": corneal_reading - base_curve,
        "coverage_margin": lens_diameter - pupil_diameter,
        "crossed": corneal_reading - base_curve / (lens_diameter - pupil_diameter),
        "swapped": (corneal_reading - base_curve) / (lens_diameter + pupil_diameter),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for corneal_reading, base_curve, lens_diameter, pupil_diameter in TABLES:
        # Every observed quantity is a plain positive number >= 2, and the tables guarantee corneal_reading > base_curve
        # (sagittal gap > 0) and lens_diameter > pupil_diameter with a comfortable margin (coverage margin =
        # lens_diameter-pupil_diameter >= 2 => fit index finite and > 0), and a-b != c-d (numerator and denominator readouts
        # distinct); the swapped denominator (lens_diameter+pupil_diameter) is >= 4 with no subtraction, so nothing is ever
        # zero, negative, or undefined.
        assert (
            corneal_reading >= 2
            and base_curve >= 2
            and lens_diameter >= 2
            and pupil_diameter >= 2
        ), (corneal_reading, base_curve, lens_diameter, pupil_diameter)
        assert corneal_reading > base_curve, (
            corneal_reading, base_curve, lens_diameter, pupil_diameter,
        )
        assert lens_diameter - pupil_diameter >= 2, (
            corneal_reading, base_curve, lens_diameter, pupil_diameter,
        )
        assert (corneal_reading - base_curve) != (lens_diameter - pupil_diameter), (
            corneal_reading, base_curve, lens_diameter, pupil_diameter,
        )
        fv = family_values(corneal_reading, base_curve, lens_diameter, pupil_diameter)
        for key, v in fv.items():
            assert v > 0, (key, corneal_reading, base_curve, lens_diameter, pupil_diameter, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    corneal_reading,
                    base_curve,
                    lens_diameter,
                    pupil_diameter,
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
                corneal_reading,
                base_curve,
                lens_diameter,
                pupil_diameter,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r107clfit-{idx + 1:02d}",
                "qtype": "clfit_fit_index",
                "stem": (
                    f"A contact-lens fitting records a corneal reading of {num(corneal_reading)} minus a base curve of "
                    f"{num(base_curve)}, divided by a lens diameter of {num(lens_diameter)} minus a pupil diameter of "
                    f"{num(pupil_diameter)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe corneal_reading({num(corneal_reading)})\n"
                    f"observe base_curve({num(base_curve)})\n"
                    f"observe lens_diameter({num(lens_diameter)})\n"
                    f"observe pupil_diameter({num(pupil_diameter)})\n"
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
            "ADJ-LADDER rung 107 — contact-lens fit index from four stated quantities (a NEW panel: optometry / contact-lens "
            "fitting). From a corneal reading minus a base curve for the sagittal gap, a lens diameter minus a pupil diameter "
            "for the coverage margin, and the sagittal gap divided by the coverage margin, compute the fit index "
            "((corneal_reading-base_curve)/(lens_diameter-pupil_diameter)), the sagittal gap (corneal_reading-base_curve), or "
            "the coverage margin (lens_diameter-pupil_diameter). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A DIFFERENCE OVER A "
            "DIFFERENCE (a-b)/(c-d) (subtract b from a, subtract d from c, divide the numerator difference by the denominator "
            "difference, so (a-b)/(c-d) = ((a-b)/(c-d)); the FIRST time the ladder divides a bare DIFFERENCE by a bare "
            "DIFFERENCE — COMPLETING the difference-denominator family: rung-105 (a+b)/(c-d) sum/difference, rung-106 "
            "a*b/(c-d) product/difference, rung-107 difference/difference; rung-104 (a-b)/(c*d) divided a difference by a "
            "product, rung-37 (a+b)/(c+d) a sum by a sum) — and the harness matches the scalar to the printed options. The fit "
            "index is a dimensionless ratio, framed as an INDEX so the value stays honest. Contamination-safe: every figure is "
            "built only from the four observed quantities via - and / — no constant leaks, and neither the sagittal gap, the "
            "coverage margin, nor any index ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the same "
            "four quantities, so the distractors are exactly the slips students make: dropping the numerator parentheses so "
            "only the base curve is divided before subtracting (a-b/(c-d), a wrong grouping) and summing the denominator pair "
            "that should be a difference ((a-b)/(c+d), a wrong pairing). The core confusion tested is that (a-b)/(c-d) is "
            "((a-b)/(c-d)), not a-b/(c-d) and not (a-b)/(c+d). Each table guarantees the corneal reading exceeds the base "
            "curve and the lens diameter exceeds the pupil diameter with a comfortable margin (both differences strictly "
            "positive, fit index finite), the two differences are unequal (numerator and denominator readouts distinct), and "
            "the swapped denominator is a sum of quantities >= 2 (never zero, no subtraction), so every figure stays strictly "
            "positive and well-defined."
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
