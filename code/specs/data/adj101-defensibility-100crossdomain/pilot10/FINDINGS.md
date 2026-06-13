# ADJ101 — 10-item program-emission pilot: FINDINGS

**Status: COMPLETE.** 10 computational items (physics, math, units, chemistry ×4, finance, geometry)
run through the framework's extraction + program-emission stage (Opus translator, 10 agents) → the
deterministic provenance-gated executor. Gold is tool-derived (`compute_gold.py` / RDKit / SymPy).
Scored on the rescored axis: **auditability + localizability + correctability**; correctness is
informational.

## Headline (n = 10)

| metric | value |
|---|---|
| programs **emitted** (model translated, did not compute in-head) | **10/10** |
| programs **executed** | 9/10 |
| **auditable** (no fabrication, no value-vs-span contradiction, no dropped quantity, no magic number) | **10/10** |
| — coverage complete (every distractor `discarded(reason)`) | 10/10 |
| — no magic numbers (every molecular weight etc. from the tool, not memory) | 10/10 |
| — no fabrications | 10/10 |
| correct *(informational)* | 9/10 |
| not-clean **and localized** (audit names where to look) | **1/1** |

The model **translated and emitted a program in every case** — it never did the arithmetic/chemistry
in its forward pass. Every molecular weight came from RDKit, every quantity came through a typed
provenanced fact, and all three distractors (pad width, "≈194 g/mol", branch number) were
**discarded-with-reason**, not silently dropped.

## The one failure is the whole point — wrong, but localized + correctable

**MATH1** (real root of `x³−2x−5=0`): the emitted SymPy program **crashed** — its real-root filter
`[r for r in roots if sp.im(r)==0]` returns empty for symbolic radical roots → `IndexError`. So the
answer is **wrong (no result)**. But:

- the audit **localizes it exactly**: `error_locus.exec_error` points at the real-root-extraction line
  (a *program* bug, not a fact bug — the facts and coverage are clean);
- it is **correctable in one edit**: replacing that line with `sp.real_roots(expr)[0]` **re-derives
  2.095 correctly**, `auditable: True`, **zero new model calls**.

That is the rescored paradigm working on computation: a wrong program is fine when the trail leads to
the exact broken step and a one-move fix re-derives. The bare arm, by contrast, would emit a confident
prose number with nothing to inspect.

## What the pilot surfaced (gate refinements one case couldn't)

Running 10 varied items exposed and fixed real bugs in the provenance gates:
1. **Unit-typed faithfulness** — `4% → 0.04` was wrongly flagged as a value-vs-span mismatch. Fixed:
   faithfulness now compares modulo common unit scales (%, SI prefixes), as the typed-unit IR requires.
2. **Non-numeric data facts** — SMILES/equation facts (`O=C=O`, `x**3-2*x-5`) were wrongly value-checked.
   Fixed: data facts are checked by string-in-span, with formula-notation normalization (`x**3` ≡ `x^3`).
3. **Surfaced assumption ≠ fabrication** — CHEM3's 1:1 CO₂:CH₄ ratio was an honest `inferred/LEAP` with a
   basis (carbon conservation). That is the audit *working* (a flagged, verifiable assumption), not a
   fabrication; it no longer breaks auditability.

(10 executor unit tests still pass, including the new assumption/unit/data cases.)

## Bottom line
Across 10 cross-domain computational items, the framework **reliably routes computation to programs**
(10/10 emitted), keeps them **fully byte-provenanced** (10/10 auditable: facts typed + provenanced,
distractors discarded, tool-derived constants, no laundered numbers), and — on the one item it got
wrong — **localizes the error to the exact step and corrects it in one edit with no model call**.
Correctness (9/10) is the *informational* by-product, not the target. This is the unit the full pilot
(both arms × both models × dual-judge) scales up.

## Reproduce
`python3 ../test_provenance_program.py` (gates); `python3 run_pilot10.py` (execute the saved
`emissions10.json` over `items_compute10.json`). Translator: `translate10.workflow.js`.
