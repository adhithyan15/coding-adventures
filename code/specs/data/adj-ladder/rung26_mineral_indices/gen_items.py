"""Generate rung-26 (mineral indices) items.json for the ADJ-LADDER.

Rung 26 opens the **bone / mineral-metabolism** panel on the quantitative band — the calcium-phosphate
arithmetic of CKD-MBD (chronic-kidney-disease mineral & bone disorder), where the calcium-phosphate
product predicts metastatic (vascular/soft-tissue) calcification — using the same contamination-safe
shape as the lipid rung (25) and the iron rung (23): a small table of *observed* laboratory quantities
and a tight family of mutually-confusable formulas built **only from those observed quantities** (no
numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a single mineral panel. Three quantities are measured:

  CA    serum calcium      (mg/dL)
  PHOS  serum phosphate    (mg/dL)
  ALB   serum albumin      (g/dL)

Three textbook mineral indices fall out as pure functions of the observed quantities — no constant
required. Crucially this rung introduces a **PRODUCT** in the numerator (the calcium-phosphate
product, CA·PHOS), contrasted directly against the DIVISION forms, so the family exercises the engine
across `*` and `/` on a fresh panel (like the cardiac rung mixed a product with an index-division):

  CALCIUM-PHOSPHATE PRODUCT   CA * PHOS     [ =CPP; >55 mg^2/dL^2 → calcification risk ]
  CALCIUM:PHOSPHATE RATIO     CA / PHOS     [ the ratio — deliberately contrasted with the product ]
  PHOSPHATE:CALCIUM RATIO     PHOS / CA     [ the inverse ratio (upside-down) ]

Each index is a `compute_dimensioned` program (observe the three quantities + `let answer = formula`);
the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/…/24/25. The core
confusion this rung tests is **product vs ratio** (CA·PHOS vs CA/PHOS) — a genuine student slip — plus
the ratio's direction (CA/PHOS vs PHOS/CA).

Contamination-safe by construction: every formula is built only from the three observed quantities via
`*`, `/` — **no structural constants** — so every program literal is grounded in the stem. The
observed quantities carry **digit-free identifiers** (`calcium`, `phosphate`, `albumin`) so no numeral
hides inside a variable name. The five options are a tight family over the same quantities: the three
real indices plus the two classic slips —

  CA / ALB       the calcium-to-albumin ratio (a plausible mineral-panel ratio — but albumin is the
                 *correction* input, not a divisor of the product), and
  PHOS / ALB     the phosphate-to-albumin ratio (the same mix-up on the other analyte),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the calcium-phosphate product lives on a different magnitude (tens) from the ratios
(order 1), so it never collides with them; the ratio pairs are reciprocals (distinct when CA != PHOS).
The tables below are chosen so the five family values are pairwise distinct — with a comfortable margin
— for every item, asserted at build time (CA != PHOS enforced so the two ratios never coincide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CA, PHOS, ALB) observed quantities. CA in mg/dL, PHOS in mg/dL, ALB in g/dL. The five index-family
# values are asserted pairwise-distinct (with margin) below. CA != PHOS for every row (so the CA:PHOS
# and PHOS:CA ratios never coincide).
#   CA   = serum calcium
#   PHOS = serum phosphate
#   ALB  = serum albumin
TABLES = [
    (10, 4, 5),
    (9, 3, 4),
    (8, 2, 4),
    (12, 6, 4),
    (10, 5, 4),
    (11, 4, 5),
    (9, 6, 3),
]

# The option family (5 members), all built from the observed quantities via `*` / `/`. Every
# identifier is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried*
# (used as gold); all five always appear as the options.
FAMILY = [
    ("ca_phos_product", "calcium-phosphate product", "calcium * phosphate"),
    ("ca_phos_ratio", "calcium-to-phosphate ratio", "calcium / phosphate"),
    ("phos_ca_ratio", "phosphate-to-calcium ratio", "phosphate / calcium"),
    ("ca_alb_ratio", "calcium-to-albumin ratio", "calcium / albumin"),
    ("phos_alb_ratio", "phosphate-to-albumin ratio", "phosphate / albumin"),
]
QUERIED = ["ca_phos_product", "ca_phos_ratio", "phos_ca_ratio"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(ca, phos, alb):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "ca_phos_product": ca * phos,
        "ca_phos_ratio": ca / phos,
        "phos_ca_ratio": phos / ca,
        "ca_alb_ratio": ca / alb,
        "phos_alb_ratio": phos / alb,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for ca, phos, alb in TABLES:
        assert ca != phos, (ca, phos)  # else CA:PHOS and PHOS:CA ratios coincide
        fv = family_values(ca, phos, alb)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (ca, phos, alb, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, ca, phos, alb, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r26min-{idx + 1:02d}",
                "qtype": "mineral_index",
                "stem": (
                    f"A mineral panel shows serum calcium {ca} mg/dL, phosphate {phos} mg/dL, and "
                    f"albumin {alb} g/dL. What is the patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe calcium({ca})\n"
                    f"observe phosphate({phos})\n"
                    f"observe albumin({alb})\n"
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
            "ADJ-LADDER rung 26 — mineral indices from a single mineral panel (a NEW panel: bone / "
            "mineral metabolism, CKD-MBD). From three stated quantities (serum calcium CA, phosphate "
            "PHOS, albumin ALB) compute the calcium-phosphate product (CA*PHOS), the calcium-to-phosphate "
            "ratio (CA/PHOS), or the phosphate-to-calcium ratio (PHOS/CA). Each item is a "
            "compute_dimensioned program (observe the three quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — a PRODUCT (CA*PHOS) contrasted with the DIVISION forms, "
            "exercising the engine across * and / on a fresh mineral-panel stem — and the harness matches "
            "the scalar to the printed options. Contamination-safe: every index is built only from the "
            "three observed quantities via * and / — no constant leaks — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same quantities, so the distractors are exactly the slips students make: the "
            "calcium-to-albumin ratio (CA/ALB, confusing the correction input with a divisor) and the "
            "phosphate-to-albumin ratio (PHOS/ALB, the same mix-up on the other analyte). The core "
            "confusion tested is product vs ratio (CA*PHOS vs CA/PHOS) plus the ratio's direction."
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
