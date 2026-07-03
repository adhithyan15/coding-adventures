"""Generate rung-41 (split renal function / renography asymmetry) items.json for the ADJ-LADDER.

Rung 41 opens the **nuclear medicine / renography** panel on the quantitative band — the arithmetic of comparing a
PAIR of organs from a radionuclide study. A ^99m^Tc-MAG3 or DMSA renogram reports the *relative uptake* of each
kidney, and the clinician summarises the pair two ways: the **split (differential) function** of each kidney (its
share of the total) and the **asymmetry index** (how lopsided the pair is). This rung introduces a genuinely NEW
arithmetic shape on the ladder: **difference-over-sum** — `(a - b) / (a + b)` — where the SAME two observed
quantities appear once in a subtraction (numerator) and once in an addition (denominator).

The setup: a renogram measures the relative uptake counts of the two kidneys — LEFT and RIGHT (unitless relative
counts). From that pair the study reports:

  ASYMMETRY INDEX   (left - right) / (left + right)   [ the differential asymmetry — how lopsided, signed ]
  LEFT FRACTION     left  / (left + right)            [ the split function of the LEFT kidney (its share) ]
  RIGHT FRACTION    right / (left + right)             [ the split function of the RIGHT kidney (its share) ]

The **asymmetry index** is what makes this rung distinctive — it is the ladder's first **difference-over-sum**:
a difference of the two observed counts divided by their sum. Contrast the neighbours already on the ladder:
rung-37 was a *ratio of two sums* `(a+b)/(c+d)`, rung-31 a *difference of two differences* `(a-b)-(c-d)`; none put
the SAME operand in both a subtraction and an addition of one quotient. (The split functions `left/(left+right)` and
`right/(left+right)` are the familiar share-of-total form; they ride alongside so the panel teaches the whole
renogram readout, exactly as rung-37 shipped its two component sums beside the headline ratio.)

Each index is a `compute_dimensioned` program (`observe` the two counts + `let answer = formula`); the ADJ engine
carries the arithmetic — including the inner `(left + right)` grouping and the numerator `(left - right)` — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as
rungs 8/16/.../39/40. This rung exercises the engine across a **difference divided by a sum of the same operands**.

Contamination-safe by construction: every formula is built ONLY from the two observed counts via `+`, `-` and `/`
— **no structural constants** (the metric asymmetry index and split fractions need none) — so no numeric literal
appears in any program, and neither the sum (left+right) nor any fraction is ever a literal (each is computed from
the observed counts). The observed quantities carry **digit-free identifiers** (`left_kidney`, `right_kidney`) so no
numeral hides inside a variable name.

The five options are a tight family over the same two counts: the three real renogram readouts plus the two classic
slips —

  REVERSED ASYMMETRY   (right - left) / (left + right)   the asymmetry index with the subtraction the WRONG way
                                                          round (a sign error), and
  LEFT:RIGHT RATIO     left / right                       the raw ratio of the two kidneys instead of a
                                                          share-of-total or a normalised difference,

which are exactly the mistakes a student makes. Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness: for any pair with `left != right` and both counts positive, all five family values are provably
pairwise distinct — e.g. `asymmetry_index == left_fraction` would force `right == 0`; `left_fraction ==
left_to_right_ratio` would force `left == 0`; `asymmetry_index == 0` (collision with a zero) only if `left == right`.
The tables below are chosen with `left != right` (so the asymmetry index is never zero) and both counts positive;
the five values are asserted pairwise-distinct with a comfortable margin at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (LEFT, RIGHT) relative uptake counts from the renogram, both strictly positive and DISTINCT (left != right) so the
# asymmetry index is never zero and the split fractions never coincide. Mix of left-dominant and right-dominant
# pairs so the signed asymmetry index takes both signs. The five family values are asserted pairwise-distinct below.
TABLES = [
    (60, 40),
    (55, 45),
    (70, 30),
    (48, 52),
    (64, 36),
    (58, 42),
    (45, 75),
]

# The option family (5 members), all built from the two observed counts via +, - and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "asymmetry_index",
        "differential asymmetry index (the difference of the two kidneys over their sum)",
        "(left_kidney - right_kidney) / (left_kidney + right_kidney)",
    ),
    (
        "left_fraction",
        "split function of the left kidney (its share of the total)",
        "left_kidney / (left_kidney + right_kidney)",
    ),
    (
        "right_fraction",
        "split function of the right kidney (its share of the total)",
        "right_kidney / (left_kidney + right_kidney)",
    ),
    (
        "reversed_asymmetry",
        "the asymmetry index with the subtraction the wrong way round (a sign error)",
        "(right_kidney - left_kidney) / (left_kidney + right_kidney)",
    ),
    (
        "left_to_right_ratio",
        "the raw left-to-right ratio instead of a share-of-total or normalised difference",
        "left_kidney / right_kidney",
    ),
]
QUERIED = ["asymmetry_index", "left_fraction", "right_fraction"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(left, right):
    # Operation order mirrors the ADJ programs exactly so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    total = left + right
    return {
        "asymmetry_index": (left - right) / total,
        "left_fraction": left / total,
        "right_fraction": right / total,
        "reversed_asymmetry": (right - left) / total,
        "left_to_right_ratio": left / right,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for left, right in TABLES:
        assert left > 0 and right > 0 and left != right, (left, right)
        fv = family_values(left, right)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (left, right, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, left, right, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r41srf-{idx + 1:02d}",
                "qtype": "split_renal_function",
                "stem": (
                    f"A renogram measures relative uptake counts of {num(left)} in the left kidney and "
                    f"{num(right)} in the right kidney. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe left_kidney({num(left)})\n"
                    f"observe right_kidney({num(right)})\n"
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
            "ADJ-LADDER rung 41 — split renal function / renography asymmetry from two stated uptake counts (a NEW "
            "panel: nuclear medicine / renography). From the relative uptake counts of the two kidneys (left, right) "
            "compute the differential asymmetry index ((left-right)/(left+right)), the left split function "
            "(left/(left+right)), or the right split function (right/(left+right)). Each item is a "
            "compute_dimensioned program (observe the two counts, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW shape, DIFFERENCE-OVER-SUM (left-right)/(left+right), the first quotient on the "
            "ladder to put the SAME operand in both a subtraction (numerator) and an addition (denominator) — and "
            "the harness matches the scalar to the printed options. Contamination-safe: every index is built only "
            "from the two observed counts via +, - and / — no constant leaks (the metric asymmetry index and split "
            "fractions need none), and neither the sum nor any fraction ever appears as a literal (each is computed "
            "from the observed counts) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same two counts, so the "
            "distractors are exactly the slips students make: the reversed asymmetry index (a sign error) and the "
            "raw left-to-right ratio (instead of a share-of-total or normalised difference). The core confusion "
            "tested is the normalised difference vs the plain share-of-total vs the raw ratio."
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
