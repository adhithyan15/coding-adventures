"""Generate rung-39 (CBC leukocyte differential total) items.json for the ADJ-LADDER.

Rung 39 opens the **hematology / complete-blood-count differential** panel on the quantitative band — the
arithmetic of adding up the absolute counts of the white-cell populations. When the lab reports a differential it
gives an absolute count (cells per microliter) for each leukocyte type; summing the major populations recovers the
combined count. It uses the same contamination-safe shape as the anion-gap rung (38), the CSF:serum rung (37) and
the transfusion-pooling rung (36): a small table of *observed* absolute counts and a tight family of
mutually-confusable formulas built **only from those observed quantities** (no numeric literal anywhere in any
program), so nothing structural can leak.

The clinical setup is a leukocyte differential. FOUR populations are measured — all as absolute counts in
cells/uL:

  NEUTROPHILS    the neutrophil absolute count       (usually the largest population)
  LYMPHOCYTES    the lymphocyte absolute count        (the second-largest)
  MONOCYTES      the monocyte absolute count          (a minor population)
  EOSINOPHILS    the eosinophil absolute count        (a minor population — the odd one out here)

The combined count of the three major populations is **their sum** — a *three-term sum* —
`NEUTROPHILS + LYMPHOCYTES + MONOCYTES`. That is what makes this rung distinctive: it is a NEW arithmetic shape on
the ladder — a flat sum of THREE observed terms, `a + b + c`, all addition. The nearest prior rung (38, serum
anion gap) chained two SUBTRACTIONS across three terms (`a - b - c`); every other prior rung composed exactly two
sub-expressions (rung-36 weighted-average, rung-37 ratio-of-two-sums). This rung is the first pure three-term
ADDITION. The core confusion this rung tests is adding the three MAJOR populations (and no others), rather than
dropping one, adding the wrong pair, or sweeping in the eosinophils too:

  MAJOR SUM       NEUTROPHILS + LYMPHOCYTES + MONOCYTES   [ the three major populations summed ]
  NEU + LYM       NEUTROPHILS + LYMPHOCYTES               [ only the two largest (a partial sum) ]
  LYM + MON       LYMPHOCYTES + MONOCYTES                 [ the two non-neutrophil majors (a partial sum) ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/37/38. This rung exercises the engine across a
flat THREE-TERM ADDITION (`(neutrophils + lymphocytes) + monocytes`).

Contamination-safe by construction: every formula is built only from the four observed quantities via `+` —
**no structural constants** — so every program literal is grounded in the stem. No total ever appears as a
literal (each is computed from the observed counts). The observed quantities carry **digit-free identifiers**
(`neutrophils`, `lymphocytes`, `monocytes`, `eosinophils`) so no numeral hides inside a variable name. The five
options are a tight family over the same quantities: the three real sums plus the two classic slips —

  ALL FOUR       NEUTROPHILS + LYMPHOCYTES + MONOCYTES + EOSINOPHILS   the eosinophils swept in too (a four-term sum), and
  NEU + EOS      NEUTROPHILS + EOSINOPHILS                             a wrong pair (a major plus the odd minor),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the major sum is order 6000-8000 cells/uL, the neutrophil+lymphocyte partial is order 6000-7200,
the lymphocyte+monocyte partial is order 1500-3800, the all-four distractor is the major sum plus the eosinophils
(a few hundred more), and the neutrophil+eosinophil distractor is order 3500-6500; the tables below are chosen so
the five family values are pairwise distinct — with a comfortable margin — for every item, asserted at build time
(all four counts positive so every sum is positive, and no two family values collide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (NEUTROPHILS, LYMPHOCYTES, MONOCYTES, EOSINOPHILS) observed absolute counts, all in cells/uL. Neutrophils are the
# largest population, lymphocytes second, monocytes and eosinophils minor. All four are strictly positive, so every
# sum is positive. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (4000, 2000, 500, 150),
    (5500, 1500, 700, 300),
    (3200, 2800, 400, 250),
    (6000, 1200, 300, 450),
    (4800, 2400, 600, 100),
    (3600, 2600, 900, 150),
    (5000, 1000, 450, 350),
]

# The option family (5 members), all built from the observed quantities via `+`. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear
# as the options.
FAMILY = [
    (
        "major_sum",
        "combined count of the three major populations (neutrophils, lymphocytes, monocytes)",
        "neutrophils + lymphocytes + monocytes",
    ),
    (
        "neu_plus_lym",
        "combined count of neutrophils and lymphocytes only",
        "neutrophils + lymphocytes",
    ),
    (
        "lym_plus_mon",
        "combined count of lymphocytes and monocytes only",
        "lymphocytes + monocytes",
    ),
    (
        "all_four",
        "count with the eosinophils swept in too (all four populations)",
        "neutrophils + lymphocytes + monocytes + eosinophils",
    ),
    (
        "neu_plus_eos",
        "wrong pair (a major population plus the odd minor one)",
        "neutrophils + eosinophils",
    ),
]
QUERIED = ["major_sum", "neu_plus_lym", "lym_plus_mon"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(neu, lym, mon, eos):
    # Operation order mirrors the ADJ program exactly (a left-folded sum: (neu + lym) + mon), so the Python option
    # value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "major_sum": neu + lym + mon,
        "neu_plus_lym": neu + lym,
        "lym_plus_mon": lym + mon,
        "all_four": neu + lym + mon + eos,
        "neu_plus_eos": neu + eos,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for neu, lym, mon, eos in TABLES:
        assert neu > 0 and lym > 0 and mon > 0 and eos > 0, (neu, lym, mon, eos)
        fv = family_values(neu, lym, mon, eos)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (neu, lym, mon, eos, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, neu, lym, mon, eos, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r39wbc-{idx + 1:02d}",
                "qtype": "wbc_differential",
                "stem": (
                    f"A complete-blood-count differential reports absolute counts of {num(neu)} neutrophils, "
                    f"{num(lym)} lymphocytes, {num(mon)} monocytes, and {num(eos)} eosinophils (all in cells/uL). "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe neutrophils({num(neu)})\n"
                    f"observe lymphocytes({num(lym)})\n"
                    f"observe monocytes({num(mon)})\n"
                    f"observe eosinophils({num(eos)})\n"
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
            "ADJ-LADDER rung 39 — CBC leukocyte differential totals from four stated absolute counts (a NEW panel: "
            "hematology / complete-blood-count differential). From four stated counts (neutrophils, lymphocytes, "
            "monocytes, eosinophils) compute the major sum (neutrophils+lymphocytes+monocytes), the "
            "neutrophil+lymphocyte partial (neutrophils+lymphocytes), or the lymphocyte+monocyte partial "
            "(lymphocytes+monocytes). Each item is a compute_dimensioned program (observe the four quantities, "
            "let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, a flat THREE-TERM SUM "
            "(neutrophils+lymphocytes+monocytes), all addition (the nearest prior rung, 38, chained two "
            "subtractions a-b-c; this is the first pure three-term addition) — and the harness matches the scalar "
            "to the printed options. Contamination-safe: every index is built only from the four observed counts "
            "via + — no constant leaks (a differential total is a pure sum), and no total ever appears as a literal "
            "(each is computed from the observed counts) — and the observed quantities carry digit-free identifiers "
            "so no numeral hides inside a variable name. The five options are a family over the same quantities, so "
            "the distractors are exactly the slips students make: sweeping in the eosinophils too "
            "(neutrophils+lymphocytes+monocytes+eosinophils, a four-term sum) and a wrong pair "
            "(neutrophils+eosinophils, a major plus the odd minor). The core confusion tested is adding the three "
            "MAJOR populations and no others."
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
