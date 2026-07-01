"""Generate rung-22 (thyroid function indices) items.json for the ADJ-LADDER.

Rung 22 opens the **thyroid / endocrine** panel on the quantitative band, using the same
contamination-safe shape as the pharmacokinetics rung (8), the renal-ratio rungs (9/13/15/21), the
cardiology rungs (16/18), the hematology rung (19), and the hepatology rung (20): a small table of
*observed* laboratory quantities, and a tight family of mutually-confusable formulas built **only
from those observed quantities** (no numeric literal anywhere in any program), so nothing structural
can leak.

The clinical setup is a single thyroid function panel. Four quantities are measured — each hormone
in both its **total** (protein-bound + free) and **free** (metabolically active) form:

  TT4  total thyroxine        (µg/dL)  — total T4, mostly TBG-bound
  FT4  free thyroxine         (ng/dL)  — the metabolically active T4
  TT3  total triiodothyronine (ng/dL)  — total T3
  FT3  free triiodothyronine  (pg/mL)  — the metabolically active T3

From those four, three textbook thyroid indices fall out as **pure ratios** of the observed
quantities — no constant required:

  TOTAL T4:T3 RATIO   total T4 / total T3   = TT4 / TT3   [ shifts with deiodination / conversion state ]
  FREE  T4:T3 RATIO   free  T4 / free  T3   = FT4 / FT3   [ high in T4-predominant states, amiodarone ]
  FREE FRACTION OF T4 free T4 / total T4    = FT4 / TT4   [ rises when binding proteins (TBG) fall ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`);
the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/18/19/20/21. Like
the renal rung this family is **all division** (no inner sum) — a pure-ratio rung on a new endocrine
panel — so it exercises the engine's `/` across a fresh clinical stem.

Contamination-safe by construction: every formula is built only from the four observed quantities via
`/` — **no structural constants** — so every program literal is grounded in the stem. The observed
quantities are given **digit-free identifiers** (`total_thyroxine`, `free_thyroxine`,
`total_triiodo`, `free_triiodo`) so that no digit hides inside a variable name (the T4/T3 labels live
only in the prose stem, never in the program). The five options are a tight family over the same
quantities: the three real indices {total T4:T3, free T4:T3, free-fraction-T4} plus the two classic
slips —

  FT3 / TT3   the free fraction of *T3* (confused with the free fraction of T4), and
  TT3 / TT4   the **inverse** total T4:T3 ratio (the ratio written upside-down),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on table choice: the five family values can collide for special quantity ratios. The tables
below are chosen so the five family values are pairwise distinct — with a comfortable margin — for
every item, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TT4, FT4, TT3, FT3) observed quantities. The five index-family values are asserted
# pairwise-distinct (with margin) below.
#   TT4 = total thyroxine        (µg/dL)
#   FT4 = free thyroxine         (ng/dL)
#   TT3 = total triiodothyronine (ng/dL)
#   FT3 = free triiodothyronine  (pg/mL)
TABLES = [
    (8, 2, 100, 4),
    (10, 1, 120, 3),
    (6, 2, 90, 3),
    (12, 3, 150, 5),
    (9, 1, 180, 6),
    (5, 2, 80, 4),
    (10, 2, 100, 5),
]

# The option family (5 members), all built from the observed quantities via `/`. Every identifier is
# DIGIT-FREE so no numeral can hide inside a variable name. key -> (display name, formula-as-adj).
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("t4_t3_total", "total T4-to-T3 ratio", "total_thyroxine / total_triiodo"),
    ("t4_t3_free", "free T4-to-T3 ratio", "free_thyroxine / free_triiodo"),
    ("free_frac_t4", "free fraction of T4", "free_thyroxine / total_thyroxine"),
    ("free_frac_t3", "free fraction of T3", "free_triiodo / total_triiodo"),
    ("t3_t4_total", "total T3-to-T4 ratio", "total_triiodo / total_thyroxine"),
]
QUERIED = ["t4_t3_total", "t4_t3_free", "free_frac_t4"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(tt4, ft4, tt3, ft3):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "t4_t3_total": tt4 / tt3,
        "t4_t3_free": ft4 / ft3,
        "free_frac_t4": ft4 / tt4,
        "free_frac_t3": ft3 / tt3,
        "t3_t4_total": tt3 / tt4,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for tt4, ft4, tt3, ft3 in TABLES:
        fv = family_values(tt4, ft4, tt3, ft3)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (tt4, ft4, tt3, ft3, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, tt4, ft4, tt3, ft3, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r22thy-{idx + 1:02d}",
                "qtype": "thyroid_index",
                "stem": (
                    f"A thyroid function panel shows total T4 (thyroxine) {tt4} ug/dL, free T4 {ft4} "
                    f"ng/dL, total T3 (triiodothyronine) {tt3} ng/dL, and free T3 {ft3} pg/mL. What is "
                    f"the patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe total_thyroxine({tt4})\n"
                    f"observe free_thyroxine({ft4})\n"
                    f"observe total_triiodo({tt3})\n"
                    f"observe free_triiodo({ft3})\n"
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
            "ADJ-LADDER rung 22 — thyroid function indices from a single thyroid panel (a NEW organ "
            "system: endocrine/thyroid on the quantitative band). From four stated quantities (total "
            "thyroxine TT4, free thyroxine FT4, total triiodothyronine TT3, free triiodothyronine FT3) "
            "compute the total T4:T3 ratio (TT4/TT3), the free T4:T3 ratio (FT4/FT3), or the free "
            "fraction of T4 (FT4/TT4). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic (pure division) "
            "and the harness matches the scalar to the printed options. Contamination-safe: every index "
            "is built only from the four observed quantities via / — no constant leaks — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same quantities, so the distractors are "
            "exactly the slips students make: the free fraction of T3 (FT3/TT3, confused with the free "
            "fraction of T4) and the inverse total ratio (TT3/TT4, written upside-down)."
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
              "=", round(it["options"][it["gold_letter"]]["value"], 4))
