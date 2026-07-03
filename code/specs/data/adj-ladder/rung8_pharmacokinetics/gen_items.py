"""Generate rung-8 (pharmacokinetics) items.json for the ADJ-LADDER.

Rung 8 opens a new quantitative clinical domain — **one-compartment pharmacokinetics** — and
keeps the same contamination-safe shape the 2×2 biostatistics family (rungs 7/7b/7c) was built
on: a small table of *observed* quantities, and a tight family of mutually-confusable formulas
that are **pure division of those observed quantities** (no numeric literal anywhere).

The clinical setup is a single IV-bolus dose into a one-compartment model. We observe three
quantities:

  D    dose administered                              (mg)
  C0   initial plasma concentration (back-extrapolated to t=0)   (mg/L)
  AUC  total area under the concentration-time curve (mg·h/L)

For a one-compartment IV bolus, C(t) = C0·e^(−ke·t), so AUC = C0 / ke. From that single fact
the three textbook bedside parameters fall out as **pure ratios** of the observed quantities —
algebraically exact, and crucially needing no constant (not even the `0.693` of the half-life
formula, which is exactly why half-life is *not* in this rung):

  Vd  (volume of distribution)      = D  / C0     (L)
  CL  (clearance)                   = D  / AUC    (L/h)     [ = ke · Vd ]
  ke  (elimination rate constant)   = C0 / AUC    (1/h)     [ = CL / Vd, since AUC = C0/ke ]

Each is a `compute_dimensioned` program (observe the three quantities + `let answer = formula`);
the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 7/7b/7c.

Contamination-safe by construction: every formula is built only from the three observed
quantities via division — **no structural constants** — so every program literal is grounded in
the stem. The five options are a tight family of ratios over the same three quantities: the
three real parameters {Vd, CL, ke} plus the two classic inversions {1/CL = AUC/D, 1/Vd = C0/D}.
The distractors are therefore exactly the slips a student makes — inverting clearance (reading
AUC/dose) or inverting the volume (reading C0/dose). Gold rotates A–E by index.

Note on table choice: several of the five family values collide for special quantity ratios
(e.g. ke = C0/AUC equals 1/CL = AUC/D whenever C0·D = AUC²). The tables below are chosen so the
five family values are pairwise distinct for every item, and this is asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (D, C0, AUC) observed quantities. The five PK-family values are asserted pairwise-distinct below.
#   D   = dose (mg)
#   C0  = initial plasma concentration (mg/L)
#   AUC = area under the concentration-time curve (mg·h/L)
TABLES = [
    (500, 20, 80),
    (400, 25, 50),
    (600, 30, 120),
    (300, 15, 60),
    (800, 40, 100),
    (250, 10, 40),
    (900, 45, 150),
]

# The option family (5 members), all division-only over the observed quantities d/c0/auc.
#   key -> (display name, formula-as-adj)
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("vd", "volume of distribution (Vd)", "d / conc"),
    ("cl", "clearance (CL)", "d / auc"),
    ("ke", "elimination rate constant (ke)", "conc / auc"),
    ("inv_cl", "inverted clearance (AUC per unit dose)", "auc / d"),
    ("inv_vd", "inverted volume (initial concentration per unit dose)", "conc / d"),
]
QUERIED = ["vd", "cl", "ke"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(d, c0, auc):
    return {
        "vd": d / c0,
        "cl": d / auc,
        "ke": c0 / auc,
        "inv_cl": auc / d,
        "inv_vd": c0 / d,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for d, c0, auc in TABLES:
        fv = family_values(d, c0, auc)
        assert len({round(fv[k], 12) for k in ORDER}) == 5, (d, c0, auc, fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 12) for v in opts_vals}) == 5, (key, d, c0, auc, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r8pk-{idx + 1:02d}",
                "qtype": "pharmacokinetic_parameter",
                "stem": (
                    f"After a single IV bolus dose of {d} mg, a one-compartment model shows an "
                    f"initial plasma concentration of {c0} mg/L and a total area under the "
                    f"concentration-time curve (AUC) of {auc} mg*h/L. What is the drug's "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe d({d})\n"
                    f"observe conc({c0})\n"
                    f"observe auc({auc})\n"
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
            "ADJ-LADDER rung 8 — one-compartment pharmacokinetic parameters from a single IV bolus. "
            "From three stated quantities (dose D, initial concentration C0, area-under-curve AUC) "
            "compute the volume of distribution (Vd = D/C0), clearance (CL = D/AUC), or elimination "
            "rate constant (ke = C0/AUC). Each item is a compute_dimensioned program (observe the "
            "three quantities, let answer = formula); the ADJ engine carries the arithmetic and the "
            "harness matches the scalar to the printed options. Contamination-safe: every parameter "
            "is a pure ratio of the three observed quantities — no constant leaks (not even the 0.693 "
            "of the half-life formula, which is why half-life is excluded). The five options are a "
            "family of ratios over the same quantities, so the distractors are exactly the inversions "
            "students confuse (reading AUC/dose for clearance; C0/dose for volume)."
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
