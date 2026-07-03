"""Generate rung-29 (serum protein indices) items.json for the ADJ-LADDER.

Rung 29 opens the **serum protein / hepatic-synthesis** panel on the quantitative band — the arithmetic of
the albumin-to-globulin (A:G) ratio, the bedside index that flags a low A:G (chronic inflammation,
myeloma, cirrhosis) versus a high A:G. It uses the same contamination-safe shape as the coagulation rung
(28), the urine rung (27), and the mineral rung (26): a small table of *observed* laboratory quantities
and a tight family of mutually-confusable formulas built **only from those observed quantities** (no
numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a single serum protein panel. Two quantities are measured (both in g/dL):

  TP   total serum protein
  ALB  serum albumin

Globulin is *not measured* — it is derived as `TP - ALB`. That is what makes this rung distinctive: the
A:G ratio is a **ratio whose denominator is itself a difference** — `ALB / (TP - ALB)` — a shape not yet
seen on the ladder (rung-24 put the difference in the numerator, `(a-b)/a`; rung-27 summed inside a
difference; this rung divides *by* a difference). The core confusion this rung tests is reconstructing
globulin (`TP - ALB`) before dividing, and getting the ratio's direction right:

  ALBUMIN:GLOBULIN RATIO   ALB / (TP - ALB)   [ the A:G ratio ]
  ALBUMIN FRACTION         ALB / TP           [ albumin as a fraction of total protein ]
  GLOBULIN LEVEL           TP - ALB           [ the derived globulin concentration ]

Each index is a `compute_dimensioned` program (observe the two quantities + `let answer = formula`); the
ADJ engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/27/28.

Contamination-safe by construction: every formula is built only from the two observed quantities via
`-`, `/` — **no structural constants** — so every program literal is grounded in the stem. The globulin
value itself never appears as a literal (it is computed as `TP - ALB`). The observed quantities carry
**digit-free identifiers** (`total_protein`, `albumin`) so no numeral hides inside a variable name. The
five options are a tight family over the same quantities: the three real indices plus the two classic
slips —

  (TP - ALB) / ALB    the *inverted* globulin:albumin ratio (right analytes, upside-down), and
  (TP - ALB) / TP     the globulin *fraction* of total protein (the albumin-fraction's complement),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the A:G ratio and the two fractions are order 1, while the globulin level lives on the
g/dL scale (a few g/dL); the tables below are chosen so the five family values are pairwise distinct —
with a comfortable margin — for every item, asserted at build time (ALB != TP - ALB so the A:G ratio and
its inverse never coincide, and no ratio accidentally equals the globulin level).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TP, ALB) observed quantities, both in g/dL. Globulin = TP - ALB is derived (never a literal). The five
# index-family values are asserted pairwise-distinct (with margin) below. ALB != TP - ALB for every row
# (so the A:G ratio and its inverse never coincide).
#   TP  = total serum protein
#   ALB = serum albumin
TABLES = [
    (7, 4),
    (8, 5),
    (9, 5),
    (7, 5),
    (8, 6),
    (10, 6),
    (6, 3.5),
]

# The option family (5 members), all built from the observed quantities via `-` / `/`. Every identifier
# is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold);
# all five always appear as the options.
FAMILY = [
    ("ag_ratio", "albumin-to-globulin ratio", "albumin / (total_protein - albumin)"),
    ("albumin_fraction", "albumin fraction of total protein", "albumin / total_protein"),
    ("globulin_level", "globulin level", "total_protein - albumin"),
    ("ga_ratio", "globulin-to-albumin ratio", "(total_protein - albumin) / albumin"),
    ("globulin_fraction", "globulin fraction of total protein", "(total_protein - albumin) / total_protein"),
]
QUERIED = ["ag_ratio", "albumin_fraction", "globulin_level"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(tp, alb):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "ag_ratio": alb / (tp - alb),
        "albumin_fraction": alb / tp,
        "globulin_level": tp - alb,
        "ga_ratio": (tp - alb) / alb,
        "globulin_fraction": (tp - alb) / tp,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for tp, alb in TABLES:
        assert alb != tp - alb, (tp, alb)  # else the A:G ratio and its inverse coincide
        fv = family_values(tp, alb)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (tp, alb, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, tp, alb, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            # Render the observed values without a trailing ".0" for whole numbers (so the stem
            # and program literals read naturally, e.g. "7" not "7.0").
            def num(x):
                return int(x) if float(x).is_integer() else x
            items.append({
                "id": f"r29prot-{idx + 1:02d}",
                "qtype": "serum_protein_index",
                "stem": (
                    f"A serum protein panel shows a total protein of {num(tp)} g/dL and an albumin of "
                    f"{num(alb)} g/dL. What is the patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe total_protein({num(tp)})\n"
                    f"observe albumin({num(alb)})\n"
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
            "ADJ-LADDER rung 29 — serum protein indices from a single protein panel (a NEW panel: serum "
            "protein / hepatic synthesis). From two stated quantities (total protein TP, albumin ALB) "
            "compute the albumin-to-globulin ratio (ALB/(TP-ALB)), the albumin fraction (ALB/TP), or the "
            "globulin level (TP-ALB). Each item is a compute_dimensioned program (observe the two "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, a "
            "ratio whose DENOMINATOR is itself a difference (ALB/(TP-ALB)), so globulin must be "
            "reconstructed as TP-ALB before dividing — and the harness matches the scalar to the printed "
            "options. Contamination-safe: every index is built only from the two observed quantities via "
            "- and / — no constant leaks, and the globulin value never appears as a literal (it is "
            "computed as TP-ALB) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same quantities, so the "
            "distractors are exactly the slips students make: the inverted globulin:albumin ratio "
            "((TP-ALB)/ALB) and the globulin fraction ((TP-ALB)/TP). The core confusion tested is "
            "reconstructing globulin before dividing, plus the ratio's direction."
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
