"""Generate rung-38 (serum anion gap) items.json for the ADJ-LADDER.

Rung 38 opens the **acid-base / serum-electrolyte** panel on the quantitative band — the arithmetic of the serum
anion gap, the single most-computed number at the bedside for sorting out a metabolic acidosis. The anion gap
asks: of the sodium cations in serum, how many are balanced by the two anions the lab routinely measures
(chloride and bicarbonate)? Whatever is left over is the "gap" — the unmeasured anions (lactate, ketoacids,
etc.). You take the sodium and subtract BOTH measured anions from it. It uses the same contamination-safe shape
as the CSF:serum rung (37), the transfusion-pooling rung (36) and the dialysis rung (35): a small table of
*observed* electrolytes and a tight family of mutually-confusable formulas built **only from those observed
quantities** (no numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a basic metabolic panel. THREE quantities are measured — all in mEq/L:

  SODIUM        the serum sodium cation                (high — the dominant cation)
  CHLORIDE      the serum chloride anion               (the first measured anion)
  BICARBONATE   the serum bicarbonate anion            (the second measured anion)

The serum anion gap is **sodium with both measured anions subtracted from it** — a *three-term successive
difference* — `sodium - chloride - bicarbonate`. That is what makes this rung distinctive: it is a NEW arithmetic
shape on the ladder — a single running subtraction of THREE observed terms, `a - b - c`. Every prior rung
composed exactly TWO sub-expressions (rung-31 subtracted one difference from another, rung-36 divided a
sum-of-products by a sum, rung-37 divided one sum by another); this rung chains two subtractions across three
raw observed quantities with no interior grouping — the associativity of a left-folded difference is the whole
point. The core confusion this rung tests is subtracting BOTH anions from the sodium (rather than adding the two
anions, subtracting only one, or crossing the wrong pair):

  ANION GAP            sodium - chloride - bicarbonate   [ sodium minus BOTH measured anions — the unmeasured-anion gap ]
  MEASURED ANIONS      chloride + bicarbonate            [ the two measured anions added — the amount subtracted from Na ]
  SODIUM - CHLORIDE    sodium - chloride                 [ only the first anion subtracted (the partial difference) ]

Each index is a `compute_dimensioned` program (observe the three quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/36/37. This rung exercises the engine across a
LEFT-FOLDED chain of TWO subtractions (`(sodium - chloride) - bicarbonate`).

Contamination-safe by construction: every formula is built only from the three observed quantities via `-` and
`+` — **no structural constants** — so every program literal is grounded in the stem. The gap itself never
appears as a literal (it is computed from the observed electrolytes). The observed quantities carry **digit-free
identifiers** (`sodium`, `chloride`, `bicarbonate`) so no numeral hides inside a variable name. The five options
are a tight family over the same quantities: the three real indices plus the two classic slips —

  SODIUM - BICARB   sodium - bicarbonate               the WRONG anion subtracted (bicarbonate instead of chloride), and
  ALL SUMMED        sodium + chloride + bicarbonate    the anions ADDED to the sodium instead of subtracted,

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the anion gap is small (order 8-16 mEq/L — sodium is only slightly more than the two anions
combined), the measured-anion sum is order 120-135, the sodium-minus-chloride partial difference is order 35, the
sodium-minus-bicarbonate distractor is order 115, and the all-summed distractor is order 260; the tables below
are chosen so the five family values are pairwise distinct — with a comfortable margin — for every item, asserted
at build time (all three electrolytes positive and the gap strictly positive, and no two family values collide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (SODIUM, CHLORIDE, BICARBONATE) observed serum electrolytes, all in mEq/L. Sodium is the dominant cation; the
# gap sodium - chloride - bicarbonate stays in the physiologic 7-18 band. All three are strictly positive and the
# gap is strictly positive. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (140, 104, 24),
    (138, 100, 22),
    (142, 108, 26),
    (136, 98, 20),
    (145, 110, 25),
    (139, 102, 23),
    (141, 106, 28),
]

# The option family (5 members), all built from the observed quantities via `-` / `+`. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "anion_gap",
        "serum anion gap",
        "sodium - chloride - bicarbonate",
    ),
    (
        "measured_anions",
        "sum of the measured anions",
        "chloride + bicarbonate",
    ),
    (
        "sodium_minus_chloride",
        "sodium with only chloride subtracted",
        "sodium - chloride",
    ),
    (
        "sodium_minus_bicarb",
        "sodium with the wrong anion (bicarbonate) subtracted",
        "sodium - bicarbonate",
    ),
    (
        "all_summed",
        "sodium and both anions all added together",
        "sodium + chloride + bicarbonate",
    ),
]
QUERIED = ["anion_gap", "measured_anions", "sodium_minus_chloride"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(na, cl, hco3):
    # Operation order mirrors the ADJ program exactly (a left-folded difference: (na - cl) - hco3), so the
    # Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match
    # tolerance).
    return {
        "anion_gap": na - cl - hco3,
        "measured_anions": cl + hco3,
        "sodium_minus_chloride": na - cl,
        "sodium_minus_bicarb": na - hco3,
        "all_summed": na + cl + hco3,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for na, cl, hco3 in TABLES:
        assert na > 0 and cl > 0 and hco3 > 0, (na, cl, hco3)
        assert na - cl - hco3 > 0, (na, cl, hco3)
        fv = family_values(na, cl, hco3)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (na, cl, hco3, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, na, cl, hco3, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r38ag-{idx + 1:02d}",
                "qtype": "serum_anion_gap",
                "stem": (
                    f"A basic metabolic panel shows a serum sodium of {num(na)} mEq/L, a chloride of "
                    f"{num(cl)} mEq/L, and a bicarbonate of {num(hco3)} mEq/L. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe sodium({num(na)})\n"
                    f"observe chloride({num(cl)})\n"
                    f"observe bicarbonate({num(hco3)})\n"
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
            "ADJ-LADDER rung 38 — serum anion gap from three stated serum electrolytes (a NEW panel: acid-base / "
            "serum-electrolyte analysis). From three stated values (sodium, chloride, bicarbonate) compute the "
            "anion gap (sodium-chloride-bicarbonate), the measured-anion sum (chloride+bicarbonate), or the "
            "sodium-minus-chloride partial difference (sodium-chloride). Each item is a compute_dimensioned "
            "program (observe the three quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW shape, a THREE-TERM SUCCESSIVE DIFFERENCE (sodium-chloride-bicarbonate), a "
            "left-folded chain of two subtractions with no interior grouping — and the harness matches the scalar "
            "to the printed options. Contamination-safe: every index is built only from the three observed "
            "electrolytes via - and + — no constant leaks (the anion gap is a pure running subtraction), and the "
            "gap itself never appears as a literal (it is computed from the observed electrolytes) — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same quantities, so the distractors are exactly the slips students "
            "make: subtracting the WRONG anion (sodium-bicarbonate, using bicarbonate in place of chloride) and "
            "ADDING the two anions to the sodium (sodium+chloride+bicarbonate) instead of subtracting them. The "
            "core confusion tested is subtracting BOTH measured anions from the sodium."
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
