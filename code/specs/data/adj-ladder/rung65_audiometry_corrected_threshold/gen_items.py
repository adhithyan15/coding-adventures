"""Generate rung-65 (audiometry corrected threshold) items.json for the ADJ-LADDER.

Rung 65 opens the **audiology / audiometry** panel on the quantitative band — the arithmetic of a corrected hearing
threshold. A pure-tone battery SUMS the measured thresholds across the test tones, subtracts a masking offset applied to
the non-test ear, divides by the number of tones to get the average threshold per tone, and finally ADDS a fixed
calibration baseline shift. Dividing a DIFFERENCE by a divisor and then ADDING a separate term introduces a genuinely NEW
arithmetic shape on the ladder: a **difference-over-a-divisor, plus a term** — `(a-b)/c + d`.

The pedagogical heart of this rung is **operator precedence**: `(a-b)/c + d` means `((a-b)/c) + d` — the baseline shift
`d` is added AFTER the division, it is NOT part of the denominator. That is exactly what distinguishes it from rung-64's
`(a-b)/(c+d)` (difference over a SUM), the single most tempting slip.

The setup: a `summed_threshold` across the tones, a `masking_offset` subtracted, a `tone_count` to divide by, and a
`baseline_shift` added. The corrected average threshold is:

  CORRECTED AVERAGE   (summed_threshold - masking_offset) / tone_count + baseline_shift   [ per-tone average + shift ]
  NET THRESHOLD       summed_threshold - masking_offset                                   [ the numerator: masked sum ]
  PER TONE            (summed_threshold - masking_offset) / tone_count                     [ the quotient, before shift ]

The **corrected average** is what makes this rung distinctive — it is the ladder's first **difference-over-divisor plus a
term**: a quotient of a difference, then a lone term added OUTSIDE the division. Contrast the neighbours already on the
ladder: rung-64 was `(a-b)/(c+d)` (a difference over a SUM — here the `+ baseline_shift` sits OUTSIDE the division, not
inside the denominator), rung-63 was `(a+b)/(c-d)`, and rung-53 was `(a+b+c)/d`. (The net threshold
`summed_threshold-masking_offset` and the per-tone quotient `(summed_threshold-masking_offset)/tone_count` ride alongside
as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-64 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the numerator difference, the division by the tone count, and the final addition of the baseline
shift — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change,
exactly as rungs 8/16/.../63/64. This rung exercises the engine across **division-then-addition precedence** — the fact
that `(a-b)/c+d` is NOT `(a-b)/(c+d)` and NOT `(a+b)/c+d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `-`, and `+`
— **no structural constants** — so no numeric literal appears in any program, and neither the net threshold, the per-tone
quotient, nor any corrected-average figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`summed_threshold`, `masking_offset`, `tone_count`, `baseline_shift`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  POOLED     (summed_threshold - masking_offset) / (tone_count + baseline_shift)   put the baseline shift in the
                                                                                   DENOMINATOR instead of adding it after
                                                                                   the division (the classic `(a-b)/c+d`
                                                                                   vs `(a-b)/(c+d)` precedence error), and
  CROSSED    (summed_threshold + masking_offset) / tone_count + baseline_shift      SUM the numerator instead of
                                                                                   differencing it (add the masking
                                                                                   offset instead of subtracting it),

which are exactly the mistakes a student makes (folding the added shift into the denominator, or adding the masking
offset instead of subtracting it). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness: all four observed quantities are strictly positive and the tables are chosen so the summed threshold exceeds
the masking offset (the net threshold — and therefore the per-tone quotient and the corrected average — is positive); the
five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (SUMMED_THRESHOLD, MASKING_OFFSET, TONE_COUNT, BASELINE_SHIFT) — a summed threshold and a masking offset, a tone count
# to divide by, and a baseline shift to add, all plain positive numbers with summed_threshold > masking_offset. The five
# family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (90, 30, 6, 10),
    (84, 24, 4, 5),
    (100, 20, 8, 6),
    (72, 12, 4, 9),
    (96, 36, 6, 4),
    (120, 30, 9, 5),
    (110, 50, 5, 8),
]

# The option family (5 members), all built from the four observed quantities via /, -, and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "corrected_average",
        "corrected average threshold (net masked sum over the tone count, plus the baseline shift)",
        "(summed_threshold - masking_offset) / tone_count + baseline_shift",
    ),
    (
        "net_threshold",
        "the net threshold (summed threshold minus the masking offset)",
        "summed_threshold - masking_offset",
    ),
    (
        "per_tone",
        "the per-tone average before the baseline shift (net threshold over the tone count)",
        "(summed_threshold - masking_offset) / tone_count",
    ),
    (
        "pooled",
        "the net threshold over the tone count PLUS the baseline shift folded into the denominator, not added after (a wrong divisor)",
        "(summed_threshold - masking_offset) / (tone_count + baseline_shift)",
    ),
    (
        "crossed",
        "the SUM of the threshold and masking offset over the tone count, plus the shift, not their difference (a wrong net threshold)",
        "(summed_threshold + masking_offset) / tone_count + baseline_shift",
    ),
]
QUERIED = ["corrected_average", "net_threshold", "per_tone"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(summed_threshold, masking_offset, tone_count, baseline_shift):
    # Operation order mirrors the ADJ programs exactly (the parenthesised difference formed first, then the division,
    # then the addition of the shift OUTSIDE the division), so the Python option value and the engine result are the same
    # IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "corrected_average": (summed_threshold - masking_offset) / tone_count + baseline_shift,
        "net_threshold": summed_threshold - masking_offset,
        "per_tone": (summed_threshold - masking_offset) / tone_count,
        "pooled": (summed_threshold - masking_offset) / (tone_count + baseline_shift),
        "crossed": (summed_threshold + masking_offset) / tone_count + baseline_shift,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for summed_threshold, masking_offset, tone_count, baseline_shift in TABLES:
        assert (
            summed_threshold > 0
            and masking_offset > 0
            and tone_count > 0
            and baseline_shift > 0
        ), (summed_threshold, masking_offset, tone_count, baseline_shift)
        # Net threshold must be positive (numerator), so the per-tone quotient and the corrected average are positive.
        assert summed_threshold > masking_offset, (summed_threshold, masking_offset, tone_count, baseline_shift)
        fv = family_values(summed_threshold, masking_offset, tone_count, baseline_shift)
        for key, v in fv.items():
            assert v > 0, (key, summed_threshold, masking_offset, tone_count, baseline_shift, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    summed_threshold,
                    masking_offset,
                    tone_count,
                    baseline_shift,
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
                summed_threshold,
                masking_offset,
                tone_count,
                baseline_shift,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r65aud-{idx + 1:02d}",
                "qtype": "audiometry_corrected_threshold",
                "stem": (
                    f"An audiometry battery sums to {num(summed_threshold)} units of threshold with a "
                    f"{num(masking_offset)} masking offset across {num(tone_count)} tones, then a {num(baseline_shift)} "
                    f"baseline shift is applied. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe summed_threshold({num(summed_threshold)})\n"
                    f"observe masking_offset({num(masking_offset)})\n"
                    f"observe tone_count({num(tone_count)})\n"
                    f"observe baseline_shift({num(baseline_shift)})\n"
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
            "ADJ-LADDER rung 65 — corrected average hearing threshold from four stated quantities (a NEW panel: "
            "audiology / audiometry). From a summed threshold and a masking offset (their difference is the net "
            "threshold), a tone count to divide by, and a baseline shift to add, compute the corrected average "
            "((summed_threshold-masking_offset)/tone_count+baseline_shift), the net threshold "
            "(summed_threshold-masking_offset), or the per-tone quotient "
            "((summed_threshold-masking_offset)/tone_count). Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, "
            "DIFFERENCE-OVER-A-DIVISOR PLUS A TERM (a-b)/c+d, the first on the ladder to divide a difference and then "
            "add a separate term OUTSIDE the division (distinct from rung-64 difference-over-sum (a-b)/(c+d), where the "
            "term is folded INTO the denominator, and rung-53 (a+b+c)/d) — and the harness matches the scalar to the "
            "printed options. The pedagogical heart is operator precedence: (a-b)/c+d is ((a-b)/c)+d, not (a-b)/(c+d). "
            "Contamination-safe: every index is built only from the four observed quantities via /, -, and + — no "
            "constant leaks, and neither the net threshold, the per-tone quotient, nor any corrected-average figure "
            "ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so "
            "no numeral hides inside a variable name. The five options are a family over the same four quantities, so "
            "the distractors are exactly the slips students make: folding the baseline shift INTO the denominator "
            "((a-b)/(c+d), a wrong divisor) and SUMMING the numerator ((a+b)/c+d, a wrong net threshold). The core "
            "confusion tested is that (a-b)/c+d is not (a-b)/(c+d) and not (a+b)/c+d."
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
