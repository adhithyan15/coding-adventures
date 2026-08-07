"""Generate rung-146 (net rate / a MIXED-OP THREE-TERM numerator over a lone denominator — net two credits and a debit over a window) items.json.

Rung 146 continues the **THREE-TERM family** opened by rung-145, taking the smallest possible step: rung-145 pooled three terms with the
SAME sign, `(a+b+c)/d`; rung-146 flips ONE numerator sign to a **MIXED-OP three-term numerator**, `(a+b−c)/d` — two credits are added and a
debit is subtracted to form a net, and the net is divided by one window. It is the net-of-three shape, `(a+b−c)/d`, the first rung whose
three-term numerator mixes `+` and `−`.

`(a+b−c)/d` combines THREE terms with mixed signs `a+b−c` and divides the whole net by ONE window `d`. The mixed-op three-term numerator
binds and stays grouped over the bar (both credits are added AND the debit is subtracted before the division), and the lone denominator `d`
divides the whole net once. The mixed sign brings a new canonical slip that rung-145's all-`+` numerator could not test: **the sign error** —
ADDING the debit instead of subtracting it, `(a+b+c)/d` (treating the debit as if it were a third credit). The other canonical slip is the
universal one, **inverting** the ratio, dividing the window by the net instead of the net by the window, `d/(a+b−c)` (the reciprocal).

The setup: two credit readings `credit_one`, `credit_two` are added and a `debit_one` is subtracted (a net total `credit_one + credit_two −
debit_one`), and that net is spread across a `window` (a net rate `(credit_one + credit_two − debit_one) / window`). A gross subtotal of the
two credits before the debit (`credit_one + credit_two`) is also read off. The figures are:

  NET RATE      (credit_one + credit_two − debit_one) / window  [ MIXED-OP THREE-TERM numerator OVER a lone window: net total / window ]
  NET TOTAL     credit_one + credit_two − debit_one            [ the mixed-op three-term numerator (divided by the window) ]
  GROSS TOTAL   credit_one + credit_two                        [ the two credits added, BEFORE the debit is subtracted (a real intermediate) ]

The **net rate** is the headline; the **net total** (credits minus the debit) and the **gross total** (the two credits before the debit)
ride alongside as component readouts, so the panel teaches the whole calculation — the same "show the components beside the headline"
discipline rungs 47-145 shipped. Critically, the gross total `(a+b)` is a *legitimate* pre-debit subtotal, whereas the distractor
`(a+b+c)/d` is the *slip* of ADDING the debit as if it were a credit — the panel puts the honest gross subtotal and the sign-error slip side
by side so the difference is exactly "did you SUBTRACT the debit, or add it?".

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the addition and the subtraction to net the three readings, then the division of the net by the window to form the compound
figure (so (a+b−c)/d evaluates as ((a+b−c)/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../144/145. This rung exercises the engine across a **mixed-op three-term numerator divided by
a lone divisor** — the fact that `(a+b−c)/d` SUBTRACTS the debit and is NOT `(a+b+c)/d` and NOT `d/(a+b−c)` made computable. The golds are
exact rationals rendered as f64s; the engine's IEEE-double division matches Python's the same way rungs 100/.../144/145 relied on (well
within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `−`, and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the net total, the gross total, nor the net rate is ever a literal
(each is computed from the observed quantities). The observed quantities carry **digit-free identifiers** (`credit_one`, `credit_two`,
`debit_one`, `window`) so no numeral hides inside a variable name. (The `_one/_two` suffixes are English words, not digits.)

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  ADDED      (credit_one + credit_two + debit_one) / window  ADD the debit instead of subtracting it, treating the debit as a third credit
                                                             (the sign error the all-`+` rung-145 could not test), and
  INVERTED   window / (credit_one + credit_two − debit_one)  divide the window BY the net total, the ratio upside down (the reciprocal of the
                                                             net rate, the wrong direction),

which are exactly the mistakes a student makes netting two credits and a debit over a window (adding the debit instead of subtracting, or
inverting the ratio). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness and positivity: the numerator subtracts the debit, so — unlike the all-`+ /` rung-145 — the net total needs a **positivity
guard**: every table is built so `credit_one + credit_two − debit_one >= 2` (asserted at build time), keeping the net total, the net rate,
and the inverted slip all strictly positive (the window `d` and the gross total `a+b` are sums of positives, so they are automatically
positive; only the net numerator can go non-positive). Every observed quantity is `>= 2`. Every family member is asserted `> 0` at build
time. The seven tables give distinct net rates, distinct net totals, and distinct gross totals so all three queried readouts vary across the
panel; the five family values are pairwise distinct with a comfortable margin (in particular the gross total `a+b` and the net total `a+b−c`
are kept apart by a debit of at least two).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CREDIT_ONE, CREDIT_TWO, DEBIT_ONE, WINDOW) — two credits added and a debit subtracted (credit_one + credit_two - debit_one) and divided by
# a lone window, giving the net rate as a mixed-op three-term numerator over a lone denominator (a+b-c)/d. The numerator subtracts the debit,
# so the net total needs a positivity guard: every row satisfies credit_one + credit_two - debit_one >= 2 (asserted below). The window and the
# gross total are sums of positives, so they are automatically positive. The seven tables give distinct net totals (a+b-c), distinct gross
# totals (a+b), and distinct net rates ((a+b-c)/d); the five family values are asserted pairwise-distinct below.
TABLES = [
    (4, 5, 3, 3),      # gross = 9,  net = 6,  rate = 2.0
    (5, 6, 3, 2),      # gross = 11, net = 8,  rate = 4.0
    (6, 7, 4, 3),      # gross = 13, net = 9,  rate = 3.0
    (6, 8, 4, 2),      # gross = 14, net = 10, rate = 5.0
    (7, 9, 4, 2),      # gross = 16, net = 12, rate = 6.0
    (8, 10, 4, 2),     # gross = 18, net = 14, rate = 7.0
    (9, 11, 4, 2),     # gross = 20, net = 16, rate = 8.0
]

# The option family (5 members), all built from the four observed quantities via +, -, and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "net_rate",
        "net rate (the net total divided by the window)",
        "(credit_one + credit_two - debit_one) / window",
    ),
    (
        "net_total",
        "the net total (the two credits added and the debit subtracted, the numerator that is divided by the window)",
        "credit_one + credit_two - debit_one",
    ),
    (
        "gross_total",
        "the gross total (the two credits added, before the debit is subtracted)",
        "credit_one + credit_two",
    ),
    (
        "added",
        "the two credits and the debit all added, divided by the window, adding the debit instead of subtracting it (a wrong operation)",
        "(credit_one + credit_two + debit_one) / window",
    ),
    (
        "inverted",
        "the window divided by the net total, the ratio upside down instead of the net total over the window (a wrong operation)",
        "window / (credit_one + credit_two - debit_one)",
    ),
]
QUERIED = ["net_rate", "net_total", "gross_total"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(credit_one, credit_two, debit_one, window):
    # Operation order mirrors the ADJ programs exactly (the addition and the subtraction net the three readings, then the net total is
    # divided by the window to form the compound figure, so (a+b-c)/d evaluates as ((a+b-c)/d)), so the Python option value and the engine
    # result are the same IEEE-double (well within the 1e-9 tolerance).
    gross = credit_one + credit_two
    net = credit_one + credit_two - debit_one
    return {
        "net_rate": net / window,
        "net_total": net,
        "gross_total": gross,
        "added": (credit_one + credit_two + debit_one) / window,
        "inverted": window / net,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for credit_one, credit_two, debit_one, window in TABLES:
        # Every observed quantity is a plain positive number >= 2, AND the mixed-op numerator is guarded positive: the net total
        # credit_one + credit_two - debit_one must be >= 2. The window and the gross total are sums of positives, so they are automatically
        # positive; only the net numerator can go non-positive, so it is the only guard needed.
        assert (
            credit_one >= 2
            and credit_two >= 2
            and debit_one >= 2
            and window >= 2
        ), (credit_one, credit_two, debit_one, window)
        assert credit_one + credit_two - debit_one >= 2, (credit_one, credit_two, debit_one)
        fv = family_values(credit_one, credit_two, debit_one, window)
        for key, v in fv.items():
            assert v > 0, (key, credit_one, credit_two, debit_one, window, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    credit_one,
                    credit_two,
                    debit_one,
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
                credit_one,
                credit_two,
                debit_one,
                window,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r146nra-{idx + 1:02d}",
                "qtype": "net_rate",
                "stem": (
                    f"A ledger study records two credit readings of {num(credit_one)} and {num(credit_two)} "
                    f"with a debit of {num(debit_one)}, netted over a window of {num(window)}. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe credit_one({num(credit_one)})\n"
                    f"observe credit_two({num(credit_two)})\n"
                    f"observe debit_one({num(debit_one)})\n"
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
            "ADJ-LADDER rung 146 — net rate from four stated quantities (CONTINUING the THREE-TERM family). rung-145 pooled three terms "
            "with the same sign (a+b+c)/d; rung-146 flips ONE numerator sign to a MIXED-OP three-term numerator (a+b−c)/d — two credits "
            "added and a debit subtracted, netted and divided by one window. From a net total (credit_one + credit_two − debit_one) divided "
            "by a window, compute the net rate ((credit_one+credit_two−debit_one)/window), the net total (credit_one+credit_two−debit_one), "
            "or the gross total (credit_one+credit_two, the two credits before the debit). Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a MIXED-OP THREE-TERM NUMERATOR "
            "OVER A LONE DIVISOR (a+b−c)/d (add the two credits AND subtract the debit FIRST, then divide the net by the window). The mixed "
            "sign brings a slip the all-`+` rung-145 could not test — the SIGN ERROR, adding the debit instead of subtracting it "
            "((a+b+c)/d, treating the debit as a third credit) — alongside the universal INVERTING slip (d/(a+b−c), the reciprocal). The "
            "panel puts the honest gross subtotal (a+b) beside the sign-error slip ((a+b+c)/d) so the difference is exactly 'did you "
            "SUBTRACT the debit, or add it?'. The harness matches the scalar to the printed options. Contamination-safe: every figure is "
            "built only from the four observed quantities via +, −, and / — no constant leaks, and neither the net total, the gross total, "
            "nor the net rate ever appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. Because the numerator subtracts the debit, the net total carries a positivity guard "
            "(credit_one + credit_two − debit_one >= 2) so every figure stays strictly positive; the window and the gross total are sums of "
            "positives and so are automatically positive. The five family values are kept pairwise distinct with all three queried readouts "
            "varying across the panel, all asserted strictly positive at build time."
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
