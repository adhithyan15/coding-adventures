"""Generate rung-16 (cardiac hemodynamics) items.json for the ADJ-LADDER.

Rung 16 opens a **new organ system** on the quantitative band — **cardiology hemodynamics** —
using the same contamination-safe shape the pharmacokinetics rung (rung 8) and the renal-ratio
rungs (9/13/15) were built on: a small table of *observed* bedside quantities, and a tight family
of mutually-confusable formulas built **only from those observed quantities** (no numeric literal
anywhere in any program), so nothing structural can leak into the answer.

The clinical setup is a single hemodynamic assessment. We observe three quantities:

  SV   stroke volume        (mL)      — blood ejected by the left ventricle per beat
  HR   heart rate           (bpm)     — beats per minute
  BSA  body-surface area    (m^2)     — the DuBois patient-size normaliser

From those three, the three textbook bedside hemodynamic parameters fall out as **exact
combinations** of the observed quantities — no constant required:

  CO   cardiac output              = SV * HR          (mL/min)      [ volume/beat * beats/min ]
  CI   cardiac index               = SV * HR / BSA    (mL/min/m^2)  [ CO indexed to body size ]
  SVI  stroke-volume index         = SV / BSA         (mL/m^2)      [ SV indexed to body size ]

Unlike the renal rungs (pure division), this rung mixes a **product** (CO = SV*HR) with an
indexing **division** (÷BSA), exercising the engine's dimensional algebra in both directions —
exactly the rung4_products × rung9-ratios combination, now on a cardiology stem.

Each parameter is a `compute_dimensioned` program (observe the three quantities + `let answer =
formula`); the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rung 8.

Contamination-safe by construction: every formula is built only from the three observed
quantities via `*` and `/` — **no structural constants** — so every program literal is grounded
in the stem. The five options are a tight family over the same three quantities: the three real
parameters {CO, CI, SVI} plus the two classic slips —

  SV*BSA   "scaling up" the stroke volume by body size instead of indexing (dividing) by it, and
  HR/BSA   indexing the *heart rate* instead of the stroke volume,

which are exactly the mistakes a student makes (multiplying where they should divide, or indexing
the wrong quantity). Gold rotates A–E by index.

Note on table choice: the five family values can collide for special quantity ratios (e.g.
SVI = SV/BSA equals HR/BSA whenever SV = HR). The tables below are chosen so the five family
values are pairwise distinct — with a comfortable margin — for every item, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (SV, HR, BSA) observed quantities. The five hemodynamic-family values are asserted
# pairwise-distinct (with margin) below.
#   SV  = stroke volume      (mL)
#   HR  = heart rate         (bpm)
#   BSA = body-surface area  (m^2)
TABLES = [
    (70, 72, 1.7),
    (60, 80, 2.0),
    (80, 60, 1.9),
    (55, 90, 1.6),
    (90, 100, 2.1),
    (50, 66, 1.5),
    (75, 88, 1.8),
]

# The option family (5 members), all built from the observed quantities sv/hr/bsa via `*` and `/`.
#   key -> (display name, formula-as-adj)
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("co", "cardiac output (CO)", "sv * hr"),
    ("ci", "cardiac index (CI)", "sv * hr / bsa"),
    ("svi", "stroke-volume index (SVI)", "sv / bsa"),
    ("sv_bsa", "stroke volume scaled by body-surface area", "sv * bsa"),
    ("hr_bsa", "heart rate indexed to body-surface area", "hr / bsa"),
]
QUERIED = ["co", "ci", "svi"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(sv, hr, bsa):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "co": sv * hr,
        "ci": sv * hr / bsa,
        "svi": sv / bsa,
        "sv_bsa": sv * bsa,
        "hr_bsa": hr / bsa,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for sv, hr, bsa in TABLES:
        fv = family_values(sv, hr, bsa)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (sv, hr, bsa, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, sv, hr, bsa, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r16co-{idx + 1:02d}",
                "qtype": "cardiac_hemodynamic_parameter",
                "stem": (
                    f"A hemodynamic assessment shows a stroke volume of {sv} mL, a heart rate of "
                    f"{hr} bpm, and a body-surface area of {bsa} m^2. What is the patient's "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe sv({sv})\n"
                    f"observe hr({hr})\n"
                    f"observe bsa({bsa})\n"
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
            "ADJ-LADDER rung 16 — cardiac hemodynamic parameters from a single bedside assessment "
            "(a NEW organ system: cardiology). From three stated quantities (stroke volume SV, heart "
            "rate HR, body-surface area BSA) compute the cardiac output (CO = SV*HR), cardiac index "
            "(CI = SV*HR/BSA), or stroke-volume index (SVI = SV/BSA). Each item is a "
            "compute_dimensioned program (observe the three quantities, let answer = formula); the "
            "ADJ engine carries the arithmetic — a product AND an indexing division — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every parameter is built "
            "only from the three observed quantities via * and / — no constant leaks. The five options "
            "are a family over the same quantities, so the distractors are exactly the slips students "
            "make: multiplying by BSA instead of indexing (dividing) by it, or indexing the heart rate "
            "instead of the stroke volume."
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
