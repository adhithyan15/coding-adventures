"""Generate rung-7 (EBM risk/effect measures) items.json for the ADJ-LADDER.

Rung 7 is a NEW reasoning kind on the ladder: evidence-based-medicine effect measures —
**absolute risk reduction** (ARR = CER − EER), **relative risk** (RR = EER / CER), and
**relative risk reduction** (RRR = (CER − EER) / CER) — computed from two stated event rates
(CER = control event rate, EER = experimental/treated event rate). These are core biostatistics
board numbers and the next step past rung-6's clinical reasoning toward full board coverage.

Each item is `compute_dimensioned` (rung-4 machinery): the program `observe`s the two rates and
`let answer = <formula>`; the engine carries the (dimensionless) arithmetic and the harness reads
the scalar from the `derived` section and matches it to the printed options. Python never computes
the value — it only compares the engine's output to the choices.

Contamination-safe by construction: the only numeric literals in a program are the two stated
rates (no structural constants — ARR/RR/RRR need none), so every literal is grounded in the stem.

The five options per item are the five natural quantities {ARR, RR, RRR, CER, EER}: the gold is the
queried measure and the four distractors are the OTHER quantities — exactly the confusions a student
makes (reporting RR when asked for ARR, or the bare control rate). Gold rotates A–E by index.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (control_event_rate, experimental_event_rate); CER > EER (a beneficial treatment). Chosen so
# the five quantities {ARR, RR, RRR, CER, EER} are pairwise distinct (asserted below).
RATE_PAIRS = [
    (0.40, 0.10),
    (0.50, 0.20),
    (0.50, 0.10),
    (0.80, 0.20),
    (0.75, 0.25),
    (0.90, 0.30),
    (0.60, 0.20),
]

# (key, name, formula-as-adj, phrase, python fn) for the three measures.
MEASURES = [
    ("arr", "absolute risk reduction (ARR)", "control - treated",
     lambda c, t: c - t),
    ("rr", "relative risk (RR) of the treated group versus control", "treated / control",
     lambda c, t: t / c),
    ("rrr", "relative risk reduction (RRR)", "(control - treated) / control",
     lambda c, t: (c - t) / c),
]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def quantities(c, t):
    """The five natural quantities for a rate pair, as a dict keyed by short name."""
    return {
        "arr": c - t,
        "rr": t / c,
        "rrr": (c - t) / c,
        "cer": c,
        "eer": t,
    }


def build():
    items = []
    idx = 0
    for c, t in RATE_PAIRS:
        q = quantities(c, t)
        vals = list(q.values())
        # the five quantities must be pairwise distinct so the options are non-degenerate
        assert len({round(v, 12) for v in vals}) == 5, (c, t, vals)
        for key, name, formula, fn in MEASURES:
            gold_val = fn(c, t)
            # options = the five natural quantities, in a fixed order; gold rotates A–E.
            order = ["arr", "rr", "rrr", "cer", "eer"]
            gold_pos = idx % 5
            # place the gold value at gold_pos, the other four (in order) around it
            others = [q[k] for k in order if abs(q[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 12) for v in opts_vals}) == 5, (key, c, t, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            gold_letter = LETTERS[gold_pos]
            iid = f"r7rm-{idx + 1:02d}"
            items.append({
                "id": iid,
                "qtype": "risk_measure",
                "stem": (
                    f"In a randomized trial the event rate was {c} in the control group and "
                    f"{t} in the treated group. What is the {name}?"
                ),
                "program": (
                    f"observe control({c})\n"
                    f"observe treated({t})\n"
                    f"let answer = {formula}\n"
                    "? answer\n"
                ),
                "answer_from": {"type": "compute_dimensioned", "name": "answer"},
                "options": options,
                "gold_letter": gold_letter,
            })
            idx += 1
    return {
        "description": (
            "ADJ-LADDER rung 7 — evidence-based-medicine effect measures (ARR / RR / RRR) computed "
            "from two stated event rates. A NEW reasoning kind: biostatistics. Each item is a "
            "compute_dimensioned program (observe the two rates, let answer = formula); the ADJ "
            "engine carries the arithmetic and the harness matches the scalar to the printed "
            "options. Contamination-safe: the only literals are the two stated rates (no structural "
            "constants). The five options are the five natural quantities {ARR, RR, RRR, CER, EER}, "
            "so the distractors are exactly the measures students confuse."
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
