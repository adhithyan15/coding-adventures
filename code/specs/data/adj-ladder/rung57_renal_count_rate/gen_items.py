"""Generate rung-57 (renal count rate) items.json for the ADJ-LADDER.

Rung 57 opens the **nuclear-medicine / renography** panel on the quantitative band — the arithmetic of a dual-kidney
radionuclide renogram. A gamma camera counts the emissions over each kidney for a timed window; each kidney's COUNT RATE
is its counts divided by its seconds — a RATIO — and the total renal count rate is the SUM of the two kidneys' ratios.
Adding one ratio to ANOTHER ratio introduces a genuinely NEW arithmetic shape on the ladder: a **sum of two ratios** —
`a/b + c/d` — two independent quotients added.

The setup: a renogram counts `left_counts` over the left kidney in `left_seconds`, and `right_counts` over the right
kidney in `right_seconds`. The total renal count rate is the left count rate plus the right count rate:

  TOTAL RATE   left_counts / left_seconds + right_counts / right_seconds   [ counts per second — both kidneys ]
  LEFT RATE    left_counts / left_seconds                                  [ one ratio: the left kidney ]
  RIGHT RATE   right_counts / right_seconds                               [ the other ratio: the right kidney ]

The **total rate** is what makes this rung distinctive — it is the ladder's first **sum of two ratios**: two separate
quotients (each with its OWN denominator) added. Contrast the neighbours already on the ladder: rung-53 was
`(a+b+c)/d` (a single sum over one denominator) and rung-37 `(a+b)/(c+d)` (a single ratio of two sums); neither ADDED
two independent quotients. (The left rate `left_counts/left_seconds` and the right rate `right_counts/right_seconds`
ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-56 shipped their
component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including both quotients and their sum — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../55/56. This rung exercises the
engine across **two divisions folded into an addition** — the fact that `a/b + c/d` is NOT `(a+c)/(b+d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/` and `+` —
**no structural constants** — so no numeric literal appears in any program, and neither the left rate, the right rate,
nor any total-rate figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`left_counts`, `left_seconds`, `right_counts`, `right_seconds`) so no numeral hides
inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  POOLED RATIO   (left_counts + right_counts) / (left_seconds + right_seconds)   POOL the counts over the pooled seconds
                                                                                 — a RATIO OF TOTALS, not a sum of
                                                                                 ratios (the classic `a/b + c/d` vs
                                                                                 `(a+c)/(b+d)` error), and
  CROSSED        left_counts / right_seconds + right_counts / left_seconds       SWAP the denominators — divide each
                                                                                 kidney's counts by the OTHER kidney's
                                                                                 seconds,

which are exactly the mistakes a student makes (pooling numerators over pooled denominators, or pairing each numerator
with the wrong denominator). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness: all four observed quantities are strictly positive, so every quotient and sum is positive; the tables
below are chosen so the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (LEFT_COUNTS, LEFT_SECONDS, RIGHT_COUNTS, RIGHT_SECONDS) — gamma counts and timed seconds per kidney, all plain
# positive numbers (seconds chosen to divide the counts into clean rates). The five family values are asserted
# pairwise-distinct (with margin) below.
TABLES = [
    (2400, 60, 1800, 90),
    (3600, 90, 2000, 40),
    (1500, 30, 2800, 70),
    (4200, 60, 1200, 80),
    (2000, 50, 3300, 60),
    (2700, 90, 1600, 40),
    (5000, 100, 900, 30),
]

# The option family (5 members), all built from the four observed quantities via / and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "total_rate",
        "total renal count rate (the left count rate plus the right count rate)",
        "left_counts / left_seconds + right_counts / right_seconds",
    ),
    (
        "left_rate",
        "the left-kidney count rate (left counts per left second)",
        "left_counts / left_seconds",
    ),
    (
        "right_rate",
        "the right-kidney count rate (right counts per right second)",
        "right_counts / right_seconds",
    ),
    (
        "pooled_ratio",
        "the pooled counts over the pooled seconds, not the sum of the two rates (a wrong total)",
        "(left_counts + right_counts) / (left_seconds + right_seconds)",
    ),
    (
        "crossed",
        "each kidney's counts divided by the OTHER kidney's seconds (swapped denominators)",
        "left_counts / right_seconds + right_counts / left_seconds",
    ),
]
QUERIED = ["total_rate", "left_rate", "right_rate"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(left_counts, left_seconds, right_counts, right_seconds):
    # Operation order mirrors the ADJ programs exactly (each quotient formed first, then folded with +; and, for the
    # pooled slip, the parenthesised sums divide), so the Python option value and the engine result are the same
    # IEEE-double (well within the harness's 1e-9 match tolerance).
    left = left_counts / left_seconds
    right = right_counts / right_seconds
    return {
        "total_rate": left + right,
        "left_rate": left,
        "right_rate": right,
        "pooled_ratio": (left_counts + right_counts) / (left_seconds + right_seconds),
        "crossed": left_counts / right_seconds + right_counts / left_seconds,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for left_counts, left_seconds, right_counts, right_seconds in TABLES:
        assert (
            left_counts > 0
            and left_seconds > 0
            and right_counts > 0
            and right_seconds > 0
        ), (left_counts, left_seconds, right_counts, right_seconds)
        fv = family_values(left_counts, left_seconds, right_counts, right_seconds)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    left_counts,
                    left_seconds,
                    right_counts,
                    right_seconds,
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
                left_counts,
                left_seconds,
                right_counts,
                right_seconds,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r57rate-{idx + 1:02d}",
                "qtype": "renal_count_rate",
                "stem": (
                    f"A renogram counts {num(left_counts)} over the left kidney in {num(left_seconds)} s and "
                    f"{num(right_counts)} over the right kidney in {num(right_seconds)} s. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe left_counts({num(left_counts)})\n"
                    f"observe left_seconds({num(left_seconds)})\n"
                    f"observe right_counts({num(right_counts)})\n"
                    f"observe right_seconds({num(right_seconds)})\n"
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
            "ADJ-LADDER rung 57 — total renal count rate from four stated quantities (a NEW panel: nuclear medicine / "
            "renography). From two kidney count totals and their two timed windows compute the total count rate "
            "(left_counts/left_seconds + right_counts/right_seconds), the left count rate (left_counts/left_seconds), "
            "or the right count rate (right_counts/right_seconds). Each item is a compute_dimensioned program (observe "
            "the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, SUM OF "
            "TWO RATIOS a/b + c/d, the first on the ladder to add two independent quotients each with its own "
            "denominator (distinct from rung-53 sum-of-three-over-one (a+b+c)/d and rung-37 ratio-of-two-sums "
            "(a+b)/(c+d)) — and the harness matches the scalar to the printed options. Contamination-safe: every index "
            "is built only from the four observed quantities via / and + — no constant leaks, and neither the left "
            "rate, the right rate, nor any total-rate figure ever appears as a literal (each is computed) — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same four quantities, so the distractors are exactly the slips students "
            "make: POOLING the counts over the pooled seconds ((a+c)/(b+d), a ratio of totals, not a+ b of ratios), "
            "and SWAPPING the denominators (a/d + c/b). The core confusion tested is that a/b + c/d is not "
            "(a+c)/(b+d)."
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
