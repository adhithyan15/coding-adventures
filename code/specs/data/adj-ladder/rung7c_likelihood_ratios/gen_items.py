"""Generate rung-7c (likelihood ratios) items.json for the ADJ-LADDER.

Rung 7c is the third biostatistics batch — it **completes the 2×2 diagnostic-table family**
begun in rung-7b. Where 7b asked for the within-table characteristics (sensitivity, specificity,
PPV, NPV, accuracy), 7c asks for the **likelihood ratios**, the numbers that actually move a
pre-test probability to a post-test probability at the bedside:

  LR+  (positive likelihood ratio) = sensitivity / (1 − specificity)
  LR−  (negative likelihood ratio) = (1 − sensitivity) / specificity
  DOR  (diagnostic odds ratio)     = LR+ / LR−

The textbook forms above contain the constant `1`, which would leak a non-stem literal into the
program. We avoid it entirely by writing each ratio in its **raw-count, division-only** form —
algebraically identical, but built from nothing but the four observed cell counts:

  LR+  = (TP / (TP + FN)) / (FP / (FP + TN))     # true-positive rate ÷ false-positive rate
  LR−  = (FN / (TP + FN)) / (TN / (FP + TN))     # false-negative rate ÷ true-negative rate
  DOR  = (TP / FP) / (FN / TN)                   # = (TP·TN)/(FN·FP)

Each is a `compute_dimensioned` program (observe the four counts + `let answer = formula`); the
ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change.

Contamination-safe by construction: every formula is built only from the four stated counts via
addition and division — **no structural constants** (not even the `1` of the textbook forms) — so
every program literal is grounded in the stem. The five options are a tight family of LR-type
ratios over the same table: the three real measures {LR+, LR−, DOR} plus the two classic
inversions {inverted-LR+ = (1−spec)/sens, inverse-DOR = LR−/LR+}. The distractors are therefore
exactly the slips a student makes — putting the false-positive rate on top, or flipping the odds
ratio. Gold rotates A–E by index.

Note on table choice: `LR+` collapses to the naive ratio `TP/FP` whenever the two column totals
(TP+FN) and (FP+TN) are equal, so the inversions — not `TP/FP` — are used as distractors; the
five family values are pairwise distinct for every table here (asserted at build time).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TP, FN, FP, TN) cell counts. The five LR-family values are asserted pairwise-distinct below.
TABLES = [
    (90, 10, 20, 80),
    (80, 20, 10, 90),
    (60, 40, 30, 70),
    (45, 5, 15, 35),
    (70, 30, 20, 80),
    (50, 50, 25, 75),
    (88, 12, 22, 78),
]

# The option family (5 members), all division-only over the observed counts tp/fn/fp/tn.
#   key -> (display name, formula-as-adj)
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("lr_pos", "positive likelihood ratio (LR+)", "(tp / (tp + fn)) / (fp / (fp + tn))"),
    ("lr_neg", "negative likelihood ratio (LR−)", "(fn / (tp + fn)) / (tn / (fp + tn))"),
    ("dor", "diagnostic odds ratio (DOR)", "(tp / fp) / (fn / tn)"),
    ("lr_pos_inv", "inverted positive likelihood ratio", "(fp / (fp + tn)) / (tp / (tp + fn))"),
    ("dor_inv", "inverse diagnostic odds ratio", "(fn / tn) / (tp / fp)"),
]
QUERIED = ["lr_pos", "lr_neg", "dor"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(tp, fn, fp, tn):
    return {
        "lr_pos": (tp / (tp + fn)) / (fp / (fp + tn)),
        "lr_neg": (fn / (tp + fn)) / (tn / (fp + tn)),
        "dor": (tp / fp) / (fn / tn),
        "lr_pos_inv": (fp / (fp + tn)) / (tp / (tp + fn)),
        "dor_inv": (fn / tn) / (tp / fp),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for tp, fn, fp, tn in TABLES:
        fv = family_values(tp, fn, fp, tn)
        assert len({round(fv[k], 12) for k in ORDER}) == 5, (tp, fn, fp, tn, fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 12) for v in opts_vals}) == 5, (key, tp, fn, fp, tn, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r7lr-{idx + 1:02d}",
                "qtype": "diagnostic_likelihood_ratio",
                "stem": (
                    f"A diagnostic test yields {tp} true positives, {fn} false negatives, "
                    f"{fp} false positives, and {tn} true negatives. What is the test's "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe tp({tp})\n"
                    f"observe fn({fn})\n"
                    f"observe fp({fp})\n"
                    f"observe tn({tn})\n"
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
            "ADJ-LADDER rung 7c — likelihood ratios from a 2×2 table (the third biostatistics "
            "batch; completes the diagnostic-table family begun in rung-7b). From four stated cell "
            "counts (TP/FN/FP/TN) compute the positive likelihood ratio (LR+), negative likelihood "
            "ratio (LR−), or diagnostic odds ratio (DOR). Each item is a compute_dimensioned program "
            "(observe the counts, let answer = formula); the ADJ engine carries the arithmetic and "
            "the harness matches the scalar to the printed options. Contamination-safe: each ratio "
            "is written in its raw-count, division-only form — algebraically identical to the "
            "textbook sens/(1−spec) forms but using ONLY the four stated counts, so not even the "
            "constant 1 leaks. The five options are a family of LR-type ratios over the same table, "
            "so the distractors are exactly the inversions students confuse (false-positive rate on "
            "top; flipped odds ratio)."
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
