# ADJ91 — byte provenance on the adversarial reader itself (grounded adversarial read)

The recursion the framework had left open: the support-auditor was held to a *lower* standard than
the solver. It could assert "fact X is unsupported because Y" with **Y ungrounded** — which is how
it destabilized earlier (the ADJ89/Haiku auditor hallucinated "the system is underdetermined / six
unknowns"). Fix = the same invariant applied to the critic: **every objection must cite a verbatim
`grounding_quote` from the PROBLEM or the FACTS, and a deterministic provenance filter discards any
objection whose quote isn't actually present in the bytes.** Same standard for the critic as for
the solver. Two arms (convergence-gated, ungrounded auditor = ADJ90 baseline, vs grounded auditor),
Al(OH)₃, N=3 each, on **both** Opus and Haiku. All-model; no hints; answer never shown to solver.

## Result
| model | ungrounded auditor | grounded auditor | objections filtered as ungrounded |
|---|---|---|---|
| Opus | 2/3 | 2/3 | **0** |
| Haiku | 1/3 | 2/3 | **0** |

**The `0` is the finding.** The deterministic filter never had to discard an objection — on *neither*
model did the grounded auditor emit an ungrounded objection to catch. Two readings, both supported:

- **Opus — no-op (2/3 = 2/3).** The disease wasn't present to begin with: Opus's auditor already
  cites real quotes, and ADJ90's convergence control had already neutralized the destabilizer (K_w
  as an explicit assumption). Grounding is correct but redundant here. The residual 1/3 failure
  (sample 3) is a **recall miss** — the auditor never *raised* the `[OH⁻]=10⁻⁷` objection — which
  grounding does not address (grounding governs the *quality* of raised objections, not whether
  they're raised).

- **Haiku — helped, but upstream, not at the filter (1/3 → 2/3, filtered 0).** The improvement did
  NOT come from the filter firing. It came from the **requirement** disciplining the critic *at
  generation*: forced to cite a verbatim quote for every objection, Haiku couldn't emit the
  ungrounded ones, so they never appeared (filtered = 0 because they weren't *produced*, not because
  they were *caught*). Concrete corroboration: the **ungrounded** Haiku arm derailed off-task in
  sample 3 — *"I need clarification on which task… the coding-adventures project?"* (codebase
  contamination) — while the **grounded** arm stayed anchored to the chemistry. The verbatim-quote
  requirement keeps the critic on the bytes in front of it.

## What this says about the invariant
Provenance-on-the-critic works **preventively, not correctively**. Requiring the adversary to ground
every objection in actual bytes suppresses ungrounded objections *at the source* rather than catching
them after — which is why the discard filter logged zero on both models. The invariant is right and
is now enforced; its benefit is concentrated on the **weaker** auditor (Haiku), where ungrounded
objections were the failure mode, and is a no-op on the stronger one (Opus), where they weren't.

## Honest caveats
- **N=3 cannot establish the Haiku accuracy delta (1→2) as significant** — Haiku is high-variance on
  this item. The *on-task* effect (sample 3 derail vs no-derail) is concrete; the +1 correct is
  directional.
- **filtered = 0 means the deterministic filter was never exercised.** I can't claim "it caught N
  hallucinated objections" — the requirement pre-empted them. To actually stress the filter you'd
  need a setup that *produces* ungrounded objections (a weak auditor with no grounding requirement,
  or a problem where the critic is more tempted to over-reach).
- **The dominant residual failure is now a recall MISS (false negative)** — the auditor failing to
  raise a real objection (`[OH⁻]=10⁻⁷` on the sample-3 regressions). Grounding addresses false
  *positives* (ungrounded objections), not false *negatives*. That's the next, orthogonal lever.

## Next
- **Multi-vote auditor recall** — union of N independent support-checks to close the false-negative
  (recall-miss) failures that grounding doesn't touch.
- A harder / adversarial problem set that actually *provokes* ungrounded objections, to exercise the
  provenance filter directly (here the requirement pre-empted it on both models).
