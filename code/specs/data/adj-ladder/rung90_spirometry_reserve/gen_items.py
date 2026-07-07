"""Generate rung-90 (pulmonary-function / spirometry ventilatory reserve index) items.json for the ADJ-LADDER.

Rung 90 opens the **pulmonary-function / spirometry** panel on the quantitative band — the arithmetic of a ventilatory
reserve index. A `forced_volume` minus a `residual_volume` gives a usable capacity span, a `peak_flow` minus a
`trough_flow` gives a flow span, and the two spans MULTIPLY into the reserve index. A **difference times a difference**
introduces a genuinely NEW arithmetic shape on the ladder: the **product of two DIFFERENCES** — `(a-b)*(c-d)`, i.e.
`((a-b)*(c-d))`.

This is genuinely new and completes the binomial-product family opened at rung-89. Rung-89 shipped the ladder's first
product of two binomials as a SUM times a DIFFERENCE (`(a+b)*(c-d)`); rung-90 ships the DIFFERENCE times a DIFFERENCE
(`(a-b)*(c-d)`) — no shipped shape ever multiplied a difference by ANOTHER difference. Every earlier shape that used a
parenthesised binomial either multiplied/divided it by a single observed factor (rung-67 `(a-b)*c/d`, rung-68
`(a+b)*c/d`, rung-75 `(a-b)/c*d`, rung-76 `(a+b)/c*d`, rung-81 `(a+b)/c-d`, rung-82 `(a-b)/c+d`, rung-84 `(a+b)*c-d`) or,
at rung-89, multiplied a SUM by a difference (`(a+b)*(c-d)`) — never a difference by another difference. `(a-b)*(c-d)` is
the ladder's first **product of two differences**. The operator order matters: `(a-b)*(c-d)` is `((a-b)*(c-d))` (each
parenthesised group evaluates first, then the two groups multiply), NOT `(a-b)*c-d` (subtracting `d` OUTSIDE the product
instead of inside the second factor) and NOT `(a+b)*(c-d)` (summing the first pair instead of differencing it) — the two
distractors exploit exactly those confusions.

The setup: a `forced_volume`, a `residual_volume`, a `peak_flow`, and a `trough_flow`. The reserve index is:

  RESERVE INDEX   (forced_volume - residual_volume) * (peak_flow - trough_flow)  [ capacity span times flow span ]
  CAPACITY SPAN   forced_volume - residual_volume                                [ the usable capacity, before scaling ]
  FLOW SPAN       peak_flow - trough_flow                                        [ the flow span, before scaling ]

The **reserve index** is what makes this rung distinctive — it is the ladder's first **product of two differences**.
(The capacity span `a-b` and the flow span `c-d` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 47-89 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the residual volume from the forced volume into a capacity span, the
subtraction of the trough flow from the peak flow into a flow span, then the multiplication of the two (each difference
group before the multiply) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../88/89. This rung exercises the engine across **a product of two
differences** — the fact that `(a-b)*(c-d)` is `((a-b)*(c-d))` and NOT `(a-b)*c-d` and NOT `(a+b)*(c-d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-`, and `*`
— **no structural constants** — so no numeric literal appears in any program, and neither the capacity span, the flow
span, nor any reserve figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`forced_volume`, `residual_volume`, `peak_flow`, `trough_flow`) so no numeral hides
inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (forced_volume - residual_volume) * peak_flow - trough_flow    subtract the trough flow OUTSIDE the product
                                                                            instead of inside the second factor (the
                                                                            classic `(a-b)*(c-d)` vs `(a-b)*c-d` error),
                                                                            and
  SWAPPED    (forced_volume + residual_volume) * (peak_flow - trough_flow)  sum the first pair instead of differencing
                                                                            it — add the volumes rather than subtract
                                                                            (`(a+b)*(c-d)` instead of `(a-b)*(c-d)`),

which are exactly the mistakes a student makes (dropping the parenthesis on the second difference, or summing the first
pair when it should be differenced). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all
five always appear as options.

Distinctness and positivity: the tables keep the guards — `forced_volume > residual_volume >= 2` (so the capacity span
is positive) and `peak_flow > trough_flow >= 2` (so the flow span is positive) — so every family member, including the
headline reserve index `(a-b)*(c-d)`, is strictly positive (a positive capacity span times a positive flow span, and
likewise for the slips); the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (FORCED_VOLUME, RESIDUAL_VOLUME, PEAK_FLOW, TROUGH_FLOW) — a forced volume and a residual volume whose difference is
# the usable capacity span, and a peak flow and a trough flow whose difference is the flow span, all plain positive
# numbers >= 2. The tables satisfy the guards: forced_volume > residual_volume >= 2 (so the capacity span (a-b) stays
# positive) and peak_flow > trough_flow >= 2 (so the flow span (c-d) is positive). The five family values are asserted
# pairwise-distinct below.
TABLES = [
    (4, 2, 6, 2),
    (5, 2, 7, 2),
    (5, 3, 7, 3),
    (6, 2, 4, 2),
    (6, 3, 8, 2),
    (6, 4, 8, 3),
    (7, 2, 5, 2),
]

# The option family (5 members), all built from the four observed quantities via +, -, and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "reserve_index",
        "ventilatory reserve index (the capacity span times the flow span)",
        "(forced_volume - residual_volume) * (peak_flow - trough_flow)",
    ),
    (
        "capacity_span",
        "the capacity span (the forced volume minus the residual volume, before scaling by the flow span)",
        "forced_volume - residual_volume",
    ),
    (
        "flow_span",
        "the flow span (the peak flow minus the trough flow, before scaling by the capacity span)",
        "peak_flow - trough_flow",
    ),
    (
        "crossed",
        "the capacity span times the peak flow, MINUS the trough flow, with the trough subtracted outside the product instead of inside the second factor (a wrong grouping)",
        "(forced_volume - residual_volume) * peak_flow - trough_flow",
    ),
    (
        "swapped",
        "the forced volume PLUS the residual volume, times the peak flow minus the trough flow, summing the first pair instead of differencing it (a wrong pairing)",
        "(forced_volume + residual_volume) * (peak_flow - trough_flow)",
    ),
]
QUERIED = ["reserve_index", "capacity_span", "flow_span"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(forced_volume, residual_volume, peak_flow, trough_flow):
    # Operation order mirrors the ADJ programs exactly (each parenthesised difference evaluates first, then the two
    # groups multiply, so (a-b)*(c-d) evaluates as ((a-b)*(c-d))), so the Python option value and the engine result are
    # the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "reserve_index": (forced_volume - residual_volume) * (peak_flow - trough_flow),
        "capacity_span": forced_volume - residual_volume,
        "flow_span": peak_flow - trough_flow,
        "crossed": (forced_volume - residual_volume) * peak_flow - trough_flow,
        "swapped": (forced_volume + residual_volume) * (peak_flow - trough_flow),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for forced_volume, residual_volume, peak_flow, trough_flow in TABLES:
        assert (
            forced_volume > 0
            and residual_volume > 0
            and peak_flow > 0
            and trough_flow > 0
        ), (forced_volume, residual_volume, peak_flow, trough_flow)
        fv = family_values(forced_volume, residual_volume, peak_flow, trough_flow)
        # The tables satisfy the guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, forced_volume, residual_volume, peak_flow, trough_flow, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    forced_volume,
                    residual_volume,
                    peak_flow,
                    trough_flow,
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
                forced_volume,
                residual_volume,
                peak_flow,
                trough_flow,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r90spr-{idx + 1:02d}",
                "qtype": "spirometry_reserve_index",
                "stem": (
                    f"A spirometry study reads a forced volume of {num(forced_volume)} minus a residual volume of "
                    f"{num(residual_volume)}, all scaled by a peak flow of {num(peak_flow)} minus a trough flow "
                    f"of {num(trough_flow)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe forced_volume({num(forced_volume)})\n"
                    f"observe residual_volume({num(residual_volume)})\n"
                    f"observe peak_flow({num(peak_flow)})\n"
                    f"observe trough_flow({num(trough_flow)})\n"
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
            "ADJ-LADDER rung 90 — pulmonary-function spirometry ventilatory reserve index from four stated quantities (a "
            "NEW panel: pulmonary-function / spirometry). From a forced volume and a residual volume whose difference is "
            "the usable capacity span and a peak flow and a trough flow whose difference is the flow span, compute the "
            "reserve index ((forced_volume-residual_volume)*(peak_flow-trough_flow)), the capacity span "
            "(forced_volume-residual_volume), or the flow span (peak_flow-trough_flow). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW shape, PRODUCT OF TWO DIFFERENCES (a-b)*(c-d) (subtract b from a, subtract d from c, "
            "multiply the two groups, so (a-b)*(c-d) = ((a-b)*(c-d)); no prior shape multiplied a DIFFERENCE by another "
            "DIFFERENCE — every earlier binomial shape multiplied or divided a binomial by a single observed factor, and "
            "rung-89 (a+b)*(c-d) multiplied a SUM by a difference, never a difference by another difference) — and the "
            "harness matches the scalar to the printed options. Contamination-safe: every index is built only from the "
            "four observed quantities via +, -, and * — no constant leaks, and neither the capacity span, the flow span, "
            "nor any reserve figure ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the "
            "same four quantities, so the distractors are exactly the slips students make: subtracting the trough flow "
            "outside the product instead of inside the second factor ((a-b)*c-d, a wrong grouping) and summing the first "
            "pair instead of differencing it ((a+b)*(c-d), a wrong pairing). The core confusion tested is that "
            "(a-b)*(c-d) is ((a-b)*(c-d)), not (a-b)*c-d and not (a+b)*(c-d)."
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
