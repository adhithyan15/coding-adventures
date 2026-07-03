"""Generate rung-17 (respiratory ventilation mechanics) items.json for the ADJ-LADDER.

Rung 17 opens ANOTHER new organ system on the quantitative band — **pulmonary / respiratory
mechanics** — using the same contamination-safe shape as the pharmacokinetics rung (rung 8) and
the cardiac-hemodynamics rung (rung 16): a small table of *observed* bedside quantities, and a
tight family of mutually-confusable formulas built **only from those observed quantities** (no
numeric literal anywhere in any program), so nothing structural can leak into the answer.

The clinical setup is a single ventilation assessment. We observe three quantities:

  TV   tidal volume            (mL)          — air moved per breath
  RR   respiratory rate        (breaths/min) — breaths per minute
  VD   (anatomic) dead space   (mL)          — conducting-airway volume that does not exchange gas

From those three, the three textbook bedside ventilation parameters fall out as **exact
combinations** of the observed quantities — no constant required:

  MV   minute ventilation          = TV * RR           (mL/min)  [ volume/breath * breaths/min ]
  VA   alveolar ventilation        = (TV - VD) * RR     (mL/min)  [ only the gas-exchanging part ]
  VDm  dead-space ventilation/min  = VD * RR            (mL/min)  [ = MV - VA ]

This rung is richer than rungs 8/16 (pure ratios / a product): it combines a **subtraction** of
two observed quantities with a **product** — `VA = (TV - VD) * RR` — exercising the engine's
arithmetic across `+`, `-`, `*` and grouping parentheses, all on a respiratory stem.

Each parameter is a `compute_dimensioned` program (observe the three quantities + `let answer =
formula`); the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8 and 16.

Contamination-safe by construction: every formula is built only from the three observed
quantities via `+`, `-`, `*` — **no structural constants** — so every program literal is grounded
in the stem. The five options are a tight family over the same three quantities: the three real
parameters {MV, VA, VDm} plus the two classic slips —

  (TV + VD) * RR  — ADDING the dead space instead of subtracting it (inflating alveolar ventilation), and
  TV - VD         — the alveolar volume PER BREATH, forgetting to multiply by the rate,

which are exactly the mistakes a student makes (wrong sign on dead space; dropping the ×RR). Gold
rotates A–E by index.

Note on table choice: the five family values can collide for special quantity relations (e.g.
VDm = VD*RR equals the alveolar-per-breath TV-VD only for particular values). The tables below are
chosen so the five family values are pairwise distinct — with a comfortable margin — for every
item, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TV, RR, VD) observed quantities. The five ventilation-family values are asserted
# pairwise-distinct (with margin) below.
#   TV  = tidal volume         (mL)
#   RR  = respiratory rate     (breaths/min)
#   VD  = anatomic dead space  (mL)
TABLES = [
    (500, 12, 150),
    (600, 14, 160),
    (450, 16, 140),
    (550, 10, 150),
    (700, 15, 170),
    (400, 18, 130),
    (650, 11, 155),
]

# The option family (5 members), all built from the observed quantities tv/rr/vd via `+ - *`.
#   key -> (display name, formula-as-adj)
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("mv", "minute ventilation (MV)", "tv * rr"),
    ("va", "alveolar ventilation (VA)", "(tv - vd) * rr"),
    ("vdm", "dead-space ventilation per minute", "vd * rr"),
    ("va_wrong", "dead-space-added ventilation", "(tv + vd) * rr"),
    ("alv_breath", "alveolar volume per breath", "tv - vd"),
]
QUERIED = ["mv", "va", "vdm"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(tv, rr, vd):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "mv": tv * rr,
        "va": (tv - vd) * rr,
        "vdm": vd * rr,
        "va_wrong": (tv + vd) * rr,
        "alv_breath": tv - vd,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for tv, rr, vd in TABLES:
        fv = family_values(tv, rr, vd)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (tv, rr, vd, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, tv, rr, vd, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r17av-{idx + 1:02d}",
                "qtype": "ventilation_parameter",
                "stem": (
                    f"A ventilation assessment shows a tidal volume of {tv} mL, a respiratory rate "
                    f"of {rr} breaths/min, and an anatomic dead space of {vd} mL. What is the "
                    f"patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe tv({tv})\n"
                    f"observe rr({rr})\n"
                    f"observe vd({vd})\n"
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
            "ADJ-LADDER rung 17 — respiratory ventilation parameters from a single bedside "
            "assessment (a NEW organ system: pulmonary mechanics). From three stated quantities "
            "(tidal volume TV, respiratory rate RR, anatomic dead space VD) compute the minute "
            "ventilation (MV = TV*RR), alveolar ventilation (VA = (TV-VD)*RR), or dead-space "
            "ventilation per minute (VDm = VD*RR). Each item is a compute_dimensioned program "
            "(observe the three quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a subtraction combined with a product — and the harness matches the scalar "
            "to the printed options. Contamination-safe: every parameter is built only from the "
            "three observed quantities via + - * — no constant leaks. The five options are a family "
            "over the same quantities, so the distractors are exactly the slips students make: adding "
            "the dead space instead of subtracting it, or dropping the multiply-by-rate."
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
