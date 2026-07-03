"""Generate rung-52 (alveolar ventilation) items.json for the ADJ-LADDER.

Rung 52 opens the **respiratory / ventilation-mechanics** panel on the quantitative band — the arithmetic of how much
FRESH gas actually reaches the alveoli each minute. Not every millilitre of a breath participates in gas exchange: the
**dead space** (the conducting airways) is re-breathed, so only `tidal_volume − dead_space` is fresh alveolar gas per
breath, and that fresh volume delivered at the respiratory rate is the **alveolar ventilation**. Scaling a
parenthesised difference by a lone factor introduces a genuinely NEW arithmetic shape on the ladder: **distributive
difference-times-one** — `(a − b) · c` — a parenthesised difference multiplied by a single factor. Contrast rungs
49/50, which multiplied a lone factor by a parenthesised sum/difference the OTHER way round (`a·(b+c)`, `a·(b−c)`);
here the parenthesised difference comes FIRST and the factor scales it.

The setup: a patient breathes a tidal volume of `tidal_volume`, of which `dead_space` is dead space, at a respiratory
rate of `respiratory_rate`. The alveolar ventilation is the fresh volume per breath times the rate:

  ALVEOLAR VENTILATION   (tidal_volume − dead_space) · respiratory_rate   [ the fresh gas reaching alveoli per minute ]
  FRESH GAS PER BREATH    tidal_volume − dead_space                        [ the difference: fresh volume in one breath ]
  MINUTE VENTILATION      tidal_volume · respiratory_rate                  [ one distributed term: the UNcorrected total ]

The **alveolar ventilation** is what makes this rung distinctive — it is the ladder's first **distributive
difference-times-one**: a parenthesised difference scaled by a single factor. (The fresh gas per breath `tidal_volume −
dead_space` and the uncorrected minute ventilation `tidal_volume · respiratory_rate` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-51 shipped their component
sums/products/differences beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the three quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including the inner `(tidal_volume − dead_space)` difference — and the harness reads the scalar
via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../50/51. This rung
exercises the engine across a **parenthesised difference times a single factor** — the distributive law
`(a−b)·c = a·c − b·c` made computable.

Contamination-safe by construction: every formula is built ONLY from the three observed quantities via `·`, `−` and
`+` — **no structural constants** — so no numeric literal appears in any program, and neither the fresh gas per
breath, the minute ventilation, nor any alveolar-ventilation figure is ever a literal (each is computed from the
observed quantities). The observed quantities carry **digit-free identifiers** (`tidal_volume`, `dead_space`,
`respiratory_rate`) so no numeral hides inside a variable name.

The five options are a tight family over the same three quantities: the three real readouts plus the two classic
slips —

  SUM VERSION   (tidal_volume + dead_space) · respiratory_rate   ADD the dead space to the tidal volume instead of
                                                                 subtracting it (dead space must be REMOVED), and
  MISGROUPED    tidal_volume · respiratory_rate − dead_space      subtract the raw dead space once instead of scaling
                                                                 it by the rate (`− dead_space`, not `− dead_space ·
                                                                 respiratory_rate`) — the distributive law broken,

which are exactly the mistakes a student makes (adding a quantity that should be subtracted, or breaking the
distributive law by not scaling the subtracted term). Gold rotates A-E by index. QUERIED (used as gold) = the three
real readouts; all five always appear as options.

Distinctness: all three observed quantities are positive with `tidal_volume > dead_space` (dead space is only part of
the breath), so every product, sum and difference is positive; the tables below are chosen so the five family values
are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TIDAL_VOLUME, DEAD_SPACE, RESPIRATORY_RATE) — tidal volume and dead space as plain positive numbers (mL), rate as
# breaths per minute. All three strictly positive with TIDAL_VOLUME > DEAD_SPACE, so the fresh-gas difference is
# positive. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (500, 150, 12),
    (600, 200, 10),
    (450, 100, 15),
    (400, 120, 14),
    (550, 180, 11),
    (700, 250, 8),
    (480, 130, 13),
]

# The option family (5 members), all built from the three observed quantities via *, - and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "alveolar_ventilation",
        "alveolar ventilation (the fresh volume per breath times the rate)",
        "(tidal_volume - dead_space) * respiratory_rate",
    ),
    (
        "fresh_gas_per_breath",
        "the fresh gas per breath (tidal volume minus the dead space)",
        "tidal_volume - dead_space",
    ),
    (
        "minute_ventilation",
        "the total minute ventilation (tidal volume times the rate, before removing dead space)",
        "tidal_volume * respiratory_rate",
    ),
    (
        "sum_version",
        "tidal volume and dead space ADDED then times the rate, not subtracted (a wrong fresh volume)",
        "(tidal_volume + dead_space) * respiratory_rate",
    ),
    (
        "misgrouped",
        "minute ventilation minus the raw dead space, forgetting to scale it by the rate (broken distribution)",
        "tidal_volume * respiratory_rate - dead_space",
    ),
]
QUERIED = ["alveolar_ventilation", "fresh_gas_per_breath", "minute_ventilation"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(tidal_volume, dead_space, respiratory_rate):
    # Operation order mirrors the ADJ programs exactly (a parenthesised difference times a single factor), so the
    # Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match
    # tolerance).
    fresh = tidal_volume - dead_space
    return {
        "alveolar_ventilation": fresh * respiratory_rate,
        "fresh_gas_per_breath": fresh,
        "minute_ventilation": tidal_volume * respiratory_rate,
        "sum_version": (tidal_volume + dead_space) * respiratory_rate,
        "misgrouped": tidal_volume * respiratory_rate - dead_space,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for tidal_volume, dead_space, respiratory_rate in TABLES:
        assert (
            tidal_volume > 0
            and dead_space > 0
            and respiratory_rate > 0
            and tidal_volume > dead_space
        ), (tidal_volume, dead_space, respiratory_rate)
        fv = family_values(tidal_volume, dead_space, respiratory_rate)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    tidal_volume,
                    dead_space,
                    respiratory_rate,
                    ORDER[i],
                    ORDER[j],
                    fv,
                )
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (
                key,
                tidal_volume,
                dead_space,
                respiratory_rate,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r52alv-{idx + 1:02d}",
                "qtype": "alveolar_ventilation",
                "stem": (
                    f"A patient breathes a tidal volume of {num(tidal_volume)} mL, of which {num(dead_space)} mL is "
                    f"dead space, at a respiratory rate of {num(respiratory_rate)} breaths per minute. What is the "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe tidal_volume({num(tidal_volume)})\n"
                    f"observe dead_space({num(dead_space)})\n"
                    f"observe respiratory_rate({num(respiratory_rate)})\n"
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
            "ADJ-LADDER rung 52 — alveolar ventilation from three stated quantities (a NEW panel: respiratory / "
            "ventilation-mechanics). From a tidal volume, a dead space and a respiratory rate compute the alveolar "
            "ventilation ((tidal_volume-dead_space)*respiratory_rate), the fresh gas per breath "
            "(tidal_volume-dead_space), or the total minute ventilation (tidal_volume*respiratory_rate). Each item is a "
            "compute_dimensioned program (observe the three quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW shape, DISTRIBUTIVE DIFFERENCE-TIMES-ONE (a-b)*c, the first on the ladder to scale a "
            "parenthesised difference by a single factor (distinct from rung-49 a*(b+c) and rung-50 a*(b-c), which put "
            "the lone factor FIRST) — and the harness matches the scalar to the printed options. Contamination-safe: "
            "every index is built only from the three observed quantities via *, - and + — no constant leaks, and "
            "neither the fresh gas per breath, the minute ventilation, nor any alveolar-ventilation figure ever appears "
            "as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same three quantities, so the "
            "distractors are exactly the slips students make: ADDING the dead space to the tidal volume instead of "
            "subtracting it, and breaking the distributive law by subtracting the raw dead space instead of the "
            "dead-space ventilation it removes (a*c-b, not (a-b)*c). The core confusion tested is distributing the rate "
            "over the fresh-gas difference."
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
