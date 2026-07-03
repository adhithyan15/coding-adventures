"""Generate rung-18 (cardiac ventricular function) items.json for the ADJ-LADDER.

Rung 18 deepens the cardiology band opened by rung 16 (cardiac *output*) with a distinct
concept — **ventricular function / ejection**, centred on the ventricular volumes — using the
same contamination-safe shape as the pharmacokinetics (rung 8), cardiac-output (rung 16), and
ventilation (rung 17) rungs: a small table of *observed* bedside quantities, and a tight family of
mutually-confusable formulas built **only from those observed quantities** (no numeric literal
anywhere in any program), so nothing structural can leak into the answer.

The clinical setup is a single ventriculography / echo measurement. We observe two quantities:

  EDV   end-diastolic volume   (mL)   — the ventricle's volume when full (just before contraction)
  ESV   end-systolic volume    (mL)   — the ventricle's volume when empty (just after contraction)

From those two, the three textbook ventricular-function parameters fall out as **exact
combinations** of the observed quantities — no constant required:

  SV   stroke volume                = EDV - ESV            (mL)      [ blood ejected per beat ]
  EF   ejection fraction            = (EDV - ESV) / EDV    (scalar)  [ fraction of EDV ejected ]
  RF   end-systolic residual frac.  = ESV / EDV            (scalar)  [ = 1 - EF ]

Like rung 17, this rung combines a **subtraction** of two observed quantities with a **division**
(EF = (EDV - ESV) / EDV), exercising the engine's arithmetic across `-`, `+`, `/` and grouping
parentheses — now on a cardiac ventricular-function stem, and producing dimensionless ratios (EF,
RF) alongside a volume (SV).

Each parameter is a `compute_dimensioned` program (observe the two quantities + `let answer =
formula`); the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/17.

Contamination-safe by construction: every formula is built only from the two observed quantities
via `+`, `-`, `/` — **no structural constants** — so every program literal is grounded in the
stem. The five options are a tight family over the same two quantities: the three real parameters
{SV, EF, RF} plus the two classic slips —

  EDV + ESV          — ADDING the two volumes instead of subtracting (a nonsense "total"), and
  (EDV - ESV) / ESV  — dividing the stroke volume by ESV instead of EDV (wrong denominator),

which are exactly the mistakes a student makes (wrong operator; wrong reference volume). Gold
rotates A-E by index.

Note on table choice: EF and RF collide (both = 0.5) exactly when EDV = 2*ESV, and other family
values can collide for special ratios. The tables below are chosen so the five family values are
pairwise distinct — with a comfortable margin — for every item, asserted at build time (and every
EDV > ESV, as physiology requires).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (EDV, ESV) observed quantities, EDV > ESV. The five family values are asserted
# pairwise-distinct (with margin) below; EDV != 2*ESV so EF != RF.
#   EDV = end-diastolic volume  (mL)
#   ESV = end-systolic volume   (mL)
TABLES = [
    (120, 50),
    (140, 60),
    (100, 40),
    (150, 45),
    (130, 55),
    (110, 45),
    (160, 70),
]

# The option family (5 members), all built from the observed quantities edv/esv via `+ - /`.
#   key -> (display name, formula-as-adj)
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("sv", "stroke volume (SV)", "edv - esv"),
    ("ef", "ejection fraction (EF)", "(edv - esv) / edv"),
    ("rf", "end-systolic residual fraction", "esv / edv"),
    ("total", "sum of the ventricular volumes", "edv + esv"),
    ("sv_over_esv", "stroke volume per end-systolic volume", "(edv - esv) / esv"),
]
QUERIED = ["sv", "ef", "rf"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(edv, esv):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "sv": edv - esv,
        "ef": (edv - esv) / edv,
        "rf": esv / edv,
        "total": edv + esv,
        "sv_over_esv": (edv - esv) / esv,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for edv, esv in TABLES:
        assert edv > esv, (edv, esv)  # physiology: the ventricle empties but never fully
        fv = family_values(edv, esv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (edv, esv, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, edv, esv, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r18ef-{idx + 1:02d}",
                "qtype": "ventricular_function_parameter",
                "stem": (
                    f"A ventriculography study shows an end-diastolic volume of {edv} mL and an "
                    f"end-systolic volume of {esv} mL. What is the patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe edv({edv})\n"
                    f"observe esv({esv})\n"
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
            "ADJ-LADDER rung 18 — cardiac ventricular-function parameters from a single "
            "ventriculography measurement (deepening the cardiology band from rung 16's output to "
            "ventricular ejection). From two stated ventricular volumes (end-diastolic EDV, "
            "end-systolic ESV) compute the stroke volume (SV = EDV-ESV), ejection fraction "
            "(EF = (EDV-ESV)/EDV), or end-systolic residual fraction (RF = ESV/EDV). Each item is a "
            "compute_dimensioned program (observe the two volumes, let answer = formula); the ADJ "
            "engine carries the arithmetic — a subtraction combined with a division — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every parameter is built "
            "only from the two observed volumes via + - / — no constant leaks. The five options are a "
            "family over the same volumes, so the distractors are exactly the slips students make: "
            "adding the volumes instead of subtracting, or dividing the stroke volume by ESV instead "
            "of EDV."
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
