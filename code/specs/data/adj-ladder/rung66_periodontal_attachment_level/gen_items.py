"""Generate rung-66 (periodontal clinical attachment level) items.json for the ADJ-LADDER.

Rung 66 opens the **dentistry / periodontics** panel on the quantitative band — the arithmetic of a corrected clinical
attachment level. A periodontal charting SUMS the probing depths recorded on the buccal surfaces and on the lingual
surfaces, divides by the number of teeth to get the mean probing depth per tooth, and finally SUBTRACTS a fixed gingival
margin offset. Dividing a SUM by a divisor and then SUBTRACTING a separate term introduces a genuinely NEW arithmetic
shape on the ladder: a **sum-over-a-divisor, minus a term** — `(a+b)/c - d`.

The pedagogical heart of this rung is **operator precedence**, and it TIES THE FAMILY TOGETHER: `(a+b)/c - d` means
`((a+b)/c) - d` — the margin offset `d` is subtracted AFTER the division, it is NOT part of the denominator, and it is
SUBTRACTED, not added. The two tempting slips are exactly two shapes already on the ladder: `(a+b)/(c-d)` (rung-63, the
offset folded INTO the denominator) and `(a+b)/c + d` (rung-65's headline, adding instead of subtracting).

The setup: a `buccal_sum` of probing depths, a `lingual_sum`, a `tooth_count` to divide by, and a `margin_offset` to
subtract. The clinical attachment level is:

  ATTACHMENT LEVEL   (buccal_sum + lingual_sum) / tooth_count - margin_offset   [ mean depth per tooth minus margin ]
  TOTAL DEPTH        buccal_sum + lingual_sum                                   [ the numerator: total probing depth ]
  MEAN DEPTH         (buccal_sum + lingual_sum) / tooth_count                   [ the quotient, before the margin ]

The **attachment level** is what makes this rung distinctive — it is the ladder's first **sum-over-a-divisor minus a
term**: a quotient of a sum, then a lone term subtracted OUTSIDE the division. It is the mirror of rung-65's
`(a-b)/c + d` (a difference numerator and an ADDED term). (The total depth `buccal_sum+lingual_sum` and the mean depth
`(buccal_sum+lingual_sum)/tooth_count` ride alongside as component readouts, so the panel teaches the whole calculation —
exactly as rungs 47-65 shipped their component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the numerator sum, the division by the tooth count, and the final subtraction of the margin
offset — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change,
exactly as rungs 8/16/.../64/65. This rung exercises the engine across **division-then-subtraction precedence** — the fact
that `(a+b)/c-d` is NOT `(a+b)/(c-d)` and NOT `(a+b)/c+d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `+`, and `-`
— **no structural constants** — so no numeric literal appears in any program, and neither the total depth, the mean
depth, nor any attachment-level figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`buccal_sum`, `lingual_sum`, `tooth_count`, `margin_offset`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  POOLED     (buccal_sum + lingual_sum) / (tooth_count - margin_offset)   subtract the margin INSIDE the denominator
                                                                          instead of after the division (the classic
                                                                          `(a+b)/c-d` vs `(a+b)/(c-d)` precedence error,
                                                                          rung-63's shape), and
  CROSSED    (buccal_sum + lingual_sum) / tooth_count + margin_offset      ADD the margin instead of subtracting it
                                                                          (rung-65's shape),

which are exactly the mistakes a student makes (folding the subtracted margin into the denominator, or adding it instead
of subtracting). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness: all four observed quantities are strictly positive; the tables are chosen so the tooth count exceeds the
margin offset (the pooled denominator is positive) and stays clear of `tooth_count - margin_offset == 1` (which would
make the pooled value coincide with the total depth), and the mean depth exceeds the margin offset (the attachment level
is positive); the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BUCCAL_SUM, LINGUAL_SUM, TOOTH_COUNT, MARGIN_OFFSET) — two summed probing depths, a tooth count to divide by, and a
# margin offset to subtract, all plain positive numbers with tooth_count > margin_offset, tooth_count - margin_offset
# != 1, and (buccal_sum + lingual_sum) / tooth_count > margin_offset. The five family values are asserted
# pairwise-distinct (with margin) below.
TABLES = [
    (60, 40, 10, 3),
    (80, 40, 8, 4),
    (50, 30, 8, 2),
    (90, 60, 10, 5),
    (70, 50, 6, 4),
    (100, 80, 12, 5),
    (60, 60, 8, 3),
]

# The option family (5 members), all built from the four observed quantities via /, +, and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "attachment_level",
        "clinical attachment level (mean probing depth per tooth minus the gingival margin offset)",
        "(buccal_sum + lingual_sum) / tooth_count - margin_offset",
    ),
    (
        "total_depth",
        "the total probing depth (buccal plus lingual sums)",
        "buccal_sum + lingual_sum",
    ),
    (
        "mean_depth",
        "the mean probing depth per tooth before the margin offset (total depth over the tooth count)",
        "(buccal_sum + lingual_sum) / tooth_count",
    ),
    (
        "pooled",
        "the total depth over the tooth count with the margin offset folded into the denominator, not subtracted after (a wrong divisor)",
        "(buccal_sum + lingual_sum) / (tooth_count - margin_offset)",
    ),
    (
        "crossed",
        "the mean depth per tooth with the margin offset ADDED instead of subtracted (a wrong attachment level)",
        "(buccal_sum + lingual_sum) / tooth_count + margin_offset",
    ),
]
QUERIED = ["attachment_level", "total_depth", "mean_depth"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(buccal_sum, lingual_sum, tooth_count, margin_offset):
    # Operation order mirrors the ADJ programs exactly (the parenthesised sum formed first, then the division, then the
    # subtraction of the margin OUTSIDE the division), so the Python option value and the engine result are the same
    # IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "attachment_level": (buccal_sum + lingual_sum) / tooth_count - margin_offset,
        "total_depth": buccal_sum + lingual_sum,
        "mean_depth": (buccal_sum + lingual_sum) / tooth_count,
        "pooled": (buccal_sum + lingual_sum) / (tooth_count - margin_offset),
        "crossed": (buccal_sum + lingual_sum) / tooth_count + margin_offset,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for buccal_sum, lingual_sum, tooth_count, margin_offset in TABLES:
        assert (
            buccal_sum > 0
            and lingual_sum > 0
            and tooth_count > 0
            and margin_offset > 0
        ), (buccal_sum, lingual_sum, tooth_count, margin_offset)
        # The tooth count must exceed the margin offset (pooled denominator positive) and avoid
        # tooth_count - margin_offset == 1 (which would make pooled == total depth); the mean depth
        # must exceed the margin offset (attachment level positive).
        assert tooth_count > margin_offset, (buccal_sum, lingual_sum, tooth_count, margin_offset)
        assert (tooth_count - margin_offset) != 1, (buccal_sum, lingual_sum, tooth_count, margin_offset)
        assert (buccal_sum + lingual_sum) / tooth_count > margin_offset, (
            buccal_sum, lingual_sum, tooth_count, margin_offset,
        )
        fv = family_values(buccal_sum, lingual_sum, tooth_count, margin_offset)
        for key, v in fv.items():
            assert v > 0, (key, buccal_sum, lingual_sum, tooth_count, margin_offset, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    buccal_sum,
                    lingual_sum,
                    tooth_count,
                    margin_offset,
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
                buccal_sum,
                lingual_sum,
                tooth_count,
                margin_offset,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r66perio-{idx + 1:02d}",
                "qtype": "periodontal_attachment_level",
                "stem": (
                    f"A periodontal chart sums to {num(buccal_sum)} units of buccal probing depth and "
                    f"{num(lingual_sum)} of lingual probing depth across {num(tooth_count)} teeth, then a "
                    f"{num(margin_offset)} gingival margin offset is applied. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe buccal_sum({num(buccal_sum)})\n"
                    f"observe lingual_sum({num(lingual_sum)})\n"
                    f"observe tooth_count({num(tooth_count)})\n"
                    f"observe margin_offset({num(margin_offset)})\n"
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
            "ADJ-LADDER rung 66 — clinical attachment level from four stated quantities (a NEW panel: dentistry / "
            "periodontics). From a buccal and a lingual sum of probing depths (their sum is the total depth), a tooth "
            "count to divide by, and a gingival margin offset to subtract, compute the attachment level "
            "((buccal_sum+lingual_sum)/tooth_count-margin_offset), the total depth (buccal_sum+lingual_sum), or the "
            "mean depth ((buccal_sum+lingual_sum)/tooth_count). Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, "
            "SUM-OVER-A-DIVISOR MINUS A TERM (a+b)/c-d, the first on the ladder to divide a sum and then subtract a "
            "separate term OUTSIDE the division (the mirror of rung-65 (a-b)/c+d) — and the harness matches the scalar "
            "to the printed options. The pedagogical heart is operator precedence, and it ties the family together: "
            "(a+b)/c-d is ((a+b)/c)-d, not (a+b)/(c-d) [rung-63, offset in the denominator] and not (a+b)/c+d [rung-65, "
            "added not subtracted]. Contamination-safe: every index is built only from the four observed quantities via "
            "/, +, and - — no constant leaks, and neither the total depth, the mean depth, nor any attachment-level "
            "figure ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the same four "
            "quantities, so the distractors are exactly the slips students make: folding the margin offset INTO the "
            "denominator ((a+b)/(c-d), a wrong divisor) and ADDING the margin instead of subtracting it ((a+b)/c+d, a "
            "wrong attachment level). The core confusion tested is that (a+b)/c-d is not (a+b)/(c-d) and not (a+b)/c+d."
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
