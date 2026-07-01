"""Generate rung-24 (oxygen extraction) items.json for the ADJ-LADDER.

Rung 24 opens the **oxygen-transport** panel on the quantitative band — the Fick-principle view of
tissue oxygen delivery and extraction, the bedside physiology of shock, sepsis and cardiac output —
using the same contamination-safe shape as the iron rung (23) and the hepatology rung (20): a small
table of *observed* quantities, and a tight family of mutually-confusable formulas built **only from
those observed quantities** (no numeric literal anywhere in any program), so nothing structural can
leak.

The clinical setup is a single arterial/venous blood-gas oxygen panel. Three quantities are measured:

  ARTERIAL   arterial oxygen content        CaO2  (mL O2/dL)  — oxygen leaving the lungs
  VENOUS     mixed venous oxygen content    CvO2  (mL O2/dL)  — oxygen still in blood after the tissues
  HB         hemoglobin concentration       Hb    (g/dL)      — the oxygen carrier

The **arterio-venous oxygen difference AVDO2 = ARTERIAL - VENOUS** is *derived by subtracting one
observed quantity from another*, never a constant — so the oxygen-extraction family puts a
**difference of observed quantities in the numerator**, a fresh arithmetic shape for the ladder (the
`(a - b)` grouping, like an ejection fraction) that complements the iron rung's sum-in-denominator.
Three textbook oxygen-transport indices fall out as pure functions of the observed quantities — no
constant required:

  OXYGEN EXTRACTION RATIO   (ARTERIAL - VENOUS) / ARTERIAL  [ =O2ER; fraction of delivered O2 extracted ]
  VENOUS FRACTION           VENOUS / ARTERIAL               [ =1 - O2ER; the fraction returned unused ]
  EXTRACTION PER VENOUS      (ARTERIAL - VENOUS) / VENOUS   [ the a-v difference scaled by venous content ]

Each index is a `compute_dimensioned` program (observe the three quantities + `let answer = formula`);
the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/18/19/20/21/22/23.
This exercises the engine across **division AND an inner subtraction-in-parentheses** (AVDO2 =
ARTERIAL - VENOUS) on a fresh oxygen-transport stem — the mirror image of the iron rung's `(a + b)`
denominator.

Contamination-safe by construction: every formula is built only from the three observed quantities via
`/`, `-`, and grouping `( )` — **no structural constants** (AVDO2 is computed, not observed, so no
x-factor appears) — so every program literal is grounded in the stem. The observed quantities carry
**digit-free identifiers** (`arterial_oxygen`, `venous_oxygen`, `hemoglobin`) so no numeral hides
inside a variable name. The five options are a tight family over the same quantities: the three real
indices plus the two classic slips —

  ARTERIAL / VENOUS       the **inverse** venous fraction (written upside-down), and
  ARTERIAL / HEMOGLOBIN   the content-per-gram-Hb ratio (confusing the carried pool with the carrier),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on table choice: the five family values can collide for special quantity ratios. The tables below
are chosen so the five family values are pairwise distinct — with a comfortable margin — for every
item, asserted at build time. Arterial content always exceeds venous content, so the extraction ratio
stays a physiologic fraction in (0, 1).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (ARTERIAL, VENOUS, HB) observed quantities. The five index-family values are asserted
# pairwise-distinct (with margin) below. ARTERIAL > VENOUS for every row (extraction ratio in (0,1)).
#   ARTERIAL = arterial oxygen content     (mL O2/dL)
#   VENOUS   = mixed venous oxygen content (mL O2/dL)
#   HB       = hemoglobin concentration    (g/dL)
TABLES = [
    (20, 15, 10),
    (18, 12, 15),
    (16, 12, 10),
    (21, 14, 15),
    (20, 16, 10),
    (24, 18, 16),
    (19, 12, 14),
]

# The option family (5 members), all built from the observed quantities via `/`, `-`, grouping. Every
# identifier is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried*
# (used as gold); all five always appear as the options.
FAMILY = [
    ("o2er", "oxygen extraction ratio", "(arterial_oxygen - venous_oxygen) / arterial_oxygen"),
    ("venous_frac", "venous fraction of arterial oxygen content", "venous_oxygen / arterial_oxygen"),
    ("ext_per_venous", "arterio-venous oxygen difference relative to venous content",
     "(arterial_oxygen - venous_oxygen) / venous_oxygen"),
    ("arterial_venous", "arterial-to-venous oxygen-content ratio", "arterial_oxygen / venous_oxygen"),
    ("arterial_hb", "arterial oxygen content per gram of hemoglobin", "arterial_oxygen / hemoglobin"),
]
QUERIED = ["o2er", "venous_frac", "ext_per_venous"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(arterial, venous, hemoglobin):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "o2er": (arterial - venous) / arterial,
        "venous_frac": venous / arterial,
        "ext_per_venous": (arterial - venous) / venous,
        "arterial_venous": arterial / venous,
        "arterial_hb": arterial / hemoglobin,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for arterial, venous, hemoglobin in TABLES:
        assert arterial > venous, (arterial, venous)  # extraction ratio must be a fraction in (0, 1)
        fv = family_values(arterial, venous, hemoglobin)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (arterial, venous, hemoglobin, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, arterial, venous, hemoglobin, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r24o2-{idx + 1:02d}",
                "qtype": "oxygen_index",
                "stem": (
                    f"A blood-gas panel shows arterial oxygen content {arterial} mL/dL, mixed venous oxygen "
                    f"content {venous} mL/dL, and hemoglobin {hemoglobin} g/dL. What is the patient's "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe arterial_oxygen({arterial})\n"
                    f"observe venous_oxygen({venous})\n"
                    f"observe hemoglobin({hemoglobin})\n"
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
            "ADJ-LADDER rung 24 — oxygen extraction from a single arterial/venous blood-gas oxygen panel "
            "(a NEW panel: oxygen transport / Fick physiology). From three stated quantities (arterial "
            "oxygen content ARTERIAL, mixed venous oxygen content VENOUS, hemoglobin HB) compute the oxygen "
            "extraction ratio ((ARTERIAL-VENOUS)/ARTERIAL), the venous fraction (VENOUS/ARTERIAL), or the "
            "a-v difference relative to venous content ((ARTERIAL-VENOUS)/VENOUS). Each item is a "
            "compute_dimensioned program (observe the three quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — a division AND an inner difference-in-parentheses for the "
            "arterio-venous oxygen difference (AVDO2 = ARTERIAL - VENOUS) — and the harness matches the "
            "scalar to the printed options. Contamination-safe: every index is built only from the three "
            "observed quantities via /, -, and grouping — no constant leaks (AVDO2 is derived from its "
            "components, not observed) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. The five options are a family over the same quantities, "
            "so the distractors are exactly the slips students make: the inverse venous fraction "
            "(ARTERIAL/VENOUS, written upside-down) and the content-per-gram-Hb ratio (ARTERIAL/HB, "
            "confusing the carried oxygen pool with its carrier)."
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
