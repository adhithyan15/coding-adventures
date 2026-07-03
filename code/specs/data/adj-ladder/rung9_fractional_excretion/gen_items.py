"""Generate rung-9 (renal indices / fractional excretion) items.json for the ADJ-LADDER.

Rung 9 stays in the quantitative-clinical band opened by the biostat family (7/7b/7c) and the
pharmacokinetics rung (8), and keeps the identical contamination-safe shape: a small table of
*observed* quantities and a tight family of mutually-confusable formulas that are **pure
multiplication/division of those observed quantities** (no numeric literal anywhere).

The clinical setting is the work-up of acute kidney injury, where the **renal indices** computed
from a paired urine/plasma chemistry distinguish a prerenal state from acute tubular necrosis. We
observe four quantities:

  UNa   urine sodium       (mEq/L)
  PNa   plasma sodium      (mEq/L)
  UCr   urine creatinine   (mg/dL)
  PCr   plasma creatinine  (mg/dL)

From these, the three bedside indices fall out as **pure ratios of products** — exact, and needing
no constant (the textbook FENa carries a `× 100` only to render the fraction as a percent; we ask
for the **fraction itself**, so not even the 100 leaks):

  FENa (fractional excretion of sodium)   = (UNa · PCr) / (PNa · UCr)   [dimensionless fraction]
  RFI  (renal failure index)              = (UNa · PCr) / UCr           [ = UNa / (UCr/PCr) ]
  U/P  (urine-to-plasma creatinine ratio) = UCr / PCr                   [dimensionless]

Each is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the
ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 4/7/7b/7c/8.

Contamination-safe by construction: every formula is built only from the four observed quantities
via multiplication and division — **no structural constants** — so every program literal is
grounded in the stem. The five options are a tight family of ratios over the same four quantities:
the three real indices {FENa, RFI, U/P-Cr} plus the two classic confusions {inverted-FENa,
urine-to-plasma sodium ratio}. The distractors are therefore exactly the slips a student makes —
inverting the fractional-excretion ratio, or reading the sodium U/P ratio when the creatinine one
was asked. Gold rotates A–E by index.

Note on table choice: several family values coincide for special quantity ratios, so the tables
below are chosen so the five family values are pairwise distinct for every item, and this is
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (UNa, PNa, UCr, PCr) observed quantities. The five renal-index values are asserted
# pairwise-distinct below.
#   UNa = urine sodium (mEq/L)
#   PNa = plasma sodium (mEq/L)
#   UCr = urine creatinine (mg/dL)
#   PCr = plasma creatinine (mg/dL)
TABLES = [
    (20, 140, 80, 2),
    (40, 135, 100, 4),
    (10, 145, 60, 3),
    (60, 138, 120, 5),
    (15, 142, 90, 1.5),
    (50, 130, 75, 2.5),
    (25, 136, 50, 4),
]

# The option family (5 members), all multiplication/division over the observed quantities
# una/pna/ucr/pcr.  key -> (display name, formula-as-adj)
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("fena", "fractional excretion of sodium (FENa), expressed as a fraction",
     "(una * pcr) / (pna * ucr)"),
    ("rfi", "renal failure index (RFI)", "(una * pcr) / ucr"),
    ("upcr", "urine-to-plasma creatinine ratio", "ucr / pcr"),
    ("fena_inv", "inverted fractional-excretion ratio", "(pna * ucr) / (una * pcr)"),
    ("upna", "urine-to-plasma sodium ratio", "una / pna"),
]
QUERIED = ["fena", "rfi", "upcr"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(una, pna, ucr, pcr):
    return {
        "fena": (una * pcr) / (pna * ucr),
        "rfi": (una * pcr) / ucr,
        "upcr": ucr / pcr,
        "fena_inv": (pna * ucr) / (una * pcr),
        "upna": una / pna,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for una, pna, ucr, pcr in TABLES:
        fv = family_values(una, pna, ucr, pcr)
        assert len({round(fv[k], 12) for k in ORDER}) == 5, (una, pna, ucr, pcr, fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 12) for v in opts_vals}) == 5, (key, una, pna, ucr, pcr, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r9fe-{idx + 1:02d}",
                "qtype": "renal_index",
                "stem": (
                    f"A patient being worked up for acute kidney injury has a urine sodium of "
                    f"{una} mEq/L, a plasma sodium of {pna} mEq/L, a urine creatinine of {ucr} "
                    f"mg/dL, and a plasma creatinine of {pcr} mg/dL. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe una({una})\n"
                    f"observe pna({pna})\n"
                    f"observe ucr({ucr})\n"
                    f"observe pcr({pcr})\n"
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
            "ADJ-LADDER rung 9 — renal indices from a paired urine/plasma chemistry (acute kidney "
            "injury work-up). From four stated quantities (urine Na, plasma Na, urine Cr, plasma Cr) "
            "compute the fractional excretion of sodium (FENa = (UNa*PCr)/(PNa*UCr), as a fraction), "
            "the renal failure index (RFI = (UNa*PCr)/UCr), or the urine-to-plasma creatinine ratio "
            "(UCr/PCr). Each item is a compute_dimensioned program (observe the four quantities, let "
            "answer = formula); the ADJ engine carries the arithmetic and the harness matches the "
            "scalar to the printed options. Contamination-safe: every index is a pure ratio of "
            "products of the four observed quantities — no constant leaks (not even the *100 of the "
            "FENa percentage, since we ask for the fraction). The five options are a family of ratios "
            "over the same quantities, so the distractors are exactly the slips students make "
            "(inverting the fractional-excretion ratio; reading the sodium U/P ratio instead of the "
            "creatinine one)."
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
