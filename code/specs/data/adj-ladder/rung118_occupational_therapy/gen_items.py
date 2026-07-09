"""Generate rung-118 (occupational therapy / grip adaptation) items.json for the ADJ-LADDER.

Rung 118 opens the **occupational-therapy / grip-adaptation** panel on the quantitative band — the arithmetic of a grip
adaptation index. An `assisted_workload` (the total assisted task load the hand moves) is DIVIDED by the combined demand, where
that demand is a `repetition_rate` TIMES a `session_count` PLUS a `baseline_tone` (a resting-tone floor that the demand never
drops below), to give the adaptation index (workload per unit of combined demand). A **single term over a PRODUCT-PLUS-TERM
denominator**, `a/(b*c+d)`, i.e. `(a / ((b * c) + d))`, introduces a genuinely NEW arithmetic family on the ladder — the ladder's
**first denominator that is a product plus a term**.

This is genuinely new. The ladder's three-term denominators so far were both three-TERM SUMS/DIFFERENCES: rung-116 `a/(b+c+d)`
(pure sum) and rung-117 `a/(b+c-d)` (sum-minus-difference) — each denominator was `term + term ± term`, never a PRODUCT plus a
term. Every product-plus-term the ladder has built was a NUMERATOR over a single divisor: 110 `(a*b+c)/d`, 111 `(a*b-c)/d`; and
every two-term ratio with a product had the product on ONE side only (99 `(a*b)/(c+d)` product over a sum, 100 `(a+b)/(c*d)` sum
over a product, 104 `(a-b)/(c*d)`). Nobody has yet put a `b*c+d` UNDER the bar. Rung-118 is `a/(b*c+d)` — a single term divided by
a product-plus-term denominator. The operator order matters: `a/(b*c+d)` is `(a / ((b * c) + d))` (the repetition rate and session
count multiply FIRST, the baseline tone adds to that product, then the workload is divided by the whole combined demand; `*` binds
tighter than `+` inside the explicit denominator parentheses and the whole demand sits under the division), NOT `a/b*c+d`
(dropping the denominator parentheses so only the repetition rate divides the workload, then the result is multiplied by the
session count and the baseline tone added) and NOT `a/(b*c)+d` (keeping the product under the bar but ADDING the baseline tone
OUTSIDE the division instead of inside the denominator) — the two distractors exploit exactly those confusions.

The setup: an `assisted_workload`, a `repetition_rate`, a `session_count`, and a `baseline_tone`. The total is:

  GRIP ADAPTATION INDEX  assisted_workload / (repetition_rate * session_count + baseline_tone)  [ one term over a product-plus-term denominator ]
  COMBINED DEMAND        repetition_rate * session_count + baseline_tone                         [ the product-plus-term denominator ]
  ASSISTED WORKLOAD      assisted_workload                                                       [ the numerator ]

The **grip adaptation index** is what makes this rung distinctive — it is the ladder's first **single term over a
product-plus-term denominator**. It is a rate (workload per unit of combined demand), framed as an *index* to keep it
dimensionless-clean — the same discipline rungs 100/104/.../116/117 used for their ratios. (The combined demand `b*c+d` and the
assisted workload `a` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-117
shipped their component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the multiplication of the repetition rate and session count, the addition of the baseline tone into the combined
demand, then the division of the workload by that whole combined demand (the single-term numerator over the product-plus-term
denominator, so a/(b*c+d) evaluates as (a/((b*c)+d))) — and the harness reads the scalar via the existing `compute_dimensioned`
extractor. No harness/engine change, exactly as rungs 8/16/.../116/117. This rung exercises the engine across a
**product-plus-term denominator** — the fact that `a/(b*c+d)` is `(a/((b*c)+d))` and NOT `a/b*c+d` and NOT `a/(b*c)+d` made
computable. The ratio golds are non-integer f64s; the engine's IEEE-double division matches Python's the same way rungs
99/100/104/.../116/117 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `+`, and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the combined demand, the assisted workload, nor
any index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`assisted_workload`, `repetition_rate`, `session_count`, `baseline_tone`) so no numeral hides inside a variable
name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    assisted_workload / repetition_rate * session_count + baseline_tone  drop the denominator parentheses so only the
                                                                    repetition rate divides the workload, then the result is
                                                                    multiplied by the session count and the baseline tone added
                                                                    (the classic `a/(b*c+d)` vs `a/b*c+d` grouping error), and
  SWAPPED    assisted_workload / (repetition_rate * session_count) + baseline_tone  keep the product under the bar but ADD the
                                                                    baseline tone OUTSIDE the division instead of inside the
                                                                    denominator (`a/(b*c)+d` instead of `a/(b*c+d)`),

which are exactly the mistakes a student makes (failing to keep the whole combined demand under the bar, or adding the baseline
tone outside the denominator). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear
as options.

Distinctness and positivity: this rung builds every figure with `*`, `+` and `/` over strictly positive quantities, so positivity
is guaranteed by construction — no subtraction anywhere means every family member is strictly positive regardless of the table
(each observed quantity is `>= 2`, so the product-plus-term denominator `b*c+d >= 4 + 2 = 6` keeps the division away from zero).
The five family values are pairwise distinct with a comfortable margin; and — so all three queried readouts vary across the panel —
the seven tables give distinct adaptation indices, distinct combined demands, and distinct assisted workloads, all asserted at
build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (ASSISTED_WORKLOAD, REPETITION_RATE, SESSION_COUNT, BASELINE_TONE) — an assisted workload divided by the combined demand (a
# repetition rate times a session count, plus a baseline tone) for the adaptation index, all plain positive numbers >= 2. This
# rung uses only *, + and / over positives, so positivity is automatic; b*c+d >= 6 keeps the division away from zero. The five
# family values are asserted pairwise-distinct below. The seven tables give distinct adaptation indices, distinct combined demands,
# and distinct assisted workloads so all three queried readouts vary across the panel.
TABLES = [
    (30, 2, 3, 4),
    (36, 2, 4, 3),
    (40, 3, 3, 4),
    (45, 3, 4, 2),
    (50, 4, 3, 3),
    (57, 4, 4, 2),
    (63, 5, 3, 4),
]

# The option family (5 members), all built from the four observed quantities via *, + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "adaptation_index",
        "grip adaptation index (the assisted workload divided by the combined demand)",
        "assisted_workload / (repetition_rate * session_count + baseline_tone)",
    ),
    (
        "combined_demand",
        "the combined demand (the repetition rate times the session count plus the baseline tone, the divisor the workload is divided by)",
        "repetition_rate * session_count + baseline_tone",
    ),
    (
        "assisted_workload",
        "the assisted workload (the numerator divided by the combined demand)",
        "assisted_workload",
    ),
    (
        "crossed",
        "the assisted workload divided by the repetition rate, times the session count, plus the baseline tone, dropping the denominator parentheses so only the repetition rate divides (a wrong grouping)",
        "assisted_workload / repetition_rate * session_count + baseline_tone",
    ),
    (
        "swapped",
        "the assisted workload divided by the product of the repetition rate and the session count, plus the baseline tone, adding the baseline tone outside the division instead of inside the denominator (a wrong pairing)",
        "assisted_workload / (repetition_rate * session_count) + baseline_tone",
    ),
]
QUERIED = ["adaptation_index", "combined_demand", "assisted_workload"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(assisted_workload, repetition_rate, session_count, baseline_tone):
    # Operation order mirrors the ADJ programs exactly (the repetition rate and session count multiply first, the baseline tone
    # adds to that product, then the workload is divided by that whole combined demand, so a/(b*c+d) evaluates as (a/((b*c)+d))),
    # so the Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "adaptation_index": assisted_workload / (repetition_rate * session_count + baseline_tone),
        "combined_demand": repetition_rate * session_count + baseline_tone,
        "assisted_workload": assisted_workload,
        "crossed": assisted_workload / repetition_rate * session_count + baseline_tone,
        "swapped": assisted_workload / (repetition_rate * session_count) + baseline_tone,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for assisted_workload, repetition_rate, session_count, baseline_tone in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung uses only *, + and / over positives, so every
        # family member is strictly positive by construction (no subtraction anywhere); the product-plus-term denominator
        # b*c+d >= 6 keeps the division away from zero.
        assert (
            assisted_workload >= 2
            and repetition_rate >= 2
            and session_count >= 2
            and baseline_tone >= 2
        ), (assisted_workload, repetition_rate, session_count, baseline_tone)
        fv = family_values(assisted_workload, repetition_rate, session_count, baseline_tone)
        for key, v in fv.items():
            assert v > 0, (key, assisted_workload, repetition_rate, session_count, baseline_tone, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    assisted_workload,
                    repetition_rate,
                    session_count,
                    baseline_tone,
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
                assisted_workload,
                repetition_rate,
                session_count,
                baseline_tone,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r118ot-{idx + 1:02d}",
                "qtype": "adaptation_index",
                "stem": (
                    f"An occupational-therapy report records an assisted workload of {num(assisted_workload)} divided by a "
                    f"repetition rate of {num(repetition_rate)} times a session count of {num(session_count)} plus a baseline "
                    f"tone of {num(baseline_tone)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe assisted_workload({num(assisted_workload)})\n"
                    f"observe repetition_rate({num(repetition_rate)})\n"
                    f"observe session_count({num(session_count)})\n"
                    f"observe baseline_tone({num(baseline_tone)})\n"
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
            "ADJ-LADDER rung 118 — grip adaptation index from four stated quantities (a NEW panel: occupational therapy / grip "
            "adaptation). From an assisted workload divided by the combined demand (a repetition rate times a session count, plus "
            "a baseline tone), compute the grip adaptation index "
            "(assisted_workload/(repetition_rate*session_count+baseline_tone)), the combined demand "
            "(repetition_rate*session_count+baseline_tone), or the assisted workload. Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a SINGLE "
            "TERM OVER A PRODUCT-PLUS-TERM DENOMINATOR a/(b*c+d) (multiply the repetition rate and session count, add the baseline "
            "tone, divide the workload by that whole combined demand, so a/(b*c+d) = (a/((b*c)+d)); the ladder's FIRST denominator "
            "that is a product plus a term. The ladder's earlier three-term denominators were three-TERM sums/differences (116 "
            "a/(b+c+d), 117 a/(b+c-d)); every product-plus-term the ladder built was a NUMERATOR over a single divisor (110 "
            "(a*b+c)/d, 111 (a*b-c)/d); and every two-term ratio with a product had the product on ONE side only (99 (a*b)/(c+d), "
            "100 (a+b)/(c*d), 104 (a-b)/(c*d)) — rung-118 is the first to put a b*c+d UNDER the bar. The harness matches the scalar "
            "to the printed options. The grip adaptation index is a rate (workload per unit of combined demand), framed as an INDEX "
            "so the dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via *, + and / — no constant leaks, and neither the combined demand, the assisted workload, nor any index "
            "ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same four quantities, so the distractors are "
            "exactly the slips students make: dropping the denominator parentheses so only the repetition rate divides "
            "(a/b*c+d, a wrong grouping) and adding the baseline tone outside the division instead of inside the denominator "
            "(a/(b*c)+d, a wrong pairing). The core confusion tested is that a/(b*c+d) is (a/((b*c)+d)), not a/b*c+d and not "
            "a/(b*c)+d. This rung uses only *, + and / over strictly positive quantities, so positivity is guaranteed by "
            "construction (no subtraction anywhere); with every observed quantity >= 2 the denominator b*c+d >= 6, so the division "
            "is never by zero and every family member is strictly positive."
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
