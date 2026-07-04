"""Generate rung-68 (gait efficiency index) items.json for the ADJ-LADDER.

Rung 68 opens the **physical therapy / gait analysis** panel on the quantitative band — the arithmetic of a gait
efficiency index. An instrumented walkway measures the time the foot spends on the ground (the stance phase) and off it
(the swing phase); their SUM is the stride time. That stride is scaled by the walker's cadence and normalised to the leg
length, giving a per-leg geared stride. Multiplying a SUM by a FACTOR and then dividing by a normaliser introduces a
genuinely NEW arithmetic shape on the ladder: a **sum-times-factor over a divisor** — `(a+b)*c/d`, i.e. `((a+b)*c)/d`.

This is the deliberate MIRROR of rung-67's `(a-b)*c/d` (ergometry specific work): rung-67 scaled a DIFFERENCE by a factor
and divided; rung-68 scales a SUM. So the two rungs sit as a matched pair, and rung-68's `crossed` distractor is exactly
rung-67's headline shape (`(a-b)*c/d`) — a student who differences the two phases instead of summing them lands on the
neighbouring rung's formula.

The setup: a `stance_phase`, a `swing_phase`, a `cadence_count`, and a `leg_length`. The gait efficiency index is:

  GAIT INDEX   (stance_phase + swing_phase) * cadence_count / leg_length   [ geared stride per unit leg length ]
  STRIDE       stance_phase + swing_phase                                  [ the numerator sum (stride time) ]
  GEARED       (stance_phase + swing_phase) * cadence_count                [ the scaled sum, before dividing ]

The **gait index** is what makes this rung distinctive — it is the ladder's first **sum scaled by a factor, then
divided**. Contrast the neighbours already on the ladder: rung-67 was `(a-b)*c/d` (a DIFFERENCE scaled then divided),
rung-49 was `a*(b+c)` (a lone factor leading a parenthesised sum — no division), and rung-53 was `(a+b+c)/d` (a bare
triple sum over a divisor, no scaling factor). Here a sum is scaled AND normalised. (The stride `stance_phase +
swing_phase` and the geared stride `(stance_phase + swing_phase) * cadence_count` ride alongside as component readouts,
so the panel teaches the whole calculation — exactly as rungs 47-67 shipped their component sums/products/differences/
ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the numerator sum, the multiplication by the cadence, and the division by leg length — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../66/67. This rung exercises the engine across **a scaled sum over a divisor** — the fact that `(a+b)*c/d` is NOT
`(a-b)*c/d` and NOT `(a+b)/c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-`, `*`, and
`/` — **no structural constants** — so no numeric literal appears in any program, and neither the stride, the geared
stride, nor any gait-index figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`stance_phase`, `swing_phase`, `cadence_count`, `leg_length`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (stance_phase - swing_phase) * cadence_count / leg_length   DIFFERENCE the two phases instead of SUMMING
                                                                         them (the classic `(a+b)*c/d` vs `(a-b)*c/d`
                                                                         error — and exactly rung-67's shape), and
  SWAPPED    (stance_phase + swing_phase) / cadence_count * leg_length    SWAP the multiply and divide — divide by the
                                                                         cadence and multiply by the leg length
                                                                         (`(a+b)/c*d` instead of `(a+b)*c/d`),

which are exactly the mistakes a student makes (differencing the two phases, or swapping which quantity multiplies and
which divides). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness: all four observed quantities are strictly positive; the tables are chosen so the stance phase exceeds the
swing phase (so the DIFFERENCE distractor `crossed` stays strictly positive), the cadence count and the leg length both
exceed one and differ from each other (so stride != geared, gait index != geared, and gait index != swapped), and the
leg length is not the square of the cadence count (so geared != swapped); the five family values are pairwise distinct
with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (STANCE_PHASE, SWING_PHASE, CADENCE_COUNT, LEG_LENGTH) — a stance and a swing phase (their sum is the stride), a
# cadence count to scale by, and a leg length to normalise by, all plain positive numbers with stance > swing,
# cadence_count > 1, leg_length > 1, cadence_count != leg_length, and leg_length != cadence_count**2. The five family
# values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (60, 40, 6, 50),
    (70, 30, 8, 50),
    (80, 40, 5, 60),
    (90, 30, 6, 40),
    (100, 20, 8, 60),
    (110, 50, 7, 40),
    (75, 45, 6, 90),
]

# The option family (5 members), all built from the four observed quantities via +, -, *, and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "gait_index",
        "gait efficiency index (stride geared by cadence, per unit leg length)",
        "(stance_phase + swing_phase) * cadence_count / leg_length",
    ),
    (
        "stride",
        "the stride time (stance plus swing phase)",
        "stance_phase + swing_phase",
    ),
    (
        "geared",
        "the geared stride before normalising to leg length (stride times the cadence count)",
        "(stance_phase + swing_phase) * cadence_count",
    ),
    (
        "crossed",
        "the DIFFERENCE of the two phases geared and normalised, not their sum (a wrong stride)",
        "(stance_phase - swing_phase) * cadence_count / leg_length",
    ),
    (
        "swapped",
        "the stride DIVIDED by the cadence count and MULTIPLIED by the leg length, the multiply and divide swapped (a wrong scaling)",
        "(stance_phase + swing_phase) / cadence_count * leg_length",
    ),
]
QUERIED = ["gait_index", "stride", "geared"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(stance_phase, swing_phase, cadence_count, leg_length):
    # Operation order mirrors the ADJ programs exactly (the parenthesised sum formed first, then the left-to-right
    # multiply-then-divide), so the Python option value and the engine result are the same IEEE-double (well within the
    # harness's 1e-9 match tolerance).
    return {
        "gait_index": (stance_phase + swing_phase) * cadence_count / leg_length,
        "stride": stance_phase + swing_phase,
        "geared": (stance_phase + swing_phase) * cadence_count,
        "crossed": (stance_phase - swing_phase) * cadence_count / leg_length,
        "swapped": (stance_phase + swing_phase) / cadence_count * leg_length,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for stance_phase, swing_phase, cadence_count, leg_length in TABLES:
        assert (
            stance_phase > 0
            and swing_phase > 0
            and cadence_count > 0
            and leg_length > 0
        ), (stance_phase, swing_phase, cadence_count, leg_length)
        # Stance must exceed swing so the DIFFERENCE distractor (crossed) stays positive. The cadence count and leg
        # length exceed one and differ, and leg length is not the cadence count squared, so no two family members
        # collide structurally.
        assert stance_phase > swing_phase, (stance_phase, swing_phase, cadence_count, leg_length)
        assert cadence_count > 1 and leg_length > 1, (stance_phase, swing_phase, cadence_count, leg_length)
        assert cadence_count != leg_length, (stance_phase, swing_phase, cadence_count, leg_length)
        assert leg_length != cadence_count * cadence_count, (stance_phase, swing_phase, cadence_count, leg_length)
        fv = family_values(stance_phase, swing_phase, cadence_count, leg_length)
        for key, v in fv.items():
            assert v > 0, (key, stance_phase, swing_phase, cadence_count, leg_length, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    stance_phase,
                    swing_phase,
                    cadence_count,
                    leg_length,
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
                stance_phase,
                swing_phase,
                cadence_count,
                leg_length,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r68gait-{idx + 1:02d}",
                "qtype": "gait_efficiency_index",
                "stem": (
                    f"A gait analysis records a stance phase of {num(stance_phase)} and a swing phase of "
                    f"{num(swing_phase)}, scaled by a cadence count of {num(cadence_count)} and normalised to a leg "
                    f"length of {num(leg_length)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe stance_phase({num(stance_phase)})\n"
                    f"observe swing_phase({num(swing_phase)})\n"
                    f"observe cadence_count({num(cadence_count)})\n"
                    f"observe leg_length({num(leg_length)})\n"
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
            "ADJ-LADDER rung 68 — gait efficiency index from four stated quantities (a NEW panel: physical therapy / "
            "gait analysis). From a stance and a swing phase (their sum is the stride time), a cadence count to scale "
            "by, and a leg length to normalise by, compute the gait index "
            "((stance_phase+swing_phase)*cadence_count/leg_length), the stride (stance_phase+swing_phase), or the "
            "geared stride ((stance_phase+swing_phase)*cadence_count). Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, "
            "SUM-TIMES-FACTOR OVER A DIVISOR (a+b)*c/d, the first on the ladder to scale a sum by a factor and then "
            "divide (the mirror of rung-67 (a-b)*c/d, and distinct from rung-49 a*(b+c) with the factor leading and "
            "rung-53 (a+b+c)/d with no scaling factor) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via +, -, *, and / — no "
            "constant leaks, and neither the stride, the geared stride, nor any gait-index figure ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same four quantities, so the distractors "
            "are exactly the slips students make: DIFFERENCING the two phases ((a-b)*c/d, a wrong stride — and exactly "
            "rung-67's shape) and SWAPPING the multiply and divide ((a+b)/c*d, a wrong scaling). The core confusion "
            "tested is that (a+b)*c/d is not (a-b)*c/d and not (a+b)/c*d."
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
