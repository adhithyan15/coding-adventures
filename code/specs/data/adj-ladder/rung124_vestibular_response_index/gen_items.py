"""Generate rung-124 (vestibular response index / lone term over an all-MINUS three-term denominator) items.json.

Rung 124 opens the **vestibular-function / caloric-testing** panel on the ADJ-LADDER's quantitative band — the arithmetic of a
vestibular response index. A single observed `caloric_response` is DIVIDED by a net gain formed as a `baseline_gain` with a
`fixation_suppression` SUBTRACTED and then a `habituation_decrement` SUBTRACTED again, to give the vestibular response index. A **lone
term over an all-MINUS three-term denominator**, `a/(b - c - d)`, i.e. `(a / ((b - c) - d))` (both subtractions apply to the lone
`baseline_gain`, left-associatively), introduces a genuinely NEW arithmetic family on the ladder — the **all-subtraction sibling of
rung-116's `a/(b + c + d)`**, and the ladder's first denominator that subtracts TWO lone terms from a lone term.

This is genuinely new. The three-term denominators shipped so far all keep at least one PLUS or a PRODUCT: rung-116 `a/(b+c+d)`
(all-plus), rung-117 `a/(b+c-d)` (plus then minus), rung-118 `a/(b*c+d)`, rung-119 `a/(b*c-d)`, rung-121 `a/(b*c*d)`, rung-122
`a/(b+c*d)`, rung-123 `a/(b-c*d)`. rung-124 is the **all-MINUS** form `a/(b-c-d)` = `a/((b-c)-d)` — the baseline gain has BOTH losses
subtracted off it, and it is NOT a product-in-the-denominator shape at all. It is distinct from rung-117 `a/(b+c-d)` (which ADDS the
first two and subtracts the third): rung-124 subtracts BOTH `c` and `d` from the lone `b`, whereas rung-117 adds `c` to `b` first.
The sign-flip on the LAST subtraction, `a/(b-c+d)` (rung-117's mirror, itself not yet built), rides alongside as the swapped
distractor — the classic "was the last term added or subtracted?" confusion. Also distinct from the term-minus-product denominators
(122 `a/(b+c*d)`, 123 `a/(b-c*d)`), the product-over-product (120), and the lone-term-over-triple-product (121).

The setup: a `caloric_response`, a `baseline_gain`, a `fixation_suppression`, and a `habituation_decrement`. The figures are:

  VESTIBULAR INDEX    caloric_response / (baseline_gain - fixation_suppression - habituation_decrement)  [ lone term / all-minus denom ]
  NET GAIN            baseline_gain - fixation_suppression - habituation_decrement                       [ the all-minus denominator ]
  COMBINED LOSS       fixation_suppression + habituation_decrement                                       [ the two losses summed ]

The **vestibular index** is what makes this rung distinctive — it is the ladder's first **lone quantity over an all-MINUS three-term
denominator**. It is a rate (caloric response per unit of net vestibular gain), framed as an *index* to keep it dimensionless-clean —
the same discipline rungs 100/.../121/122/123 used for their ratios. (The net gain `b-c-d` and the combined loss `c+d` ride alongside
as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-123 shipped their component
sums/products/differences/ratios beside the headline figure. The combined loss `c+d` is the total amount deducted from the baseline
gain, reported straight, anchoring the "subtract both losses" grouping against the swapped distractor.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the subtraction of the fixation suppression from the baseline gain, then the subtraction of the habituation decrement to
form the whole net gain, then the division of the caloric response by that whole net gain (so a/(b-c-d) evaluates as (a/((b-c)-d))) —
and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../122/123. This rung exercises the engine across a **lone-term-over-(all-minus-three-term) ratio** — the fact that `a/(b-c-d)`
is `(a/((b-c)-d))` and NOT `a/b-c-d` and NOT `a/(b-c+d)` made computable. The ratio golds are non-integer f64s; the engine's IEEE-
double division matches Python's the same way rungs 100/.../122/123 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `+`, and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the net gain, the combined loss, nor the
vestibular index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`caloric_response`, `baseline_gain`, `fixation_suppression`, `habituation_decrement`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    caloric_response / baseline_gain - fixation_suppression - habituation_decrement  drop the denominator parentheses so only
                                                                    the baseline gain divides the caloric response, then the two losses
                                                                    are subtracted (the classic `a/(b-c-d)` vs `a/b-c-d` grouping
                                                                    error, evaluating (a/b)-c-d), and
  SWAPPED    caloric_response / (baseline_gain - fixation_suppression + habituation_decrement)  subtract the fixation suppression but
                                                                    ADD the habituation decrement (`a/(b-c+d)` instead of `a/(b-c-d)` —
                                                                    the "was the last term added or subtracted?" slip),

which are exactly the mistakes a student makes (failing to keep the whole all-minus denominator under the bar, or flipping the sign of
the last subtracted term). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: this rung SUBTRACTS two lone terms inside the denominator (and both distractors subtract), so positivity
is NOT automatic — it is guarded explicitly per table. Every observed quantity is `>= 2`, and each table guarantees **baseline_gain >
fixation_suppression + habituation_decrement** with the net gain `b-c-d >= 2` (headline denominator positive, away from zero) and
**caloric_response > baseline_gain*(fixation_suppression + habituation_decrement)** so the crossed slip `(a/b)-c-d` stays positive (its
`a/b` exceeds the subtracted `c+d`). The swapped denominator `b-c+d` is automatically positive whenever `b-c-d >= 2` (adding `d` back
only makes it larger), and is asserted `>= 2` for good measure. Every family member is asserted `> 0` at build time. And — so all
three queried readouts vary across the panel — the seven tables give distinct vestibular indices, distinct net gains, and distinct
combined losses, all asserted at build time; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CALORIC_RESPONSE, BASELINE_GAIN, FIXATION_SUPPRESSION, HABITUATION_DECREMENT) — a lone caloric response divided by a net gain (a
# baseline gain with a fixation suppression subtracted and a habituation decrement subtracted again) for the vestibular index. This
# rung SUBTRACTS two lone terms inside the denominator (and both distractors subtract), so positivity is NOT automatic; each table
# guarantees baseline_gain > fixation_suppression + habituation_decrement (net gain b-c-d >= 2), caloric_response >
# baseline_gain*(fixation_suppression + habituation_decrement) (crossed (a/b)-c-d positive), and the swapped denom b-c+d >= 2 (it is
# always > 0 once b-c-d >= 2). The five family values are asserted pairwise-distinct below. The seven tables give distinct vestibular
# indices, distinct net gains, and distinct combined losses so all three queried readouts vary across the panel.
TABLES = [
    (42, 8, 2, 3),
    (64, 10, 2, 4),
    (85, 12, 2, 5),
    (114, 14, 2, 6),
    (154, 16, 2, 7),
    (184, 18, 2, 8),
    (225, 20, 2, 9),
]

# The option family (5 members), all built from the four observed quantities via -, + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "vestibular_index",
        "vestibular index (the caloric response divided by the net gain)",
        "caloric_response / (baseline_gain - fixation_suppression - habituation_decrement)",
    ),
    (
        "net_gain",
        "the net gain (the baseline gain minus the fixation suppression minus the habituation decrement, the divisor the caloric response is divided by)",
        "baseline_gain - fixation_suppression - habituation_decrement",
    ),
    (
        "combined_loss",
        "the combined loss (the fixation suppression plus the habituation decrement, the total amount subtracted from the baseline gain)",
        "fixation_suppression + habituation_decrement",
    ),
    (
        "crossed",
        "the caloric response divided by the baseline gain, minus the fixation suppression minus the habituation decrement, dropping the denominator parentheses so only the baseline gain divides (a wrong grouping)",
        "caloric_response / baseline_gain - fixation_suppression - habituation_decrement",
    ),
    (
        "swapped",
        "the caloric response divided by the baseline gain minus the fixation suppression plus the habituation decrement, flipping the sign of the last subtracted term (a wrong grouping)",
        "caloric_response / (baseline_gain - fixation_suppression + habituation_decrement)",
    ),
]
QUERIED = ["vestibular_index", "net_gain", "combined_loss"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(caloric_response, baseline_gain, fixation_suppression, habituation_decrement):
    # Operation order mirrors the ADJ programs exactly (the fixation suppression is subtracted from the baseline gain, then the
    # habituation decrement is subtracted to form the whole net gain, then the caloric response is divided by that whole net gain, so
    # a/(b-c-d) evaluates as (a/((b-c)-d))), so the Python option value and the engine result are the same IEEE-double (well within
    # the 1e-9 tolerance).
    return {
        "vestibular_index": caloric_response / (baseline_gain - fixation_suppression - habituation_decrement),
        "net_gain": baseline_gain - fixation_suppression - habituation_decrement,
        "combined_loss": fixation_suppression + habituation_decrement,
        "crossed": caloric_response / baseline_gain - fixation_suppression - habituation_decrement,
        "swapped": caloric_response / (baseline_gain - fixation_suppression + habituation_decrement),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for caloric_response, baseline_gain, fixation_suppression, habituation_decrement in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung SUBTRACTS two lone terms inside the denominator (and both
        # distractors subtract), so positivity is NOT automatic; it is guarded explicitly per table.
        assert (
            caloric_response >= 2
            and baseline_gain >= 2
            and fixation_suppression >= 2
            and habituation_decrement >= 2
        ), (caloric_response, baseline_gain, fixation_suppression, habituation_decrement)
        assert baseline_gain - fixation_suppression - habituation_decrement >= 2, (
            "net gain baseline_gain - fixation_suppression - habituation_decrement must be >= 2",
            caloric_response,
            baseline_gain,
            fixation_suppression,
            habituation_decrement,
        )
        assert caloric_response > baseline_gain * (fixation_suppression + habituation_decrement), (
            "caloric_response must exceed baseline_gain*(fixation_suppression+habituation_decrement) so the crossed slip stays positive",
            caloric_response,
            baseline_gain,
            fixation_suppression,
            habituation_decrement,
        )
        assert baseline_gain - fixation_suppression + habituation_decrement >= 2, (
            "swapped denom baseline_gain - fixation_suppression + habituation_decrement must be >= 2",
            caloric_response,
            baseline_gain,
            fixation_suppression,
            habituation_decrement,
        )
        fv = family_values(caloric_response, baseline_gain, fixation_suppression, habituation_decrement)
        for key, v in fv.items():
            assert v > 0, (key, caloric_response, baseline_gain, fixation_suppression, habituation_decrement, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    caloric_response,
                    baseline_gain,
                    fixation_suppression,
                    habituation_decrement,
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
                caloric_response,
                baseline_gain,
                fixation_suppression,
                habituation_decrement,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r124vt-{idx + 1:02d}",
                "qtype": "vestibular_index",
                "stem": (
                    f"A vestibular caloric-testing chart records a caloric response of {num(caloric_response)} divided by a baseline "
                    f"gain of {num(baseline_gain)} minus a fixation suppression of {num(fixation_suppression)} minus a habituation "
                    f"decrement of {num(habituation_decrement)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe caloric_response({num(caloric_response)})\n"
                    f"observe baseline_gain({num(baseline_gain)})\n"
                    f"observe fixation_suppression({num(fixation_suppression)})\n"
                    f"observe habituation_decrement({num(habituation_decrement)})\n"
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
            "ADJ-LADDER rung 124 — vestibular response index from four stated quantities (a NEW panel: vestibular-function / caloric "
            "testing). From a lone caloric response divided by a net gain (a baseline gain with a fixation suppression subtracted and "
            "a habituation decrement subtracted again), compute the vestibular index "
            "(caloric_response/(baseline_gain-fixation_suppression-habituation_decrement)), the net gain "
            "(baseline_gain-fixation_suppression-habituation_decrement), or the combined loss "
            "(fixation_suppression+habituation_decrement). Each item is a compute_dimensioned program (observe the four quantities, "
            "let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a LONE TERM OVER AN ALL-MINUS THREE-TERM "
            "DENOMINATOR a/(b-c-d) (subtract the fixation suppression from the baseline gain, subtract the habituation decrement, then "
            "divide the caloric response by that whole net gain, so a/(b-c-d) = (a/((b-c)-d)); the ALL-SUBTRACTION sibling of "
            "rung-116's a/(b+c+d), and the ladder's first denominator that subtracts TWO lone terms from a lone term. Distinct from "
            "rung-117 a/(b+c-d), which adds the first two and subtracts the third; rung-124 subtracts BOTH c and d from the lone b. "
            "The last-term sign-flip a/(b-c+d) rides alongside as the swapped distractor. The harness matches the scalar to the "
            "printed options. The vestibular index is a rate (caloric response per unit of net vestibular gain), framed as an INDEX "
            "so the dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed "
            "quantities via -, + and / — no constant leaks, and neither the net gain, the combined loss, nor the vestibular index "
            "ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same four quantities, so the distractors are "
            "exactly the slips students make: dropping the denominator parentheses so only the baseline gain divides (a/b-c-d, "
            "evaluating (a/b)-c-d, a wrong grouping) and flipping the sign of the last subtracted term (a/(b-c+d), a wrong grouping). "
            "The core confusion tested is that a/(b-c-d) is (a/((b-c)-d)), not a/b-c-d and not a/(b-c+d). This rung SUBTRACTS two "
            "lone terms inside the denominator so positivity is NOT automatic; each table guards baseline_gain > "
            "fixation_suppression + habituation_decrement (net gain >= 2), caloric_response > "
            "baseline_gain*(fixation_suppression+habituation_decrement) (crossed positive), and baseline_gain - fixation_suppression "
            "+ habituation_decrement >= 2 (swapped positive), keeping the five family values pairwise distinct and all three queried "
            "readouts varying across the panel, all asserted strictly positive at build time."
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
