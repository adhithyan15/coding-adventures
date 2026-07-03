"""Generate rung-15 (fractional excretion of urea) items.json for the ADJ-LADDER.

Rung 15 is the direct sibling of rung 9 (fractional excretion of sodium): the same
contamination-safe shape — a small table of *observed* quantities and a tight family of
mutually-confusable formulas that are **pure multiplication/division of those observed
quantities** (no numeric literal anywhere) — applied to a different, high-yield renal index.

The clinical setting is acute kidney injury in a patient on **diuretics**, where the classic
FENa is unreliable (the diuretic itself raises urine sodium). The **fractional excretion of urea
(FEUrea)** sidesteps that: urea handling is not affected by loop or thiazide diuretics, so a low
FEUrea still points to a prerenal state. We observe four quantities:

  UUrea   urine urea nitrogen    (mg/dL)
  PUrea   plasma urea nitrogen   (mg/dL, i.e. the BUN)
  UCr     urine creatinine       (mg/dL)
  PCr     plasma creatinine      (mg/dL)

From these, the bedside indices fall out as **pure ratios of products** — exact, and needing no
constant (the textbook FEUrea carries a `× 100` only to render the fraction as a percent; we ask
for the **fraction itself**, so not even the 100 leaks):

  FEUrea (fractional excretion of urea)     = (UUrea · PCr) / (PUrea · UCr)   [dimensionless fraction]
  UFI    (urea failure index)               = (UUrea · PCr) / UCr             [ = UUrea / (UCr/PCr) ]
  U/P-Cr (urine-to-plasma creatinine ratio) = UCr / PCr                       [dimensionless]

Each is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the
ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 4/7/7b/7c/8/9/13.

Contamination-safe by construction: every formula is built only from the four observed quantities
via multiplication and division — **no structural constants** — so every program literal is
grounded in the stem, and every identifier is DIGIT-FREE (`uurea`/`purea`/`ucr`/`pcr`/`answer`).
The five options are a tight family of ratios over the same four quantities: the three real indices
{FEUrea, UFI, U/P-Cr} plus the two classic confusions {inverted-FEUrea, urine-to-plasma urea ratio}.
The distractors are therefore exactly the slips a student makes — inverting the fractional-excretion
ratio, or reading the urea U/P ratio when the creatinine one was asked. Gold rotates A–E by index.

Note on table choice: several family values coincide for special quantity ratios, so the tables
below are chosen so the five family values are pairwise distinct for every item, and this is
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (UUrea, PUrea, UCr, PCr) observed quantities. The five urea-index values are asserted
# pairwise-distinct below.
#   UUrea = urine urea nitrogen   (mg/dL)
#   PUrea = plasma urea nitrogen  (mg/dL, the BUN)
#   UCr   = urine creatinine      (mg/dL)
#   PCr   = plasma creatinine     (mg/dL)
TABLES = [
    (300, 20, 80, 2),
    (400, 15, 100, 3),
    (600, 40, 120, 4),
    (250, 10, 60, 1.5),
    (500, 25, 90, 5),
    (350, 30, 70, 2.5),
    (450, 18, 110, 3),
]

# The option family (5 members), all multiplication/division over the observed quantities
# uurea/purea/ucr/pcr.  key -> (display name, formula-as-adj)
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("feurea", "fractional excretion of urea (FEUrea), expressed as a fraction",
     "(uurea * pcr) / (purea * ucr)"),
    ("ufi", "urea failure index", "(uurea * pcr) / ucr"),
    ("upcr", "urine-to-plasma creatinine ratio", "ucr / pcr"),
    ("feurea_inv", "inverted fractional-excretion-of-urea ratio", "(purea * ucr) / (uurea * pcr)"),
    ("upurea", "urine-to-plasma urea ratio", "uurea / purea"),
]
QUERIED = ["feurea", "ufi", "upcr"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(uurea, purea, ucr, pcr):
    return {
        "feurea": (uurea * pcr) / (purea * ucr),
        "ufi": (uurea * pcr) / ucr,
        "upcr": ucr / pcr,
        "feurea_inv": (purea * ucr) / (uurea * pcr),
        "upurea": uurea / purea,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for uurea, purea, ucr, pcr in TABLES:
        fv = family_values(uurea, purea, ucr, pcr)
        assert len({round(fv[k], 12) for k in ORDER}) == 5, (uurea, purea, ucr, pcr, fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 12) for v in opts_vals}) == 5, (key, uurea, purea, ucr, pcr, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r15feu-{idx + 1:02d}",
                "qtype": "urea_index",
                "stem": (
                    f"A patient on diuretics is being worked up for acute kidney injury (so FENa is "
                    f"unreliable). The labs show a urine urea nitrogen of {uurea} mg/dL, a plasma urea "
                    f"nitrogen of {purea} mg/dL, a urine creatinine of {ucr} mg/dL, and a plasma "
                    f"creatinine of {pcr} mg/dL. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe uurea({uurea})\n"
                    f"observe purea({purea})\n"
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
            "ADJ-LADDER rung 15 — fractional excretion of urea, the sibling of rung 9 (FENa) for the "
            "AKI work-up when the patient is on diuretics (FENa unreliable). From four stated "
            "quantities of a paired urine/plasma chemistry (urine urea, plasma urea/BUN, urine Cr, "
            "plasma Cr) compute the fractional excretion of urea (FEUrea = (UUrea*PCr)/(PUrea*UCr), as "
            "a fraction), the urea failure index (UFI = (UUrea*PCr)/UCr), or the urine-to-plasma "
            "creatinine ratio (UCr/PCr). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the multiply/divide arithmetic "
            "via the existing compute_dimensioned extractor (no harness/engine change). "
            "Contamination-safe: no structural constants — every index is a pure ratio of products of "
            "the four stated quantities (not even the *100 of the FEUrea percentage, since we ask for "
            "the fraction), and every identifier is digit-free. The five options are a family of ratios "
            "over the same quantities {FEUrea, UFI, U/P-Cr, inverted-FEUrea, U/P-urea}, so the "
            "distractors are exactly the slips students make (inverting the fractional-excretion ratio; "
            "reading the urea U/P ratio for the creatinine one); gold letter rotated A–E."
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
