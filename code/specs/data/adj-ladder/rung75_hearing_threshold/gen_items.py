"""Generate rung-75 (audiology corrected-hearing-threshold index) items.json for the ADJ-LADDER.

Rung 75 opens the **audiology / hearing-threshold** panel on the quantitative band — the arithmetic of a corrected
hearing threshold. An audiometry test measures a `raw_threshold`, subtracts a `masking_level`, normalises the remainder
by a `calibration_factor`, and scales it by a `frequency_gain`. Taking a DIFFERENCE FIRST, then dividing, then scaling
introduces a genuinely NEW arithmetic shape on the ladder: a **difference-quotient scaled by a factor** — `(a-b)/c*d`,
i.e. `((a-b)/c)*d`.

This is the deliberate contrast to every neighbour so far: rung-73/74 opened with a bare leading term (`a ∓ b/c*d`);
rung-75 leads with a PARENTHESISED difference that is then divided and scaled. It is the first ladder rung whose whole
numerator is a subtraction fed into a divide-then-multiply. The operation order matters: `(a-b)/c*d` is left-to-right
`((a-b)/c)*d = (a-b)*d/c`, NOT `(a-b)/(c*d)` — the distractor exploits exactly that confusion. Contrast the other
neighbours: rung-67 was `(a-b)*c/d` (a difference scaled THEN divided) and rung-53 was `(a+b+c)/d` (a bare triple sum
over a divisor). Here a difference is divided THEN scaled.

The setup: a `raw_threshold`, a `masking_level`, a `calibration_factor`, and a `frequency_gain`. The corrected
threshold is:

  CORRECTED THRESHOLD    (raw_threshold - masking_level) / calibration_factor * frequency_gain   [ diff, normalised, scaled ]
  NET THRESHOLD          raw_threshold - masking_level                                            [ the difference ]
  CALIBRATED THRESHOLD   (raw_threshold - masking_level) / calibration_factor                     [ the difference-quotient ]

The **corrected threshold** is what makes this rung distinctive — it is the ladder's first **difference-quotient scaled
by a factor** (divide the difference, then multiply). (The net threshold `raw_threshold - masking_level` and the
calibrated threshold `(raw_threshold - masking_level) / calibration_factor` ride alongside as component readouts, so the
panel teaches the whole calculation — exactly as rungs 47-74 shipped their component sums/products/differences/ratios
beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the masking level, the division by the calibration factor, and the
multiplication by the frequency gain (left-to-right) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../73/74. This rung exercises the
engine across **a difference-quotient scaled by a factor** — the fact that `(a-b)/c*d` is NOT `(a-b)/(c*d)` and NOT
`(a-b)*c/d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `/`, and `*`
— **no structural constants** — so no numeric literal appears in any program, and neither the net threshold, the
calibrated threshold, nor any corrected figure is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`raw_threshold`, `masking_level`, `calibration_factor`,
`frequency_gain`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (raw_threshold - masking_level) / (calibration_factor * frequency_gain)   DIVIDE the difference by the
                                                                                       PRODUCT of the calibration factor
                                                                                       and frequency gain, not divide-
                                                                                       then-multiply (the classic
                                                                                       `(a-b)/c*d` vs `(a-b)/(c*d)`
                                                                                       error), and
  SWAPPED    (raw_threshold - masking_level) * calibration_factor / frequency_gain     MULTIPLY the difference by the
                                                                                       calibration factor and divide by
                                                                                       the frequency gain — the
                                                                                       operations swapped (`(a-b)*c/d`
                                                                                       instead of `(a-b)/c*d`),

which are exactly the mistakes a student makes (folding both denominators into one product, or swapping which quantity
divides and which multiplies). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness: the raw threshold exceeds the masking level (`raw_threshold > masking_level`) so the difference is
positive and every family member — a positive difference times/over positive factors — is positive; the calibration
factor and the frequency gain both exceed one (so the calibrated difference-quotient differs from the net threshold)
and differ from each other (so the corrected value `(a-b)*d/c` differs from the swapped value `(a-b)*c/d`); the five
family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (RAW_THRESHOLD, MASKING_LEVEL, CALIBRATION_FACTOR, FREQUENCY_GAIN) — a raw threshold, a masking level to subtract, a
# calibration factor to divide by, and a frequency gain to scale by, all plain positive numbers with
# raw_threshold > masking_level (positive difference), calibration_factor > 1, frequency_gain > 1, and
# calibration_factor != frequency_gain. The five family values are asserted pairwise-distinct below.
TABLES = [
    (20, 8, 3, 2),
    (30, 6, 4, 3),
    (25, 5, 5, 2),
    (40, 4, 6, 3),
    (28, 13, 5, 3),
    (22, 6, 2, 3),
    (50, 10, 5, 4),
]

# The option family (5 members), all built from the four observed quantities via -, /, and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "corrected_threshold",
        "corrected hearing threshold (net threshold normalised by the calibration factor, then gain-scaled)",
        "(raw_threshold - masking_level) / calibration_factor * frequency_gain",
    ),
    (
        "net_threshold",
        "the net threshold (raw threshold minus the masking level)",
        "raw_threshold - masking_level",
    ),
    (
        "calibrated_threshold",
        "the calibrated threshold before scaling (net threshold over the calibration factor)",
        "(raw_threshold - masking_level) / calibration_factor",
    ),
    (
        "crossed",
        "the net threshold divided by the PRODUCT of the calibration factor and frequency gain, not divide-then-multiply (a wrong scaling)",
        "(raw_threshold - masking_level) / (calibration_factor * frequency_gain)",
    ),
    (
        "swapped",
        "the net threshold MULTIPLIED by the calibration factor and DIVIDED by the frequency gain, the operations swapped (a wrong scaling)",
        "(raw_threshold - masking_level) * calibration_factor / frequency_gain",
    ),
]
QUERIED = ["corrected_threshold", "net_threshold", "calibrated_threshold"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(raw_threshold, masking_level, calibration_factor, frequency_gain):
    # Operation order mirrors the ADJ programs exactly (the parenthesised difference is normalised by the calibration
    # factor, then the left-to-right multiply by the frequency gain), so the Python option value and the engine result
    # are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "corrected_threshold": (raw_threshold - masking_level) / calibration_factor * frequency_gain,
        "net_threshold": raw_threshold - masking_level,
        "calibrated_threshold": (raw_threshold - masking_level) / calibration_factor,
        "crossed": (raw_threshold - masking_level) / (calibration_factor * frequency_gain),
        "swapped": (raw_threshold - masking_level) * calibration_factor / frequency_gain,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for raw_threshold, masking_level, calibration_factor, frequency_gain in TABLES:
        assert (
            raw_threshold > 0
            and masking_level > 0
            and calibration_factor > 0
            and frequency_gain > 0
        ), (raw_threshold, masking_level, calibration_factor, frequency_gain)
        # The raw threshold exceeds the masking level so the difference is positive (=> every family member is
        # positive). The calibration factor and frequency gain exceed one so the calibrated difference-quotient differs
        # from the net threshold, and they differ from each other so the corrected value ((a-b)*d/c) differs from the
        # swapped value ((a-b)*c/d).
        assert raw_threshold > masking_level, (raw_threshold, masking_level, calibration_factor, frequency_gain)
        assert calibration_factor > 1, (raw_threshold, masking_level, calibration_factor, frequency_gain)
        assert frequency_gain > 1, (raw_threshold, masking_level, calibration_factor, frequency_gain)
        assert calibration_factor != frequency_gain, (raw_threshold, masking_level, calibration_factor, frequency_gain)
        fv = family_values(raw_threshold, masking_level, calibration_factor, frequency_gain)
        for key, v in fv.items():
            assert v > 0, (key, raw_threshold, masking_level, calibration_factor, frequency_gain, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    raw_threshold,
                    masking_level,
                    calibration_factor,
                    frequency_gain,
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
                raw_threshold,
                masking_level,
                calibration_factor,
                frequency_gain,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r75hear-{idx + 1:02d}",
                "qtype": "hearing_threshold",
                "stem": (
                    f"An audiometry test records a raw threshold of {num(raw_threshold)}, a masking level of "
                    f"{num(masking_level)} to subtract, a calibration factor of {num(calibration_factor)} to divide by "
                    f"and a frequency gain of {num(frequency_gain)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe raw_threshold({num(raw_threshold)})\n"
                    f"observe masking_level({num(masking_level)})\n"
                    f"observe calibration_factor({num(calibration_factor)})\n"
                    f"observe frequency_gain({num(frequency_gain)})\n"
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
            "ADJ-LADDER rung 75 — audiology corrected-hearing-threshold index from four stated quantities (a NEW panel: "
            "audiology / hearing-threshold). From a raw threshold, a masking level to subtract, a calibration factor to "
            "divide by, and a frequency gain to scale by, compute the corrected threshold "
            "((raw_threshold-masking_level)/calibration_factor*frequency_gain), the net threshold "
            "(raw_threshold-masking_level), or the calibrated threshold "
            "((raw_threshold-masking_level)/calibration_factor). Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, "
            "DIFFERENCE-QUOTIENT SCALED BY A FACTOR (a-b)/c*d (subtract first, then DIVIDE, then scale — contrast "
            "rung-67 (a-b)*c/d which scales THEN divides; the left-to-right (a-b)/c*d = (a-b)*d/c, not (a-b)/(c*d)) — "
            "and the harness matches the scalar to the printed options. Contamination-safe: every index is built only "
            "from the four observed quantities via -, /, and * — no constant leaks, and neither the net threshold, the "
            "calibrated threshold, nor any corrected figure ever appears as a literal (each is computed) — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same four quantities, so the distractors are exactly the slips students "
            "make: DIVIDING by the PRODUCT ((a-b)/(c*d), a wrong scaling) and SWAPPING the multiply and divide "
            "((a-b)*c/d, a wrong scaling). The core confusion tested is that (a-b)/c*d is not (a-b)/(c*d) and not "
            "(a-b)*c/d."
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
