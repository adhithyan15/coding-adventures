# ADJ-LADDER — A Graduated, Two-Arm Proof that Reasoning Lives in the Framework

**Status:** PR-0 shipped the instrument + rung 0; the ladder now climbs in small,
audited rungs from the existing engine surface before heavier symbolic work.
**Author:** evaluation-systems architecture pass, 2026-06-26.
**North star:** A Haiku- or Gemma-class (small, non-frontier) model + the ADJ engine
and its content-addressed *standard library* passes a Medical Licensing Exam where the
same model **alone** cannot — with an audit trail an independent checker re-verifies,
and **zero math performed by the LLM**.

This spec composes with two siblings committed alongside it:
- **[ADJ-REASON-MATH](ADJ-REASON-MATH.md)** — the engine/language evolution (the
  deduction↔evidence bridge, CAS wiring, exact/dimensional compute, a unified proof
  object, an `adj-verify` re-checker). The *capabilities* each rung needs.
- **[MLE-PASS](MLE-PASS.md)** — the medical-exam harness: option-mapping,
  contamination protocol, the two-factor failure diagnostic. The *clinical rung*.

ADJ-LADDER is the **curriculum** that drives both: each rung pulls in the next
ADJ-REASON-MATH engine PR in dependency order, and the clinical rung *is* MLE-PASS.

---

## 1. Why a ladder, and why two arms

We want one falsifiable, externally-legible result. Rather than jump straight to
USMLE (where contamination and knowledge-breadth confound everything), we climb a
**complexity ladder** and, at every rung, run the SAME question set through two arms:

| Arm | What runs | Who does the math |
|-----|-----------|-------------------|
| **A** | the small model **alone** | the model, in-context (it is bad at this) |
| **B** | the small model **+ the ADJ engine** | the **engine**, on the CPU, exactly |

In Arm B the model's only job is to **decompose** the question into an ADJ program;
the engine evaluates it and selects an answer, emitting a machine-checkable proof.

**Base target: Gemma.** The canonical model is **Gemma** — a small, non-frontier model
that runs **fully locally** (no API, offline, on commodity Apple-silicon via MLX). The
default is `gemma-3-4b-it`; `gemma-3-1b-it` is available for an even-smaller probe where
the gap should appear earlier. This is a deliberate choice: the standing claim is
"a Haiku- or **Gemma**-class model + ADJ passes an exam the model alone cannot", and a
local model makes the whole pipeline reproducible end-to-end with zero network.

**The headline number is the divergence, B − A.** At the bottom of the ladder the gap
is small (a small model can do `7 * 8 + 3`). As computation deepens, Arm A degrades
while Arm B stays pinned near 100% — because the engine never makes an arithmetic
slip. *The widening gap with complexity is the money curve* — it localizes the
"intelligence" in the framework, not the weights. (Ties to the project theses: dumber
models in constrained envs; total-coverage forces reasoning; CPU-bound reasoning.)

---

## 2. The instrument (shipped in PR-0)

Everything lives under `code/specs/data/adj-ladder/`:

- **`rung0_arithmetic/items.json`** — a rung's question bank. Schema per item:
  `{id, qtype, stem, formula, options:{A..E}, gold_letter}`. `formula` is the **gold
  decomposition** — an arithmetic expression whose literals *all appear in the stem*.
- **`ladder_eval.py`** — the two-arm scorer (see §3).
- **`contamination_check.py`** — the bank-integrity / anti-circularity gate (see §4).
- **`ladder-scorecard.json`** — emitted artifact: per-arm metrics + divergence +
  per-item failure buckets.

### How Arm B selects an option WITHOUT ever computing the answer itself

For options `{A:59, B:60, C:61, D:58, E:62}` and gold formula `7 * 8 + 3`, the harness
emits this ADJ program and reads back the engine's decision:

```adj
prior 0.0001 for opt_a            % five equal-prior hypotheses, one per option
prior 0.0001 for opt_b
prior 0.0001 for opt_c
prior 0.0001 for opt_d
prior 0.0001 for opt_e
let answer = 7 * 8 + 3            % the ENGINE computes this — Python never does
contributes 1000000 from answer == 59 to opt_a   % option VALUES come from the
contributes 1000000 from answer == 60 to opt_b   % question, never from us solving it
contributes 1000000 from answer == 61 to opt_c
contributes 1000000 from answer == 58 to opt_d
contributes 1000000 from answer == 62 to opt_e
? opt_a … ? opt_e
```

