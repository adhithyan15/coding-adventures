"""Generate rung-42 (hemofiltration concentration after fluid removal) items.json for the ADJ-LADDER.

Rung 42 opens the **renal-replacement / hemofiltration** panel on the quantitative band — the arithmetic of what a
solute's concentration becomes after some plasma water is removed. When a volume of fluid is ultrafiltered off, the
solute that stays behind is now dissolved in a SMALLER volume, so its concentration rises. This rung introduces a
genuinely NEW arithmetic shape on the ladder: **sum-over-difference** — `(a + b) / (c - d)` — a sum in the numerator
divided by a difference in the denominator.

The setup: two solute pools are pooled (`solute_a`, `solute_b`, in mmol) in a starting volume `initial_volume`
(L), and `removed_volume` (L) of fluid is ultrafiltered off. The post-filtration concentration is the total solute
divided by the volume that REMAINS:

  POST CONCENTRATION   (solute_a + solute_b) / (initial_volume - removed_volume)   [ the concentration after removal ]
  TOTAL SOLUTE         solute_a + solute_b                                          [ the numerator: pooled solute ]
  REMAINING VOLUME     initial_volume - removed_volume                             [ the denominator: what is left ]

The **post concentration** is what makes this rung distinctive — it is the ladder's first **sum-over-difference**:
a parenthesised sum divided by a parenthesised difference. Contrast the neighbours already on the ladder: rung-37
was a *ratio of two sums* `(a+b)/(c+d)`, rung-41 a *difference-over-sum* `(a-b)/(a+b)`; none divided a SUM by a
DIFFERENCE. (The total solute `solute_a + solute_b` and the remaining volume `initial_volume - removed_volume` ride
alongside as the two component quantities, so the panel teaches the whole calculation — exactly as rung-37 shipped
its two component sums and rung-41 its two split fractions beside the headline quotient.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — including the inner `(solute_a + solute_b)` sum and the `(initial_volume -
removed_volume)` difference — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../40/41. This rung exercises the engine across a **sum divided by a
difference**.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-` and
`/` — **no structural constants** — so no numeric literal appears in any program, and neither the total, the
remaining volume, nor any concentration is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`solute_a`, `solute_b`, `initial_volume`, `removed_volume`)
so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic
slips —

  PRE CONCENTRATION   (solute_a + solute_b) / initial_volume                       divide by the STARTING volume
                                                                                    instead of the remaining one, and
  CROSSED             (solute_a + solute_b) / (initial_volume + removed_volume)     ADD the removed volume instead of
                                                                                    subtracting it,

which are exactly the mistakes a student makes (forgetting the volume shrank, or getting the sign of the volume
change wrong). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear
as options.

Distinctness: the tables below are chosen with `initial_volume > removed_volume` (so the remaining volume — and
every denominator — is strictly positive, no division by zero, and the post concentration exceeds the pre
concentration as it physically must) and all four quantities positive; the five family values are asserted
pairwise-distinct with a comfortable margin at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (SOLUTE_A, SOLUTE_B, INITIAL_VOLUME, REMOVED_VOLUME) — solutes in mmol, volumes in L. INITIAL_VOLUME >
# REMOVED_VOLUME > 0 so the remaining volume (every denominator) is strictly positive. The five family values are
# asserted pairwise-distinct (with margin) below.
TABLES = [
    (30, 20, 5, 1),
    (24, 16, 6, 2),
    (28, 12, 5, 3),
    (45, 15, 8, 4),
    (18, 22, 4, 1),
    (36, 24, 7, 2),
    (40, 20, 6, 1),
]

# The option family (5 members), all built from the four observed quantities via +, - and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "post_concentration",
        "concentration after fluid removal (total solute over the remaining volume)",
        "(solute_a + solute_b) / (initial_volume - removed_volume)",
    ),
    (
        "total_solute",
        "total pooled solute (the two amounts added)",
        "solute_a + solute_b",
    ),
    (
        "remaining_volume",
        "the volume remaining after removal (initial minus removed)",
        "initial_volume - removed_volume",
    ),
    (
        "pre_concentration",
        "concentration over the STARTING volume instead of the remaining one (a wrong denominator)",
        "(solute_a + solute_b) / initial_volume",
    ),
    (
        "crossed",
        "total solute over initial PLUS removed volume (the removed volume added instead of subtracted)",
        "(solute_a + solute_b) / (initial_volume + removed_volume)",
    ),
]
QUERIED = ["post_concentration", "total_solute", "remaining_volume"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(solute_a, solute_b, initial_volume, removed_volume):
    # Operation order mirrors the ADJ programs exactly so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    total = solute_a + solute_b
    return {
        "post_concentration": total / (initial_volume - removed_volume),
        "total_solute": total,
        "remaining_volume": initial_volume - removed_volume,
        "pre_concentration": total / initial_volume,
        "crossed": total / (initial_volume + removed_volume),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for solute_a, solute_b, initial_volume, removed_volume in TABLES:
        assert solute_a > 0 and solute_b > 0 and initial_volume > removed_volume > 0, (
            solute_a,
            solute_b,
            initial_volume,
            removed_volume,
        )
        fv = family_values(solute_a, solute_b, initial_volume, removed_volume)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    solute_a,
                    solute_b,
                    initial_volume,
                    removed_volume,
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
                solute_a,
                solute_b,
                initial_volume,
                removed_volume,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r42hf-{idx + 1:02d}",
                "qtype": "hemofiltration_concentration",
                "stem": (
                    f"A hemofiltration circuit pools {num(solute_a)} mmol and {num(solute_b)} mmol of a solute in "
                    f"an initial volume of {num(initial_volume)} L, then ultrafilters off {num(removed_volume)} L "
                    f"of fluid. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe solute_a({num(solute_a)})\n"
                    f"observe solute_b({num(solute_b)})\n"
                    f"observe initial_volume({num(initial_volume)})\n"
                    f"observe removed_volume({num(removed_volume)})\n"
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
            "ADJ-LADDER rung 42 — hemofiltration concentration after fluid removal from four stated quantities (a "
            "NEW panel: renal-replacement / hemofiltration). From two pooled solute amounts and the initial and "
            "removed volumes compute the post-filtration concentration ((solute_a+solute_b)/(initial_volume-"
            "removed_volume)), the total pooled solute (solute_a+solute_b), or the remaining volume (initial_volume-"
            "removed_volume). Each item is a compute_dimensioned program (observe the four quantities, let answer = "
            "formula); the ADJ engine carries the arithmetic — a NEW shape, SUM-OVER-DIFFERENCE (a+b)/(c-d), the "
            "first quotient on the ladder to divide a parenthesised sum by a parenthesised difference (distinct from "
            "rung-37 ratio-of-two-sums (a+b)/(c+d) and rung-41 difference-over-sum (a-b)/(a+b)) — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every index is built only from the four "
            "observed quantities via +, - and / — no constant leaks, and neither the total, the remaining volume, "
            "nor any concentration ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over "
            "the same four quantities, so the distractors are exactly the slips students make: dividing by the "
            "STARTING volume instead of the remaining one, and ADDING the removed volume instead of subtracting it. "
            "The core confusion tested is dividing the pooled solute by the shrunken (remaining) volume."
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
