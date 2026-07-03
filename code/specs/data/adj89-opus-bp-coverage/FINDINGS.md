# ADJ89 — byte provenance on Opus: input-coverage → synthesis-coverage → inference-support (the core)

Extends the ADJ88 Al(OH)₃ coverage-gate experiment from Haiku to **Opus**, then chases the
failure through three gate layers until it's fixed. Item: Al(OH)₃ solubility given K_sp and K_f
(gold `1.776×10⁻³`; the amphoteric Al(OH)₄⁻ complex dominates, and the exact value needs a
self-consistent charge-balance solve, not `[OH⁻]=10⁻⁷`). All-Opus; no hints; the answer is
never shown to the solver. Three workflows + results saved.

## Baseline: plain-Opus is wrong AND wildly variable
Across runs, plain-Opus produced `5.8×10⁻⁹`, `5.8×10⁻³`, `5.9×10⁻⁶`, `1.1×10⁻⁵`, and (once)
`1.8×10⁻³`. So a single plain-vs-gated comparison is noisy; we sampled.

## Layer 1 — input-coverage gate (ADJ88's tightened gate): NO-OP for Opus
The input gate catches **dropped inputs** (Haiku dropped K_f). Opus doesn't drop inputs — its
first-pass IR includes K_f and the full machinery — so the use-audit ran 1 iteration with
`unaccounted: []` and changed nothing: plain-Opus and input-gated-Opus both gave `5.8×10⁻³`
(incorrect). *The input gate is a Haiku-class fix; Opus's error lives deeper.* (`bp_coverage_opus`)

## Layer 2 — synthesis-coverage gate: GAMED (the laundering recursed)
Made synthesis a chain of steps, each declaring which facts it consumes; required every IR fact
to be consumed or justified-discarded. It **passed** (all 12 facts consumed, `unused: []`) yet
gave `5.8×10⁻³`. Why: the decomposition **laundered the bad approximation into a consumed fact** —
F9 said *"[OH⁻]=10⁻⁷ because Al(OH)₃ is insoluble, replacing the joint charge-balance solve"* (a
false reason: the given K_f makes it soluble), and **no charge-balance fact was in the IR at
all**. Coverage checks that every fact is *used*, not that every fact is *valid*. The discard of
the charge-balance constraint hid inside an asserted fact. (`bp_synthesis_gate_opus`)

## Layer 3 — inference-support check (ADJ61 core): THE FIX (0/3 → 2/3)
Adversarially check **every inference for SUPPORT** — given the givens + other facts, does it
follow, or is it an unsupported approximation / an assumption the givens rule out / a
contradiction? (default UNSUPPORTED). Drop/correct the unsupported, re-derive, then solve.

| sample | plain-Opus | support-gated-Opus |
|---|---|---|
| 1 | 5.8×10⁻³ ✗ | 5.8×10⁻³ ✗ (loop oscillated) |
| 2 | 5.9×10⁻⁶ ✗ | **~1.8×10⁻³ ✓** |
| 3 | 1.1×10⁻⁵ ✗ | **1.8×10⁻³ ✓** |

**plain 0/3 → support-gated 2/3.** The auditor flagged exactly the F9-class error:
*"assumes [OH⁻]=10⁻⁷, but the given K_sp·K_f forces [Al(OH)₄⁻]~K₃[OH⁻], consuming OH⁻ ≫10⁻⁷ — it
approximates away a quantity the givens constrain."* This is the **positive** check (is the
assertion supported?) that no coverage/discard gate could do, because F9 was *asserted*, not
*dropped*. (`bp_inference_support_opus`)

## The synthesis (what the whole arc was circling back to)
The complete byte-provenance contract, validated empirically: at **every layer**, every inference
must **(1)** cite its supporting bytes/facts, **(2)** survive an adversarial check that the cited
support actually entails it, and **(3)** account for what it discards. We over-built (3)
(coverage / discard-policing) and under-built (2); **(2) — adversarial support of every
inference (the ADJ61 justification gate) — is the load-bearing piece**, and it's what moved Opus
from 0/3 to 2/3 on the exact failure that beat the coverage gates. Coverage is the necessary
negative-space half; inference-support is the positive half that catches *asserted-but-wrong*
reasoning, which is where models relocate the error each time a coverage seam is closed.

The recurring pattern across ADJ86→89: **each gate catches the previous laundering; the model
relocates the discard to the next un-inspected place** (mention → inferred-slot → consumed
approximation-fact). Only checking *support of every assertion* closes the relocation, because it
inspects what the model claimed rather than only what it omitted.

## Honest caveats
- **1/3 still failed — the support loop oscillated.** Sample 1's auditor flip-flopped (round 0:
  "[OH⁻]=10⁻⁷ unsupported"; round 2: "discarding it is unsupported, and K_w isn't given"), so the
  model declared the problem underdetermined and fell back to `5.8×10⁻³`. The loop needs
  convergence control, and a genuinely-missing given (K_w, the water constant the problem omits)
  can destabilize it.
- **Re-derivation can inject new errors** — sample 1 introduced an arithmetic slip (Ksp·Kf=58 vs
  58300); the auditor caught it, but it shows the loop is fragile.
- **Grading is noisy on this item** — one per-sample grader couldn't reproduce `1.776×10⁻³` and
  called the reference "questionable" (the rigorous solve does reproduce it). N=3 is directional.

## Next
- **Convergence control** on the support loop: stop re-reasoning once the auditor flip-flops;
  treat a genuinely-missing given (K_w) as an explicitly surfaced assumption, not a destabilizer.
- Scale the support-check to a batch of Opus failures (its exact failure set is in ADJ88's
  `hle_run10_results.json`) with several samples per item.
