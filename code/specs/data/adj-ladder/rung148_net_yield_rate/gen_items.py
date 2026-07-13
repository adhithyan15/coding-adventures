"""Generate rung-148 (net yield rate / a PRODUCT-MINUS-TERM three-term numerator over a lone denominator — precedence + subtraction) items.json.

Rung 148 completes the FOUR 4-quantity three-term numerators. rung-145 (a+b+c)/d was all-sum; rung-146 (a+b−c)/d added a subtraction; rung-147
(a+b*c)/d added a multiply (operator precedence). rung-148 combines the last two axes at once — a **PRODUCT-MINUS-TERM numerator**,
`(a*b−c)/d`: a product `a*b` forms FIRST (multiply binds tighter than subtract) and only THEN is `c` subtracted, the net divided by one window.
It is the gross-minus-loss shape, `(a*b−c)/d`, the first rung whose three-term numerator carries BOTH a `*` and a `−`.

`(a*b−c)/d` is a product `a*b` MINUS a term `c`, the whole net divided by ONE window `d`. Because `*` binds tighter than `−`, the numerator
is `(a*b)−c`, NOT `a*(b−c)`. This rung inherits BOTH slips from its parents: the **sign error** — ADDING the loss instead of subtracting it,
`(a*b+c)/d` (rung-146's mistake, now over a product) — and, implicitly guarded against by the panel's honest gross readout, the
precedence question of multiplying before combining (rung-147's lesson). The other canonical slip is the universal one, **inverting** the
ratio, dividing the window by the net instead of the net by the window, `d/(a*b−c)` (the reciprocal).

The setup: a `batch_size` per batch times a `batch_count` of batches (a gross yield `batch_size * batch_count`) minus a `spoilage` loss (a
net yield `batch_size * batch_count − spoilage`), spread across a `window` (a net yield rate `(batch_size * batch_count − spoilage) /
window`). The gross product on its own (`batch_size * batch_count`) is also read off. The figures are:

  NET YIELD RATE  (batch_size * batch_count − spoilage) / window  [ PRODUCT-MINUS-TERM numerator OVER a lone window: net yield / window ]
  NET YIELD       batch_size * batch_count − spoilage            [ the product-minus-term numerator (divided by the window) ]
  GROSS YIELD     batch_size * batch_count                       [ the product term alone, before the spoilage is subtracted (a real intermediate) ]

The **net yield rate** is the headline; the **net yield** (gross minus loss) and the **gross yield** (the product alone) ride alongside as
component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline" discipline rungs 47-147
shipped. Critically, the gross yield `(a*b)` is the *legitimate* multiply-first intermediate, whereas the distractor `(a*b+c)/d` is the *slip*
of adding the loss instead of subtracting it — the panel puts the honest gross product and the sign-error slip side by side so the difference
is exactly "did you SUBTRACT the spoilage, or add it?".

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the multiplication to form the gross yield, the subtraction of the spoilage, then the division by the window to form the
compound figure (so (a*b−c)/d evaluates as (((a*b)−c)/d), honoring standard precedence) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../146/147. This rung exercises the engine across a
**product-minus-term numerator divided by a lone divisor** — the fact that `(a*b−c)/d` multiplies FIRST, subtracts, and is NOT `(a*b+c)/d`
and NOT `d/(a*b−c)` made computable. The golds are exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the
same way rungs 100/.../146/147 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `−`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the net yield, the gross yield, nor the net yield rate is ever a
literal (each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`batch_size`,
`batch_count`, `spoilage`, `window`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  ADDED      (batch_size * batch_count + spoilage) / window  ADD the spoilage instead of subtracting it, treating the loss as a gain (the
                                                             sign error, now over a product), and
  INVERTED   window / (batch_size * batch_count − spoilage)  divide the window BY the net yield, the ratio upside down (the reciprocal of the
                                                             net yield rate, the wrong direction),

which are exactly the mistakes a student makes on a gross-minus-loss over a window (adding the loss instead of subtracting, or inverting the
ratio). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: the numerator subtracts the spoilage, so — like rung-146 — the net yield needs a **positivity guard**: every
table is built so `batch_size * batch_count − spoilage >= 2` (asserted at build time), keeping the net yield, the net yield rate, and the
inverted slip all strictly positive (the window `d` and the gross yield `a*b` are products of positives, so they are automatically positive;
only the net numerator can go non-positive). Every observed quantity is `>= 2`. Every family member is asserted `> 0` at build time. The
seven tables give distinct net yield rates, distinct net yields, and distinct gross yields so all three queried readouts vary across the
panel; the five family values are pairwise distinct with a comfortable margin (in particular the gross yield `a*b` and the net yield `a*b−c`
are kept apart by a spoilage of at least two).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BATCH_SIZE, BATCH_COUNT, SPOILAGE, WINDOW) — a gross product (batch_size * batch_count) minus a spoilage (batch_size * batch_count -
# spoilage) and divided by a lone window, giving the net yield rate as a product-minus-term numerator over a lone denominator (a*b-c)/d.
# Standard precedence applies: the * binds tighter than the -, so the numerator is (a*b) - c. The numerator subtracts the spoilage, so the net
# yield needs a positivity guard: every row satisfies batch_size * batch_count - spoilage >= 2 (asserted below). The window and the gross
# yield are products of positives, so they are automatically positive. The seven tables give distinct net yields (a*b-c), distinct gross
# yields (a*b), and distinct net yield rates ((a*b-c)/d); the five family values are asserted pairwise-distinct below.
TABLES = [
    (3, 4, 2, 2),      # gross = 12, net = 10, rate = 5.0
    (2, 5, 2, 2),      # gross = 10, net = 8,  rate = 4.0
    (2, 4, 2, 4),      # gross = 8,  net = 6,  rate = 1.5
    (3, 5, 3, 2),      # gross = 15, net = 12, rate = 6.0
    (3, 6, 4, 2),      # gross = 18, net = 14, rate = 7.0
    (3, 3, 2, 2),      # gross = 9,  net = 7,  rate = 3.5
    (4, 5, 4, 2),      # gross = 20, net = 16, rate = 8.0
]

# The option family (5 members), all built from the four observed quantities via *, -, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "net_yield_rate",
        "net yield rate (the net yield divided by the window)",
        "(batch_size * batch_count - spoilage) / window",
    ),
    (
        "net_yield",
        "the net yield (the gross yield minus the spoilage, the numerator that is divided by the window)",
        "batch_size * batch_count - spoilage",
    ),
    (
        "gross_yield",
        "the gross yield (the batch size times the batch count, the product before the spoilage is subtracted)",
        "batch_size * batch_count",
    ),
    (
        "added",
        "the gross yield plus the spoilage, divided by the window, adding the spoilage instead of subtracting it (a wrong operation)",
        "(batch_size * batch_count + spoilage) / window",
    ),
    (
        "inverted",
        "the window divided by the net yield, the ratio upside down instead of the net yield over the window (a wrong operation)",
        "window / (batch_size * batch_count - spoilage)",
    ),
]
QUERIED = ["net_yield_rate", "net_yield", "gross_yield"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(batch_size, batch_count, spoilage, window):
    # Operation order mirrors the ADJ programs exactly (the multiplication forms the gross yield, the spoilage is subtracted, then the net
    # yield is divided by the window to form the compound figure, so (a*b-c)/d evaluates as (((a*b)-c)/d) under standard precedence), so the
    # Python option value and the engine result are the same IEEE-double (well within the 1e-9 tolerance).
    gross = batch_size * batch_count
    net = gross - spoilage
    return {
        "net_yield_rate": net / window,
        "net_yield": net,
        "gross_yield": gross,
        "added": (gross + spoilage) / window,
        "inverted": window / net,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for batch_size, batch_count, spoilage, window in TABLES:
        # Every observed quantity is a plain positive number >= 2, AND the product-minus-term numerator is guarded positive: the net yield
        # batch_size * batch_count - spoilage must be >= 2. The window and the gross yield are products of positives, so they are
        # automatically positive; only the net numerator can go non-positive, so it is the only guard needed.
        assert (
            batch_size >= 2
            and batch_count >= 2
            and spoilage >= 2
            and window >= 2
        ), (batch_size, batch_count, spoilage, window)
        assert batch_size * batch_count - spoilage >= 2, (batch_size, batch_count, spoilage)
        fv = family_values(batch_size, batch_count, spoilage, window)
        for key, v in fv.items():
            assert v > 0, (key, batch_size, batch_count, spoilage, window, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    batch_size,
                    batch_count,
                    spoilage,
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
                batch_size,
                batch_count,
                spoilage,
                window,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r148nya-{idx + 1:02d}",
                "qtype": "net_yield_rate",
                "stem": (
                    f"A production study records a batch size of {num(batch_size)} across {num(batch_count)} "
                    f"batches with a spoilage of {num(spoilage)}, spread over a window of {num(window)}. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe batch_size({num(batch_size)})\n"
                    f"observe batch_count({num(batch_count)})\n"
                    f"observe spoilage({num(spoilage)})\n"
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
            "ADJ-LADDER rung 148 — net yield rate from four stated quantities (COMPLETING the four 4-quantity three-term numerators). "
            "rung-145 (a+b+c)/d was all-sum; rung-146 (a+b−c)/d added a subtraction; rung-147 (a+b*c)/d added a multiply (precedence); "
            "rung-148 combines the last two — a PRODUCT-MINUS-TERM numerator (a*b−c)/d: a product a*b forms FIRST (multiply binds tighter "
            "than subtract) and only then is c subtracted, the net divided by one window. From a net yield (batch_size * batch_count − "
            "spoilage) divided by a window, compute the net yield rate ((batch_size*batch_count−spoilage)/window), the net yield "
            "(batch_size*batch_count−spoilage), or the gross yield (batch_size*batch_count, the product before the spoilage is subtracted). "
            "Each item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a PRODUCT-MINUS-TERM NUMERATOR OVER A LONE DIVISOR (a*b−c)/d (multiply the batch size by the batch count FIRST, "
            "subtract the spoilage, then divide by the window, honoring standard *-before-− precedence). It inherits the sign-error slip "
            "from rung-146 (now over a product) — ADDING the spoilage instead of subtracting it ((a*b+c)/d) — alongside the universal "
            "INVERTING slip (d/(a*b−c), the reciprocal). The panel puts the honest gross product (a*b) beside the sign-error slip "
            "((a*b+c)/d) so the difference is exactly 'did you SUBTRACT the spoilage, or add it?'. The harness matches the scalar to the "
            "printed options. Contamination-safe: every figure is built only from the four observed quantities via *, −, and / — no constant "
            "leaks, and neither the net yield, the gross yield, nor the net yield rate ever appears as a literal (each is computed) — and "
            "the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. Because the numerator "
            "subtracts the spoilage, the net yield carries a positivity guard (batch_size * batch_count − spoilage >= 2) so every figure "
            "stays strictly positive; the window and the gross yield are products of positives and so are automatically positive. The five "
            "family values are kept pairwise distinct with all three queried readouts varying across the panel, all asserted strictly "
            "positive at build time."
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
