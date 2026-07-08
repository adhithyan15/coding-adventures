"""Generate rung-106 (physical therapy / exercise prescription) items.json for the ADJ-LADDER.

Rung 106 opens the **physical therapy / exercise-prescription** panel on the quantitative band — the arithmetic of a
training-session intensity. A `resistance_level` (the load setting) TIMES a `repetition_count` (how many reps) gives the work
volume (the total mechanical work, the product), a `total_minutes` (the whole session) MINUS a `rest_minutes` (the time spent
resting between sets) gives the active minutes (the difference the work is spread across), and the work volume is DIVIDED by
the active minutes to give the exercise intensity. A **product over a difference** introduces a genuinely NEW arithmetic
family on the ladder: `a*b/(c-d)`, i.e. `((a*b) / (c-d))`.

This is genuinely new — the first time the ladder divides a bare PRODUCT by a bare DIFFERENCE. It extends the
**ratio-with-difference family** that rung-105 opened (a difference in the denominator is the fresh frontier): rung-105
`(a+b)/(c-d)` divided a SUM by a difference, rung-104 `(a-b)/(c*d)` a difference by a product, rung-100 `(a+b)/(c*d)` a sum by
a product, rung-99 `(a*b)/(c+d)` a product by a SUM — but no prior rung divided a PRODUCT by a DIFFERENCE. Rung-106 does: a
PRODUCT over a DIFFERENCE, the product-numerator sibling of rung-105's sum-numerator and the difference-denominator sibling of
rung-99's sum-denominator. The operator order matters: `a*b/(c-d)` is `((a*b) / (c-d))` (the product forms, the difference
forms, then the product is divided by the difference — the parenthesis on the denominator is what makes it a clean ratio),
NOT `a*b/c-d` (dropping the denominator parentheses so the product is divided by `total_minutes` alone and then the rest
minutes are subtracted) and NOT `(a*b)/(c+d)` (summing the denominator pair instead of subtracting, mispairing which pair is
the difference) — the two distractors exploit exactly those confusions.

The setup: a `resistance_level`, a `repetition_count`, a `total_minutes`, and a `rest_minutes`. The total is:

  EXERCISE INTENSITY  (resistance_level * repetition_count) / (total_minutes - rest_minutes)  [ a product over a difference ]
  WORK VOLUME         resistance_level * repetition_count                                      [ the product, the numerator ]
  ACTIVE MINUTES      total_minutes - rest_minutes                                             [ the difference, the denominator ]

The **exercise intensity** is what makes this rung distinctive — it is the ladder's first **bare PRODUCT over a bare
DIFFERENCE**. It is a rate (work volume per active minute), framed as an *intensity* to keep it dimensionless-clean — the same
discipline rungs 100/104/105 used for their ratios. (The work volume `a*b` and the active minutes `c-d` ride alongside as
component readouts, so the panel teaches the whole calculation — exactly as rungs 47-105 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the resistance level by the repetition count into the work volume, the
subtraction of the rest minutes from the total minutes into the active minutes, then the division of the work volume by the
active minutes (denominator parenthesized, so a*b/(c-d) evaluates as ((a*b)/(c-d))) — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../104/105. This rung exercises the
engine across a **product over a difference** — the fact that `a*b/(c-d)` is `((a*b)/(c-d))` and NOT `a*b/c-d` and NOT
`(a*b)/(c+d)` made computable. The ratio golds are non-integer f64s; the engine's IEEE-double division matches Python's the
same way rungs 99/100/104/105 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `-`, and `/` —
**no structural constants** — so no numeric literal appears in any program, and neither the work volume, the active minutes,
nor any intensity is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`resistance_level`, `repetition_count`, `total_minutes`, `rest_minutes`) so no numeral hides
inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    resistance_level * repetition_count / total_minutes - rest_minutes  drop the denominator parentheses so the work
                                                                                 volume is divided by the total minutes alone
                                                                                 and then the rest minutes are subtracted (the
                                                                                 classic `a*b/(c-d)` vs `a*b/c-d` precedence
                                                                                 error), and
  SWAPPED    (resistance_level * repetition_count) / (total_minutes + rest_minutes)  sum the denominator pair instead of
                                                                                 subtracting, mispairing which pair is the
                                                                                 difference (`(a*b)/(c+d)` instead of
                                                                                 `a*b/(c-d)`),

which are exactly the mistakes a student makes (dropping the denominator parentheses before dividing, or summing the
denominator pair that should be a difference). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts;
all five always appear as options.

Distinctness and positivity: the tables are chosen so `total_minutes > rest_minutes` with a comfortable margin (the active
minutes `total_minutes - rest_minutes` — the headline denominator — is strictly positive and the intensity stays finite and
clean, never blowing up on a tiny difference) and `resistance_level * repetition_count > total_minutes * rest_minutes` (so the
crossed figure `a*b/c - d` stays strictly positive: `a*b/c > d`), and every observed quantity is `>= 2`. The swapped
denominator `c+d >= 4` is a sum with no subtraction, never zero; the work volume `a*b` and every ratio are products/quotients
of positives. The tables are chosen so the five family values are pairwise distinct with a comfortable margin, and — so all
three queried readouts vary across the panel — the seven tables give distinct exercise intensities, distinct work-volume
figures, and distinct active-minute figures, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (RESISTANCE_LEVEL, REPETITION_COUNT, TOTAL_MINUTES, REST_MINUTES) — a resistance level times a repetition count for the
# work volume, a total-minutes minus a rest-minutes for the active minutes, all plain positive numbers >= 2. Each table
# satisfies total_minutes > rest_minutes with a comfortable margin (active_minutes = c-d >= 2 => intensity finite and > 0)
# and resistance_level*repetition_count > total_minutes*rest_minutes (so the crossed figure a*b/c - d stays > 0); the swapped
# denominator (c+d) is >= 4 with no subtraction, so nothing is ever zero or undefined. The five family values are asserted
# pairwise-distinct below. The seven tables give distinct exercise intensities, distinct work-volume figures, and distinct
# active-minute figures so all three queried readouts vary across the panel.
TABLES = [
    (6, 4, 5, 2),
    (7, 4, 6, 2),
    (9, 4, 7, 2),
    (5, 8, 9, 3),
    (10, 5, 10, 3),
    (12, 5, 11, 3),
    (11, 6, 12, 3),
]

# The option family (5 members), all built from the four observed quantities via *, -, and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "exercise_intensity",
        "exercise intensity (the work volume divided by the active minutes)",
        "(resistance_level * repetition_count) / (total_minutes - rest_minutes)",
    ),
    (
        "work_volume",
        "the work volume (the resistance level times the repetition count, the numerator divided by the active minutes)",
        "resistance_level * repetition_count",
    ),
    (
        "active_minutes",
        "the active minutes (the total minutes minus the rest minutes, the denominator the work volume is divided by)",
        "total_minutes - rest_minutes",
    ),
    (
        "crossed",
        "the work volume divided by the total minutes and then the rest minutes subtracted, dropping the denominator parentheses so only the total minutes divides before subtracting (a wrong grouping)",
        "resistance_level * repetition_count / total_minutes - rest_minutes",
    ),
    (
        "swapped",
        "the resistance level times the repetition count, divided by the total minutes plus the rest minutes, summing the denominator pair instead of subtracting (a wrong pairing)",
        "(resistance_level * repetition_count) / (total_minutes + rest_minutes)",
    ),
]
QUERIED = ["exercise_intensity", "work_volume", "active_minutes"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(resistance_level, repetition_count, total_minutes, rest_minutes):
    # Operation order mirrors the ADJ programs exactly (the product forms, the difference forms, then the product is divided
    # by the difference, so a*b/(c-d) evaluates as ((a*b)/(c-d))), so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "exercise_intensity": (resistance_level * repetition_count) / (total_minutes - rest_minutes),
        "work_volume": resistance_level * repetition_count,
        "active_minutes": total_minutes - rest_minutes,
        "crossed": (resistance_level * repetition_count) / total_minutes - rest_minutes,
        "swapped": (resistance_level * repetition_count) / (total_minutes + rest_minutes),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for resistance_level, repetition_count, total_minutes, rest_minutes in TABLES:
        # Every observed quantity is a plain positive number >= 2, and the tables guarantee total_minutes > rest_minutes
        # with a comfortable margin (active_minutes = total_minutes-rest_minutes >= 2 => intensity finite and > 0) and
        # resistance_level*repetition_count > total_minutes*rest_minutes (so the crossed figure a*b/c - d stays > 0); the
        # swapped denominator (total_minutes+rest_minutes) is >= 4 with no subtraction, so nothing is ever zero, negative,
        # or undefined.
        assert (
            resistance_level >= 2
            and repetition_count >= 2
            and total_minutes >= 2
            and rest_minutes >= 2
        ), (resistance_level, repetition_count, total_minutes, rest_minutes)
        assert total_minutes - rest_minutes >= 2, (
            resistance_level, repetition_count, total_minutes, rest_minutes,
        )
        assert resistance_level * repetition_count > total_minutes * rest_minutes, (
            resistance_level, repetition_count, total_minutes, rest_minutes,
        )
        fv = family_values(resistance_level, repetition_count, total_minutes, rest_minutes)
        for key, v in fv.items():
            assert v > 0, (key, resistance_level, repetition_count, total_minutes, rest_minutes, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    resistance_level,
                    repetition_count,
                    total_minutes,
                    rest_minutes,
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
                resistance_level,
                repetition_count,
                total_minutes,
                rest_minutes,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r106ptld-{idx + 1:02d}",
                "qtype": "ptld_exercise_intensity",
                "stem": (
                    f"A physical-therapy session records a resistance level of {num(resistance_level)} times a repetition "
                    f"count of {num(repetition_count)}, divided by a total time of {num(total_minutes)} minutes minus a rest "
                    f"time of {num(rest_minutes)} minutes. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe resistance_level({num(resistance_level)})\n"
                    f"observe repetition_count({num(repetition_count)})\n"
                    f"observe total_minutes({num(total_minutes)})\n"
                    f"observe rest_minutes({num(rest_minutes)})\n"
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
            "ADJ-LADDER rung 106 — physical-therapy exercise intensity from four stated quantities (a NEW panel: physical "
            "therapy / exercise prescription). From a resistance level times a repetition count for the work volume, a total "
            "time minus a rest time for the active minutes, and the work volume divided by the active minutes, compute the "
            "exercise intensity ((resistance_level*repetition_count)/(total_minutes-rest_minutes)), the work volume "
            "(resistance_level*repetition_count), or the active minutes (total_minutes-rest_minutes). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, A PRODUCT OVER A DIFFERENCE a*b/(c-d) (multiply a and b, subtract d from c, divide the "
            "product by the difference, so a*b/(c-d) = ((a*b)/(c-d)); the FIRST time the ladder divides a bare PRODUCT by a "
            "bare DIFFERENCE — extending the ratio-with-difference family rung-105 opened: rung-105 (a+b)/(c-d) divided a sum "
            "by a difference, rung-104 (a-b)/(c*d) a difference by a product, rung-99 (a*b)/(c+d) a product by a SUM, but no "
            "prior rung divided a product by a DIFFERENCE) — and the harness matches the scalar to the printed options. The "
            "exercise intensity is a rate (work volume per active minute), framed as an INTENSITY so the dimensionless value "
            "stays honest. Contamination-safe: every figure is built only from the four observed quantities via *, -, and / — "
            "no constant leaks, and neither the work volume, the active minutes, nor any intensity ever appears as a literal "
            "(each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a "
            "variable name. The five options are a family over the same four quantities, so the distractors are exactly the "
            "slips students make: dropping the denominator parentheses so only the total minutes divides before subtracting "
            "(a*b/c-d, a wrong grouping) and summing the denominator pair that should be a difference ((a*b)/(c+d), a wrong "
            "pairing). The core confusion tested is that a*b/(c-d) is ((a*b)/(c-d)), not a*b/c-d and not (a*b)/(c+d). Each "
            "table guarantees the total time exceeds the rest time with a comfortable margin (active minutes strictly "
            "positive, intensity finite) and the work volume exceeds total_minutes*rest_minutes (so the crossed figure stays "
            "positive), and the swapped denominator is a sum of quantities >= 2 (never zero, no subtraction), so every figure "
            "stays strictly positive and well-defined."
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
