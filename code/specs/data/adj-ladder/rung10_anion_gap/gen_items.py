"""Generate rung-10 (serum anion gap) items.json for the ADJ-LADDER.

Rung 10 stays in the quantitative-clinical band (biostat 7/7b/7c, pharmacokinetics 8, renal indices
9) and keeps the identical contamination-safe shape: a small table of *observed* quantities and a
tight family of mutually-confusable formulas. Where rungs 8/9 used pure multiplication/division,
rung 10 is the **pure addition/subtraction** case — the serum anion gap — so the lever this rung
adds to the band is signed arithmetic with no numeric literal anywhere.

The clinical setting is the bedside acid–base work-up. From a basic metabolic panel we observe four
serum electrolytes:

  Na      sodium        (mEq/L)
  K       potassium     (mEq/L)
  Cl      chloride      (mEq/L)
  bicarb  bicarbonate   (mEq/L)   [HCO3-]

From these the anion gap falls out as a pure signed sum of the observed quantities — exact, and
needing no constant (the textbook gap carries no multiplier; the "normal" reference value ~12 is a
comparison, not part of the computation, so nothing leaks):

  AG    (serum anion gap)                 = Na - Cl - bicarb
  AG+K  (anion gap including potassium)   = Na + K - Cl - bicarb

Each is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the
ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 4/7/7b/7c/8/9.

Contamination-safe by construction: every formula is built only from the four observed quantities
via `+`/`-` — **no structural constants** — so every program literal is grounded in the stem. The
variable names are deliberately **digit-free** (`bicarb`, not `hco3`): the contamination check reads
digit-runs out of identifiers, so a `3` inside a variable name would leak as the literal `3` (the
rung-8 lesson). The five options are a tight family of signed sums over the same quantities: the two
real gaps {AG, AG+K} plus the three classic slips {bicarbonate sign flipped, chloride sign flipped,
bicarbonate omitted}. The distractors are therefore exactly the sign-errors a student makes. Gold
rotates A–E by index.

Note on table choice: the five family values are pairwise distinct for every physiologic table below
(potassium is far smaller than bicarbonate or chloride, so the offsets never coincide), and this is
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (Na, K, Cl, bicarb) observed serum electrolytes (mEq/L). Physiologic-ish ranges, varied so the
# five anion-gap family values are pairwise distinct for every row (asserted below).
TABLES = [
    (140, 4, 104, 24),
    (135, 5, 100, 20),
    (145, 3, 110, 30),
    (138, 4, 100, 14),
    (142, 5, 96, 18),
    (130, 3, 95, 22),
    (136, 4, 108, 16),
    (144, 5, 105, 28),
    (139, 3, 99, 26),
    (141, 4, 112, 20),
    (133, 5, 98, 24),
    (137, 3, 102, 19),
]

# The option family (5 members), all addition/subtraction over the observed quantities
# na/k/cl/bicarb.  key -> (display name, formula-as-adj)
# Only the first two are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("ag", "serum anion gap", "na - cl - bicarb"),
    ("ag_k", "serum anion gap including potassium", "na + k - cl - bicarb"),
    ("ag_bicarb_slip", "anion gap with the bicarbonate sign flipped", "na - cl + bicarb"),
    ("ag_cl_slip", "anion gap with the chloride sign flipped", "na + cl - bicarb"),
    ("ag_no_bicarb", "sodium minus chloride only (bicarbonate omitted)", "na - cl"),
]
QUERIED = ["ag", "ag_k"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(na, k, cl, bicarb):
    return {
        "ag": na - cl - bicarb,
        "ag_k": na + k - cl - bicarb,
        "ag_bicarb_slip": na - cl + bicarb,
        "ag_cl_slip": na + cl - bicarb,
        "ag_no_bicarb": na - cl,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for na, k, cl, bicarb in TABLES:
        fv = family_values(na, k, cl, bicarb)
        assert len({fv[key] for key in ORDER}) == 5, (na, k, cl, bicarb, fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[o] for o in ORDER if fv[o] != gold_val]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if opts_vals[gold_pos] != gold_val:
                opts_vals[gold_pos] = gold_val
            assert len(set(opts_vals)) == 5, (key, na, k, cl, bicarb, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r10ag-{idx + 1:02d}",
                "qtype": "anion_gap",
                "stem": (
                    f"A patient's basic metabolic panel shows a sodium of {na} mEq/L, a potassium of "
                    f"{k} mEq/L, a chloride of {cl} mEq/L, and a bicarbonate (HCO3-) of {bicarb} "
                    f"mEq/L. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe na({na})\n"
                    f"observe k({k})\n"
                    f"observe cl({cl})\n"
                    f"observe bicarb({bicarb})\n"
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
            "ADJ-LADDER rung 10 — the serum anion gap from a basic metabolic panel (bedside acid-base "
            "work-up). From four stated electrolytes (Na, K, Cl, bicarbonate) compute the anion gap "
            "(AG = Na - Cl - bicarb) or the anion gap including potassium (Na + K - Cl - bicarb). Each "
            "item is a compute_dimensioned program (observe the four quantities, let answer = formula); "
            "the ADJ engine carries the signed arithmetic and the harness matches the scalar to the "
            "printed options. This is the pure addition/subtraction case of the quantitative-clinical "
            "band (rungs 8/9 were multiplication/division). Contamination-safe: every gap is a pure "
            "signed sum of the four observed quantities — no constant leaks (the 'normal' gap of ~12 is "
            "a comparison, not part of the computation), and the variable names are digit-free so no "
            "identifier leaks a literal. The five options are a family of signed sums over the same "
            "quantities, so the distractors are exactly the sign-errors students make (flipping the "
            "bicarbonate or chloride sign; omitting bicarbonate)."
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
              "=", it["options"][it["gold_letter"]]["value"])
