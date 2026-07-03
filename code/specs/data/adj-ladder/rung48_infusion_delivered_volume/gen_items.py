"""Generate rung-48 (infusion delivered-volume) items.json for the ADJ-LADDER.

Rung 48 opens the **infusion-therapy / delivered-volume** panel on the quantitative band — the arithmetic of the
volume an IV pump actually delivers when two channels run together over a treatment window. Two lines infuse
simultaneously at `rate_first` and `rate_second` (mL per hour); their combined rate is the SUM `rate_first +
rate_second`. The window they run over is the DIFFERENCE between the clock the pump stops and the clock it starts,
`stop_hour − start_hour` (hours). The volume delivered is the combined rate multiplied by the elapsed time. This rung
introduces a genuinely NEW arithmetic shape on the ladder: **sum-times-difference** — `(a + b) · (c − d)` — a
parenthesised sum multiplied by a parenthesised difference.

The setup: two IV channels run at `rate_first` and `rate_second` mL/h from `start_hour` to `stop_hour`. The delivered
volume is the combined rate times the elapsed time:

  DELIVERED VOLUME   (rate_first + rate_second) · (stop_hour − start_hour)   [ mL ]
  COMBINED RATE      rate_first + rate_second                                [ the first factor: both channels ]
  ELAPSED TIME       stop_hour − start_hour                                  [ the second factor ]

The **delivered volume** is what makes this rung distinctive — it is the ladder's first **sum-times-difference**: a
parenthesised sum multiplied by a parenthesised difference. Contrast the neighbours already on the ladder: rung-33 was
a *product of two differences* `(a−b)·(c−d)`, rung-34 a *sum of two products* `a·b + c·d`, rung-43 a *sum of three
products*; none multiplied a SUM by a DIFFERENCE. (The combined rate `rate_first + rate_second` and the elapsed time
`stop_hour − start_hour` ride alongside as the two component quantities, so the panel teaches the whole calculation —
exactly as rung-46/47 shipped their component sums/products beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — including the inner `(rate_first + rate_second)` sum and the `(stop_hour −
start_hour)` difference — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../46/47. This rung exercises the engine across a **sum multiplied by a
difference**.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `−` and
`·` — **no structural constants** — so no numeric literal appears in any program, and neither the combined rate, the
elapsed time, nor any delivered-volume figure is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`rate_first`, `rate_second`, `start_hour`, `stop_hour`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic
slips —

  SUMMED CLOCK    (rate_first + rate_second) · (stop_hour + start_hour)   ADD the two clock times instead of
                                                                          subtracting them (elapsed should be a
                                                                          difference), and
  DIFF RATE       (rate_first − rate_second) · (stop_hour − start_hour)   SUBTRACT the two channel rates instead of
                                                                          adding them (the combined rate should be a
                                                                          sum),

which are exactly the mistakes a student makes (adding two quantities that should be subtracted, or subtracting two
quantities that should be added). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all
five always appear as options.

Distinctness: all four observed quantities are positive with `stop_hour > start_hour` and `rate_first ≠ rate_second`,
so the combined rate and elapsed time are strictly positive and the two slips are well-defined and non-zero; the
tables below are chosen so the five family values are pairwise distinct with a comfortable margin, asserted at build
time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (RATE_FIRST, RATE_SECOND, START_HOUR, STOP_HOUR) — rates in mL/h, hours are clock readings. All four quantities are
# strictly positive with STOP_HOUR > START_HOUR (so the elapsed time is positive) and RATE_FIRST != RATE_SECOND (so
# the diff-rate slip is non-zero and distinct). The five family values are asserted pairwise-distinct (with margin)
# below.
TABLES = [
    (60, 40, 2, 6),
    (80, 20, 1, 5),
    (90, 30, 1, 4),
    (40, 25, 3, 9),
    (55, 35, 2, 5),
    (100, 60, 4, 10),
    (45, 15, 1, 7),
]

# The option family (5 members), all built from the four observed quantities via +, - and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "delivered_volume",
        "delivered volume (combined rate times the elapsed time)",
        "(rate_first + rate_second) * (stop_hour - start_hour)",
    ),
    (
        "combined_rate",
        "the combined infusion rate (the two channel rates added)",
        "rate_first + rate_second",
    ),
    (
        "elapsed_time",
        "the elapsed time (stop clock minus start clock)",
        "stop_hour - start_hour",
    ),
    (
        "summed_clock",
        "combined rate times the two clock times ADDED, not subtracted (a wrong window)",
        "(rate_first + rate_second) * (stop_hour + start_hour)",
    ),
    (
        "diff_rate",
        "the two channel rates SUBTRACTED instead of added, times the elapsed time",
        "(rate_first - rate_second) * (stop_hour - start_hour)",
    ),
]
QUERIED = ["delivered_volume", "combined_rate", "elapsed_time"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(rate_first, rate_second, start_hour, stop_hour):
    # Operation order mirrors the ADJ programs exactly (a sum times a difference), so the Python option value and the
    # engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    combined = rate_first + rate_second
    elapsed = stop_hour - start_hour
    return {
        "delivered_volume": combined * elapsed,
        "combined_rate": combined,
        "elapsed_time": elapsed,
        "summed_clock": combined * (stop_hour + start_hour),
        "diff_rate": (rate_first - rate_second) * elapsed,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for rate_first, rate_second, start_hour, stop_hour in TABLES:
        assert (
            rate_first > 0
            and rate_second > 0
            and start_hour > 0
            and stop_hour > start_hour
            and rate_first != rate_second
        ), (rate_first, rate_second, start_hour, stop_hour)
        fv = family_values(rate_first, rate_second, start_hour, stop_hour)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    rate_first,
                    rate_second,
                    start_hour,
                    stop_hour,
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
                rate_first,
                rate_second,
                start_hour,
                stop_hour,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r48inf-{idx + 1:02d}",
                "qtype": "infusion_delivered_volume",
                "stem": (
                    f"Two IV channels run at {num(rate_first)} and {num(rate_second)} mL/h from hour "
                    f"{num(start_hour)} to hour {num(stop_hour)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe rate_first({num(rate_first)})\n"
                    f"observe rate_second({num(rate_second)})\n"
                    f"observe start_hour({num(start_hour)})\n"
                    f"observe stop_hour({num(stop_hour)})\n"
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
            "ADJ-LADDER rung 48 — infusion delivered-volume from four stated quantities (a NEW panel: "
            "infusion-therapy / delivered-volume). From two channel infusion rates plus a start clock and a stop "
            "clock compute the delivered volume ((rate_first+rate_second)*(stop_hour-start_hour)), the combined rate "
            "(rate_first+rate_second), or the elapsed time (stop_hour-start_hour). Each item is a compute_dimensioned "
            "program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a "
            "NEW shape, SUM-TIMES-DIFFERENCE (a+b)*(c-d), the first product on the ladder to multiply a parenthesised "
            "sum by a parenthesised difference (distinct from rung-33 product-of-two-differences (a-b)*(c-d), rung-34 "
            "sum-of-two-products a*b+c*d, and rung-43 sum-of-three-products) — and the harness matches the scalar to "
            "the printed options. Contamination-safe: every index is built only from the four observed quantities via "
            "+, - and * — no constant leaks, and neither the combined rate, the elapsed time, nor any delivered-volume "
            "figure ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the same four "
            "quantities, so the distractors are exactly the slips students make: ADDING the two clock times instead of "
            "subtracting them (a wrong window), and SUBTRACTING the two channel rates instead of adding them. The core "
            "confusion tested is multiplying the combined-rate sum by the elapsed-time difference."
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
