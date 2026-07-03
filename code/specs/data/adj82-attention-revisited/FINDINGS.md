# ADJ82 — attention revisited: the framework routes attention via task design (conditionally)

Re-does the ADJ75 attention probe correctly, now that the prompt-overload issue is
understood. Fixes ADJ75's three flaws: measure DURING GENERATION (not a static
last-position probe), normalize by attention-to-the-PASSAGE (length-robust, not by 1/S),
report raw shares. Same passage, two framings: COPY (extract the words describing the
subject's rule) vs ANSWER (free answer). Model: Qwen2.5-0.5B-Instruct.

## Result: real but CONDITIONAL ~2x attention routing

override-attn-share during generation (share of passage-attention landing on the
load-bearing override span):

| override span | COPY | ANSWER | ratio |
|---|---:|---:|---:|
| reduced rate of 12 days | 0.154 | 0.071 | 2.2x |
| lower limit of 55 mph | 0.151 | 0.075 | 2.0x |
| under-eighteen exempt from fines | 0.303 | 0.147 | 2.1x |
| refurbished 90-day warranty | 0.377 | 0.319 | 1.2x |
| clearance excluded from discounts | 0.173 | 0.174 | 1.0x (none) |
| **mean (n=5)** | **0.232** | **0.157** | **~1.5x** |

Strong ~2x routing on 3/5; weak/none on 2/5. **The exception explains the mechanism:** on
the two weak items the COPY response did not actually copy the *override* -- it started
copying the *general rule* ("Products carry a one-year warranty...", "Loyalty members
receive a 10% discount..."). Attention follows **what is actually copied**, not the "copy"
framing itself.

## The mechanistic rephrase

The framework helps a small model by **task-routing attention**: asking it to copy/extract
a *specific span* routes attention onto that span (you attend to what you reproduce) --
~2x onto the load-bearing override *when the copy lands there*. It does NOT magically
refocus the model. So the framework's real job is to **target the extraction at the
load-bearing span** (anchor to the subject's distinguishing attribute -- the ADJ81/ADJ77
design). Loose extraction -> the model copies the salient general rule -> attention goes
there instead.

This unifies the prior results:
- Explains ADJ81: well-targeted copy-phrase extraction worked.
- Explains ADJ81's failure mode: forced either/or ("full or part") routed nothing -> "full".
- Explains ADJ75's null: the routing is in the *generation of a targeted extraction*, not a
  static instruction effect at the answer position.

## Honest limitations
- n=5, one tiny model. Attention weights are a contested proxy (Jain & Wallace 2019);
  mitigated here because the interpretation is near-definitional (you attend to what you
  copy) and triangulates with the behavioral results (ADJ77/78/81).
- The "share of passage-attention" metric aggregates over heads+layers+generation
  positions; head/layer-resolved analysis and a causal attention-knockout are the stronger
  follow-ups.
- The 2 weak items are explained post-hoc by the copy missing the override; a pre-registered
  "did the copy target the override?" split would make this rigorous.

## Bottom line
Mechanistically, the framework's small-model benefit is **attention routing through task
design**: a targeted extraction makes the model attend to (and carry forward) the
load-bearing span; the effect is ~2x and real, but conditional on the extraction actually
targeting that span -- which is exactly what the framework must engineer.
