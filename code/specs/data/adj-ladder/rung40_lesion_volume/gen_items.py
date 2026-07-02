"""Generate rung-40 (rectangular bounding-box lesion volume) items.json for the ADJ-LADDER.

Rung 40 opens the **imaging / lesion volumetrics** panel on the quantitative band — the arithmetic of estimating a
mass's volume from its three measured dimensions. The simplest bedside estimate is the *rectangular
(bounding-box)* volume: multiply the three orthogonal calliper measurements (length x width x depth). It uses the
same contamination-safe shape as the CBC rung (39), the anion-gap rung (38) and the CSF:serum rung (37): a small
table of *observed* dimensions and a tight family of mutually-confusable formulas built **only from those observed
quantities** (no numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a lesion measured on imaging. THREE orthogonal dimensions are recorded — all in cm:

  LENGTH   the longest axis            (cm)
  WIDTH    the perpendicular in-plane axis   (cm)
  DEPTH    the out-of-plane / craniocaudal axis   (cm)

The rectangular bounding-box volume is **the product of all three dimensions** — a *three-term product* —
`LENGTH * WIDTH * DEPTH`. That is what makes this rung distinctive: it is a NEW arithmetic shape on the ladder — a
flat product of THREE observed terms, `a * b * c`, all multiplication. It completes the same-operator trio: rung-38
was a flat three-term SUBTRACTION (`a - b - c`), rung-39 a flat three-term SUM (`a + b + c`), and this is the flat
three-term PRODUCT (`a * b * c`). (Deliberately the plain rectangular estimate, with NO shape constant — the
ellipsoid volume would multiply by pi/6 approximately 0.52, a structural constant we avoid.) The core confusion
this rung tests is multiplying ALL THREE dimensions (rather than a single cross-sectional pair, or adding them):

  BOX VOLUME    LENGTH * WIDTH * DEPTH   [ all three dimensions multiplied — the bounding-box volume ]
  LENGTH*WIDTH  LENGTH * WIDTH           [ the in-plane cross-sectional area (a partial product) ]
  WIDTH*DEPTH   WIDTH * DEPTH            [ a different cross-sectional area (a partial product) ]

Each index is a `compute_dimensioned` program (observe the three quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/38/39. This rung exercises the engine across a
flat THREE-TERM MULTIPLICATION (`(length * width) * depth`).

Contamination-safe by construction: every formula is built only from the three observed quantities via `*` and
`+` — **no structural constants** — so every program literal is grounded in the stem. No volume or area ever
appears as a literal (each is computed from the observed dimensions). The observed quantities carry **digit-free
identifiers** (`length`, `width`, `depth`) so no numeral hides inside a variable name. The five options are a tight
family over the same quantities: the three real products plus the two classic slips —

  LENGTH*DEPTH   LENGTH * DEPTH      the third cross-sectional pairing (a wrong two-of-three product), and
  ALL SUMMED     LENGTH + WIDTH + DEPTH   the dimensions ADDED instead of multiplied,

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: with dimensions of a few cm the box volume is order tens of cm^3, each cross-sectional-area product
is order 6-30 cm^2, and the all-summed distractor is order 10-15 cm; the tables below (three DISTINCT dimensions
each) are chosen so the five family values are pairwise distinct — with a comfortable margin — for every item,
asserted at build time (all three dimensions positive so every product and the sum are positive, and no two family
values collide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (LENGTH, WIDTH, DEPTH) observed lesion dimensions, all in cm, all strictly positive and pairwise DISTINCT so the
# cross-sectional-area products and the sum stay well separated. The five family values are asserted
# pairwise-distinct (with margin) below.
TABLES = [
    (6, 4, 3),
    (4, 2, 3),
    (7, 3, 2),
    (5, 2, 6),
    (8, 3, 4),
    (6, 5, 2),
    (4, 7, 3),
]

# The option family (5 members), all built from the observed quantities via `*` / `+`. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "box_volume",
        "rectangular bounding-box volume (all three dimensions multiplied)",
        "length * width * depth",
    ),
    (
        "length_width",
        "in-plane cross-sectional area (length times width only)",
        "length * width",
    ),
    (
        "width_depth",
        "cross-sectional area from width and depth only",
        "width * depth",
    ),
    (
        "length_depth",
        "cross-sectional area from length and depth only (a wrong two-of-three product)",
        "length * depth",
    ),
    (
        "all_summed",
        "the three dimensions added together instead of multiplied",
        "length + width + depth",
    ),
]
QUERIED = ["box_volume", "length_width", "width_depth"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(length, width, depth):
    # Operation order mirrors the ADJ program exactly (a left-folded product: (length * width) * depth), so the
    # Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match
    # tolerance).
    return {
        "box_volume": length * width * depth,
        "length_width": length * width,
        "width_depth": width * depth,
        "length_depth": length * depth,
        "all_summed": length + width + depth,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for length, width, depth in TABLES:
        assert length > 0 and width > 0 and depth > 0, (length, width, depth)
        fv = family_values(length, width, depth)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (length, width, depth, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, length, width, depth, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r40vol-{idx + 1:02d}",
                "qtype": "lesion_volume",
                "stem": (
                    f"A lesion is measured on imaging with a length of {num(length)} cm, a width of "
                    f"{num(width)} cm, and a depth of {num(depth)} cm. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe length({num(length)})\n"
                    f"observe width({num(width)})\n"
                    f"observe depth({num(depth)})\n"
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
            "ADJ-LADDER rung 40 — rectangular bounding-box lesion volume from three stated dimensions (a NEW "
            "panel: imaging / lesion volumetrics). From three stated dimensions (length, width, depth) compute "
            "the box volume (length*width*depth), the length*width cross-sectional area, or the width*depth "
            "cross-sectional area. Each item is a compute_dimensioned program (observe the three quantities, "
            "let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, a flat THREE-TERM "
            "PRODUCT (length*width*depth), all multiplication (completing the same-operator trio: rung-38 was a-b-c, "
            "rung-39 a+b+c, this is a*b*c) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the three observed dimensions via * and + — no "
            "constant leaks (the rectangular estimate has no shape constant, unlike the ellipsoid's pi/6), and no "
            "volume or area ever appears as a literal (each is computed from the observed dimensions) — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same quantities, so the distractors are exactly the slips students make: "
            "a wrong two-of-three product (length*depth) and the dimensions ADDED instead of multiplied "
            "(length+width+depth). The core confusion tested is multiplying ALL THREE dimensions."
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
