"""Generate rung-119 (allergy skin testing / reactivity index) items.json for the ADJ-LADDER.

Rung 119 opens the **allergy-skin-testing / reactivity** panel on the quantitative band — the arithmetic of a skin-test reactivity
index. An `allergen_load` (the total allergen dose applied across the panel) is DIVIDED by the net reactive demand, where that
demand is a `wheal_rate` TIMES a `panel_count` MINUS a `baseline_reactivity` (a background-reactivity floor that is subtracted out
of the raw product), to give the reactivity index (allergen load per unit of net reactive demand). A **single term over a
PRODUCT-MINUS-TERM denominator**, `a/(b*c-d)`, i.e. `(a / ((b * c) - d))`, introduces a genuinely NEW arithmetic family on the
ladder — the ladder's **first denominator that is a product minus a term**.

This is genuinely new. Rung-118 put the ladder's first product-PLUS-term under the bar, `a/(b*c+d)`. Rung-119 is its
subtraction twin: `a/(b*c-d)`, a product MINUS a term under the bar. The ladder's three-term denominators before rung-118 were
three-TERM sums/differences: rung-116 `a/(b+c+d)` (pure sum) and rung-117 `a/(b+c-d)` (sum-minus-difference); rung-118 introduced
the product-plus-term denominator `a/(b*c+d)`. Every product-minus-term the ladder has built was a NUMERATOR over a single divisor:
111 `(a*b-c)/d`; and every two-term ratio with a product had the product on ONE side only (99 `(a*b)/(c+d)`, 100 `(a+b)/(c*d)`,
104 `(a-b)/(c*d)`, 106 `a*b/(c-d)`). Nobody has yet put a `b*c-d` UNDER the bar. Rung-119 is `a/(b*c-d)` — a single term divided by
a product-minus-term denominator. The operator order matters: `a/(b*c-d)` is `(a / ((b * c) - d))` (the wheal rate and panel count
multiply FIRST, the baseline reactivity is subtracted from that product, then the allergen load is divided by the whole net
reactive demand; `*` binds tighter than `-` inside the explicit denominator parentheses and the whole net demand sits under the
division), NOT `a/b*c-d` (dropping the denominator parentheses so only the wheal rate divides the allergen load, then the result
is multiplied by the panel count and the baseline reactivity subtracted) and NOT `a/(b*c)-d` (keeping the product under the bar
but SUBTRACTING the baseline reactivity OUTSIDE the division instead of inside the denominator) — the two distractors exploit
exactly those confusions.

The setup: an `allergen_load`, a `wheal_rate`, a `panel_count`, and a `baseline_reactivity`. The total is:

  REACTIVITY INDEX      allergen_load / (wheal_rate * panel_count - baseline_reactivity)  [ one term over a product-minus-term denominator ]
  NET REACTIVE DEMAND   wheal_rate * panel_count - baseline_reactivity                    [ the product-minus-term denominator ]
  ALLERGEN LOAD         allergen_load                                                     [ the numerator ]

The **reactivity index** is what makes this rung distinctive — it is the ladder's first **single term over a product-minus-term
denominator**. It is a rate (allergen load per unit of net reactive demand), framed as an *index* to keep it dimensionless-clean —
the same discipline rungs 100/104/.../117/118 used for their ratios. (The net reactive demand `b*c-d` and the allergen load `a`
ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-118 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication of the wheal rate and panel count, the subtraction of the baseline reactivity from that product to
form the net reactive demand, then the division of the allergen load by that whole net reactive demand (the single-term numerator
over the product-minus-term denominator, so a/(b*c-d) evaluates as (a/((b*c)-d))) — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../117/118. This rung exercises the
engine across a **product-minus-term denominator** — the fact that `a/(b*c-d)` is `(a/((b*c)-d))` and NOT `a/b*c-d` and NOT
`a/(b*c)-d` made computable. The ratio golds are non-integer f64s; the engine's IEEE-double division matches Python's the same way
rungs 99/100/104/.../117/118 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `-`, and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the net reactive demand, the allergen load, nor
any index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`allergen_load`, `wheal_rate`, `panel_count`, `baseline_reactivity`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    allergen_load / wheal_rate * panel_count - baseline_reactivity  drop the denominator parentheses so only the wheal
                                                                    rate divides the allergen load, then the result is multiplied
                                                                    by the panel count and the baseline reactivity subtracted (the
                                                                    classic `a/(b*c-d)` vs `a/b*c-d` grouping error), and
  SWAPPED    allergen_load / (wheal_rate * panel_count) - baseline_reactivity  keep the product under the bar but SUBTRACT the
                                                                    baseline reactivity OUTSIDE the division instead of inside the
                                                                    denominator (`a/(b*c)-d` instead of `a/(b*c-d)`),

which are exactly the mistakes a student makes (failing to keep the whole net reactive demand under the bar, or subtracting the
baseline reactivity outside the denominator). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: this rung uses subtraction inside the denominator and in both distractors, so positivity is NOT
automatic and is guarded explicitly. Every observed quantity is `>= 2`, and the tables are chosen so `wheal_rate * panel_count >
baseline_reactivity` with the net reactive demand `b*c-d >= 2` (the denominator never touches zero and stays positive), so
`allergen_load / (b*c-d) > 0`; the tables also keep `allergen_load / (wheal_rate*panel_count) > baseline_reactivity` so the SWAPPED
value is strictly positive, and `(allergen_load/wheal_rate)*panel_count > baseline_reactivity` so the CROSSED value is strictly
positive. Every family member is asserted `> 0` at build time. The five family values are pairwise distinct with a comfortable
margin; and — so all three queried readouts vary across the panel — the seven tables give distinct reactivity indices, distinct net
reactive demands, and distinct allergen loads, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (ALLERGEN_LOAD, WHEAL_RATE, PANEL_COUNT, BASELINE_REACTIVITY) — an allergen load divided by the net reactive demand (a wheal rate
# times a panel count, minus a baseline reactivity subtracted out of the product) for the reactivity index, all plain positive
# numbers >= 2. Subtraction is used inside the denominator and both distractors, so positivity is guarded explicitly (not
# automatic): b*c > d with b*c-d >= 2 keeps the denominator positive and away from zero, a/(b*c) > d keeps SWAPPED positive, and
# (a/b)*c > d keeps CROSSED positive. The five family values are asserted pairwise-distinct below. The seven tables give distinct
# reactivity indices, distinct net reactive demands, and distinct allergen loads so all three queried readouts vary across the panel.
TABLES = [
    (60, 2, 3, 2),
    (80, 2, 4, 3),
    (72, 2, 4, 2),
    (91, 2, 5, 3),
    (90, 3, 4, 2),
    (88, 2, 5, 2),
    (100, 2, 6, 3),
]

# The option family (5 members), all built from the four observed quantities via *, - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "reactivity_index",
        "skin-test reactivity index (the allergen load divided by the net reactive demand)",
        "allergen_load / (wheal_rate * panel_count - baseline_reactivity)",
    ),
    (
        "net_reactive_demand",
        "the net reactive demand (the wheal rate times the panel count minus the baseline reactivity, the divisor the allergen load is divided by)",
        "wheal_rate * panel_count - baseline_reactivity",
    ),
    (
        "allergen_load",
        "the allergen load (the numerator divided by the net reactive demand)",
        "allergen_load",
    ),
    (
        "crossed",
        "the allergen load divided by the wheal rate, times the panel count, minus the baseline reactivity, dropping the denominator parentheses so only the wheal rate divides (a wrong grouping)",
        "allergen_load / wheal_rate * panel_count - baseline_reactivity",
    ),
    (
        "swapped",
        "the allergen load divided by the product of the wheal rate and the panel count, minus the baseline reactivity, subtracting the baseline reactivity outside the division instead of inside the denominator (a wrong pairing)",
        "allergen_load / (wheal_rate * panel_count) - baseline_reactivity",
    ),
]
QUERIED = ["reactivity_index", "net_reactive_demand", "allergen_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(allergen_load, wheal_rate, panel_count, baseline_reactivity):
    # Operation order mirrors the ADJ programs exactly (the wheal rate and panel count multiply first, the baseline reactivity is
    # subtracted from that product, then the allergen load is divided by that whole net reactive demand, so a/(b*c-d) evaluates as
    # (a/((b*c)-d))), so the Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    return {
        "reactivity_index": allergen_load / (wheal_rate * panel_count - baseline_reactivity),
        "net_reactive_demand": wheal_rate * panel_count - baseline_reactivity,
        "allergen_load": allergen_load,
        "crossed": allergen_load / wheal_rate * panel_count - baseline_reactivity,
        "swapped": allergen_load / (wheal_rate * panel_count) - baseline_reactivity,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for allergen_load, wheal_rate, panel_count, baseline_reactivity in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses subtraction inside the denominator and both
        # distractors, so positivity is guarded explicitly: b*c > d with net reactive demand b*c-d >= 2 (denominator positive and
        # away from zero), a/(b*c) > d (SWAPPED positive), and (a/b)*c > d (CROSSED positive).
        assert (
            allergen_load >= 2
            and wheal_rate >= 2
            and panel_count >= 2
            and baseline_reactivity >= 2
        ), (allergen_load, wheal_rate, panel_count, baseline_reactivity)
        assert wheal_rate * panel_count - baseline_reactivity >= 2, (
            allergen_load, wheal_rate, panel_count, baseline_reactivity,
        )
        fv = family_values(allergen_load, wheal_rate, panel_count, baseline_reactivity)
        for key, v in fv.items():
            assert v > 0, (key, allergen_load, wheal_rate, panel_count, baseline_reactivity, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    allergen_load,
                    wheal_rate,
                    panel_count,
                    baseline_reactivity,
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
                allergen_load,
                wheal_rate,
                panel_count,
                baseline_reactivity,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r119as-{idx + 1:02d}",
                "qtype": "reactivity_index",
                "stem": (
                    f"An allergy skin-test report records an allergen load of {num(allergen_load)} divided by a wheal rate of "
                    f"{num(wheal_rate)} times a panel count of {num(panel_count)} minus a baseline reactivity of "
                    f"{num(baseline_reactivity)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe allergen_load({num(allergen_load)})\n"
                    f"observe wheal_rate({num(wheal_rate)})\n"
                    f"observe panel_count({num(panel_count)})\n"
                    f"observe baseline_reactivity({num(baseline_reactivity)})\n"
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
            "ADJ-LADDER rung 119 — skin-test reactivity index from four stated quantities (a NEW panel: allergy skin testing / "
            "reactivity). From an allergen load divided by the net reactive demand (a wheal rate times a panel count, minus a "
            "baseline reactivity), compute the reactivity index "
            "(allergen_load/(wheal_rate*panel_count-baseline_reactivity)), the net reactive demand "
            "(wheal_rate*panel_count-baseline_reactivity), or the allergen load. Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a SINGLE "
            "TERM OVER A PRODUCT-MINUS-TERM DENOMINATOR a/(b*c-d) (multiply the wheal rate and panel count, subtract the baseline "
            "reactivity, divide the allergen load by that whole net reactive demand, so a/(b*c-d) = (a/((b*c)-d)); the ladder's "
            "FIRST denominator that is a product minus a term — the subtraction twin of rung-118's product-plus-term a/(b*c+d). "
            "Every product-minus-term the ladder built was a NUMERATOR over a single divisor (111 (a*b-c)/d); every two-term ratio "
            "with a product had the product on ONE side only (99 (a*b)/(c+d), 100 (a+b)/(c*d), 104 (a-b)/(c*d), 106 a*b/(c-d)) — "
            "rung-119 is the first to put a b*c-d UNDER the bar. The harness matches the scalar to the printed options. The "
            "reactivity index is a rate (allergen load per unit of net reactive demand), framed as an INDEX so the dimensionless "
            "value stays honest. Contamination-safe: every figure is built only from the four observed quantities via *, - and / — "
            "no constant leaks, and neither the net reactive demand, the allergen load, nor any index ever appears as a literal "
            "(each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the slips students "
            "make: dropping the denominator parentheses so only the wheal rate divides (a/b*c-d, a wrong grouping) and subtracting "
            "the baseline reactivity outside the division instead of inside the denominator (a/(b*c)-d, a wrong pairing). The core "
            "confusion tested is that a/(b*c-d) is (a/((b*c)-d)), not a/b*c-d and not a/(b*c)-d. Subtraction is used inside the "
            "denominator and both distractors, so positivity is guarded explicitly: with every observed quantity >= 2 the tables "
            "keep wheal_rate*panel_count > baseline_reactivity and the net reactive demand b*c-d >= 2 (the division is never by "
            "zero and stays positive), and every family member is asserted strictly positive at build time."
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
