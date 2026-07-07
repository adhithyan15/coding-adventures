"""Generate rung-98 (vascular / segmental-limb-pressure gradient) items.json for the ADJ-LADDER.

Rung 98 opens the **vascular / segmental-limb-pressure** panel on the quantitative band — the arithmetic of a segmental
pressure gradient. A `proximal_cuff` pressure MINUS a `distal_cuff` pressure gives the two-cuff pressure drop (how much
pressure is lost between the two cuff sites), that pressure drop MULTIPLIES the `width_factor` (the cuff-width scaling
factor) into the scaled drop, and a `baseline_offset` is SUBTRACTED off to give the segmental gradient load. A **binomial
difference times a bare factor, minus a term** introduces a genuinely NEW arithmetic family on the ladder: `(a-b)*c-d`,
i.e. `(((a-b)*c)-d)`.

This is genuinely new. It is the **last open corner of the (a±b)*c±d quartet** — the binomial (sum or difference) times a
bare factor, plus-or-minus a trailing term. Rung-84 shipped `(a+b)*c-d` (sum × factor, MINUS a term), rung-96 shipped
`(a-b)*c+d` (difference × factor, PLUS a term), rung-97 shipped `(a+b)*c+d` (sum × factor, PLUS a term); rung-98 supplies
the final corner `(a-b)*c-d` (difference × factor, MINUS a term), completing the quartet. No prior shape multiplied a
binomial DIFFERENCE by a bare factor and SUBTRACTED a term: rung-96 added the trailing term, rung-67 `(a-b)*c/d` divided the
difference-product by a term (not subtract), rung-90 `(a-b)*(c-d)` multiplied a difference by a DIFFERENCE (both binomials),
rung-35 `a*b-c*d` subtracted two products. The operator order matters: `(a-b)*c-d` is `(((a-b)*c)-d)` (the difference forms
first inside its parentheses, then it is multiplied by the factor, then the term is subtracted), NOT `a-b*c-d` (dropping the
parentheses so the factor multiplies only the distal cuff) and NOT `(a-b)-c*d` (subtracting the bare pressure drop and
multiplying the factor into the baseline offset instead) — the two distractors exploit exactly those confusions.

The setup: a `proximal_cuff`, a `distal_cuff`, a `width_factor`, and a `baseline_offset`. The total is:

  SEGMENTAL GRADIENT LOAD  (proximal_cuff - distal_cuff) * width_factor - baseline_offset  [ a difference times a factor, minus a term ]
  PRESSURE DROP            proximal_cuff - distal_cuff                                      [ the difference, before the product ]
  SCALED DROP              (proximal_cuff - distal_cuff) * width_factor                     [ the difference times the factor, before subtracting the term ]

The **segmental gradient load** is what makes this rung distinctive — it is the ladder's first **binomial DIFFERENCE times
a bare factor, minus a trailing term**, the last corner of the (a±b)*c±d quartet (84 (a+b)*c-d, 96 (a-b)*c+d, 97 (a+b)*c+d,
98 (a-b)*c-d). (The pressure drop `a-b` and the scaled drop `(a-b)*c` ride alongside as component readouts, so the panel
teaches the whole calculation — exactly as rungs 47-97 shipped their component sums/products/differences/ratios beside the
headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the distal cuff from the proximal cuff into the pressure drop, the
multiplication of that drop by the width factor into the scaled drop, then the subtraction of the baseline offset (the
difference forming inside its parentheses before the product, and the product forming before the trailing subtraction, so
(a-b)*c-d evaluates as (((a-b)*c)-d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor.
No harness/engine change, exactly as rungs 8/16/.../96/97. This rung exercises the engine across a **difference times a
factor, minus a term** — the fact that `(a-b)*c-d` is `(((a-b)*c)-d)` and NOT `a-b*c-d` and NOT `(a-b)-c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-` and `*` — **no
structural constants** — so no numeric literal appears in any program, and neither the pressure drop, the scaled drop, nor
any gradient figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`proximal_cuff`, `distal_cuff`, `width_factor`, `baseline_offset`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    proximal_cuff - distal_cuff * width_factor - baseline_offset  drop the parentheses so the width factor
                                                                          multiplies only the distal cuff and the bare
                                                                          proximal cuff and baseline offset are left in the
                                                                          subtraction (the classic `(a-b)*c-d` vs `a-b*c-d`
                                                                          error), and
  SWAPPED    (proximal_cuff - distal_cuff) - width_factor * baseline_offset  subtract the bare pressure drop and multiply the
                                                                          width factor into the baseline offset instead,
                                                                          mispairing which terms are subtracted and which
                                                                          are multiplied (`(a-b)-c*d` instead of
                                                                          `(a-b)*c-d`),

which are exactly the mistakes a student makes (dropping the parentheses around the binomial before applying the factor, or
mispairing the factor with the trailing term). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts;
all five always appear as options.

Distinctness and positivity: because this shape SUBTRACTS, positivity is NOT automatic — it is guaranteed by table
construction. Every table satisfies, with every observed quantity >= 2:
  * proximal_cuff > distal_cuff             (so the pressure drop a-b is positive),
  * (proximal_cuff - distal_cuff) * width_factor > baseline_offset   (so the gradient load (a-b)*c-d is positive),
  * proximal_cuff > distal_cuff * width_factor + baseline_offset     (so the crossed slip a-b*c-d is positive),
  * proximal_cuff - distal_cuff > width_factor * baseline_offset     (so the swapped slip (a-b)-c*d is positive).
The tables are chosen so the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PROXIMAL_CUFF, DISTAL_CUFF, WIDTH_FACTOR, BASELINE_OFFSET) — a proximal cuff pressure minus a distal cuff pressure for
# the two-cuff pressure drop, a width factor that scales it, and a baseline offset subtracted off, all plain positive
# numbers >= 2. Because this shape subtracts, positivity is guaranteed by table construction (see the four guards asserted
# in build()): proximal_cuff > distal_cuff, (proximal_cuff-distal_cuff)*width_factor > baseline_offset,
# proximal_cuff > distal_cuff*width_factor + baseline_offset, and proximal_cuff-distal_cuff > width_factor*baseline_offset.
# The five family values are asserted pairwise-distinct below.
TABLES = [
    (9, 2, 2, 3),
    (10, 3, 2, 2),
    (12, 2, 3, 3),
    (13, 3, 3, 2),
    (15, 2, 4, 3),
    (16, 3, 4, 2),
    (15, 4, 3, 2),
]

# The option family (5 members), all built from the four observed quantities via - and *. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "segmental_gradient_load",
        "total segmental gradient load (the pressure drop times the width factor, minus the baseline offset)",
        "(proximal_cuff - distal_cuff) * width_factor - baseline_offset",
    ),
    (
        "pressure_drop",
        "the pressure drop (the proximal cuff minus the distal cuff, before multiplying by the width factor)",
        "proximal_cuff - distal_cuff",
    ),
    (
        "scaled_drop",
        "the scaled drop (the pressure drop times the width factor, before subtracting the baseline offset)",
        "(proximal_cuff - distal_cuff) * width_factor",
    ),
    (
        "crossed",
        "the proximal cuff minus the distal cuff times the width factor, minus the baseline offset, dropping the parentheses so the width factor multiplies only the distal cuff (a wrong grouping)",
        "proximal_cuff - distal_cuff * width_factor - baseline_offset",
    ),
    (
        "swapped",
        "the pressure drop minus the width factor times the baseline offset, subtracting the bare pressure drop and multiplying the width factor into the baseline offset instead (a wrong pairing)",
        "(proximal_cuff - distal_cuff) - width_factor * baseline_offset",
    ),
]
QUERIED = ["segmental_gradient_load", "pressure_drop", "scaled_drop"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(proximal_cuff, distal_cuff, width_factor, baseline_offset):
    # Operation order mirrors the ADJ programs exactly (the difference forms inside its parentheses first, then it is
    # multiplied by the width factor, then the baseline offset is subtracted, so (a-b)*c-d evaluates as (((a-b)*c)-d)), so
    # the Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match
    # tolerance).
    return {
        "segmental_gradient_load": (proximal_cuff - distal_cuff) * width_factor - baseline_offset,
        "pressure_drop": proximal_cuff - distal_cuff,
        "scaled_drop": (proximal_cuff - distal_cuff) * width_factor,
        "crossed": proximal_cuff - distal_cuff * width_factor - baseline_offset,
        "swapped": (proximal_cuff - distal_cuff) - width_factor * baseline_offset,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for proximal_cuff, distal_cuff, width_factor, baseline_offset in TABLES:
        # Every observed quantity is a plain positive number >= 2.
        assert (
            proximal_cuff >= 2
            and distal_cuff >= 2
            and width_factor >= 2
            and baseline_offset >= 2
        ), (proximal_cuff, distal_cuff, width_factor, baseline_offset)
        # Because this shape SUBTRACTS, positivity is guaranteed by table construction, not automatically. Each guard keeps
        # exactly one family member strictly positive:
        #   proximal_cuff > distal_cuff                                   -> pressure_drop (a-b) > 0
        #   (proximal_cuff-distal_cuff)*width_factor > baseline_offset    -> segmental_gradient_load (a-b)*c-d > 0
        #   proximal_cuff > distal_cuff*width_factor + baseline_offset    -> crossed a-b*c-d > 0
        #   proximal_cuff-distal_cuff > width_factor*baseline_offset      -> swapped (a-b)-c*d > 0
        assert proximal_cuff > distal_cuff, (proximal_cuff, distal_cuff)
        assert (proximal_cuff - distal_cuff) * width_factor > baseline_offset, (
            proximal_cuff, distal_cuff, width_factor, baseline_offset,
        )
        assert proximal_cuff > distal_cuff * width_factor + baseline_offset, (
            proximal_cuff, distal_cuff, width_factor, baseline_offset,
        )
        assert proximal_cuff - distal_cuff > width_factor * baseline_offset, (
            proximal_cuff, distal_cuff, width_factor, baseline_offset,
        )
        fv = family_values(proximal_cuff, distal_cuff, width_factor, baseline_offset)
        # Belt-and-suspenders: every family member is strictly positive under the guards above.
        for key, v in fv.items():
            assert v > 0, (key, proximal_cuff, distal_cuff, width_factor, baseline_offset, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    proximal_cuff,
                    distal_cuff,
                    width_factor,
                    baseline_offset,
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
                proximal_cuff,
                distal_cuff,
                width_factor,
                baseline_offset,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r98segp-{idx + 1:02d}",
                "qtype": "segmental_pressure_gradient",
                "stem": (
                    f"A segmental-limb-pressure study records a proximal cuff of {num(proximal_cuff)} minus a distal "
                    f"cuff of {num(distal_cuff)}, times a width factor of {num(width_factor)}, minus a baseline offset "
                    f"of {num(baseline_offset)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe proximal_cuff({num(proximal_cuff)})\n"
                    f"observe distal_cuff({num(distal_cuff)})\n"
                    f"observe width_factor({num(width_factor)})\n"
                    f"observe baseline_offset({num(baseline_offset)})\n"
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
            "ADJ-LADDER rung 98 — segmental limb-pressure gradient from four stated quantities (a NEW panel: vascular / "
            "segmental-limb-pressure). From a proximal cuff minus a distal cuff for the pressure drop, a width factor that "
            "scales it, and a baseline offset subtracted off, compute the segmental gradient load "
            "((proximal_cuff-distal_cuff)*width_factor-baseline_offset), the pressure drop "
            "(proximal_cuff-distal_cuff), or the scaled drop "
            "((proximal_cuff-distal_cuff)*width_factor). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A DIFFERENCE TIMES A "
            "FACTOR, MINUS A TERM (a-b)*c-d (subtract b from a, multiply the difference by c, subtract d, so (a-b)*c-d = "
            "(((a-b)*c)-d); this is the LAST OPEN CORNER of the (a±b)*c±d quartet, completing it alongside rung-84 (a+b)*c-d, "
            "rung-96 (a-b)*c+d, and rung-97 (a+b)*c+d — no prior shape multiplied a binomial DIFFERENCE by a bare factor and "
            "subtracted a term, e.g. rung-67 (a-b)*c/d divided a difference-product by a term and rung-90 (a-b)*(c-d) "
            "multiplied a difference by a difference) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every figure is built only from the four observed quantities via - and * — no constant "
            "leaks, and neither the pressure drop, the scaled drop, nor any gradient figure ever appears as a literal (each "
            "is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make: dropping the parentheses so the width factor multiplies only the distal cuff (a-b*c-d, a wrong "
            "grouping) and subtracting the bare pressure drop while multiplying the width factor into the baseline offset "
            "((a-b)-c*d, a wrong pairing). The core confusion tested is that (a-b)*c-d is (((a-b)*c)-d), not a-b*c-d and not "
            "(a-b)-c*d. Because this shape subtracts, positivity is guaranteed by table construction: every table keeps the "
            "proximal cuff above the distal cuff, the scaled drop above the baseline offset, the proximal cuff above the "
            "distal-cuff product plus the offset, and the pressure drop above the width-factor product, so all figures stay "
            "strictly positive."
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
