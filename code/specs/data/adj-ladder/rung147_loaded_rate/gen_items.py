"""Generate rung-147 (loaded rate / a BASE-PLUS-PRODUCT three-term numerator over a lone denominator — precedence: multiply before add) items.json.

Rung 147 continues the **THREE-TERM family** along a NEW axis: **operator precedence**. rung-145 (a+b+c)/d and rung-146 (a+b−b)/d kept
every numerator term at the SAME binding level (all `+`/`−`, left to right). rung-147 makes the three-term numerator **mix `+` with `*`** —
`(a+b*c)/d` — so the `*` MUST bind tighter than the `+`: the product `b*c` forms FIRST and only then is `a` added. It is the base-plus-product
shape, `(a+b*c)/d`, the first rung whose numerator forces a multiply-before-add precedence decision.

`(a+b*c)/d` is a base `a` PLUS a product `b*c`, the whole loaded total divided by ONE window `d`. Because `*` binds tighter than `+`, the
numerator is `a + (b*c)`, NOT `(a+b)*c`. This precedence is the whole point of the rung, and it brings a new canonical slip the same-level
three-term rungs could not test: **the precedence error** — ADDING before multiplying, `(a+b)*c/d`, which distributes the multiply across the
base and inflates the result. The other canonical slip is the universal one, **inverting** the ratio, dividing the window by the loaded total
instead of the loaded total by the window, `d/(a+b*c)` (the reciprocal).

The setup: a fixed `base_load` plus a variable part of `unit_load` per unit times a `unit_count` of units (a loaded total `base_load +
unit_load * unit_count`), spread across a `window` (a loaded rate `(base_load + unit_load * unit_count) / window`). The product part on its
own (`unit_load * unit_count`) is also read off. The figures are:

  LOADED RATE   (base_load + unit_load * unit_count) / window  [ BASE-PLUS-PRODUCT numerator OVER a lone window: loaded total / window ]
  LOADED TOTAL  base_load + unit_load * unit_count            [ the base-plus-product numerator (divided by the window) ]
  PRODUCT PART  unit_load * unit_count                        [ the product term alone, before the base is added (a real intermediate) ]

The **loaded rate** is the headline; the **loaded total** (base plus product) and the **product part** (the product alone) ride alongside as
component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline rungs 47-146
shipped. Critically, the product part `(b*c)` is the *legitimate* multiply-first intermediate, whereas the distractor `(a+b)*c/d` is the
*slip* of adding the base into the multiply — the panel puts the honest product term and the precedence-error slip side by side so the
difference is exactly "did you multiply BEFORE adding the base?".

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication to form the product part, the addition of the base, then the division by the window to form the compound
figure (so (a+b*c)/d evaluates as ((a+(b*c))/d), honoring standard precedence) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../145/146 (and rung-34 already relied on the engine's
`*`-before-`+` precedence for sums of products). This rung exercises the engine across a **base-plus-product numerator divided by a lone
divisor** — the fact that `(a+b*c)/d` multiplies FIRST and is NOT `(a+b)*c/d` and NOT `d/(a+b*c)` made computable. The golds are exact
rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs 100/.../145/146 relied on (well within the
harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `*`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the loaded total, the product part, nor the loaded rate is ever a
literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`base_load`, `unit_load`,
`unit_count`, `window`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  ADDED_FIRST  (base_load + unit_load) * unit_count / window  ADD the base into the multiply instead of multiplying first, the
                                                             operator-precedence error (the slip the same-level three-term rungs could not
                                                             test), and
  INVERTED     window / (base_load + unit_load * unit_count)  divide the window BY the loaded total, the ratio upside down (the reciprocal of
                                                             the loaded rate, the wrong direction),

which are exactly the mistakes a student makes on a base-plus-product over a window (adding before multiplying, or inverting the ratio). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `+`, `*`, and `/` over positive quantities, so **every figure is automatically positive** (no
subtraction anywhere) — no positivity guards are needed. Every observed quantity is `>= 2`. Every family member is asserted `> 0` at build
time. The seven tables give distinct loaded rates, distinct loaded totals, and distinct product parts so all three queried readouts vary
across the panel; the five family values are pairwise distinct with a comfortable margin (in particular the product part `b*c` and the
precedence-error slip `(a+b)*c/d` are kept apart).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BASE_LOAD, UNIT_LOAD, UNIT_COUNT, WINDOW) — a base plus a product (base_load + unit_load * unit_count) divided by a lone window, giving the
# loaded rate as a base-plus-product numerator over a lone denominator (a+b*c)/d. Standard precedence applies: the * binds tighter than the +,
# so the numerator is a + (b*c). This rung uses only +, *, and / over positive quantities, so every figure is automatically positive; no
# positivity guards are needed. The seven tables give distinct loaded totals (a+b*c), distinct product parts (b*c), and distinct loaded rates
# ((a+b*c)/d); the five family values are asserted pairwise-distinct below.
TABLES = [
    (2, 3, 4, 2),      # product = 12, loaded = 14, rate = 7.0
    (6, 2, 5, 2),      # product = 10, loaded = 16, rate = 8.0
    (2, 2, 4, 4),      # product = 8,  loaded = 10, rate = 2.5
    (3, 3, 5, 3),      # product = 15, loaded = 18, rate = 6.0
    (6, 2, 7, 2),      # product = 14, loaded = 20, rate = 10.0
    (3, 3, 3, 3),      # product = 9,  loaded = 12, rate = 4.0
    (4, 3, 6, 2),      # product = 18, loaded = 22, rate = 11.0
]

# The option family (5 members), all built from the four observed quantities via +, *, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "loaded_rate",
        "loaded rate (the loaded total divided by the window)",
        "(base_load + unit_load * unit_count) / window",
    ),
    (
        "loaded_total",
        "the loaded total (the base plus the product of unit load and unit count, the numerator that is divided by the window)",
        "base_load + unit_load * unit_count",
    ),
    (
        "product_part",
        "the product part (the unit load times the unit count, the product term before the base is added)",
        "unit_load * unit_count",
    ),
    (
        "added_first",
        "the base and unit load added first, then multiplied by the unit count and divided by the window, adding the base into the multiply instead of multiplying first (a wrong operation)",
        "(base_load + unit_load) * unit_count / window",
    ),
    (
        "inverted",
        "the window divided by the loaded total, the ratio upside down instead of the loaded total over the window (a wrong operation)",
        "window / (base_load + unit_load * unit_count)",
    ),
]
QUERIED = ["loaded_rate", "loaded_total", "product_part"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(base_load, unit_load, unit_count, window):
    # Operation order mirrors the ADJ programs exactly (the multiplication forms the product part, the base is added, then the loaded total
    # is divided by the window to form the compound figure, so (a+b*c)/d evaluates as ((a+(b*c))/d) under standard precedence), so the Python
    # option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    product = unit_load * unit_count
    loaded = base_load + product
    return {
        "loaded_rate": loaded / window,
        "loaded_total": loaded,
        "product_part": product,
        "added_first": (base_load + unit_load) * unit_count / window,
        "inverted": window / loaded,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for base_load, unit_load, unit_count, window in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only +, *, and / over positive quantities, so positivity is
        # automatic — no positivity guards are needed.
        assert (
            base_load >= 2
            and unit_load >= 2
            and unit_count >= 2
            and window >= 2
        ), (base_load, unit_load, unit_count, window)
        fv = family_values(base_load, unit_load, unit_count, window)
        for key, v in fv.items():
            assert v > 0, (key, base_load, unit_load, unit_count, window, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    base_load,
                    unit_load,
                    unit_count,
                    window,
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
                base_load,
                unit_load,
                unit_count,
                window,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r147lra-{idx + 1:02d}",
                "qtype": "loaded_rate",
                "stem": (
                    f"A loading study records a base load of {num(base_load)} plus a unit load of "
                    f"{num(unit_load)} across {num(unit_count)} units, spread over a window of {num(window)}. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe base_load({num(base_load)})\n"
                    f"observe unit_load({num(unit_load)})\n"
                    f"observe unit_count({num(unit_count)})\n"
                    f"observe window({num(window)})\n"
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
            "ADJ-LADDER rung 147 — loaded rate from four stated quantities (CONTINUING the THREE-TERM family along the OPERATOR-PRECEDENCE "
            "axis). rungs 145/146 kept every numerator term at the same binding level (all +/−); rung-147 mixes + with * in the numerator "
            "(a+b*c)/d, so the * MUST bind tighter than the +: the product b*c forms FIRST and only then is the base a added. From a loaded "
            "total (base_load + unit_load * unit_count) divided by a window, compute the loaded rate "
            "((base_load+unit_load*unit_count)/window), the loaded total (base_load+unit_load*unit_count), or the product part "
            "(unit_load*unit_count, the product term before the base is added). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a BASE-PLUS-PRODUCT NUMERATOR OVER A LONE DIVISOR "
            "(a+b*c)/d (multiply the unit load by the unit count FIRST, add the base, then divide by the window, honoring standard "
            "*-before-+ precedence, as rung-34 already relied on). The mixed precedence brings a slip the same-level three-term rungs could "
            "not test — the PRECEDENCE ERROR, adding the base into the multiply instead of multiplying first ((a+b)*c/d, which distributes "
            "the multiply across the base and inflates the result) — alongside the universal INVERTING slip (d/(a+b*c), the reciprocal). "
            "The panel puts the honest product part (b*c) beside the precedence-error slip ((a+b)*c/d) so the difference is exactly 'did you "
            "multiply BEFORE adding the base?'. The harness matches the scalar to the printed options. Contamination-safe: every figure is "
            "built only from the four observed quantities via +, *, and / — no constant leaks, and neither the loaded total, the product "
            "part, nor the loaded rate ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. This rung uses only +, *, and / over positive quantities, so every "
            "figure is automatically positive — no positivity guards are needed — and the five family values are kept pairwise distinct with "
            "all three queried readouts varying across the panel, all asserted strictly positive at build time."
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
