"""Generate rung-61 (polysomnography apnea rate) items.json for the ADJ-LADDER.

Rung 61 opens the **sleep medicine / polysomnography** panel on the quantitative band — the arithmetic of a
sleep study. During a polysomnogram a patient lies in bed for some minutes, part of which is spent AWAKE; the
true SLEEP time is the bed time MINUS the awake time. The apnea RATE is the number of apneas divided by that
sleep time. Dividing a lone count by the DIFFERENCE of two other quantities introduces a genuinely NEW
arithmetic shape on the ladder: **one over a difference** — `a/(b-c)` — a single numerator over a subtraction
sitting in the denominator.

The setup: the study records `apnea_events` apneas while the patient is in bed for `bed_minutes`, of which
`awake_minutes` were spent awake. The apnea rate per minute of actual sleep is:

  APNEA RATE     apnea_events / (bed_minutes - awake_minutes)   [ apneas per minute of SLEEP ]
  SLEEP MINUTES  bed_minutes - awake_minutes                    [ the difference: true sleep time ]
  BED RATE       apnea_events / bed_minutes                     [ the naive rate over TOTAL bed time ]

The **apnea rate** is what makes this rung distinctive — it is the ladder's first **one over a difference**: a
lone quantity divided by a subtraction of two others. Contrast the neighbours already on the ladder: rung-59 was
`(a*b)/(c-d)` (a PRODUCT over a difference) and rung-32 was `(a-b)/(c-d)` (a DIFFERENCE over a difference); here
a SINGLE count sits over the difference. (The sleep time `bed_minutes-awake_minutes` and the bed rate
`apnea_events/bed_minutes` ride alongside as component readouts, so the panel teaches the whole calculation —
exactly as rungs 47-60 shipped their component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the three quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — the numerator, the parenthesised difference, and their quotient — and the harness
reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../59/60. This rung exercises the engine across **a division whose divisor is itself a subtraction** — the
fact that `a/(b-c)` is NOT `a/b - c` and NOT `a/(b+c)` made computable.

Contamination-safe by construction: every formula is built ONLY from the three observed quantities via `/`, `-`,
and `+` — **no structural constants** — so no numeric literal appears in any program, and neither the sleep time,
the bed rate, nor any apnea-rate figure is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`apnea_events`, `bed_minutes`, `awake_minutes`) so no
numeral hides inside a variable name.

The five options are a tight family over the same three quantities: the three real readouts plus the two classic
slips —

  POOLED     apnea_events / (bed_minutes + awake_minutes)   SUM the denominator instead of DIFFERENCING it (the
                                                            classic `a/(b-c)` vs `a/(b+c)` error), and
  CROSSED    apnea_events / awake_minutes                   divide by the AWAKE minutes instead of the sleep
                                                            minutes (the wrong single denominator),

which are exactly the mistakes a student makes (adding the two times instead of subtracting, or dividing by the
wrong one). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear
as options.

Distinctness: all three observed quantities are strictly positive and the tables are chosen so the bed time
exceeds the awake time (the sleep time — and therefore every rate — is positive). The tables also avoid
`bed_minutes == 2*awake_minutes` (which would collide the apnea rate with the crossed distractor) and
`apnea_events == (bed_minutes-awake_minutes)**2` (which would collide the apnea rate with the sleep-minutes
component); the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (APNEA_EVENTS, BED_MINUTES, AWAKE_MINUTES) — an apnea count and two timed minutes (total bed time and the
# awake minutes within it), all plain positive numbers with bed_minutes > awake_minutes so the sleep time (and
# every rate) is positive. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (120, 480, 80),
    (90, 420, 60),
    (150, 500, 100),
    (60, 360, 60),
    (200, 540, 120),
    (75, 300, 50),
    (180, 450, 90),
]

# The option family (5 members), all built from the three observed quantities via /, -, and +. Every identifier
# is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all
# five always appear as the options.
FAMILY = [
    (
        "apnea_rate",
        "apnea rate per minute of actual sleep (apneas over the bed-minus-awake sleep time)",
        "apnea_events / (bed_minutes - awake_minutes)",
    ),
    (
        "sleep_minutes",
        "the true sleep time (bed minutes minus awake minutes)",
        "bed_minutes - awake_minutes",
    ),
    (
        "bed_rate",
        "the naive apnea rate over TOTAL bed time (apneas per bed minute)",
        "apnea_events / bed_minutes",
    ),
    (
        "pooled",
        "apneas over the SUM of bed and awake minutes, not the difference (a wrong sleep time)",
        "apnea_events / (bed_minutes + awake_minutes)",
    ),
    (
        "crossed",
        "apneas divided by the AWAKE minutes instead of the sleep minutes (wrong denominator)",
        "apnea_events / awake_minutes",
    ),
]
QUERIED = ["apnea_rate", "sleep_minutes", "bed_rate"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(apnea_events, bed_minutes, awake_minutes):
    # Operation order mirrors the ADJ programs exactly (the parenthesised difference/sum formed first, then the
    # division), so the Python option value and the engine result are the same IEEE-double (well within the
    # harness's 1e-9 match tolerance).
    return {
        "apnea_rate": apnea_events / (bed_minutes - awake_minutes),
        "sleep_minutes": bed_minutes - awake_minutes,
        "bed_rate": apnea_events / bed_minutes,
        "pooled": apnea_events / (bed_minutes + awake_minutes),
        "crossed": apnea_events / awake_minutes,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for apnea_events, bed_minutes, awake_minutes in TABLES:
        assert apnea_events > 0 and bed_minutes > 0 and awake_minutes > 0, (
            apnea_events,
            bed_minutes,
            awake_minutes,
        )
        # bed time must exceed awake time so the sleep time (the divisor) is positive.
        assert bed_minutes > awake_minutes, (apnea_events, bed_minutes, awake_minutes)
        # Guard the two structural collisions the a/(b-c) shape admits.
        assert bed_minutes != 2 * awake_minutes, (apnea_events, bed_minutes, awake_minutes)
        assert apnea_events != (bed_minutes - awake_minutes) ** 2, (
            apnea_events,
            bed_minutes,
            awake_minutes,
        )
        fv = family_values(apnea_events, bed_minutes, awake_minutes)
        # Every family member must be positive (a sensible reading).
        for key, v in fv.items():
            assert v > 0, (key, apnea_events, bed_minutes, awake_minutes, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    apnea_events,
                    bed_minutes,
                    awake_minutes,
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
                apnea_events,
                bed_minutes,
                awake_minutes,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r61apnea-{idx + 1:02d}",
                "qtype": "polysomnography_apnea_rate",
                "stem": (
                    f"A polysomnogram records {num(apnea_events)} apneas over {num(bed_minutes)} min in bed, of "
                    f"which {num(awake_minutes)} min were spent awake. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe apnea_events({num(apnea_events)})\n"
                    f"observe bed_minutes({num(bed_minutes)})\n"
                    f"observe awake_minutes({num(awake_minutes)})\n"
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
            "ADJ-LADDER rung 61 — apnea rate per minute of sleep from three stated quantities (a NEW panel: sleep "
            "medicine / polysomnography). From an apnea count, the total bed minutes, and the awake minutes within "
            "them (the sleep time is bed minus awake), compute the apnea rate "
            "(apnea_events/(bed_minutes-awake_minutes)), the sleep minutes (bed_minutes-awake_minutes), or the naive "
            "bed rate (apnea_events/bed_minutes). Each item is a compute_dimensioned program (observe the three "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, ONE OVER A "
            "DIFFERENCE a/(b-c), the first on the ladder to divide a lone numerator by a subtraction (distinct from "
            "rung-59 product-over-difference (a*b)/(c-d) and rung-32 difference-over-difference (a-b)/(c-d)) — and the "
            "harness matches the scalar to the printed options. Contamination-safe: every index is built only from "
            "the three observed quantities via /, -, and + — no constant leaks, and neither the sleep time, the bed "
            "rate, nor any apnea-rate figure ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are "
            "a family over the same three quantities, so the distractors are exactly the slips students make: SUMMING "
            "the denominator (a/(b+c), the classic a/(b-c) vs a/(b+c) error) and dividing by the AWAKE minutes "
            "instead of the sleep minutes (a/c). The core confusion tested is that a/(b-c) is not a/b - c and not "
            "a/(b+c)."
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
