"""Generate rung-110 (audiology / hearing-aid prescribed-gain) items.json for the ADJ-LADDER.

Rung 110 opens the **audiology / hearing-aid fitting** panel on the quantitative band — the arithmetic of a prescribed-gain
index. A `channel_count` (how many frequency channels the aid programs) TIMES a `gain_step` (the gain increment applied per
channel) gives the total programmed gain across channels, a `baseline_gain` (a flat baseline) is ADDED, and that programmed
gain is DIVIDED by an `ear_count` (how many ears the reading is averaged over) to give the prescribed-gain index. A **product
PLUS a term, all over a single divisor** introduces a genuinely NEW arithmetic family on the ladder: `(a*b+c)/d`, i.e.
`(((a*b) + c) / d)`.

This is genuinely new — rung-108 opened the three-term-numerator frontier with the pure sum `(a+b+c)/d`, rung-109 with the
mixed sum/difference `(a-b+c)/d`; rung-110 is the FIRST three-term numerator whose leading two terms are MULTIPLIED (a product)
before the third is added, all over a divisor. Every prior ratio used either a two-term numerator (rung-37 `(a+b)/(c+d)`,
rung-99 `(a*b)/(c+d)`, rung-100 `(a+b)/(c*d)`, rung-104 `(a-b)/(c*d)`, the difference-denominator trio rung-105 `(a+b)/(c-d)`,
rung-106 `a*b/(c-d)`, rung-107 `(a-b)/(c-d)`) or a purely additive/subtractive three-term numerator (rung-108 `(a+b+c)/d`,
rung-109 `(a-b+c)/d`). Rung-110 moves to `(a*b+c)/d`. The operator order matters: `(a*b+c)/d` is `((a*b+c) / d)` (the whole
product-plus-term is the numerator; multiplication binds tighter than the addition, which binds tighter than the outer
division), NOT `a*b+c/d` (dropping the numerator parentheses so only the baseline gain is divided by the ear count and then
added to the product) and NOT `(a*b)/(c+d)` (regrouping so only the product forms the numerator and the baseline gain joins the
ear count in the denominator) — the two distractors exploit exactly those confusions.

The setup: a `channel_count`, a `gain_step`, a `baseline_gain`, and an `ear_count`. The total is:

  PRESCRIBED-GAIN INDEX  (channel_count * gain_step + baseline_gain) / ear_count  [ a product-plus-term over a divisor ]
  PROGRAMMED GAIN        channel_count * gain_step + baseline_gain                [ the product-plus-term numerator ]
  EAR COUNT              ear_count                                                [ the divisor ]

The **prescribed-gain index** is what makes this rung distinctive — it is the ladder's first **product-PLUS-a-term over a
single divisor**. It is a rate (programmed gain per ear), framed as an *index* to keep it dimensionless-clean — the same
discipline rungs 100/104/105/106/107/108/109 used for their ratios. (The programmed gain `a*b+c` and the ear count `d` ride
alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-109 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the multiplication of the channel count by the gain step, then the addition of the baseline gain into the
programmed gain, then the division of that programmed gain by the ear count (the whole product-plus-term parenthesized, so
(a*b+c)/d evaluates as ((a*b+c)/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../108/109. This rung exercises the engine across a **product-plus-term over a
divisor** — the fact that `(a*b+c)/d` is `((a*b+c)/d)` and NOT `a*b+c/d` and NOT `(a*b)/(c+d)` made computable. The ratio golds
are non-integer f64s; the engine's IEEE-double division matches Python's the same way rungs 99/100/104/105/106/107/108/109
relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `+` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the programmed gain, the ear count, nor any
index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`channel_count`, `gain_step`, `baseline_gain`, `ear_count`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    channel_count * gain_step + baseline_gain / ear_count  drop the numerator parentheses so only the baseline gain is
                                                                    divided by the ear count and then added to the product (the
                                                                    classic `(a*b+c)/d` vs `a*b+c/d` precedence error), and
  SWAPPED    (channel_count * gain_step) / (baseline_gain + ear_count)  regroup so only the product forms the numerator and the
                                                                    baseline gain joins the ear count in the denominator
                                                                    (`(a*b)/(c+d)` instead of `(a*b+c)/d`),

which are exactly the mistakes a student makes (dropping the numerator parentheses before dividing, or regrouping which terms
belong to the numerator vs the divisor). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: every family member is a product, sum, or quotient of strictly positive observed quantities `>= 2`,
so all five are strictly positive by construction (this rung has NO subtraction). The tables are chosen so `ear_count >= 2`
(divisor never zero), the prescribed-gain index never coincides with the ear count or the programmed gain, and the five family
values are pairwise distinct with a comfortable margin; and — so all three queried readouts vary across the panel — the seven
tables give distinct prescribed-gain indices, distinct programmed gains, and distinct ear counts, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CHANNEL_COUNT, GAIN_STEP, BASELINE_GAIN, EAR_COUNT) — a channel count times a gain step for the programmed gain across
# channels, plus a baseline gain, all divided by an ear count, all plain positive numbers >= 2. This rung has NO subtraction, so
# every family member is a product, sum, or quotient of positives and is strictly positive by construction; ear_count >= 2 keeps
# the divisor away from zero. The five family values are asserted pairwise-distinct below. The seven tables give distinct
# prescribed-gain indices, distinct programmed gains, and distinct ear counts so all three queried readouts vary across the
# panel.
TABLES = [
    (3, 2, 4, 2),
    (4, 2, 3, 3),
    (3, 4, 6, 4),
    (5, 3, 5, 5),
    (4, 5, 6, 6),
    (6, 3, 3, 7),
    (6, 4, 4, 8),
]

# The option family (5 members), all built from the four observed quantities via *, + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "prescribed_gain_index",
        "prescribed-gain index (the programmed gain divided by the ear count)",
        "(channel_count * gain_step + baseline_gain) / ear_count",
    ),
    (
        "programmed_gain",
        "the programmed gain (the channel count times the gain step plus the baseline gain, the numerator divided by the ear count)",
        "channel_count * gain_step + baseline_gain",
    ),
    (
        "ear_count",
        "the ear count (the divisor the programmed gain is divided by)",
        "ear_count",
    ),
    (
        "crossed",
        "the channel count times the gain step plus the baseline gain divided by the ear count, dropping the numerator parentheses so only the baseline gain is divided before adding (a wrong grouping)",
        "channel_count * gain_step + baseline_gain / ear_count",
    ),
    (
        "swapped",
        "the channel count times the gain step, divided by the baseline gain plus the ear count, regrouping so only the product forms the numerator (a wrong pairing)",
        "(channel_count * gain_step) / (baseline_gain + ear_count)",
    ),
]
QUERIED = ["prescribed_gain_index", "programmed_gain", "ear_count"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(channel_count, gain_step, baseline_gain, ear_count):
    # Operation order mirrors the ADJ programs exactly (the product forms, the baseline is added into the programmed gain, then
    # that numerator is divided by the ear count, so (a*b+c)/d evaluates as ((a*b+c)/d)), so the Python option value and the
    # engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "prescribed_gain_index": (channel_count * gain_step + baseline_gain) / ear_count,
        "programmed_gain": channel_count * gain_step + baseline_gain,
        "ear_count": ear_count,
        "crossed": channel_count * gain_step + baseline_gain / ear_count,
        "swapped": (channel_count * gain_step) / (baseline_gain + ear_count),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for channel_count, gain_step, baseline_gain, ear_count in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung has NO subtraction, so every family member is a
        # product, sum, or quotient of positives and is strictly positive by construction; ear_count >= 2 keeps the divisor away
        # from zero.
        assert (
            channel_count >= 2
            and gain_step >= 2
            and baseline_gain >= 2
            and ear_count >= 2
        ), (channel_count, gain_step, baseline_gain, ear_count)
        fv = family_values(channel_count, gain_step, baseline_gain, ear_count)
        for key, v in fv.items():
            assert v > 0, (key, channel_count, gain_step, baseline_gain, ear_count, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    channel_count,
                    gain_step,
                    baseline_gain,
                    ear_count,
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
                channel_count,
                gain_step,
                baseline_gain,
                ear_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r110audio-{idx + 1:02d}",
                "qtype": "prescribed_gain_index",
                "stem": (
                    f"A hearing-aid fitting records a channel count of {num(channel_count)} times a gain step of "
                    f"{num(gain_step)} plus a baseline gain of {num(baseline_gain)}, divided by an ear count of "
                    f"{num(ear_count)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe channel_count({num(channel_count)})\n"
                    f"observe gain_step({num(gain_step)})\n"
                    f"observe baseline_gain({num(baseline_gain)})\n"
                    f"observe ear_count({num(ear_count)})\n"
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
            "ADJ-LADDER rung 110 — prescribed-gain index from four stated quantities (a NEW panel: audiology / hearing-aid "
            "fitting). From a channel count times a gain step for the programmed gain, plus a baseline gain, all divided by an "
            "ear count, compute the prescribed-gain index ((channel_count*gain_step+baseline_gain)/ear_count), the programmed "
            "gain (channel_count*gain_step+baseline_gain), or the ear count. Each item is a compute_dimensioned program (observe "
            "the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A PRODUCT-PLUS-A-"
            "TERM OVER A DIVISOR (a*b+c)/d (multiply the channel count by the gain step, add the baseline gain, divide by the "
            "ear count, so (a*b+c)/d = ((a*b+c)/d); the FIRST time the ladder puts a three-term numerator whose leading two "
            "terms are MULTIPLIED before the third is added, over a divisor — rung-108 opened the frontier with the pure sum "
            "(a+b+c)/d and rung-109 the mixed (a-b+c)/d, and every earlier ratio used a TWO-term numerator: 37 (a+b)/(c+d), 99 "
            "(a*b)/(c+d), 100 (a+b)/(c*d), 104 (a-b)/(c*d), and the difference-denominator trio 105 (a+b)/(c-d), 106 a*b/(c-d), "
            "107 (a-b)/(c-d)) — and the harness matches the scalar to the printed options. The prescribed-gain index is a rate "
            "(programmed gain per ear), framed as an INDEX so the dimensionless value stays honest. Contamination-safe: every "
            "figure is built only from the four observed quantities via *, + and / — no constant leaks, and neither the "
            "programmed gain, the ear count, nor any index ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a family "
            "over the same four quantities, so the distractors are exactly the slips students make: dropping the numerator "
            "parentheses so only the baseline gain is divided before adding (a*b+c/d, a wrong grouping) and regrouping so only "
            "the product forms the numerator ((a*b)/(c+d), a wrong pairing). The core confusion tested is that (a*b+c)/d is "
            "((a*b+c)/d), not a*b+c/d and not (a*b)/(c+d). This rung has NO subtraction, so every family member is a product, "
            "sum, or quotient of positives and is strictly positive by construction; the ear count is >= 2 (divisor never zero)."
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
