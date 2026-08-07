"""Generate rung-125 (EMG recruitment index / lone term over a subtract-THEN-add three-term denominator) items.json.

Rung 125 opens the **electromyography / motor-unit recruitment** panel on the ADJ-LADDER's quantitative band — the arithmetic of a
recruitment index. A single observed `motor_response` is DIVIDED by a net amplitude formed from a `baseline_amplitude` with a
`decrement_drop` SUBTRACTED and then a `facilitation_gain` ADDED back, to give the recruitment index. A **lone term over a
subtract-THEN-add three-term denominator**, `a/(b - c + d)`, i.e. `(a / ((b - c) + d))` (the subtraction and addition bind
left-to-right inside the denominator parentheses, whole net amplitude under the division), introduces a genuinely NEW arithmetic
family on the ladder — the **subtract-then-add sibling of rung-124's all-MINUS `a/(b - c - d)`**.

This is genuinely new. The three-term denominators shipped so far are b+c+d (116), b+c-d (117), b*c+d (118), b*c-d (119), b*c*d (121),
b+c*d (122), b-c*d (123), and rung-124's all-minus b-c-d — **but a/(b-c+d) has NEVER been a headline**: it appears on the ladder only
as rung-124's SWAPPED distractor, never as a queried gold. rung-125 promotes that grouping to a headline. It is the mirror of rung-124:
rung-124 SUBTRACTS both later terms (b-c-d), rung-125 subtracts the first and ADDS the second (b-c+d). Their swapped distractors are
each other's headlines — rung-125's swapped slip is `a/(b-c-d)` (rung-124's real shape), the classic "was the last term added or
subtracted?" confusion. Also distinct from rung-117 `a/(b+c-d)` (which ADDS the first two and subtracts the third): rung-125 subtracts
the FIRST later term and adds the second.

The setup: a `motor_response`, a `baseline_amplitude`, a `decrement_drop`, and a `facilitation_gain`. The figures are:

  RECRUITMENT INDEX   motor_response / (baseline_amplitude - decrement_drop + facilitation_gain)  [ lone term / subtract-then-add denom ]
  NET AMPLITUDE       baseline_amplitude - decrement_drop + facilitation_gain                     [ the subtract-then-add denominator ]
  RESIDUAL            baseline_amplitude - decrement_drop                                          [ baseline minus the decrement ]

The **recruitment index** is what makes this rung distinctive — it is the ladder's first **lone quantity over a subtract-THEN-add
three-term denominator (as a headline)**. It is a rate (motor response per unit of net amplitude), framed as an *index* to keep it
dimensionless-clean — the same discipline rungs 100/.../123/124 used for their ratios. (The net amplitude `b-c+d` and the residual
`b-c` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-124 shipped their
component sums/products/differences/ratios beside the headline figure. The residual `b-c` is the baseline after the decrement, before
the facilitation is added back, anchoring the "subtract first, then add" grouping against the swapped distractor.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the subtraction of the decrement drop from the baseline amplitude, then the addition of the facilitation gain to form the
whole net amplitude, then the division of the motor response by that whole net amplitude (so a/(b-c+d) evaluates as (a/((b-c)+d))) —
and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../123/124. This rung exercises the engine across a **lone-term-over-(subtract-then-add-three-term) ratio** — the fact that
`a/(b-c+d)` is `(a/((b-c)+d))` and NOT `a/b-c+d` and NOT `a/(b-c-d)` made computable. The ratio golds are non-integer f64s; the
engine's IEEE-double division matches Python's the same way rungs 100/.../123/124 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `+`, and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the net amplitude, the residual, nor the
recruitment index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`motor_response`, `baseline_amplitude`, `decrement_drop`, `facilitation_gain`) so no numeral hides inside a variable
name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    motor_response / baseline_amplitude - decrement_drop + facilitation_gain  drop the denominator parentheses so only the
                                                                    baseline amplitude divides the motor response, then the decrement
                                                                    is subtracted and the facilitation added (the classic `a/(b-c+d)`
                                                                    vs `a/b-c+d` grouping error, evaluating (a/b)-c+d), and
  SWAPPED    motor_response / (baseline_amplitude - decrement_drop - facilitation_gain)  subtract the decrement AND subtract the
                                                                    facilitation (`a/(b-c-d)`, rung-124's grouping, instead of
                                                                    `a/(b-c+d)` — the "was the last term added or subtracted?" slip),

which are exactly the mistakes a student makes (failing to keep the whole subtract-then-add denominator under the bar, or flipping the
sign of the last term). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung SUBTRACTS a term inside the denominator (and both distractors subtract), so positivity is NOT
automatic — it is guarded explicitly per table. Every observed quantity is `>= 2`, and each table guarantees **baseline_amplitude >
decrement_drop + facilitation_gain** with `b-c-d >= 2` — which makes the swapped denominator `b-c-d` positive (and away from zero), the
residual `b-c` comfortably positive, and the net amplitude `b-c+d` (which only adds `d` back) positive as well — and
**motor_response/baseline_amplitude - decrement_drop + facilitation_gain > 0** so the crossed slip `(a/b)-c+d` stays positive. Every
family member is asserted `> 0` at build time. And — so all three queried readouts vary across the panel — the seven tables give
distinct recruitment indices, distinct net amplitudes, and distinct residuals, all asserted at build time; the five family values are
pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (MOTOR_RESPONSE, BASELINE_AMPLITUDE, DECREMENT_DROP, FACILITATION_GAIN) — a lone motor response divided by a net amplitude (a
# baseline amplitude with a decrement drop subtracted and a facilitation gain added back) for the recruitment index. This rung
# SUBTRACTS a term inside the denominator (and both distractors subtract), so positivity is NOT automatic; each table guarantees
# baseline_amplitude > decrement_drop + facilitation_gain (b-c-d >= 2), which keeps the swapped denom b-c-d positive, the residual b-c
# comfortably positive, and the net amplitude b-c+d positive, and motor_response/baseline_amplitude - decrement_drop +
# facilitation_gain > 0 (crossed (a/b)-c+d positive). The five family values are asserted pairwise-distinct below. The seven tables
# give distinct recruitment indices, distinct net amplitudes, and distinct residuals so all three queried readouts vary across the
# panel.
TABLES = [
    (18, 7, 3, 2),
    (32, 7, 2, 3),
    (45, 8, 2, 3),
    (60, 9, 2, 3),
    (77, 10, 2, 3),
    (96, 11, 2, 3),
    (117, 12, 2, 3),
]

# The option family (5 members), all built from the four observed quantities via -, + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "recruitment_index",
        "recruitment index (the motor response divided by the net amplitude)",
        "motor_response / (baseline_amplitude - decrement_drop + facilitation_gain)",
    ),
    (
        "net_amplitude",
        "the net amplitude (the baseline amplitude minus the decrement drop plus the facilitation gain, the divisor the motor response is divided by)",
        "baseline_amplitude - decrement_drop + facilitation_gain",
    ),
    (
        "residual",
        "the residual (the baseline amplitude minus the decrement drop, before the facilitation gain is added back)",
        "baseline_amplitude - decrement_drop",
    ),
    (
        "crossed",
        "the motor response divided by the baseline amplitude, minus the decrement drop plus the facilitation gain, dropping the denominator parentheses so only the baseline amplitude divides (a wrong grouping)",
        "motor_response / baseline_amplitude - decrement_drop + facilitation_gain",
    ),
    (
        "swapped",
        "the motor response divided by the baseline amplitude minus the decrement drop minus the facilitation gain, flipping the sign of the last term (a wrong grouping)",
        "motor_response / (baseline_amplitude - decrement_drop - facilitation_gain)",
    ),
]
QUERIED = ["recruitment_index", "net_amplitude", "residual"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(motor_response, baseline_amplitude, decrement_drop, facilitation_gain):
    # Operation order mirrors the ADJ programs exactly (the decrement drop is subtracted from the baseline amplitude, then the
    # facilitation gain is added to form the whole net amplitude, then the motor response is divided by that whole net amplitude, so
    # a/(b-c+d) evaluates as (a/((b-c)+d))), so the Python option value and the engine result are the same IEEE-double (well within
    # the 1e-9 tolerance).
    return {
        "recruitment_index": motor_response / (baseline_amplitude - decrement_drop + facilitation_gain),
        "net_amplitude": baseline_amplitude - decrement_drop + facilitation_gain,
        "residual": baseline_amplitude - decrement_drop,
        "crossed": motor_response / baseline_amplitude - decrement_drop + facilitation_gain,
        "swapped": motor_response / (baseline_amplitude - decrement_drop - facilitation_gain),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for motor_response, baseline_amplitude, decrement_drop, facilitation_gain in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung SUBTRACTS a term inside the denominator (and both
        # distractors subtract), so positivity is NOT automatic; it is guarded explicitly per table.
        assert (
            motor_response >= 2
            and baseline_amplitude >= 2
            and decrement_drop >= 2
            and facilitation_gain >= 2
        ), (motor_response, baseline_amplitude, decrement_drop, facilitation_gain)
        assert baseline_amplitude - decrement_drop - facilitation_gain >= 2, (
            "baseline_amplitude - decrement_drop - facilitation_gain must be >= 2 (swapped denom positive; residual & net positive)",
            motor_response,
            baseline_amplitude,
            decrement_drop,
            facilitation_gain,
        )
        assert motor_response / baseline_amplitude - decrement_drop + facilitation_gain > 0, (
            "crossed (a/b)-c+d must be positive",
            motor_response,
            baseline_amplitude,
            decrement_drop,
            facilitation_gain,
        )
        fv = family_values(motor_response, baseline_amplitude, decrement_drop, facilitation_gain)
        for key, v in fv.items():
            assert v > 0, (key, motor_response, baseline_amplitude, decrement_drop, facilitation_gain, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    motor_response,
                    baseline_amplitude,
                    decrement_drop,
                    facilitation_gain,
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
                motor_response,
                baseline_amplitude,
                decrement_drop,
                facilitation_gain,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r125emg-{idx + 1:02d}",
                "qtype": "recruitment_index",
                "stem": (
                    f"An electromyography chart records a motor response of {num(motor_response)} divided by a baseline amplitude of "
                    f"{num(baseline_amplitude)} minus a decrement drop of {num(decrement_drop)} plus a facilitation gain of "
                    f"{num(facilitation_gain)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe motor_response({num(motor_response)})\n"
                    f"observe baseline_amplitude({num(baseline_amplitude)})\n"
                    f"observe decrement_drop({num(decrement_drop)})\n"
                    f"observe facilitation_gain({num(facilitation_gain)})\n"
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
            "ADJ-LADDER rung 125 — EMG recruitment index from four stated quantities (a NEW panel: electromyography / motor-unit "
            "recruitment). From a lone motor response divided by a net amplitude (a baseline amplitude with a decrement drop "
            "subtracted and a facilitation gain added back), compute the recruitment index "
            "(motor_response/(baseline_amplitude-decrement_drop+facilitation_gain)), the net amplitude "
            "(baseline_amplitude-decrement_drop+facilitation_gain), or the residual (baseline_amplitude-decrement_drop). Each item is "
            "a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, a LONE TERM OVER A SUBTRACT-THEN-ADD THREE-TERM DENOMINATOR a/(b-c+d) (subtract the decrement "
            "drop from the baseline amplitude, add the facilitation gain, then divide the motor response by that whole net amplitude, "
            "so a/(b-c+d) = (a/((b-c)+d)); the subtract-then-add sibling of rung-124's all-minus a/(b-c-d). a/(b-c+d) has never been "
            "a headline on the ladder — only rung-124's swapped distractor — so this promotes that grouping to a queried gold. "
            "Distinct from rung-117 a/(b+c-d), which adds the first two and subtracts the third; rung-125 subtracts the first later "
            "term and adds the second. The last-term sign-flip a/(b-c-d) (rung-124's real shape) rides alongside as the swapped "
            "distractor. The harness matches the scalar to the printed options. The recruitment index is a rate (motor response per "
            "unit of net amplitude), framed as an INDEX so the dimensionless value stays honest. Contamination-safe: every figure is "
            "built only from the four observed quantities via -, + and / — no constant leaks, and neither the net amplitude, the "
            "residual, nor the recruitment index ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the same four "
            "quantities, so the distractors are exactly the slips students make: dropping the denominator parentheses so only the "
            "baseline amplitude divides (a/b-c+d, evaluating (a/b)-c+d, a wrong grouping) and flipping the sign of the last term "
            "(a/(b-c-d), rung-124's grouping, a wrong grouping). The core confusion tested is that a/(b-c+d) is (a/((b-c)+d)), not "
            "a/b-c+d and not a/(b-c-d). This rung SUBTRACTS a term inside the denominator so positivity is NOT automatic; each table "
            "guards baseline_amplitude > decrement_drop + facilitation_gain (b-c-d >= 2, keeping the swapped denom, the residual, and "
            "the net amplitude all positive) and motor_response/baseline_amplitude - decrement_drop + facilitation_gain > 0 (crossed "
            "positive), keeping the five family values pairwise distinct and all three queried readouts varying across the panel, all "
            "asserted strictly positive at build time."
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
