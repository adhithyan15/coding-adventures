# E2 correctability — findings (honest; the localize-by-reading claim does NOT survive)

Implements [`PAPER1-E2-correctability-study.md`](../../papers/PAPER1-E2-correctability-study.md).
Pre-registration: [`00-preregistration.md`](00-preregistration.md). This run reports the
**pre-registered honest null** the spec said we must be able to surface — and a sharper,
more defensible reframe of where the framework's correctability payoff actually lives.

## TL;DR — the metric is cost-to-correct, paid ONCE, not recurring

The real question is **cost-to-correct: the cost should be paid one time and not recur over the
same facts.** Decomposed against that metric:

| component | hypothesis | result |
|---|---|---|
| **find** the error (RQ1) | trail lets a reviewer find it more often than prose | **NULL** — dead even after the format-confound guard |
| **fix** it (RQ2) | framework = edit one fact; prose = rewrite the answer | **supported** |
| **propagate + recurrence** (RQ3) | one edit covers all current + future cases at 0 model calls; prose re-errs forever | **supported** — the real win |

**The localize null is not a weakness — it clears the ground.** Since *finding* the error costs
about the same in both arms, the entire cost-to-correct difference lives in what happens **after**:

```
framework total = 1            (paid once, O(1), non-recurring)
prose total     = M + G        (recurring, O(M+G): M current + G future cases sharing the fact)
```

Plain prose has **no persistence layer** — each answer is stateless, so a correction to one answer
does nothing for the next case and a fresh case confidently re-asserts the same false fact. The
framework writes the correction **once** into the CAS; every dependent case, present and future,
inherits it for free. *Correctability ≠ localizability-by-reading; it is non-recurring cost.*
(Cost model + curves: `cost_to_correct.json`.)

### Empirically measured (`recurring_cost/`): recurring error is capability-graded, the framework erases it

One policy with a buried **override** (the shared dispositive fact), 7 override cases + 1 control.
Prose arm re-reasons the override on every stateless call; framework derives it into a byte-verified
rule **once** then decides every case at **0 answer-time model calls**.

| prose model | override correct | miss-rate | | framework (rule derived once) |
|---|---|---|---|---|
| qwen2.5:1.5b | **0/7** | **1.0** | | **7/7 + control, 0 answer-time calls** |
| qwen2.5:0.5b | 3/7 | 0.57 | | (same rule, any model) |
| qwen2.5:3b / llama3.1:8b / Haiku | 7/7 | 0.0 | | |

- A **capable** model (Haiku, 3B+) re-derives the buried override correctly every call — so for it the
  recurring cost is redundant work + lost determinism + *no place to store a correction*, not error.
- A **sub-threshold** model (1.5B) misses the override on **all 7** cases (recurring error) — but the
  **same 1.5B model through the framework is 7/7**. Capability paid once into the pipeline lifts a
  cheap model above threshold: *intelligence accumulates in the framework, not the weights.*

## 1. The localize panel is a clean null

96 cells = 48 matched **both-arms-wrong** items × {framework, plain}, drawn from the ADJ99 HLE
solutions (same solver, same question), across all 8 categories. Solver scales: Haiku (primary,
n=32 items) + Opus (robustness, n=16). Auditor/oracle: Opus, **same reviewer on both arms**.

| stratum | framework localize | plain localize | paired Δ (fw−plain) | 95% CI | excludes 0? |
|---|---|---|---|---|---|
| primary (Haiku-solver) | 0.797 | 0.823 | −0.032 | [−0.21, 0.13] | **no** |
| robustness (Opus-solver) | 0.563 | 0.643 | 0.000 | [−0.32, 0.32] | **no** |
| pooled | 0.719 | 0.767 | −0.097 | [−0.31, 0.10] | **no** |

Directional split (pooled, 45 scorable matched items): **framework strictly better on 10, plain
strictly better on 10, 25 ties.** Dead even. `auditor_fooled` rate is also equal: framework
9/48, plain 8/48. There is **no localization advantage** for the byte-provenance trail on
open-ended HLE reasoning once (a) the citation-format confound is removed and (b) the prose
baseline shows full work.

