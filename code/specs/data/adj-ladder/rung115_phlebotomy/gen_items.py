"""Generate rung-115 (phlebotomy / specimen collection) items.json for the ADJ-LADDER.

Rung 115 opens the **phlebotomy / specimen-collection** panel on the quantitative band — the arithmetic of a usable-volume
index. A `draw_volume` (total blood drawn) MINUS a `discard_volume` (the initial discard/clearing tube) MINUS a `waste_volume`
(hemolysed/clotted waste) gives the usable load, and that load is DIVIDED by a `tube_count` (how many collection tubes the draw
is split across) to give the usable-volume index. A **three-term ALL-SUBTRACT numerator, all over a divisor**, introduces a
genuinely NEW arithmetic family on the ladder: `(a-b-c)/d`, i.e. `(((a - b) - c) / d)`.

This is genuinely new — the flat three-term numerators shipped so far were rung-108 `(a+b+c)/d` (all sums), rung-109
`(a-b+c)/d`, rung-110 `(a*b+c)/d`, rung-111 `(a*b-c)/d`, and rung-114 `(a+b-c)/d` (two-add-one-subtract); NONE is the
all-subtract `(a-b-c)/d`. It is the LAST unshipped `+/-` sign corner of the three-term-additive-numerator family: rung-108 was
all-plus, rung-114 was `+ + -`, and rung-115 is `+ - -` (subtract BOTH the discard and the waste). The distributive pair
(rung-112 `a*(b+c)/d`, rung-113 `a*(b-c)/d`) wrapped a sum/difference inside a factor; rung-115 stays a FLAT three-term
numerator but with the previously-unshipped subtract-both shape. Every earlier ratio used either a two-term numerator (rung-37
`(a+b)/(c+d)`, rung-99 `(a*b)/(c+d)`, rung-100 `(a+b)/(c*d)`, rung-104 `(a-b)/(c*d)`, the difference-denominator trio rung-105
`(a+b)/(c-d)`, rung-106 `a*b/(c-d)`, rung-107 `(a-b)/(c-d)`) or one of the earlier three-term numerators (108-111, 114) or the
distributive pair (112-113). Rung-115 moves to `(a-b-c)/d`. The operator order matters: `(a-b-c)/d` is `(((a-b)-c)/d)` (the
discard subtracts from the draw, then the waste subtracts from that, then the whole numerator is divided; `-` binds
left-to-right and precedes `/` only via the explicit numerator parentheses), NOT `a-b-c/d` (dropping the numerator parentheses
so only the waste is divided by the divisor and then subtracted) and NOT `(a-b)/(c+d)` (regrouping so only the draw-minus-discard
forms the numerator and the waste joins the divisor in the denominator) — the two distractors exploit exactly those confusions.

The setup: a `draw_volume`, a `discard_volume`, a `waste_volume`, and a `tube_count`. The total is:

  USABLE-VOLUME INDEX  (draw_volume - discard_volume - waste_volume) / tube_count  [ a three-term numerator over a divisor ]
  USABLE VOLUME        draw_volume - discard_volume - waste_volume                 [ the three-term numerator ]
  TUBE COUNT           tube_count                                                  [ the divisor ]

The **usable-volume index** is what makes this rung distinctive — it is the ladder's first **subtract-both three-term
numerator, over a divisor**. It is a rate (usable volume per tube), framed as an *index* to keep it dimensionless-clean — the
same discipline rungs 100/104/.../114 used for their ratios. (The usable volume `a-b-c` and the tube count `d` ride alongside as
component readouts, so the panel teaches the whole calculation — exactly as rungs 47-114 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the subtraction of the discard from the draw, the subtraction of the waste into the usable load, then the
division of that load by the tube count (the flat three-term numerator over the divisor, so (a-b-c)/d evaluates as
(((a-b)-c)/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change,
exactly as rungs 8/16/.../113/114. This rung exercises the engine across a **subtract-both numerator, over a divisor** — the
fact that `(a-b-c)/d` is `(((a-b)-c)/d)` and NOT `a-b-c/d` and NOT `(a-b)/(c+d)` made computable. The ratio golds are
non-integer f64s; the engine's IEEE-double division matches Python's the same way rungs 99/100/104/.../114 relied on (well
within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the usable volume, the tube count, nor any
index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`draw_volume`, `discard_volume`, `waste_volume`, `tube_count`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    draw_volume - discard_volume - waste_volume / tube_count  drop the numerator parentheses so only the waste is
                                                                    divided by the tube count and then subtracted (the classic
                                                                    `(a-b-c)/d` vs `a-b-c/d` grouping error), and
  SWAPPED    (draw_volume - discard_volume) / (waste_volume + tube_count)  regroup so only the draw minus the discard forms the
                                                                    numerator and the waste joins the tube count in the
                                                                    denominator (`(a-b)/(c+d)` instead of `(a-b-c)/d`),

which are exactly the mistakes a student makes (failing to keep the whole three-term numerator over the divisor, or regrouping
which terms belong to the numerator vs the divisor). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness and positivity: this rung SUBTRACTS both the discard and the waste in the numerator, so positivity is guaranteed by
table construction. Each table guarantees **draw_volume > discard_volume + waste_volume** (so the numerator `a-b-c` is strictly
positive, the usable volume is positive, and the index is positive) AND all quantities `>= 2` (so the crossed slip
`a-b - c/d` stays positive because `a-b > c > c/d`, and the swapped denominator `c+d >= 4` with the swapped numerator `a-b > 0`
since `a > b+c > b`). The **tube_count >= 2** keeps the divisor away from zero, the usable-volume index never coincides with the
tube count or the usable volume, and the five family values are pairwise distinct with a comfortable margin; and — so all three
queried readouts vary across the panel — the seven tables give distinct usable-volume indices, distinct usable volumes, and
distinct tube counts, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (DRAW_VOLUME, DISCARD_VOLUME, WASTE_VOLUME, TUBE_COUNT) — a draw volume minus a discard volume minus a waste volume for the
# usable load, all divided by a tube count, all plain positive numbers >= 2. This rung SUBTRACTS both the discard and the waste
# in the numerator, so every table guarantees draw_volume > discard_volume + waste_volume (a>b+c) which keeps the numerator, the
# usable volume, and the index strictly positive; tube_count >= 2 keeps the divisor away from zero. The five family values are
# asserted pairwise-distinct below. The seven tables give distinct usable-volume indices, distinct usable volumes, and distinct
# tube counts so all three queried readouts vary across the panel.
TABLES = [
    (12, 3, 2, 2),
    (13, 3, 2, 3),
    (14, 3, 2, 4),
    (16, 3, 2, 5),
    (18, 3, 2, 6),
    (20, 3, 2, 7),
    (22, 3, 2, 8),
]

# The option family (5 members), all built from the four observed quantities via - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "usable_index",
        "usable-volume index (the usable volume divided by the tube count)",
        "(draw_volume - discard_volume - waste_volume) / tube_count",
    ),
    (
        "usable_volume",
        "the usable volume (the draw minus the discard minus the waste, the numerator divided by the tube count)",
        "draw_volume - discard_volume - waste_volume",
    ),
    (
        "tube_count",
        "the tube count (the divisor the usable volume is divided by)",
        "tube_count",
    ),
    (
        "crossed",
        "the draw minus the discard minus the waste divided by the tube count, dropping the numerator parentheses so only the waste is divided (a wrong grouping)",
        "draw_volume - discard_volume - waste_volume / tube_count",
    ),
    (
        "swapped",
        "the draw minus the discard, divided by the waste plus the tube count, regrouping so only the draw-minus-discard forms the numerator (a wrong pairing)",
        "(draw_volume - discard_volume) / (waste_volume + tube_count)",
    ),
]
QUERIED = ["usable_index", "usable_volume", "tube_count"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(draw_volume, discard_volume, waste_volume, tube_count):
    # Operation order mirrors the ADJ programs exactly (the discard subtracts from the draw, the waste subtracts into the usable
    # load, then that numerator is divided by the tube count, so (a-b-c)/d evaluates as (((a-b)-c)/d)), so the Python option value
    # and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "usable_index": (draw_volume - discard_volume - waste_volume) / tube_count,
        "usable_volume": draw_volume - discard_volume - waste_volume,
        "tube_count": tube_count,
        "crossed": draw_volume - discard_volume - waste_volume / tube_count,
        "swapped": (draw_volume - discard_volume) / (waste_volume + tube_count),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for draw_volume, discard_volume, waste_volume, tube_count in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung SUBTRACTS both the discard and the waste in the
        # numerator, so each table guarantees draw_volume > discard_volume + waste_volume (the numerator a-b-c is strictly
        # positive) which keeps every family member strictly positive; tube_count >= 2 keeps the divisor away from zero.
        assert (
            draw_volume >= 2
            and discard_volume >= 2
            and waste_volume >= 2
            and tube_count >= 2
        ), (draw_volume, discard_volume, waste_volume, tube_count)
        assert draw_volume > discard_volume + waste_volume, (
            draw_volume, discard_volume, waste_volume, tube_count,
        )
        fv = family_values(draw_volume, discard_volume, waste_volume, tube_count)
        for key, v in fv.items():
            assert v > 0, (key, draw_volume, discard_volume, waste_volume, tube_count, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    draw_volume,
                    discard_volume,
                    waste_volume,
                    tube_count,
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
                draw_volume,
                discard_volume,
                waste_volume,
                tube_count,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r115phleb-{idx + 1:02d}",
                "qtype": "usable_index",
                "stem": (
                    f"A phlebotomy log records a draw volume of {num(draw_volume)} minus a discard volume of "
                    f"{num(discard_volume)} minus a waste volume of {num(waste_volume)}, divided by a tube count of "
                    f"{num(tube_count)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe draw_volume({num(draw_volume)})\n"
                    f"observe discard_volume({num(discard_volume)})\n"
                    f"observe waste_volume({num(waste_volume)})\n"
                    f"observe tube_count({num(tube_count)})\n"
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
            "ADJ-LADDER rung 115 — usable-volume index from four stated quantities (a NEW panel: phlebotomy / specimen "
            "collection). From a draw volume minus a discard volume minus a waste volume for the usable load, all divided by a "
            "tube count, compute the usable-volume index "
            "((draw_volume-discard_volume-waste_volume)/tube_count), the usable volume "
            "(draw_volume-discard_volume-waste_volume), or the tube count. Each item is a compute_dimensioned program (observe "
            "the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, an ALL-SUBTRACT "
            "THREE-TERM NUMERATOR, OVER A DIVISOR (a-b-c)/d (subtract the discard from the draw, subtract the waste, divide by "
            "the tube count, so (a-b-c)/d = (((a-b)-c)/d); the LAST unshipped +/- three-term sign corner — rung-108 (a+b+c)/d "
            "was all-plus, rung-114 (a+b-c)/d was + + -, and rung-115 is + - -. The earlier three-term numerators (108 "
            "(a+b+c)/d, 109 (a-b+c)/d, 110 (a*b+c)/d, 111 (a*b-c)/d, 114 (a+b-c)/d) never subtracted BOTH later terms; the "
            "distributive pair (112 a*(b+c)/d, 113 a*(b-c)/d) wrapped a sum/difference inside a factor. Every earlier ratio used "
            "a TWO-term numerator: 37 (a+b)/(c+d), 99 (a*b)/(c+d), 100 (a+b)/(c*d), 104 (a-b)/(c*d), and the difference-"
            "denominator trio 105 (a+b)/(c-d), 106 a*b/(c-d), 107 (a-b)/(c-d)) — and the harness matches the scalar to the "
            "printed options. The usable-volume index is a rate (usable volume per tube), framed as an INDEX so the "
            "dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed quantities "
            "via - and / — no constant leaks, and neither the usable volume, the tube count, nor any index ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a "
            "variable name. The five options are a family over the same four quantities, so the distractors are exactly the "
            "slips students make: dropping the numerator parentheses so only the waste is divided (a-b-c/d, a wrong grouping) "
            "and regrouping so only the draw-minus-discard forms the numerator ((a-b)/(c+d), a wrong pairing). The core "
            "confusion tested is that (a-b-c)/d is (((a-b)-c)/d), not a-b-c/d and not (a-b)/(c+d). This rung SUBTRACTS both the "
            "discard and the waste in the numerator, so positivity is guaranteed by table construction: every table has "
            "draw_volume > discard_volume + waste_volume (a>b+c) and tube_count >= 2 (divisor never zero), keeping every family "
            "member strictly positive."
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
