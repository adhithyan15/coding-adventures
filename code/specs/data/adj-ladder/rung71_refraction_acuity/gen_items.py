"""Generate rung-71 (optometry corrected-acuity index) items.json for the ADJ-LADDER.

Rung 71 opens the **optometry / refraction** panel on the quantitative band — the arithmetic of a corrected visual
acuity index. A refraction test measures a `raw_acuity`, divides it by a `blur_factor` to deblur it, scales the result
by a `contrast_gain`, and adds a `baseline_offset`. Dividing FIRST, then scaling, then adding a term introduces a
genuinely NEW arithmetic shape on the ladder: a **quotient-times-factor plus a term** — `a/b*c+d`, i.e. `((a/b)*c)+d`.

This is the deliberate contrast to rung-69's `a*b/c+d` (skeletal traction), which MULTIPLIES first then divides; rung-71
DIVIDES first then scales. The operation order matters: `a/b*c` is left-to-right `((a/b)*c) = a*c/b`, NOT `a/(b*c)` —
the distractor exploits exactly that confusion. Contrast the other neighbours: rung-53 was `(a+b+c)/d` (a bare triple
sum over a divisor) and rung-68 was `(a+b)*c/d` (a sum scaled then divided). Here a quotient is scaled AND offset.

The setup: a `raw_acuity`, a `blur_factor`, a `contrast_gain`, and a `baseline_offset`. The corrected acuity is:

  CORRECTED ACUITY   raw_acuity / blur_factor * contrast_gain + baseline_offset   [ deblurred, gain-scaled, offset ]
  DEBLURRED          raw_acuity / blur_factor                                     [ the quotient ]
  GAINED             raw_acuity / blur_factor * contrast_gain                     [ the quotient scaled, before the offset ]

The **corrected acuity** is what makes this rung distinctive — it is the ladder's first **quotient scaled by a factor,
then a term added** (divide-first). (The deblurred acuity `raw_acuity / blur_factor` and the gained acuity `raw_acuity /
blur_factor * contrast_gain` ride alongside as component readouts, so the panel teaches the whole calculation — exactly
as rungs 47-70 shipped their component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the division by the blur factor, the multiplication by the contrast gain (left-to-right), and
the addition of the baseline offset — and the harness reads the scalar via the existing `compute_dimensioned` extractor.
No harness/engine change, exactly as rungs 8/16/.../69/70. This rung exercises the engine across **a quotient scaled then
offset** — the fact that `a/b*c+d` is NOT `a/(b*c)+d` and NOT `a*b/c+d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `*`, and `+`
— **no structural constants** — so no numeric literal appears in any program, and neither the deblurred acuity, the
gained acuity, nor any corrected figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`raw_acuity`, `blur_factor`, `contrast_gain`, `baseline_offset`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    raw_acuity / (blur_factor * contrast_gain) + baseline_offset   DIVIDE by the PRODUCT of the blur factor and
                                                                            contrast gain, not divide-then-multiply (the
                                                                            classic `a/b*c+d` vs `a/(b*c)+d` error), and
  SWAPPED    raw_acuity * blur_factor / contrast_gain + baseline_offset     MULTIPLY by the blur factor and divide by the
                                                                            contrast gain — the operations swapped
                                                                            (`a*b/c+d` instead of `a/b*c+d`),

which are exactly the mistakes a student makes (folding both denominators into one product, or swapping which quantity
divides and which multiplies). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness: all four observed quantities are strictly positive (so every family member is positive); the blur factor
and the contrast gain both exceed one (so the deblurred quotient differs from the gained value) and differ from each
other (so the corrected value `a*c/b` differs from the swapped value `a*b/c`); the five family values are pairwise
distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (RAW_ACUITY, BLUR_FACTOR, CONTRAST_GAIN, BASELINE_OFFSET) — a raw acuity to deblur, a blur factor to divide by, a
# contrast gain to scale by, and a baseline offset to add, all plain positive numbers with blur_factor > 1,
# contrast_gain > 1, and blur_factor != contrast_gain. The five family values are asserted pairwise-distinct below.
TABLES = [
    (24, 4, 3, 5),
    (30, 5, 2, 4),
    (20, 4, 3, 6),
    (36, 6, 3, 2),
    (28, 7, 2, 5),
    (45, 5, 3, 4),
    (18, 3, 4, 6),
]

# The option family (5 members), all built from the four observed quantities via /, *, and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "corrected_acuity",
        "corrected visual acuity (raw acuity deblurred, gain-scaled, then offset)",
        "raw_acuity / blur_factor * contrast_gain + baseline_offset",
    ),
    (
        "deblurred",
        "the deblurred acuity (raw acuity over the blur factor)",
        "raw_acuity / blur_factor",
    ),
    (
        "gained",
        "the gained acuity before adding the baseline offset (deblurred acuity times the contrast gain)",
        "raw_acuity / blur_factor * contrast_gain",
    ),
    (
        "crossed",
        "the raw acuity divided by the PRODUCT of the blur factor and contrast gain, not divide-then-multiply (a wrong scaling)",
        "raw_acuity / (blur_factor * contrast_gain) + baseline_offset",
    ),
    (
        "swapped",
        "the raw acuity MULTIPLIED by the blur factor and DIVIDED by the contrast gain, the operations swapped (a wrong scaling)",
        "raw_acuity * blur_factor / contrast_gain + baseline_offset",
    ),
]
QUERIED = ["corrected_acuity", "deblurred", "gained"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(raw_acuity, blur_factor, contrast_gain, baseline_offset):
    # Operation order mirrors the ADJ programs exactly (the left-to-right divide-then-multiply forms the quotient scaled
    # by the gain, then the trailing add), so the Python option value and the engine result are the same IEEE-double
    # (well within the harness's 1e-9 match tolerance).
    return {
        "corrected_acuity": raw_acuity / blur_factor * contrast_gain + baseline_offset,
        "deblurred": raw_acuity / blur_factor,
        "gained": raw_acuity / blur_factor * contrast_gain,
        "crossed": raw_acuity / (blur_factor * contrast_gain) + baseline_offset,
        "swapped": raw_acuity * blur_factor / contrast_gain + baseline_offset,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for raw_acuity, blur_factor, contrast_gain, baseline_offset in TABLES:
        assert (
            raw_acuity > 0
            and blur_factor > 0
            and contrast_gain > 0
            and baseline_offset > 0
        ), (raw_acuity, blur_factor, contrast_gain, baseline_offset)
        # Blur factor and contrast gain exceed one so the deblurred quotient differs from the gained value, and they
        # differ from each other so the corrected value (a*c/b) differs from the swapped value (a*b/c). All four
        # quantities are positive so every family member is positive.
        assert blur_factor > 1, (raw_acuity, blur_factor, contrast_gain, baseline_offset)
        assert contrast_gain > 1, (raw_acuity, blur_factor, contrast_gain, baseline_offset)
        assert blur_factor != contrast_gain, (raw_acuity, blur_factor, contrast_gain, baseline_offset)
        fv = family_values(raw_acuity, blur_factor, contrast_gain, baseline_offset)
        for key, v in fv.items():
            assert v > 0, (key, raw_acuity, blur_factor, contrast_gain, baseline_offset, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    raw_acuity,
                    blur_factor,
                    contrast_gain,
                    baseline_offset,
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
                raw_acuity,
                blur_factor,
                contrast_gain,
                baseline_offset,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r71refr-{idx + 1:02d}",
                "qtype": "refraction_acuity",
                "stem": (
                    f"A refraction test records a raw acuity of {num(raw_acuity)}, deblurred by a blur factor of "
                    f"{num(blur_factor)}, scaled by a contrast gain of {num(contrast_gain)} and offset by a baseline of "
                    f"{num(baseline_offset)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe raw_acuity({num(raw_acuity)})\n"
                    f"observe blur_factor({num(blur_factor)})\n"
                    f"observe contrast_gain({num(contrast_gain)})\n"
                    f"observe baseline_offset({num(baseline_offset)})\n"
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
            "ADJ-LADDER rung 71 — optometry corrected-acuity index from four stated quantities (a NEW panel: optometry "
            "/ refraction). From a raw acuity, a blur factor to divide by, a contrast gain to scale by, and a baseline "
            "offset to add, compute the corrected acuity "
            "(raw_acuity/blur_factor*contrast_gain+baseline_offset), the deblurred acuity (raw_acuity/blur_factor), or "
            "the gained acuity (raw_acuity/blur_factor*contrast_gain). Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, "
            "QUOTIENT-TIMES-FACTOR PLUS A TERM a/b*c+d (DIVIDE first, then scale, then offset — contrast rung-69 "
            "a*b/c+d which multiplies first; the left-to-right a/b*c = a*c/b, not a/(b*c)) — and the harness matches "
            "the scalar to the printed options. Contamination-safe: every index is built only from the four observed "
            "quantities via /, *, and + — no constant leaks, and neither the deblurred acuity, the gained acuity, nor "
            "any corrected figure ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the "
            "same four quantities, so the distractors are exactly the slips students make: DIVIDING by the PRODUCT "
            "(a/(b*c)+d, a wrong scaling) and SWAPPING the multiply and divide (a*b/c+d, a wrong scaling). The core "
            "confusion tested is that a/b*c+d is not a/(b*c)+d and not a*b/c+d."
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