### Why this is the *correct* null, not a weak experiment
- **The format-confound guard is what flips it.** ADJ96 reported framework 5/5 vs plain 2/5 — but
  that auditor saw the **raw** citation-shaped trail vs raw prose, the exact confound the ADJ99
  rescore (PR #5261) proved with a 100% regex arm-classifier. Here we strip the chrome (regex
  arm-separation falls **100% → 51%**, chance; `leak_check.json`) while keeping the stepwise
  decomposition. With the confound gone, the advantage is gone. ADJ96's result was largely
  *format* (and tiny n), not substance.
- **The prose baseline is strong** (full work shown), as the spec's anti-strawman threat demanded.
  A strong reviewer reading strong prose localizes about as well as from a trail.
- **The paired design is robust to gold noise** — any HLE-gold unreliability hits both arms
  equally, so the *delta* is clean. It is ~0.

## 2. The run reflexively audited the HLE gold (again)

3 of the "wrong" solutions are **actually correct — the HLE gold is wrong**, caught by the
gold-aware oracle:
- `66ed86e620`: gold `−32/13` is impossible (it requires `sin > 1`); the solution's `≈ −4.18` is
  right (bounded `sin` forces `dy/dt ∈ [−5,−3]`).
- `66e8e473f5`: gold `8`, but independent brute force confirms the solution's `9`.
- (+1 more flagged at the answer-key level.)

This is the ADJ99 measurement-validity theme recurring a third time (after the run100 and run100b
gold-vets): the adversarial reading that tests the framework keeps catching errors in the
benchmark's own answer key. Scored against final-answer HLE gold, the localize task is partly
measuring the wrong axis — the framework is built for *auditable reasoning*, not *recall*.

## 3. Fix + persist + propagate — the real correctability win (RQ2/RQ3)

This is where the framework does what prose cannot, and it holds (`fix_propagate.json`):

- **Single-edit fix that persists & is regression-safe** (meningitis CAS, `adj52/cas/`): one human
  override of the correlated CSF-chemistry facts **de-saturates** the over-confident pre-culture
  case **0.9999 → 0.7709**, while the dispositive culture-positive sibling is **unchanged
  (1.000 → 1.000)** — no regression. The base corpus is immutable; the edit is a versioned,
  attributed, cited override. *Fix the fact, not the weight.*
- **Derive-once propagation at zero answer-time model cost** (TAX CAS, `run100/cas/`): a
  byte-verified rulebook compiled **once** into a content-addressed library decides **3 held-out
  cases on CPU** (`REQUIRED` / `NOT_REQUIRED` / `REQUIRED`), **answer-time model calls = 0**, each
  verdict carrying a proof tracing rule → fact → cited policy bytes.
- **Prose has no localized handle:** a "fix" is a rewrite of the derivation, and there is nothing
  to propagate — no re-derivable artifact. This asymmetry is structural, not empirical.

## 4. What this means for the paper

The defensible E2 claim is **not** "the trail helps you find the bug." It is:

> When a model is wrong, the framework turns the wrong answer into a **correctable artifact**: the
> error reduces to an editable fact/clause, the fix **persists** under deterministic re-derivation,
> and it **propagates** to every dependent case at zero answer-time model cost. Plain prose offers
> none of this — and, on open-ended reasoning, a strong reviewer localizes errors in prose just as
> well as in a trail, so localization is *not* the differentiator.

This is a stronger position than the original three-panel figure: it concedes the localize panel
honestly (which a reviewer would otherwise attack) and rests the headline on the
fix/persist/propagate machinery that is genuinely unique to the framework.

### Open follow-up (not yet run)
The localize null is on **open-ended HLE reasoning**, where the error is often a deep conceptual
leap. The framework's premise-exposure might still aid localization where the locus is a
**discrete fixable fact/clause** (the run100/run100b adjudication items, the meningitis CAS). A
structured-domain localize arm would test whether H1 holds in the regime the framework is actually
built for. Flagged for decision; the current headline does not depend on it.

## 5. Reproduce
`python3 build_items.py` (assemble cells + leak-check) → `localize.workflow.js` (BATCH=10, Opus
auditor) → merge into `localize_results.json` → `python3 fix_propagate.py` → `python3 aggregate.py`
→ `aggregate.json` + this file.
