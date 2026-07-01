"""Generate rung-21 (renal azotemia indices) items.json for the ADJ-LADDER.

Rung 21 opens the **renal / acute-kidney-injury workup** on the quantitative band, using the same
contamination-safe shape as the pharmacokinetics rung (8), the earlier renal-ratio rungs (9/13/15),
the cardiology rungs (16/18), the hematology rung (19), and the hepatology rung (20): a small table
of *observed* laboratory quantities, and a tight family of mutually-confusable formulas built
**only from those observed quantities** (no numeric literal anywhere in any program), so nothing
structural can leak.

The clinical setup is the classic bedside question — is this azotemia **prerenal** (a volume/perfusion
problem, kidneys concentrating hard) or **intrinsic** (tubular injury, kidneys no longer concentrating)?
Four quantities are measured across blood and urine:

  BUN    blood urea nitrogen         (mg/dL)  — the serum nitrogenous waste
  CREAT  serum creatinine            (mg/dL)  — the serum filtration marker
  UUN    urine urea nitrogen         (mg/dL)  — urea concentrated in the urine
  UCR    urine creatinine            (mg/dL)  — creatinine concentrated in the urine

From those four, three textbook discriminators fall out as **pure ratios** of the observed
quantities — no constant required:

  BUN:Cr RATIO   serum BUN / serum creatinine   = BUN / CREAT   [ >20 suggests PRERENAL ]
  U/P UREA       urine-to-plasma urea nitrogen  = UUN / BUN     [ high → concentrating → prerenal ]
  U/P CREAT      urine-to-plasma creatinine     = UCR / CREAT   [ >40 → prerenal, <20 → intrinsic ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`);
the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/18/19/20. Unlike the
hepatology rung this family is **all division** (no inner sum) — a pure-ratio rung on a new panel,
matching rungs 16/19 — so it exercises the engine's `/` across a fresh clinical stem.

Contamination-safe by construction: every formula is built only from the four observed quantities via
`/` — **no structural constants** — so every program literal is grounded in the stem. The five options
are a tight family over the same quantities: the three real discriminators {BUN:Cr, U/P urea, U/P
creatinine} plus the two classic slips —

  CREAT / BUN   the **inverse** BUN:Cr ratio (the ratio written upside-down), and
  UUN / UCR     the urine urea-to-creatinine ratio (a plausible urine-only mix-up),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on table choice: the five family values can collide for special quantity ratios. The tables below
are chosen so the five family values are pairwise distinct — with a comfortable margin — for every
item, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BUN, CREAT, UUN, UCR) observed quantities. The five index-family values are asserted
# pairwise-distinct (with margin) below.
#   BUN   = blood urea nitrogen   (mg/dL)
#   CREAT = serum creatinine      (mg/dL)
#   UUN   = urine urea nitrogen   (mg/dL)
#   UCR   = urine creatinine      (mg/dL)
TABLES = [
    (40, 2, 200, 80),
    (60, 3, 180, 90),
    (30, 2, 150, 60),
    (80, 4, 320, 100),
    (50, 5, 250, 75),
    (36, 3, 216, 90),
    (45, 3, 135, 60),
]

# The option family (5 members), all built from the observed quantities bun/creat/uun/ucr via `/`.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    ("bun_creat", "BUN-to-creatinine ratio", "bun / creat"),
    ("up_urea", "urine-to-plasma urea nitrogen ratio", "uun / bun"),
    ("up_creat", "urine-to-plasma creatinine ratio", "ucr / creat"),
    ("creat_bun", "creatinine-to-BUN ratio", "creat / bun"),
    ("uun_ucr", "urine urea-to-creatinine ratio", "uun / ucr"),
]
QUERIED = ["bun_creat", "up_urea", "up_creat"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(bun, creat, uun, ucr):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "bun_creat": bun / creat,
        "up_urea": uun / bun,
        "up_creat": ucr / creat,
        "creat_bun": creat / bun,
        "uun_ucr": uun / ucr,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for bun, creat, uun, ucr in TABLES:
        fv = family_values(bun, creat, uun, ucr)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (bun, creat, uun, ucr, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, bun, creat, uun, ucr, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r21ren-{idx + 1:02d}",
                "qtype": "renal_index",
                "stem": (
                    f"A patient with acute kidney injury has blood urea nitrogen (BUN) {bun} mg/dL, "
                    f"serum creatinine {creat} mg/dL, urine urea nitrogen {uun} mg/dL, and urine "
                    f"creatinine {ucr} mg/dL. What is the patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe bun({bun})\n"
                    f"observe creat({creat})\n"
                    f"observe uun({uun})\n"
                    f"observe ucr({ucr})\n"
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
            "ADJ-LADDER rung 21 — renal azotemia indices from a blood-and-urine panel in acute kidney "
            "injury (the prerenal-vs-intrinsic workup). From four stated quantities (blood urea nitrogen "
            "BUN, serum creatinine CREAT, urine urea nitrogen UUN, urine creatinine UCR) compute the "
            "BUN:creatinine ratio (BUN/CREAT), the urine-to-plasma urea nitrogen ratio (UUN/BUN), or the "
            "urine-to-plasma creatinine ratio (UCR/CREAT). Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic "
            "(pure division) and the harness matches the scalar to the printed options. Contamination-safe: "
            "every index is built only from the four observed quantities via / — no constant leaks. The five "
            "options are a family over the same quantities, so the distractors are exactly the slips students "
            "make: the inverse BUN:Cr ratio (CREAT/BUN, written upside-down) and the urine urea-to-creatinine "
            "ratio (UUN/UCR, a urine-only mix-up)."
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
