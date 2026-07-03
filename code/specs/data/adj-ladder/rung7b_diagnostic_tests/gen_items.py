"""Generate rung-7b (diagnostic test characteristics) items.json for the ADJ-LADDER.

Rung 7b is the second biostatistics batch (after rung-7's effect measures): the **2×2
diagnostic table**. From four stated cell counts — true positives (TP), false negatives (FN),
false positives (FP), true negatives (TN) — compute a test characteristic:

  sensitivity = TP / (TP + FN)            specificity = TN / (TN + FP)
  PPV         = TP / (TP + FP)            NPV         = TN / (TN + FN)
  accuracy    = (TP + TN) / (TP + TN + FP + FN)

These are core board numbers, and a USMLE classic trap: sensitivity vs PPV (and specificity vs
NPV) are constantly confused. Each item is a `compute_dimensioned` program (observe the four
counts + `let answer = formula`); the ADJ engine carries the arithmetic and the harness reads the
scalar via the existing `compute_dimensioned` extractor — no harness/engine change.

Contamination-safe by construction: every formula is built only from the four stated counts via
addition and division — **no structural constants** — so every program literal is grounded in the
stem. The five options are the five natural characteristics {sensitivity, specificity, PPV, NPV,
accuracy}: the gold is the queried one and the four distractors are the OTHER characteristics —
exactly the confusions a student makes. Gold rotates A–E by index.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TP, FN, FP, TN) cell counts, chosen so the five characteristics are pairwise distinct.
TABLES = [
    (90, 10, 20, 80),
    (80, 20, 10, 90),
    (60, 40, 30, 70),
    (45, 5, 15, 35),
    (70, 30, 20, 80),
]

# (key, name, formula-as-adj) — `answer` over the observed counts tp/fn/fp/tn.
MEASURES = [
    ("sensitivity", "sensitivity", "tp / (tp + fn)"),
    ("specificity", "specificity", "tn / (tn + fp)"),
    ("ppv", "positive predictive value (PPV)", "tp / (tp + fp)"),
    ("npv", "negative predictive value (NPV)", "tn / (tn + fn)"),
    ("accuracy", "accuracy", "(tp + tn) / (tp + tn + fp + fn)"),
]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def characteristics(tp, fn, fp, tn):
    return {
        "sensitivity": tp / (tp + fn),
        "specificity": tn / (tn + fp),
        "ppv": tp / (tp + fp),
        "npv": tn / (tn + fn),
        "accuracy": (tp + tn) / (tp + tn + fp + fn),
    }


def build():
    items = []
    idx = 0
    order = ["sensitivity", "specificity", "ppv", "npv", "accuracy"]
    for tp, fn, fp, tn in TABLES:
        ch = characteristics(tp, fn, fp, tn)
        assert len({round(ch[k], 12) for k in order}) == 5, (tp, fn, fp, tn, ch)
        for key, name, formula in MEASURES:
            gold_val = ch[key]
            gold_pos = idx % 5
            others = [ch[k] for k in order if abs(ch[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 12) for v in opts_vals}) == 5, (key, tp, fn, fp, tn, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r7dx-{idx + 1:02d}",
                "qtype": "diagnostic_test_characteristic",
                "stem": (
                    f"A diagnostic test yields {tp} true positives, {fn} false negatives, "
                    f"{fp} false positives, and {tn} true negatives. What is the test's {name}?"
                ),
                "program": (
                    f"observe tp({tp})\n"
                    f"observe fn({fn})\n"
                    f"observe fp({fp})\n"
                    f"observe tn({tn})\n"
                    f"let answer = {formula}\n"
                    "? answer\n"
                ),
                "answer_from": {"type": "compute_dimensioned", "name": "answer"},
                "options": options,
                "gold_letter": LETTERS[gold_pos],
            })
            idx += 1
    return {
        "description": (
            "ADJ-LADDER rung 7b — diagnostic test characteristics from a 2×2 table (the second "
            "biostatistics batch). From four stated cell counts (TP/FN/FP/TN) compute sensitivity, "
            "specificity, PPV, NPV, or accuracy. Each item is a compute_dimensioned program (observe "
            "the counts, let answer = formula); the ADJ engine carries the arithmetic and the harness "
            "matches the scalar to the printed options. Contamination-safe: every formula uses only "
            "the four stated counts via addition and division — no structural constants. The five "
            "options are the five natural characteristics, so the distractors are exactly the ones "
            "students confuse (sensitivity vs PPV, specificity vs NPV)."
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
              "=", it["options"][it["gold_letter"]]["value"])