The engine evaluates `answer` (= 59), the predicate `answer == 59` fires, opt_a's
log-odds jump decisively, and the decision returns `determinate` with `leader = opt_a`
→ letter **A**. Fractional options use the same path with an expression RHS, for
example `contributes 1000000 from answer == 3 / 10 to opt_a`. If the computed answer
matches **no** option (or, via a duplicate-value accident, two) the hypotheses stay tied
→ `kickback` → the harness **ABSTAINS** rather than guess. The harness supplies only
the formula and the printed option values; the arithmetic, the comparison, and the
selection are all the engine's.

---

## 3. Scoring (reuses board_eval.py's three-outcome, never-fabricate model)

Each item, per arm, resolves to exactly one outcome:

- **correct** — chose the gold letter.
- **abstained** — declined to commit (kickback, or no parseable model letter) — the
  *honest* miss, the discriminator against a hallucinating model.
- **wrong** — committed to the wrong letter — the only real failure.

Per-arm metrics: `raw_accuracy = correct/total`,
`defensibility = (correct+abstained)/total`,
`accuracy_on_attempted = correct/(correct+wrong)`. Cross-arm: **divergence**
(B−A on raw accuracy and on correct count).

**Arm B is GATED in cached mode**: a single `wrong` engine selection exits non-zero —
the engine's arithmetic must be exact by construction.

**Failure buckets** (the two-factor diagnostic, separating decompose-fidelity from
engine-correctness; full taxonomy in MLE-PASS §2.5):

