"""Generate rung-64 (gastric-acid buffer index) items.json for the ADJ-LADDER.

Rung 64 opens the **gastroenterology / gastric-acid** panel on the quantitative band — the arithmetic of acid
buffering. A stimulation test raises acid output above its resting level: the net acid the stomach adds is the
stimulated output MINUS the basal output. That acid is met by the mucosal buffers, whose total capacity is the
bicarbonate secretion PLUS the mucus buffer. The buffer index is the net acid over the total buffer. Dividing the
DIFFERENCE of two quantities by the SUM of two others introduces a genuinely NEW arithmetic shape on the ladder: a
**difference over a sum** — `(a-b)/(c+d)`.

The setup: a `stimulated_output` of acid after stimulation, a `basal_output` at rest, buffered by a `bicarbonate`
secretion and a `mucus_buffer`. The gastric-acid buffer index is:

  BUFFER INDEX     (stimulated_output - basal_output) / (bicarbonate + mucus_buffer)   [ net acid per unit buffer ]
  NET ACID         stimulated_output - basal_output                                    [ the numerator: acid added ]
  TOTAL BUFFER     bicarbonate + mucus_buffer                                          [ the denominator: buffering ]

The **buffer index** is what makes this rung distinctive — it is the ladder's first **difference over a sum**: a
difference of two quantities divided by a sum of two others. Contrast the neighbours already on the ladder: rung-63 was
`(a+b)/(c-d)` (a SUM over a difference — the mirror image), rung-59 was `(a*b)/(c-d)` (a PRODUCT over a difference) and
rung-32 was `(a-b)/(c-d)` (a difference over a DIFFERENCE); here a difference sits over a SUM. (The net acid
`stimulated_output-basal_output` and the total buffer `bicarbonate+mucus_buffer` ride alongside as component readouts,
so the panel teaches the whole calculation — exactly as rungs 47-63 shipped their component sums/products/differences/
ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the numerator difference, the parenthesised denominator sum, and their quotient — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../62/63. This rung exercises the engine across **a division of a difference by a sum** — the fact that
`(a-b)/(c+d)` is NOT `(a+b)/(c+d)` and NOT `(a-b)/(c-d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `-`, and `+`
— **no structural constants** — so no numeric literal appears in any program, and neither the net acid, the total
buffer, nor any buffer-index figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`stimulated_output`, `basal_output`, `bicarbonate`, `mucus_buffer`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  POOLED     (stimulated_output + basal_output) / (bicarbonate + mucus_buffer)   SUM the numerator instead of
                                                                                DIFFERENCING it (the classic
                                                                                `(a-b)/(c+d)` vs `(a+b)/(c+d)` error), and
  CROSSED    (stimulated_output - basal_output) / (bicarbonate - mucus_buffer)   DIFFERENCE the denominator instead of
                                                                                SUMMING it (subtract the two buffers),

which are exactly the mistakes a student makes (adding the two acid outputs, or subtracting the two buffers). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness: all four observed quantities are strictly positive, the tables are chosen so the stimulated output exceeds
the basal output (the net acid — and therefore the buffer index — is positive) and the bicarbonate exceeds the mucus
buffer (so the difference-denominator distractor stays positive); the five family values are pairwise distinct with a
comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (STIMULATED_OUTPUT, BASAL_OUTPUT, BICARBONATE, MUCUS_BUFFER) — two acid outputs and two buffer figures, all plain
# positive numbers with stimulated > basal and bicarbonate > mucus_buffer. The five family values are asserted
# pairwise-distinct (with margin) below.
TABLES = [
    (60, 20, 50, 10),
    (80, 40, 60, 20),
    (90, 30, 40, 10),
    (70, 50, 80, 40),
    (100, 20, 30, 10),
    (50, 30, 70, 30),
    (120, 40, 90, 50),
]

# The option family (5 members), all built from the four observed quantities via /, -, and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "buffer_index",
        "gastric-acid buffer index (net acid over the total buffer)",
        "(stimulated_output - basal_output) / (bicarbonate + mucus_buffer)",
    ),
    (
        "net_acid",
        "the net acid added by stimulation (stimulated minus basal output)",
        "stimulated_output - basal_output",
    ),
    (
        "total_buffer",
        "the total buffering capacity (bicarbonate plus mucus buffer)",
        "bicarbonate + mucus_buffer",
    ),
    (
        "pooled",
        "the SUM of the two acid outputs over the total buffer, not their difference (a wrong net acid)",
        "(stimulated_output + basal_output) / (bicarbonate + mucus_buffer)",
    ),
    (
        "crossed",
        "the net acid over the DIFFERENCE of the two buffers, not their sum (a wrong total buffer)",
        "(stimulated_output - basal_output) / (bicarbonate - mucus_buffer)",
    ),
]
QUERIED = ["buffer_index", "net_acid", "total_buffer"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(stimulated_output, basal_output, bicarbonate, mucus_buffer):
    # Operation order mirrors the ADJ programs exactly (each parenthesised difference/sum formed first, then the
    # division), so the Python option value and the engine result are the same IEEE-double (well within the harness's
    # 1e-9 match tolerance).
    return {
        "buffer_index": (stimulated_output - basal_output) / (bicarbonate + mucus_buffer),
        "net_acid": stimulated_output - basal_output,
        "total_buffer": bicarbonate + mucus_buffer,
        "pooled": (stimulated_output + basal_output) / (bicarbonate + mucus_buffer),
        "crossed": (stimulated_output - basal_output) / (bicarbonate - mucus_buffer),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for stimulated_output, basal_output, bicarbonate, mucus_buffer in TABLES:
        assert (
            stimulated_output > 0
            and basal_output > 0
            and bicarbonate > 0
            and mucus_buffer > 0
        ), (stimulated_output, basal_output, bicarbonate, mucus_buffer)
        # Net acid must be positive (numerator) and the bicarbonate must exceed the mucus buffer
        # (so the difference-denominator distractor is positive).
        assert stimulated_output > basal_output, (stimulated_output, basal_output, bicarbonate, mucus_buffer)
        assert bicarbonate > mucus_buffer, (stimulated_output, basal_output, bicarbonate, mucus_buffer)
        fv = family_values(stimulated_output, basal_output, bicarbonate, mucus_buffer)
        for key, v in fv.items():
            assert v > 0, (key, stimulated_output, basal_output, bicarbonate, mucus_buffer, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    stimulated_output,
                    basal_output,
                    bicarbonate,
                    mucus_buffer,
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
                stimulated_output,
                basal_output,
                bicarbonate,
                mucus_buffer,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r64acid-{idx + 1:02d}",
                "qtype": "gastric_acid_buffer",
                "stem": (
                    f"An acid-secretion study measures a stimulated output of {num(stimulated_output)} units and a "
                    f"basal output of {num(basal_output)}, buffered by {num(bicarbonate)} of bicarbonate and a "
                    f"{num(mucus_buffer)} mucus buffer. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe stimulated_output({num(stimulated_output)})\n"
                    f"observe basal_output({num(basal_output)})\n"
                    f"observe bicarbonate({num(bicarbonate)})\n"
                    f"observe mucus_buffer({num(mucus_buffer)})\n"
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
            "ADJ-LADDER rung 64 — gastric-acid buffer index from four stated quantities (a NEW panel: gastroenterology "
            "/ gastric-acid). From a stimulated and a basal acid output (their difference is the net acid) and a "
            "bicarbonate secretion plus a mucus buffer (their sum is the total buffer), compute the buffer index "
            "((stimulated_output-basal_output)/(bicarbonate+mucus_buffer)), the net acid "
            "(stimulated_output-basal_output), or the total buffer (bicarbonate+mucus_buffer). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW shape, DIFFERENCE OVER A SUM (a-b)/(c+d), the first on the ladder to divide a "
            "difference by a sum (the mirror of rung-63 sum-over-difference (a+b)/(c-d), and distinct from rung-59 "
            "product-over-difference (a*b)/(c-d) and rung-32 difference-over-difference (a-b)/(c-d)) — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every index is built only from the four "
            "observed quantities via /, -, and + — no constant leaks, and neither the net acid, the total buffer, nor "
            "any buffer-index figure ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the "
            "same four quantities, so the distractors are exactly the slips students make: SUMMING the numerator "
            "((a+b)/(c+d), a wrong net acid) and DIFFERENCING the denominator ((a-b)/(c-d), a wrong total buffer). The "
            "core confusion tested is that (a-b)/(c+d) is not (a+b)/(c+d) and not (a-b)/(c-d)."
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
