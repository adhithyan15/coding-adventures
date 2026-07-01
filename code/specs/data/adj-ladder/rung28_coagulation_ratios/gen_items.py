"""Generate rung-28 (coagulation ratios) items.json for the ADJ-LADDER.

Rung 28 opens the **coagulation / hemostasis** panel on the quantitative band — the arithmetic of the
routine clotting screen (PT / aPTT), where a prolonged prothrombin time relative to its normal control
is reported as a *ratio* (the dimensionless basis of the INR, before the ISI power correction). It uses
the same contamination-safe shape as rungs 23/25/26/27: a small table of *observed* laboratory
quantities and a tight family of mutually-confusable formulas built **only from those observed
quantities** (no numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a single coagulation panel. Three times are measured (all in seconds):

  PT       prothrombin time                (extrinsic / common pathway)
  APTT     activated partial thromboplastin time  (intrinsic / common pathway)
  CONTROL  the laboratory's normal PT control

The textbook indices fall out as pure functions of the observed quantities — no constant required. The
core confusion this rung tests is **the ratio vs the fractional prolongation**: PT/CONTROL (the PT
ratio) versus (PT − CONTROL)/CONTROL (the *fractional* prolongation, i.e. how much longer than baseline
as a fraction) — the two differ by exactly 1, and mixing them up is a classic student slip. This makes
the rung mix **division** with a **subtraction-in-the-numerator fraction** (the shape rung-24 used for
oxygen extraction), on a fresh organ system, and contrasts the two directly:

  PT RATIO               PT / CONTROL              [ the ratio underlying the INR ]
  PT FRACTIONAL PROLONG. (PT - CONTROL) / CONTROL  [ = PT ratio - 1; the *extra* fraction over baseline ]
  PT:APTT RATIO          PT / APTT                 [ which pathway is relatively more prolonged ]

Each index is a `compute_dimensioned` program (observe the three quantities + `let answer = formula`);
the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/…/26/27.

Contamination-safe by construction: every formula is built only from the three observed quantities via
`-`, `/` — **no structural constants** — so every program literal is grounded in the stem. The observed
quantities carry **digit-free identifiers** (`prothrombin_time`, `partial_thromboplastin_time`,
`control_time`) so no numeral hides inside a variable name. The five options are a tight family over the
same quantities: the three real indices plus the two classic slips —

  APTT / PT              the *inverted* PT:aPTT ratio (right analytes, upside-down), and
  PT - CONTROL          the *absolute* prolongation in seconds (forgetting to divide by the control),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the ratios and the fractional prolongation are order-1, while the absolute difference
(PT − CONTROL) lives on the seconds scale (tens), so it never collides with them; PT ratio and its
fractional prolongation differ by exactly 1 (never equal); the PT:aPTT ratio and its inverse are
distinct whenever PT ≠ aPTT. The tables below are chosen so the five family values are pairwise distinct
— with a comfortable margin — for every item, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PT, APTT, CONTROL) observed times, all in seconds. CONTROL is the normal PT control (used only by the
# PT-vs-control indices); APTT enters only the dimensionless PT:aPTT comparison, so no aPTT control is
# needed. The five index-family values are asserted pairwise-distinct (with margin) below.
#   PT      = prothrombin time
#   APTT    = activated partial thromboplastin time
#   CONTROL = normal PT control
TABLES = [
    (18, 40, 12),
    (15, 30, 12),
    (24, 60, 12),
    (16, 32, 10),
    (20, 50, 10),
    (14, 35, 12),
    (21, 60, 14),
]

# The option family (5 members), all built from the observed quantities via `-` / `/`. Every identifier
# is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as
# gold); all five always appear as the options.
FAMILY = [
    ("pt_ratio", "PT ratio", "prothrombin_time / control_time"),
    ("pt_fractional_prolongation", "PT fractional prolongation",
     "(prothrombin_time - control_time) / control_time"),
    ("pt_aptt_ratio", "PT-to-aPTT ratio", "prothrombin_time / partial_thromboplastin_time"),
    ("aptt_pt_ratio", "aPTT-to-PT ratio", "partial_thromboplastin_time / prothrombin_time"),
    ("pt_control_diff", "absolute PT prolongation", "prothrombin_time - control_time"),
]
QUERIED = ["pt_ratio", "pt_fractional_prolongation", "pt_aptt_ratio"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(pt, aptt, control):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "pt_ratio": pt / control,
        "pt_fractional_prolongation": (pt - control) / control,
        "pt_aptt_ratio": pt / aptt,
        "aptt_pt_ratio": aptt / pt,
        "pt_control_diff": pt - control,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for pt, aptt, control in TABLES:
        assert pt != aptt and control != 0, (pt, aptt, control)  # ratios well-defined & non-degenerate
        fv = family_values(pt, aptt, control)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (pt, aptt, control, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, pt, aptt, control, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r28coag-{idx + 1:02d}",
                "qtype": "coagulation_ratio",
                "stem": (
                    f"A coagulation panel shows a prothrombin time of {pt} s, an activated partial "
                    f"thromboplastin time of {aptt} s, and a normal PT control of {control} s. What is "
                    f"the patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe prothrombin_time({pt})\n"
                    f"observe partial_thromboplastin_time({aptt})\n"
                    f"observe control_time({control})\n"
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
            "ADJ-LADDER rung 28 — coagulation ratios from a single clotting screen (a NEW panel: "
            "coagulation / hemostasis). From three stated times (prothrombin time PT, activated partial "
            "thromboplastin time APTT, normal PT control CONTROL) compute the PT ratio (PT/CONTROL), the "
            "PT fractional prolongation ((PT-CONTROL)/CONTROL), or the PT:aPTT ratio (PT/APTT). Each item "
            "is a compute_dimensioned program (observe the three times, let answer = formula); the ADJ "
            "engine carries the arithmetic — mixing DIVISION with a SUBTRACTION-IN-THE-NUMERATOR fraction "
            "on a fresh organ system — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the three observed quantities via - and / "
            "— no constant leaks — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same quantities, so the "
            "distractors are exactly the slips students make: the inverted PT:aPTT ratio (aPTT/PT) and the "
            "absolute prolongation in seconds (PT-CONTROL, forgetting to divide by the control). The core "
            "confusion tested is the ratio vs the fractional prolongation — PT/CONTROL versus "
            "(PT-CONTROL)/CONTROL, which differ by exactly 1."
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
