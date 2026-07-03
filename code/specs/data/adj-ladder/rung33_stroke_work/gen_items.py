"""Generate rung-33 (ventricular stroke work) items.json for the ADJ-LADDER.

Rung 33 opens the **cardiovascular ventricular-mechanics** panel on the quantitative band — the arithmetic of
the ventricle's pressure-volume work over one beat. On the pressure-volume loop, the work the ventricle does to
eject blood is (to a first approximation) the *area* of the loop: how much blood it moves times the pressure it
moves it against. The two edges of that rectangle are the **stroke volume** (how much the chamber empties) and
the **pulse pressure** (how far the arterial pressure swings), and the stroke-work estimate is their product.
It uses the same contamination-safe shape as the respiratory rung (32), the Starling rung (31), and the
body-mass rung (30): a small table of *observed* volumes and pressures and a tight family of mutually-confusable
formulas built **only from those observed quantities** (no numeric literal anywhere in any program), so nothing
structural can leak.

The clinical setup is a single ventricle characterised over one cardiac cycle. FOUR quantities are measured —
two chamber volumes (mL) and two arterial pressures (mmHg):

  END_DIASTOLIC_VOLUME   EDV   volume in the ventricle when it is fullest, just before ejection   (larger volume)
  END_SYSTOLIC_VOLUME    ESV   volume left in the ventricle after ejection                        (smaller volume)
  SYSTOLIC_PRESSURE      SBP   peak arterial pressure during ejection                             (higher pressure)
  DIASTOLIC_PRESSURE     DBP   trough arterial pressure between beats                             (lower pressure)

The stroke-work estimate is the **volume the ventricle ejected multiplied by the pressure it swings through** —
a *product of two differences* — `(EDV − ESV) * (SBP − DBP)`. That is what makes this rung distinctive: it is a
NEW arithmetic shape on the ladder — a product whose TWO factors are each their own difference (rung-31
subtracted one difference from another; rung-32 divided one difference by another; this rung MULTIPLIES one
difference by another). The core confusion this rung tests is pairing the right two quantities into each
difference (a volume difference times a pressure difference), rather than crossing a volume with a pressure:

  STROKE WORK ESTIMATE   (EDV − ESV) * (SBP − DBP)   [ ejected volume  x  pressure swing = loop area ]
  STROKE VOLUME          EDV − ESV                   [ the volume ejected, one factor ]
  PULSE PRESSURE         SBP − DBP                   [ the pressure swing, the other factor ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/31/32. This rung exercises the engine across a
MULTIPLICATION of two parenthesised DIFFERENCES.

Contamination-safe by construction: every formula is built only from the four observed quantities via `-`, `*`,
`+` — **no structural constants** — so every program literal is grounded in the stem. Neither the stroke volume
nor the pulse pressure ever appears as a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`end_diastolic_volume`, `end_systolic_volume`, `systolic_pressure`,
`diastolic_pressure`) so no numeral hides inside a variable name. The five options are a tight family over the
same quantities: the three real indices plus the two classic slips —

  CROSSED PRODUCT   (EDV − DBP) * (SBP − ESV)   the volumes and pressures *crossed* (a volume minus a pressure), and
  SUMMED AREA       (EDV − ESV) + (SBP − DBP)   the two differences ADDED instead of multiplied (a perimeter, not an area),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the stroke-work estimate is order 1e3 (SV ~ 70-90 mL times PP ~ 40-60 mmHg ~ 3000-5000), while
the stroke volume (tens of mL) and pulse pressure (tens of mmHg) are two orders of magnitude smaller, and the
summed area (~120-150) sits between them; the crossed product is close to but never equal to the stroke work.
The tables below are chosen so the five family values are pairwise distinct — with a comfortable margin — for
every item, asserted at build time (stroke volume and pulse pressure both strictly positive, and the crossed
product kept off the stroke-work value so no two options collide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (END_DIASTOLIC_VOLUME, END_SYSTOLIC_VOLUME, SYSTOLIC_PRESSURE, DIASTOLIC_PRESSURE) observed per beat. Volumes
# in mL, pressures in mmHg. EDV exceeds ESV (the chamber ejected blood) and SBP exceeds DBP (the pressure
# swings up during ejection), so both differences are strictly positive. The five family values are asserted
# pairwise-distinct (with margin) below — in particular the crossed product is kept off the stroke-work value.
#   EDV = end-diastolic volume    (fullest, larger)
#   ESV = end-systolic volume     (after ejection, smaller)
#   SBP = systolic pressure       (peak, higher)
#   DBP = diastolic pressure      (trough, lower)
TABLES = [
    (124, 52, 126, 78),
    (135, 60, 130, 85),
    (150, 70, 140, 90),
    (128, 48, 118, 74),
    (142, 55, 148, 88),
    (118, 45, 110, 66),
    (155, 72, 135, 82),
]

# The option family (5 members), all built from the observed quantities via `-` / `*` / `+`. Every identifier
# is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all
# five always appear as the options.
FAMILY = [
    (
        "stroke_work_estimate",
        "stroke work estimate",
        "(end_diastolic_volume - end_systolic_volume) * (systolic_pressure - diastolic_pressure)",
    ),
    (
        "stroke_volume",
        "stroke volume",
        "end_diastolic_volume - end_systolic_volume",
    ),
    (
        "pulse_pressure",
        "pulse pressure",
        "systolic_pressure - diastolic_pressure",
    ),
    (
        "crossed_product",
        "crossed product (volume and pressure crossed)",
        "(end_diastolic_volume - diastolic_pressure) * (systolic_pressure - end_systolic_volume)",
    ),
    (
        "summed_area",
        "summed differences (stroke volume plus pulse pressure)",
        "(end_diastolic_volume - end_systolic_volume) + (systolic_pressure - diastolic_pressure)",
    ),
]
QUERIED = ["stroke_work_estimate", "stroke_volume", "pulse_pressure"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(edv, esv, sbp, dbp):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    stroke_volume = edv - esv
    pulse_pressure = sbp - dbp
    return {
        "stroke_work_estimate": stroke_volume * pulse_pressure,
        "stroke_volume": stroke_volume,
        "pulse_pressure": pulse_pressure,
        "crossed_product": (edv - dbp) * (sbp - esv),
        "summed_area": stroke_volume + pulse_pressure,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for edv, esv, sbp, dbp in TABLES:
        stroke_volume = edv - esv
        pulse_pressure = sbp - dbp
        assert stroke_volume > 0 and pulse_pressure > 0, (edv, esv, sbp, dbp)
        fv = family_values(edv, esv, sbp, dbp)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (edv, esv, sbp, dbp, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, edv, esv, sbp, dbp, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r33stroke-{idx + 1:02d}",
                "qtype": "stroke_work",
                "stem": (
                    f"A single cardiac cycle is characterised for one ventricle: the end-diastolic volume is "
                    f"{num(edv)} mL and the end-systolic volume is {num(esv)} mL; the systolic pressure is "
                    f"{num(sbp)} mmHg and the diastolic pressure is {num(dbp)} mmHg. What is the patient's "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe end_diastolic_volume({num(edv)})\n"
                    f"observe end_systolic_volume({num(esv)})\n"
                    f"observe systolic_pressure({num(sbp)})\n"
                    f"observe diastolic_pressure({num(dbp)})\n"
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
            "ADJ-LADDER rung 33 — ventricular stroke work from paired chamber volumes and arterial pressures "
            "over one beat (a NEW panel: cardiovascular ventricular mechanics). From four stated quantities "
            "(end-diastolic volume EDV, end-systolic volume ESV, systolic pressure SBP, diastolic pressure DBP) "
            "compute the stroke work estimate ((EDV-ESV)*(SBP-DBP)), the stroke volume (EDV-ESV), or the pulse "
            "pressure (SBP-DBP). Each item is a compute_dimensioned program (observe the four quantities, let "
            "answer = formula); the ADJ engine carries the arithmetic — a NEW shape, a PRODUCT OF TWO "
            "DIFFERENCES ((EDV-ESV)*(SBP-DBP)), so one parenthesised difference is multiplied by another — and "
            "the harness matches the scalar to the printed options. Contamination-safe: every index is built "
            "only from the four observed quantities via -, * and + — no constant leaks (stroke work is a pure "
            "product of differences), and neither the stroke volume nor the pulse pressure ever appears as a "
            "literal (each is computed from the observed quantities) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family "
            "over the same quantities, so the distractors are exactly the slips students make: the crossed "
            "product ((EDV-DBP)*(SBP-ESV), a volume and a pressure crossed into each difference) and the summed "
            "area ((EDV-ESV)+(SBP-DBP), the two differences added instead of multiplied). The core confusion "
            "tested is multiplying a volume difference by a pressure difference with each pairing correct."
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
