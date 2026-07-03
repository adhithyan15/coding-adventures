# E2 recurring-cost — the cost is paid once in the framework; in prose it recurs (capability-graded)

This is the empirical leg of the cost-to-correct thesis: *the cost should be paid one time and not
recur over the same facts.* One policy, one buried **override** (clause 8 of 12) that is the shared
dispositive fact for every case. The default distance rule is cued prominently; the override
quietly beats it. We measure how often each arm gets the override right, across model scales.

## Setup
- **Shared fact:** a "self-owned-destination" override that beats the headline distance rule,
  regardless of distance. 7 override cases (gold NOT_ENTITLED) + 1 control (owns nothing → ENTITLED).
- **Framework arm:** the policy is translated into a byte-verified rulebook **once** (1 Haiku call;
  the override span is verbatim-verified, precedence honored). Then the engine decides every case
  — present and held-out — at **zero answer-time model calls** (`score.py`, `run_raw*.json`).
- **Prose arm:** one stateless call per case, model re-reads the whole policy and re-reasons the
  override every time (`run_weak.py` for small models, the workflow for Haiku).

## Result — recurring error is capability-graded; the framework removes it

| prose model | override correct | miss-rate | control ok |
|---|---|---|---|
| qwen2.5:0.5b | 3/7 | 0.571 | ✗ |
| **qwen2.5:1.5b** | **0/7** | **1.000** | ✓ |
| qwen2.5:3b | 7/7 | 0.000 | ✓ |
| llama3.1:8b | 7/7 | 0.000 | ✓ |
| Haiku (stated + buried variants) | 7/7 | 0.000 | ✓ |
| **framework (rule derived once)** | **7/7 + control** | **0.000** | ✓ |

Two honest findings, both important:

1. **A capable model does NOT recur errors on a well-specified override.** Haiku (and qwen-3b, llama-8b)
   re-derive the buried clause-8 override correctly on every single case. So for capable models the
   recurring cost is *not* an error cost — it is **redundant interpretation work re-paid per call**
   plus **lost determinism/auditability** (prose is right, but unverified and free to drift), and
   **no place to store a correction** (see below).

2. **Below a capability threshold, the error recurs — and the framework erases it.** qwen2.5:**1.5b**
   anchors on the prominent distance cue and **misses the override on all 7 cases** (miss-rate 1.0);
   0.5b misses 4/7. Because each call is stateless, the same fact-error recurs every time, with no
   durable fix. **The same 1.5b model, run through the framework pipeline (override derived once,
   engine reuses it), is 7/7 at zero answer-time model calls.** Capability paid once into the
   pipeline lifts a sub-threshold model above threshold — *intelligence accumulates in the framework,
   not the weights.*

## The cost-to-correct accounting this instantiates

```
                       interpretation     correction lives        cost over M current + G future cases
  framework            paid ONCE          one editable rule       O(1)  (re-derive; 0 answer-time calls)
  prose (capable)      re-paid per call   nowhere (stateless)     O(M+G) work; correct-but-unverified
  prose (sub-threshold) re-paid per call  nowhere                 O(M+G) work AND O(miss·(M+G)) errors
```

**Where the correction lives is the crux.** Suppose the interpretation must change (counsel rules
inherited property is exempt). Framework: edit one rule → re-derive → all dependent cases flip,
propagated, 0 model calls. Prose: the amendment has nowhere to persist — it must be re-injected
into every case prompt and every future case re-incurs the omission. The cost recurs **whether or
not the model errs**, because prose has no artifact to write the correction into.

## Honest scope
- The error-recurrence number is model-dependent: it is large for sub-3B models, ~0 for Haiku-class.
  We do not claim capable models error-recur on stated facts — they don't. We claim (a) the
  *interpretation/correction* cost recurs in prose regardless, and (b) the *error* cost recurs for
  the cheap models the framework is designed to make sufficient.
- Single policy / one fact family; small n. This is a mechanism demonstration, not a rate estimate.

## Reproduce
`corpus.json` / `corpus_hard.json` → `recurrence.workflow.js` (Haiku, BATCH=10) +
`run_weak.py` (Ollama small models) → `score.py` (framework engine, 0 answer-time calls) →
`recurrence_results*.json`, `recurrence_weak.json`.
