"""Generate rung-20 (hepatic panel indices) items.json for the ADJ-LADDER.

Rung 20 opens a **new organ system** on the quantitative band — **hepatology** — using the same
contamination-safe shape as the pharmacokinetics rung (8), the renal-ratio rungs (9/13/15), the
cardiology rungs (16/18), and the hematology rung (19): a small table of *observed* laboratory
quantities, and a tight family of mutually-confusable formulas built **only from those observed
quantities** (no numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a single liver-function panel. We observe four quantities:

  AST   aspartate aminotransferase   (U/L)     — the "SGOT" transaminase
  ALT   alanine aminotransferase     (U/L)     — the "SGPT" transaminase, liver-specific
  DBIL  direct (conjugated) bilirubin (mg/dL)  — post-conjugation, water-soluble
  IBIL  indirect (unconjugated) bilirubin (mg/dL) — pre-conjugation, albumin-bound

From those four, three textbook hepatic indices fall out as **pure ratios** of the observed
quantities — no constant required:

  DE RITIS   AST : ALT ratio            = AST / ALT            [ >2 suggests alcoholic liver injury ]
  DIRECT %   conjugated fraction        = DBIL / (DBIL + IBIL) [ high → obstructive / hepatocellular ]
  INDIRECT % unconjugated fraction      = IBIL / (DBIL + IBIL) [ high → hemolysis / Gilbert ]

Note the bilirubin fractions divide by **total bilirubin = DBIL + IBIL**, which is itself derived
from the two observed components — so the denominator is a *sum of observed quantities*, never a
constant. This exercises the engine across **division AND an inner addition-in-parentheses** —
the same `(a + b)` grouping the ventilation/ejection rungs (17/18) used for subtraction — on a
hepatology stem. Total bilirubin is deliberately NOT observed (it is computed), so no `×`-factor or
constant appears.

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer =
formula`); the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/18/19.

Contamination-safe by construction: every formula is built only from the four observed quantities
via `/`, `+`, and grouping `( )` — **no structural constants** — so every program literal is
grounded in the stem. The five options are a tight family over the same quantities: the three real
indices {De Ritis, direct %, indirect %} plus the two classic slips —

  DBIL / IBIL   the direct-to-indirect *ratio* (confused with the direct *fraction* of total), and
  ALT / AST     the **inverse** De Ritis ratio (the ratio written upside-down),

which are exactly the mistakes a student makes. Gold rotates A–E by index.

Note on table choice: the five family values can collide for special quantity ratios (e.g.
De Ritis AST/ALT equals DBIL/IBIL whenever AST·IBIL = ALT·DBIL). The tables below are chosen so the
five family values are pairwise distinct — with a comfortable margin — for every item, asserted at
build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (AST, ALT, DBIL, IBIL) observed quantities. The five index-family values are asserted
# pairwise-distinct (with margin) below.
#   AST  = aspartate aminotransferase  (U/L)
#   ALT  = alanine aminotransferase    (U/L)
#   DBIL = direct bilirubin            (mg/dL)
#   IBIL = indirect bilirubin          (mg/dL)
TABLES = [
    (90, 45, 5, 3),
    (60, 50, 4, 6),
    (120, 40, 7, 2),
    (75, 50, 3, 5),
    (100, 80, 8, 4),
    (45, 90, 2, 7),
    (66, 55, 5, 4),
]

# The option family (5 members), all built from the observed quantities ast/alt/dbil/ibil via
# `/`, `+`, and grouping. key -> (display name, formula-as-adj). Only the first three are *queried*
# (used as gold); all five always appear as the options.
FAMILY = [
    ("de_ritis", "AST:ALT (De Ritis) ratio", "ast / alt"),
    ("direct_frac", "direct (conjugated) fraction of total bilirubin", "dbil / (dbil + ibil)"),
    ("indirect_frac", "indirect (unconjugated) fraction of total bilirubin", "ibil / (dbil + ibil)"),
    ("di_ratio", "direct-to-indirect bilirubin ratio", "dbil / ibil"),
    ("alt_ast", "ALT:AST ratio", "alt / ast"),
]
QUERIED = ["de_ritis", "direct_frac", "indirect_frac"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(ast, alt, dbil, ibil):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "de_ritis": ast / alt,
        "direct_frac": dbil / (dbil + ibil),
        "indirect_frac": ibil / (dbil + ibil),
        "di_ratio": dbil / ibil,
        "alt_ast": alt / ast,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for ast, alt, dbil, ibil in TABLES:
        fv = family_values(ast, alt, dbil, ibil)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (ast, alt, dbil, ibil, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, ast, alt, dbil, ibil, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r20hep-{idx + 1:02d}",
                "qtype": "hepatic_index",
                "stem": (
                    f"A liver-function panel shows AST {ast} U/L, ALT {alt} U/L, direct bilirubin "
                    f"{dbil} mg/dL, and indirect bilirubin {ibil} mg/dL. What is the patient's "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe ast({ast})\n"
                    f"observe alt({alt})\n"
                    f"observe dbil({dbil})\n"
                    f"observe ibil({ibil})\n"
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
            "ADJ-LADDER rung 20 — hepatic panel indices from a single liver-function test (a NEW "
            "organ system: hepatology). From four stated quantities (AST, ALT, direct bilirubin DBIL, "
            "indirect bilirubin IBIL) compute the AST:ALT De Ritis ratio (AST/ALT), the direct "
            "(conjugated) fraction of total bilirubin (DBIL/(DBIL+IBIL)), or the indirect "
            "(unconjugated) fraction (IBIL/(DBIL+IBIL)). Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a division AND an inner sum-in-parentheses for total bilirubin — and the "
            "harness matches the scalar to the printed options. Contamination-safe: every index is "
            "built only from the four observed quantities via /, +, and grouping — no constant leaks "
            "(total bilirubin is derived from its components, not observed, so no x-factor appears). "
            "The five options are a family over the same quantities, so the distractors are exactly "
            "the slips students make: the direct-to-indirect ratio (DBIL/IBIL, confused with the "
            "direct fraction) and the inverse De Ritis ratio (ALT/AST, written upside-down)."
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
