# ADJ76 — why does qwen2.5:0.5b crap out on the monolithic contract?

Diagnostic follow-up to ADJ74 (staging rescued 0.5b: monolithic 0.00 → staged 0.58).
Question: *why* does the monolithic contract collapse the 0.5b model — cognitive overload,
or an artifact (e.g., output truncated before FINAL ANSWER)?

## Result: it's cognitive instruction-overload, NOT truncation

**Failure taxonomy (monolithic 0.5b, 12 present-but-skimmed items):**

| failure mode | count |
|---|---|
| absorbed in tagging → defaulted to the salient **general rule** (skim) | 7/12 |
| answered with a **tag** (`[DISCARD]`) instead of a value | 3/12 |
| no / garbled FINAL ANSWER | 2/12 |
| (of the above, **invented a non-spec tag** like `[FIXED]`) | 2 |

**Truncation control — the decisive test:**

| token budget (num_predict) | monolithic 0.5b PS accuracy |
|---|---|
| 128 | 0.00 |
| 512 | 0.00 |
| 1024 | 0.00 |

Accuracy is **flat at 0.00 across an 8× budget increase.** The 0.5b reaches FINAL ANSWER
in 10/12 cases (mean output ~461 chars) — it is not running out of room. **The failure is
not truncation.**

## Diagnosis

The monolithic prompt imposes **four simultaneous objectives** — (1) list every clause,
(2) mark each [USE]/[DISCARD], (3) justify every discard, (4) answer with the override
applied. The 0.5b cannot track all four. It spends its limited instruction-following
capacity on the formatting subtask and then:
- conflates the marking task with the answer (replies `[DISCARD]`), or
- pattern-matches the tag format without understanding it (invents `[FIXED]`), or
- falls back to the salient general rule (7/12 skim).

**Why atomic staging fixes it (0.00 → 0.58):** each staged turn is a *single* objective
within the model's capacity, and by the final answer turn the relevant clause has already
been surfaced in the conversation context. The benefit is **decomposing cognitive load**,
not freeing token budget (the budget was never the constraint).

## Why this matters (ties to local deployment)

This is the concrete content of the "capability floor": the monolithic byte-accounting
contract **overloads small models via multi-objective instruction overload.** For the
HIPAA-mandated local-model regime (Paper 2), the implication is direct: **on a small local
model you must decompose the contract into atomic, one-objective-per-turn stages** — the
monolithic contract does not just underperform, it collapses (0.00), and staging recovers
it (0.58). This is a deployment requirement, not a nicety.

## Limitations
- One model (0.5b), n=12, one item family. The taxonomy is illustrative; the truncation
  control (flat 0.00 across 8× budget) is the rigorous part.
- "Cognitive overload" is inferred from the output taxonomy + the truncation control, not
  from internal probing. A stronger version would track where instruction-following breaks
  down token-by-token.
