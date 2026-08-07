"""Generate rung-145 (pooled rate / a THREE-TERM SUM over a lone denominator — average three readings over a window) items.json.

Rung 145 **OPENS a NEW family beyond the compound-fraction matrix**. Rungs 130-144 filled a four-numerator x four-denominator matrix in
which BOTH the numerator and the denominator were TWO-TERM (a sum/difference/product/quotient over a sum/difference/product/rate). Rung 145
steps up one structural axis — **arity** — to a **THREE-TERM numerator** over a lone denominator: `(a+b+c)/d`. Three readings are summed and
the pooled total is divided by a single window. It is the pooled-average shape, `(a+b+c)/d`, the first rung whose numerator carries THREE
observed terms.

`(a+b+c)/d` sums THREE terms `a+b+c` and divides the whole pooled total by ONE window `d`. The three-term sum binds and stays grouped over
the bar (ALL THREE readings pool before the division), and the lone denominator `d` divides the whole pooled total once. The new arity brings
a new canonical slip that the two-term rungs could not test: **dropping a term** — pooling only TWO of the three readings and dividing that,
`(a+b)/d` (the "forgot the third reading" slip). The other canonical slip is the universal one, **inverting** the ratio, dividing the window
by the pooled total instead of the pooled total by the window, `d/(a+b+c)` (the reciprocal).

The setup: three shift readings `shift_one`, `shift_two`, `shift_three` are pooled (a pooled total `shift_one + shift_two + shift_three`) and
spread across a `window` (a pooled rate `(shift_one + shift_two + shift_three) / window`). A running subtotal of the first two shifts
(`shift_one + shift_two`) is also read off. The figures are:

  POOLED RATE     (shift_one + shift_two + shift_three) / window  [ THREE-TERM sum OVER a lone window: pooled total / window ]
  POOLED TOTAL    shift_one + shift_two + shift_three            [ the full three-term numerator (divided by the window) ]
  PARTIAL TOTAL   shift_one + shift_two                          [ a running subtotal of the first two shifts (a real intermediate) ]

The **pooled rate** is the headline; the **pooled total** (all three readings) and the **partial total** (the first two) ride alongside as
component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline rungs 47-144
shipped. Critically, the partial total `(a+b)` is a *legitimate* running subtotal, whereas the distractor `(a+b)/d` is the *slip* of dividing
that subtotal as if it were the whole pool — the panel puts the honest subtotal and the term-dropping slip side by side so the difference is
exactly "did you pool all three readings before dividing?".

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the two additions to pool the three readings, then the division of the pooled total by the window to form the compound figure
(so (a+b+c)/d evaluates as ((a+b+c)/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine
change, exactly as rungs 8/16/.../143/144. This rung exercises the engine across a **three-term sum divided by a lone divisor** — the fact
that `(a+b+c)/d` pools ALL THREE terms and is NOT `(a+b)/d` and NOT `d/(a+b+c)` made computable. The golds are exact rationals rendered as
f64s; the engine's IEEE-double division matches Python's the same way rungs 100/.../143/144 relied on (well within the harness's 1e-9
tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `/` — **no structural
constants** — so no numeric literal appears in any program (in particular the term COUNT three is never written as a literal; the pooling is
spelled out as `shift_one + shift_two + shift_three`), and neither the pooled total, the partial total, nor the pooled rate is ever a literal
(each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`shift_one`, `shift_two`,
`shift_three`, `window`) so no numeral hides inside a variable name. (The `_one/_two/_three` suffixes are English words, not digits.)

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  DROPPED    (shift_one + shift_two) / window  pool only TWO of the three readings and divide that, forgetting the third reading (the
                                               three-term-specific slip the two-term rungs could not test), and
  INVERTED   window / (shift_one + shift_two + shift_three)  divide the window BY the pooled total, the ratio upside down (the reciprocal of
                                                             the pooled rate, the wrong direction),

which are exactly the mistakes a student makes pooling three readings over a window (dropping a reading, or inverting the ratio). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: this rung uses only `+` and `/` over positive quantities, so **every figure is automatically positive** (no
subtraction, no product) — no positivity guards are needed. Every observed quantity is `>= 2`. Every family member is asserted `> 0` at
build time. The seven tables give distinct pooled rates, distinct pooled totals, and distinct partial totals so all three queried readouts
vary across the panel; the five family values are pairwise distinct with a comfortable margin (in particular the partial total `a+b` and the
dropped-rate slip `(a+b)/d` are kept apart by the window).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (SHIFT_ONE, SHIFT_TWO, SHIFT_THREE, WINDOW) — three readings pooled (shift_one + shift_two + shift_three) and divided by a lone window,
# giving the pooled rate as a three-term sum over a lone denominator (a+b+c)/d. This rung uses only + and / over positive quantities, so every
# figure is automatically positive; no positivity guards are needed. The seven tables give distinct pooled totals (a+b+c), distinct partial
# totals (a+b), and distinct pooled rates ((a+b+c)/d); the five family values are asserted pairwise-distinct below.
TABLES = [
    (2, 4, 6, 4),      # partial = 6,  total = 12, rate = 3.0
    (3, 5, 12, 5),     # partial = 8,  total = 20, rate = 4.0
    (4, 6, 20, 6),     # partial = 10, total = 30, rate = 5.0
    (3, 9, 12, 4),     # partial = 12, total = 24, rate = 6.0
    (5, 9, 21, 5),     # partial = 14, total = 35, rate = 7.0
    (6, 10, 32, 6),    # partial = 16, total = 48, rate = 8.0
    (7, 11, 18, 4),    # partial = 18, total = 36, rate = 9.0
]

# The option family (5 members), all built from the four observed quantities via + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "pooled_rate",
        "pooled rate (the pooled total of all three shifts divided by the window)",
        "(shift_one + shift_two + shift_three) / window",
    ),
    (
        "pooled_total",
        "the pooled total (all three shifts added, the numerator that is divided by the window)",
        "shift_one + shift_two + shift_three",
    ),
    (
        "partial_total",
        "the partial total (the first two shifts added, a running subtotal before the third shift)",
        "shift_one + shift_two",
    ),
    (
        "dropped",
        "the first two shifts divided by the window, pooling only two of the three readings and forgetting the third shift (a wrong operation)",
        "(shift_one + shift_two) / window",
    ),
    (
        "inverted",
        "the window divided by the pooled total, the ratio upside down instead of the pooled total over the window (a wrong operation)",
        "window / (shift_one + shift_two + shift_three)",
    ),
]
QUERIED = ["pooled_rate", "pooled_total", "partial_total"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(shift_one, shift_two, shift_three, window):
    # Operation order mirrors the ADJ programs exactly (the two additions pool the three readings, then the pooled total is divided by the
    # window to form the compound figure, so (a+b+c)/d evaluates as ((a+b+c)/d)), so the Python option value and the engine result are the
    # same IEEE-double (well within the 1e-9 tolerance).
    partial = shift_one + shift_two
    total = shift_one + shift_two + shift_three
    return {
        "pooled_rate": total / window,
        "pooled_total": total,
        "partial_total": partial,
        "dropped": partial / window,
        "inverted": window / total,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for shift_one, shift_two, shift_three, window in TABLES:
        # Every observed quantity is a plain positive number >= 2. This rung uses only + and / over positive quantities, so positivity is
        # automatic — no positivity guards are needed.
        assert (
            shift_one >= 2
            and shift_two >= 2
            and shift_three >= 2
            and window >= 2
        ), (shift_one, shift_two, shift_three, window)
        fv = family_values(shift_one, shift_two, shift_three, window)
        for key, v in fv.items():
            assert v > 0, (key, shift_one, shift_two, shift_three, window, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    shift_one,
                    shift_two,
                    shift_three,
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
                shift_one,
                shift_two,
                shift_three,
                window,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r145psa-{idx + 1:02d}",
                "qtype": "pooled_rate",
                "stem": (
                    f"A staffing study records three shift readings of {num(shift_one)}, {num(shift_two)}, and "
                    f"{num(shift_three)} pooled over a window of {num(window)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe shift_one({num(shift_one)})\n"
                    f"observe shift_two({num(shift_two)})\n"
                    f"observe shift_three({num(shift_three)})\n"
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
            "ADJ-LADDER rung 145 — pooled rate from four stated quantities (OPENS a NEW family beyond the compound-fraction matrix). Rungs "
            "130-144 filled a four-numerator x four-denominator matrix in which both the numerator and the denominator were TWO-TERM. "
            "rung-145 steps up the ARITY axis to a THREE-TERM numerator over a lone denominator: (a+b+c)/d — three shift readings pooled "
            "and divided by one window. From a pooled total (shift_one + shift_two + shift_three) divided by a window, compute the pooled "
            "rate ((shift_one+shift_two+shift_three)/window), the pooled total (shift_one+shift_two+shift_three), or the partial total "
            "(shift_one+shift_two, a running subtotal of the first two shifts). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a THREE-TERM SUM OVER A LONE DIVISOR (a+b+c)/d (pool "
            "all three readings FIRST, then divide the pooled total by the window). The new three-term arity brings a slip the two-term "
            "rungs could not test — DROPPING a term, pooling only two of the three readings and dividing that ((a+b)/d, the 'forgot the "
            "third reading' slip) — alongside the universal INVERTING slip (d/(a+b+c), the reciprocal). The panel puts the honest running "
            "subtotal (a+b) beside the term-dropping slip ((a+b)/d) so the difference is exactly 'did you pool all three readings before "
            "dividing?'. The harness matches the scalar to the printed options. Contamination-safe: every figure is built only from the "
            "four observed quantities via + and / — no constant leaks (in particular the term COUNT three is never a literal; the pooling "
            "is spelled out as shift_one + shift_two + shift_three), and neither the pooled total, the partial total, nor the pooled rate "
            "ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. This rung uses only + and / over positive quantities, so every figure is automatically positive — no "
            "positivity guards are needed — and the five family values are kept pairwise distinct with all three queried readouts varying "
            "across the panel, all asserted strictly positive at build time."
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
