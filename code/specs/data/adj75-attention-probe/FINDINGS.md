# ADJ75 — does asking for discards change attention? (honest: NOT as a static refocus)

Controlled probe on Qwen2.5-0.5B-Instruct (open weights, attention extracted via
transformers, eager attention). Same passage+question; only the instruction framing
changes (BARE vs DISCARD). Prompt ends "FINAL ANSWER: " so the next-token IS the answer.
At that final position we measure attention onto the buried-override span and the
P(correct)/P(trap) answer-token ratio. n=8 numeric present-but-skimmed items.

## Result — and the confound that flips it

**First pass (length-normalized "× uniform"):** discard raised override-attention in
**8/8** items (0.79 → 0.95 × uniform). Looked like clean support for "discards refocus
attention."

**The confound:** the DISCARD prompt is longer (extra instruction tokens), so the uniform
baseline 1/S is smaller, mechanically inflating the normalized ratio even if raw
attention is unchanged. Checking the **raw, unnormalized** attention-per-token to the
span:

| metric | bare | discard | direction |
|---|---|---|---|
| raw attn-per-token to override span (mean, n=8) | 0.00697 | 0.00652 | **DOWN** |
| items where raw attn rose under discard | — | — | **1/8** |

**On raw attention, the discard instruction does NOT increase attention to the
load-bearing span — it slightly decreases it** (the longer prompt dilutes attention).
The "8/8 increase" was a normalization artifact. **The static attention-refocusing
hypothesis is not supported by this measurement.**

## Behavioral measure (also mixed)

P(correct)/P(trap) at the answer position rose under discard in only **4/8** items; the
aggregate ratio fell (outlier-dominated). So the framing change does not reliably shift
the 0.5B model's answer distribution either.

## What this means (honest)

1. **The framework's mechanism is NOT a static attention rewrite.** This is consistent
   with the interpretation argued earlier: the contract operates at the OUTPUT /
   verification level. It does not, by mere instruction, redirect last-position attention
   onto the load-bearing span (at least at 0.5B, with this metric).
2. **Where the framework helps, the cause is more plausibly generation-time + auditability,
   not attention:** the model writing out the discard justification forces it to *process*
   the span during generation (a per-token effect across the generated justification, not
   a static effect at the answer position), and the written discard is an auditable
   artifact a verifier can catch. This probe measured the wrong locus to see that — it
   measured a static instruction effect, not the generation dynamics.
3. **Methodological lesson:** attention metrics must be confound-controlled (prompt-length
   normalization can manufacture an effect). Report raw values, not just normalized.

## Honest limitations
- One tiny model (0.5B), n=8, single static metric (last-position attention). Attention
  weights are a contested proxy (Jain & Wallace 2019).
- The right test of "does *generating a discard justification* change processing of the
  span" measures attention/activations ACROSS the generated justification tokens, and at
  larger scale — a more involved experiment, flagged as next.

## Bottom line
We did NOT prove that asking for discards changes the attention mechanism. Measured as a
static, answer-position effect at 0.5B, raw attention to the load-bearing span slightly
*decreases* under the discard instruction. This argues against a "refocusing" story and
for the generation-time + verification account of how the contract helps — and it is
reported as a negative result, honestly.