| bucket | meaning |
|--------|---------|
| a | missing-library (no lib expresses the needed fact/op) — higher rungs |
| b | decompose-error (model's formula failed the faithfulness gate) |
| c | engine-gap (faithful decomposition, engine still missed) |
| d | genuinely-hard (correct decomposition + engine, item still wrong) — higher rungs |

### Modes

- **`--mode cached`** (default; what CI runs): Arm B only, using each item's gold
  `formula`. Isolates the ENGINE → expect ~100% correct, proving the mechanism with no
  model in the loop.
- **`--model <spec>`**: run BOTH arms with a real local model (`mlx:<repo>` or
  `cmd:<shell>`). The model answers directly (Arm A) and decomposes (Arm B). A
  model-produced formula must pass the **no-result-literals** gate (every number in the
  formula appears in the stem) or that item abstains in Arm B as a `b` bucket — the
  model may write the recipe, never the answer.

---

## 4. Anti-circularity (contamination_check.py)

A two-arm proof is only worth something if the bank is honest. The gate asserts,
offline and off the answer-path: unique ids; five **distinct** option values (so a
correct compute can't tie-and-abstain as an artifact); `gold_letter ∈ options`; the
gold key is internally correct (a *restricted, safe* arithmetic eval of `formula`
equals the gold value — the only place the bank's answer is computed in Python, and
only to validate the key); the no-result-literals property on the gold formula; and,
at rung 0, no external source/import (self-contained → contamination structurally
impossible). Higher rungs add a source-disjointness check vs any external bank, and
**freeze the standard library by content hash** before running (so a rung can't be
"taught to the test").

The deeper anti-circularity discipline for self-authored banks (per MLE-PASS): the
question author is **blind to the libraries**; answer keys are **independently
grounded**; libraries are **frozen then run**; vignettes match real exam format. At
rung 0 these are trivial (fresh numbers); they bind hardest at the clinical rung.

---

## 5. The rungs (each = its own items.json + mini-stdlib, reusing ladder_eval.py)

Climbing **drives the ADJ-REASON-MATH PR order** — each rung surfaces exactly the
engine gap that blocks it, and every rung ships a two-arm divergence number.

| Rung | Content | Engine capability it pulls |
|------|---------|----------------------------|
| **0** | grade-school arithmetic + 1-step word problems | **none** (value-math exists) — shipped |
| **1** | fractions / percent | native predicate RHS expressions plus exact rational sidecars for fractional equality; harder banks climb from here |
| **2** | pre-algebra / algebra word problems | native ADJ solve programs now mix with rule-derived setup premises; broader CAS solve trail (PR-6) |
| **3** | algebra / calculus | native ADJ solve now covers two-variable linear systems, linear optimization, plus `solved_roots` banks for quadratic, cubic, quartic, and factored-polynomial equations; broader CAS wiring (PR-6) + rewrite trail (PR-4) |
| 4 | physics / chem with units | dimensional engine (exists) + exact compute (PR-3) |
| 5 | clinical / MLE → **apex: pediatrics** | the **MLE-PASS** harness (shares the option-map); multi-hop→PR-1, calculation→PR-3/6 |
| — | defensibility hardening | `adj-verify` (PR-9): every correct item's proof re-checks |

**Why pediatrics as apex:** it is computation-dense (weight-based dosing, growth
percentiles, fluid calc) — exactly where in-context LLM arithmetic fails and an exact
engine wins. (The "no LLM has passed peds MLE" claim is *unverified*; peds is a sound
hard target regardless and must not be asserted as established fact.)

---

## 6. PR-0 scope & verification (this PR)

**Scope:** the instrument (`ladder_eval.py`, `contamination_check.py`), rung 0
(`rung0_arithmetic/items.json`, 20 fresh items), the test suite, and the three specs
of record (this file + ADJ-REASON-MATH + MLE-PASS). **No engine/grammar change.**

**Verification (reproduce):**
1. `cargo build -p adj-lang-cli`
2. `python3 contamination_check.py rung0_arithmetic` → clean.
3. `python3 ladder_eval.py rung0_arithmetic` → Arm B raw **100%**, wrong **0**
   (engine-correctness sanity; no model).
4. `python3 -m pytest test_ladder_eval.py -q` → green.
5. (where a local model exists) `python3 ladder_eval.py rung0_arithmetic --model
   mlx:<repo>` → the first real two-arm number + divergence.

**Result — engine sanity (cached, no model):** rung-0 Arm B = **20/20 correct, 0
wrong, 0 abstain** — the engine computed every answer exactly and selected the gold
option, with the computed value visible in each item's proof.

**Result — first real two-arm run (Gemma-3-4b, greedy, fully local):**

| Arm | raw accuracy | wrong (fabrications) | defensibility |
|-----|--------------|----------------------|---------------|
| **A** — Gemma alone | **60%** (12/20) | **8** | 0.60 |
| **B** — Gemma + ADJ | **95%** (19/20) | **0** | **1.00** |

**Divergence B − A = +35% (+7 items).** The money curve is visible at the very bottom
of the ladder: even on grade-school arithmetic a small local model fabricates 8 wrong
answers, while the engine arm makes **zero** — its single miss is a *decompose* error
(bucket `b`) the engine caught and **abstained** on, not a fabrication. The defensibility
gap (0.60 → 1.00) is the headline the ladder will widen rung by rung. (Artifact:
`code/specs/data/adj-ladder/ladder-scorecard.gemma.json`.)

The mechanism is proven; the ladder can climb.

---

## 7. Next

PR-1 is the ADJ-REASON-MATH **deduction→evidence bridge**: `logic-engine`
`observed_evidence` falls back to SLD provability, attenuates an LR contribution by
the proof confidence, and threads the rule/fact proof into the aggregate evidence
step. This unlocks the first true multi-step Arm B programs: the model can emit
observations plus a rule, and ADJ can derive the intermediate premise before weighing
it probabilistically.

After this bridge, rung 2 starts with native ADJ solve programs: a small model can
emit `symbol` / `constrain` / `solve for` from a messy pre-algebra stem, the ADJ
constraint solver computes the unknown, and the ladder maps that engine value to the
printed options. The next rung now mixes that solve path with rule-derived premises:
the program derives a setup atom, uses it as evidence for a queried readiness
decision, and then solves the numeric unknown in the same native ADJ run. Rung 3 now
also exercises native constraint optimization: ADJ returns `optimize.value` for
linear `maximize`/`minimize` programs, and the ladder maps the engine optimum to
printed options without host-side solving. It then climbs through native polynomial
root solving: ADJ returns `solved_roots` for quadratic, cubic, quartic, and
factored-polynomial programs, and the ladder maps those root sets to printed options
without host-side solving. From here, rungs 3→5 gate the broader
CAS/dimensional/clinical slices in order, culminating in the MLE-PASS clinical rung
and the pediatrics apex.
