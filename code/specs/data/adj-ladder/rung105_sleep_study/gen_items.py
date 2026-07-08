"""Generate rung-105 (sleep medicine / polysomnography) items.json for the ADJ-LADDER.

Rung 105 opens the **sleep medicine / polysomnography** panel on the quantitative band — the arithmetic of an
apnea-hypopnea index. An `apnea_count` (complete airflow cessations) PLUS a `hypopnea_count` (partial reductions) gives the
respiratory events (how many breathing disruptions in all, the sum), a `recording_hours` (total time the study recorded)
MINUS a `wake_hours` (the time awake in bed) gives the sleep hours (the difference the events are spread across), and the
respiratory events are DIVIDED by the sleep hours to give the apnea-hypopnea index. A **sum over a difference** introduces a
genuinely NEW arithmetic family on the ladder: `(a+b)/(c-d)`, i.e. `((a+b) / (c-d))`.

This is genuinely new — the first time the ladder divides a bare SUM by a bare DIFFERENCE. It completes the
**ratio-with-difference family** the ladder has been building: rung-104 `(a-b)/(c*d)` divided a difference by a product,
rung-100 `(a+b)/(c*d)` a sum by a product, rung-99 `(a*b)/(c+d)` a product by a sum, rung-37 `(a+b)/(c+d)` a sum by a sum —
but no prior rung ever put a DIFFERENCE in the denominator. Rung-105 does: a SUM over a DIFFERENCE. The operator order
matters: `(a+b)/(c-d)` is `((a+b) / (c-d))` (the sum forms, the difference forms, then the sum is divided by the difference —
the parentheses on both sides are what make it a clean ratio), NOT `a+b/(c-d)` (dropping the numerator parentheses so only
the hypopnea count is divided by the sleep hours and then added to the apnea count) and NOT `(a-b)/(c+d)` (subtracting the
numerator pair and summing the denominator pair, mispairing which pair is the sum and which is the difference) — the two
distractors exploit exactly those confusions.

The setup: an `apnea_count`, a `hypopnea_count`, a `recording_hours`, and a `wake_hours`. The total is:

  APNEA-HYPOPNEA INDEX  (apnea_count + hypopnea_count) / (recording_hours - wake_hours)  [ a sum over a difference ]
  RESPIRATORY EVENTS    apnea_count + hypopnea_count                                      [ the sum, the numerator ]
  SLEEP HOURS           recording_hours - wake_hours                                      [ the difference, the denominator ]

The **apnea-hypopnea index** is what makes this rung distinctive — it is the ladder's first **bare SUM over a bare
DIFFERENCE**. It is a rate (respiratory events per hour of sleep), framed as an *index* to keep it dimensionless-clean — the
same discipline rungs 100/104 used for their ratios. (The respiratory events `a+b` and the sleep hours `c-d` ride alongside
as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-104 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the addition of the hypopnea count to the apnea count into the respiratory events, the subtraction
of the wake hours from the recording hours into the sleep hours, then the division of the respiratory events by the sleep
hours (both parenthesized, so (a+b)/(c-d) evaluates as ((a+b)/(c-d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../103/104. This rung exercises the engine
across a **sum over a difference** — the fact that `(a+b)/(c-d)` is `((a+b)/(c-d))` and NOT `a+b/(c-d)` and NOT `(a-b)/(c+d)`
made computable. The ratio golds are non-integer f64s; the engine's IEEE-double division matches Python's the same way
rungs 99/100/104 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-`, and `/` —
**no structural constants** — so no numeric literal appears in any program, and neither the respiratory events, the sleep
hours, nor any index is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`apnea_count`, `hypopnea_count`, `recording_hours`, `wake_hours`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    apnea_count + hypopnea_count / (recording_hours - wake_hours)  drop the numerator parentheses so only the
                                                                            hypopnea count is divided by the sleep hours and
                                                                            then added to the apnea count (the classic
                                                                            `(a+b)/(c-d)` vs `a+b/(c-d)` precedence error), and
  SWAPPED    (apnea_count - hypopnea_count) / (recording_hours + wake_hours)  subtract the numerator pair and sum the
                                                                            denominator pair, mispairing which pair is the
                                                                            sum and which is the difference (`(a-b)/(c+d)`
                                                                            instead of `(a+b)/(c-d)`),

which are exactly the mistakes a student makes (dropping the numerator parentheses before dividing, or mispairing which pair
is a sum and which is a difference). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: the tables are chosen so `apnea_count > hypopnea_count` (so the swapped numerator
`apnea_count - hypopnea_count` stays strictly positive) and `recording_hours > wake_hours` with a comfortable margin (so the
sleep hours `recording_hours - wake_hours` — the headline denominator — is strictly positive and the index stays finite and
clean, never blowing up on a tiny difference), and every observed quantity is `>= 2`. The swapped denominator `c+d >= 4` is
never zero and has no subtraction, so no family member is ever zero, negative, or undefined; the crossed figure
`a + b/(c-d)` is positive because every part is positive. The tables are chosen so the five family values are pairwise
distinct with a comfortable margin, and — so all three queried readouts vary across the panel — the seven tables give distinct
apnea-hypopnea indices, distinct respiratory-event counts, and distinct sleep-hour figures, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (APNEA_COUNT, HYPOPNEA_COUNT, RECORDING_HOURS, WAKE_HOURS) — an apnea count plus a hypopnea count for the respiratory
# events, a recording-hours minus a wake-hours for the sleep hours, all plain positive numbers >= 2. Each table satisfies
# apnea_count > hypopnea_count (so the swapped numerator a-b stays positive) and recording_hours > wake_hours with a
# comfortable margin (sleep_hours = c-d >= 2 => index finite and > 0); the swapped denominator (c+d) is >= 4 with no
# subtraction, so nothing is ever zero or undefined. The five family values are asserted pairwise-distinct below. The seven
# tables give distinct apnea-hypopnea indices, distinct respiratory-event counts, and distinct sleep-hour figures so all
# three queried readouts vary across the panel.
TABLES = [
    (6, 2, 8, 4),
    (8, 3, 9, 4),
    (9, 4, 10, 3),
    (12, 5, 11, 2),
    (7, 3, 9, 3),
    (14, 6, 13, 5),
    (11, 4, 8, 5),
]

# The option family (5 members), all built from the four observed quantities via +, -, and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "apnea_hypopnea_index",
        "apnea-hypopnea index (the respiratory events divided by the sleep hours)",
        "(apnea_count + hypopnea_count) / (recording_hours - wake_hours)",
    ),
    (
        "respiratory_events",
        "the respiratory events (the apnea count plus the hypopnea count, the numerator divided by the sleep hours)",
        "apnea_count + hypopnea_count",
    ),
    (
        "sleep_hours",
        "the sleep hours (the recording hours minus the wake hours, the denominator the respiratory events are divided by)",
        "recording_hours - wake_hours",
    ),
    (
        "crossed",
        "the apnea count plus the hypopnea count divided by the sleep hours, dropping the numerator parentheses so only the hypopnea count is divided before adding (a wrong grouping)",
        "apnea_count + hypopnea_count / (recording_hours - wake_hours)",
    ),
    (
        "swapped",
        "the apnea count minus the hypopnea count, divided by the recording hours plus the wake hours, subtracting the numerator pair and summing the denominator pair instead (a wrong pairing)",
        "(apnea_count - hypopnea_count) / (recording_hours + wake_hours)",
    ),
]
QUERIED = ["apnea_hypopnea_index", "respiratory_events", "sleep_hours"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(apnea_count, hypopnea_count, recording_hours, wake_hours):
    # Operation order mirrors the ADJ programs exactly (the sum forms, the difference forms, then the sum is divided by the
    # difference, so (a+b)/(c-d) evaluates as ((a+b)/(c-d))), so the Python option value and the engine result are the same
    # IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "apnea_hypopnea_index": (apnea_count + hypopnea_count) / (recording_hours - wake_hours),
        "respiratory_events": apnea_count + hypopnea_count,
        "sleep_hours": recording_hours - wake_hours,
        "crossed": apnea_count + hypopnea_count / (recording_hours - wake_hours),
        "swapped": (apnea_count - hypopnea_count) / (recording_hours + wake_hours),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for apnea_count, hypopnea_count, recording_hours, wake_hours in TABLES:
        # Every observed quantity is a plain positive number >= 2, and the tables guarantee apnea_count > hypopnea_count
        # (so the swapped numerator a-b stays positive) and recording_hours > wake_hours with a comfortable margin
        # (sleep_hours = recording_hours-wake_hours >= 2 => index finite and > 0); the swapped denominator
        # (recording_hours+wake_hours) is >= 4 with no subtraction, so nothing is ever zero, negative, or undefined.
        assert (
            apnea_count >= 2
            and hypopnea_count >= 2
            and recording_hours >= 2
            and wake_hours >= 2
        ), (apnea_count, hypopnea_count, recording_hours, wake_hours)
        assert apnea_count > hypopnea_count, (
            apnea_count, hypopnea_count, recording_hours, wake_hours,
        )
        assert recording_hours - wake_hours >= 2, (
            apnea_count, hypopnea_count, recording_hours, wake_hours,
        )
        fv = family_values(apnea_count, hypopnea_count, recording_hours, wake_hours)
        for key, v in fv.items():
            assert v > 0, (key, apnea_count, hypopnea_count, recording_hours, wake_hours, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    apnea_count,
                    hypopnea_count,
                    recording_hours,
                    wake_hours,
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
                apnea_count,
                hypopnea_count,
                recording_hours,
                wake_hours,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r105sleep-{idx + 1:02d}",
                "qtype": "sleep_apnea_hypopnea_index",
                "stem": (
                    f"A polysomnography study records an apnea count of {num(apnea_count)} plus a hypopnea count of "
                    f"{num(hypopnea_count)}, divided by a recording time of {num(recording_hours)} hours minus a wake time "
                    f"of {num(wake_hours)} hours. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe apnea_count({num(apnea_count)})\n"
                    f"observe hypopnea_count({num(hypopnea_count)})\n"
                    f"observe recording_hours({num(recording_hours)})\n"
                    f"observe wake_hours({num(wake_hours)})\n"
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
            "ADJ-LADDER rung 105 — polysomnography apnea-hypopnea index from four stated quantities (a NEW panel: sleep "
            "medicine / polysomnography). From an apnea count plus a hypopnea count for the respiratory events, a recording "
            "time minus a wake time for the sleep hours, and the respiratory events divided by the sleep hours, compute the "
            "apnea-hypopnea index ((apnea_count+hypopnea_count)/(recording_hours-wake_hours)), the respiratory events "
            "(apnea_count+hypopnea_count), or the sleep hours (recording_hours-wake_hours). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, A SUM OVER A DIFFERENCE (a+b)/(c-d) (add b to a, subtract d from c, divide the sum by "
            "the difference, so (a+b)/(c-d) = ((a+b)/(c-d)); the FIRST time the ladder divides a bare SUM by a bare "
            "DIFFERENCE — completing the ratio-with-difference family: rung-104 (a-b)/(c*d) divided a difference by a "
            "product, rung-100 (a+b)/(c*d) a sum by a product, rung-99 (a*b)/(c+d) a product by a sum, rung-37 (a+b)/(c+d) a "
            "sum by a sum, but no prior rung put a DIFFERENCE in the denominator) — and the harness matches the scalar to the "
            "printed options. The apnea-hypopnea index is a rate (events per sleep hour), framed as an INDEX so the "
            "dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via +, -, and / — no constant leaks, and neither the respiratory events, the sleep hours, nor any "
            "index ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so "
            "no numeral hides inside a variable name. The five options are a family over the same four quantities, so the "
            "distractors are exactly the slips students make: dropping the numerator parentheses so only the hypopnea count "
            "is divided before adding (a+b/(c-d), a wrong grouping) and subtracting the numerator pair while summing the "
            "denominator pair ((a-b)/(c+d), a wrong pairing). The core confusion tested is that (a+b)/(c-d) is "
            "((a+b)/(c-d)), not a+b/(c-d) and not (a-b)/(c+d). Each table guarantees the apnea count exceeds the hypopnea "
            "count and the recording time exceeds the wake time with a comfortable margin (sleep hours strictly positive, "
            "index finite), and the swapped denominator is a sum of quantities >= 2 (never zero, no subtraction), so every "
            "figure stays strictly positive and well-defined."
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
