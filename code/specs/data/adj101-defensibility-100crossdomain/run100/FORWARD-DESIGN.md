# ADJ101 — forward design: adversarial reading at every link, the CAS-write gate, and model-role separation

Three design decisions crystallized by the 100-run + the entailment-gate experiment. Each reuses
machinery the project already has; this is the architecture they compose into.

## 1. Adversarial reading applies to *every link*, at *every level* — not just inferences

The entailment gate ([`ENTAILMENT_GATE_FINDINGS.md`](ENTAILMENT_GATE_FINDINGS.md)) checked one link
type — **inferences** ("do the bytes entail this value?"). But the byte-accounting contract has a
**second** link type that needs the same adversarial treatment: **discards**.

- **Inference read** (built): *does the cited evidence ENTAIL the asserted value?* → ENTAILED / LEAP.
  A LEAP is an over-read. (39/240 dispositive slots were LEAP.)
- **Discard read** (the symmetric gate): when a span is `discarded(reason)` as irrelevant, an adversary
  must try to show **the discarded fact is actually load-bearing**. A successful refutation means the
  framework dropped a dispositive fact under a bad justification. (The coverage gate currently checks
  that a span is *accounted for*; it does not check that a *discard* is *justified* — same gap as the
  self-reported `ENTAILED`.)
- **At every decomposition level.** The recursive contract decomposes input → IR → rulebook → decision.
  The adversarial read runs at **each** level on **both** link types — an over-read or a bad discard can
  enter at any layer (this is the per-level discipline applied to adversarial verification, not just
  coverage).

Both reads are then **gated by decision-sensitivity (ADJ65)**: act on a LEAP inference or a load-bearing
discard **only if it is outcome-pivotal** — its correction would change the verdict. (The 100-run showed
the un-gated version over-abstains: 88→74. Sensitivity is what makes the adversarial read usable.)

> Symmetry: *no fact inferred without a verified justification; no fact discarded without a verified
> justification* — and "verified" means an **independent adversary** could not refute it, not that the
> extractor labeled it so.

## 2. The CAS-write gate — earn trust *before* commit, so reuse is trust-free

The CAS (content-addressed store of facts/rules/conclusions) is only valuable if a cached entry can be
**reused without re-checking** (paper-2 MYCIN: ground once, run forever on CPU). That requires the
entry to have **earned trust at write time**. The gate, before any commit to CAS:

1. **N independent adversarial readers** (not the extractor; ideally ≥3) read the candidate entry and
   try to refute its grounding. Commit requires concurrence (e.g. ≥⌈N/2⌉ find it unrefutable) — the
   W5 finding (a different/stronger reader localizes better; cross-Opus 90% vs same-Haiku 82%) says
   these readers should be **independent of the writer**.
2. **Byte-stability** over the source bytes — resample the read; a low-consistency entry is not stable
   enough to cache (the SelfCheckGPT/semantic-entropy layer, used here as a *commit gate*, not a
   detector — see the positioning memo: byte-stability detects *invention*, the adversarial readers
   catch *over-read/stable-error*; together they cover both).
3. **Blind-judge agreement** — an independent blind judge reaches the *same conclusion* the entry
   encodes. Disagreement blocks the commit.

Only an entry that is **(N-reader-unrefuted) ∧ (byte-stable) ∧ (judge-concurring)** is written to CAS.
Everything else is kicked back (to a human, or for re-derivation). This is what makes the cache a
**capitalized, correctable asset** rather than a pile of unverified model output — the precondition for
the derive-once/reuse-indefinitely claim.

## 3. (Future note) The adversary model ≠ the reasoning model

The framework already separates model *roles*: the model that **decomposes the input** is not
necessarily the one that **builds the rulebook**, nor the one that performs the **spider** retrieval
(cf. ADJ17's independent two-model rulebook elicitation, ADJ05's `(vendor, model_family)` independence
requirement for the adversary). The **adversarial reader** and the **byte-stability/blind judge** belong
on that list too:

- the adversary should be **independent of** the writer/reasoner (ADJ05 independence) and may be a
  **different family or a stronger tier** (W5: cross-model audit beats same-model; the auditor wants the
  bigger model even when the solver is cheap);
- nothing requires the adversary and the reasoner to be the same model — and there are reasons they
  shouldn't be (shared blind spots; an extractor can't reliably refute its own over-read).

Architecturally: **role → model** is a configuration, not a constant. Decompose / rulebook-build /
spider / reason / adversary-read / byte-stability / blind-judge are separable seats, each independently
assignable (cheapest-capable model per seat) — which is also the deployment story (small local models
for the cheap seats, a stronger auditor on the gate).

## How this composes
Extraction emits typed, provenanced IR → **per-level adversarial reads** on inferences *and* discards →
**decision-sensitivity** filter (act only on outcome-pivotal failures) → engine/program verdict →
**CAS-write gate** (N adversaries × byte-stability × blind judge) before anything is cached → trust-free
reuse. The 100-run validated the spine (byte-accounting holds at scale); this is the discipline that
hardens the two soft links (over-read inferences, unjustified discards) and earns CAS trust.
