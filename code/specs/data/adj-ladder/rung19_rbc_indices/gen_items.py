"""Generate rung-19 (red-blood-cell indices) items.json for the ADJ-LADDER.

Rung 19 opens a **new organ system** on the quantitative band — **hematology** — using the same
contamination-safe shape the pharmacokinetics rung (rung 8), the renal-ratio rungs (9/13/15), and
the cardiology rungs (16 cardiac output, 18 ejection fraction) were built on: a small table of
*observed* laboratory quantities, and a tight family of mutually-confusable formulas built **only
from those observed quantities** (no numeric literal anywhere in any program), so nothing
structural can leak into the answer.

The clinical setup is a single complete-blood-count (CBC) read-out. We observe three quantities:

  HCT  hematocrit          (%)              — packed red-cell volume fraction
  RBC  red-cell count      (millions/uL)    — number of red cells per unit blood
  HGB  hemoglobin          (g/dL)           — oxygen-carrying pigment concentration

From those three, the three textbook red-cell indices fall out as **pure ratios** of the observed
quantities — no constant required:

  MCV   mean corpuscular volume            = HCT / RBC     [ how big each red cell is    ]
  MCH   mean corpuscular hemoglobin        = HGB / RBC     [ hemoglobin mass per red cell ]
  MCHC  mean corpuscular hemoglobin conc.  = HGB / HCT     [ hemoglobin per packed volume ]

(The bedside indices carry a ×10 unit-conversion factor — MCV = HCT×10/RBC etc. — but that factor
is a *constant* that would leak a literal into the program and defeat contamination-safety. So this
rung uses the constant-free **pure-ratio** form: each index is exactly one observed quantity divided
by another. The three ratios are still mutually distinct and still exercise the same "which lab over
which lab" reasoning the real indices require.)

Each index is a `compute_dimensioned` program (observe the three quantities + `let answer =
formula`); the ADJ engine carries the division and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/18.

Contamination-safe by construction: every formula is built only from the three observed quantities
via `/` — **no structural constants** — so every program literal is grounded in the stem. The five
options are a tight family over the same three quantities: the three real indices {MCV, MCH, MCHC}
plus the two classic slips —

  RBC / HCT   the **inverse** of MCV (dividing the count by the volume fraction, the ratio flipped), and
  RBC / HGB   the **inverse** of MCH (dividing the count by the hemoglobin, the ratio flipped),

which are exactly the mistakes a student makes — writing the index ratio upside-down. Gold rotates
A–E by index.

Note on table choice: the five family values can collide for special quantity ratios. The tables
below are chosen so the five family values are pairwise distinct — with a comfortable margin — for
every item, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (HCT, RBC, HGB) observed quantities. The five index-family values are asserted
# pairwise-distinct (with margin) below.
#   HCT = hematocrit      (%)
#   RBC = red-cell count  (millions/uL)
#   HGB = hemoglobin      (g/dL)
TABLES = [
    (44, 5.0, 15),
    (30, 3.4, 10),
    (51, 6.2, 17),
    (36, 4.1, 12),
    (27, 3.1, 9),
    (48, 5.4, 16),
    (33, 3.6, 11),
]

# The option family (5 members), all built from the observed quantities hct/rbc/hgb via `/`.
#   key -> (display name, formula-as-adj)
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("mcv", "mean corpuscular volume (MCV)", "hct / rbc"),
    ("mch", "mean corpuscular hemoglobin (MCH)", "hgb / rbc"),
    ("mchc", "mean corpuscular hemoglobin concentration (MCHC)", "hgb / hct"),
    ("rbc_hct", "red-cell count divided by hematocrit", "rbc / hct"),
    ("rbc_hgb", "red-cell count divided by hemoglobin", "rbc / hgb"),
]
QUERIED = ["mcv", "mch", "mchc"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(hct, rbc, hgb):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "mcv": hct / rbc,
        "mch": hgb / rbc,
        "mchc": hgb / hct,
        "rbc_hct": rbc / hct,
        "rbc_hgb": rbc / hgb,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for hct, rbc, hgb in TABLES:
        fv = family_values(hct, rbc, hgb)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (hct, rbc, hgb, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, hct, rbc, hgb, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r19rbc-{idx + 1:02d}",
                "qtype": "red_cell_index",
                "stem": (
                    f"A complete blood count shows a hematocrit of {hct} %, a red-cell count of "
                    f"{rbc} million/uL, and a hemoglobin of {hgb} g/dL. What is the patient's "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe hct({hct})\n"
                    f"observe rbc({rbc})\n"
                    f"observe hgb({hgb})\n"
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
            "ADJ-LADDER rung 19 — red-blood-cell indices from a single complete-blood-count (a NEW "
            "organ system: hematology). From three stated quantities (hematocrit HCT, red-cell count "
            "RBC, hemoglobin HGB) compute the mean corpuscular volume (MCV = HCT/RBC), mean "
            "corpuscular hemoglobin (MCH = HGB/RBC), or mean corpuscular hemoglobin concentration "
            "(MCHC = HGB/HCT). Each item is a compute_dimensioned program (observe the three "
            "quantities, let answer = formula); the ADJ engine carries the division and the harness "
            "matches the scalar to the printed options. Contamination-safe: every index is built only "
            "from the three observed quantities via / — no constant leaks (the bedside ×10 factor is "
            "dropped so the ratios stay literal-free). The five options are a family over the same "
            "quantities, so the distractors are exactly the slips students make: writing the index "
            "ratio upside-down (RBC/HCT, the inverse of MCV; RBC/HGB, the inverse of MCH)."
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
