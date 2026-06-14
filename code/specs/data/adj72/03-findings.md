# ADJ72 — findings (written AFTER the blind run + open-book verification)

## Raw result

| Claim | exact-pair rate | mean Jaccard | mean conf | stability verdict | ground-truth of the passages |
|---|---:|---:|---:|---|---|
| C1 Pride & Prejudice | 1.00 | 1.000 | 0.99 | **STABLE** | **CORRECT** (verbatim opening) |
| C2 UDHR Art. 1 | 1.00 | 1.000 | 0.99 | **STABLE** | **CORRECT** (verbatim Article 1) |
| C3 Einstein light postulate | 0.33 | 0.515 | 0.67 | **SPLIT** | both renderings are REAL sentences from the paper; the highest-confidence resample returned the canonical second-postulate wording |
| C4 Darwin "survival of the fittest" | 0.33 | 0.722 | 0.58 | **SPLIT** | the **divergent** resample (r3) was CORRECT (verbatim 6th-ed); the **stable pair** (r1=r2) was a slightly-off paraphrase |

Verification sources: Darwin 6th-ed text confirmed verbatim ("This preservation of
favourable individual differences and variations, and the destruction of those which
are injurious, I have called Natural Selection, or the Survival of the Fittest");
Einstein second postulate confirmed verbatim in the Perrett-Jeffery translation
("light is always propagated in empty space with a definite velocity c which is
independent of the state of motion of the emitting body").

## What held (hypothesis confirmed at the claim level)

**Claim-level byte-stability cleanly separated solid from soft: 2/2 and 2/2.**
- The canonical-memorized claims (C1, C2) returned **byte-identical** passages across
  all three independent resamples (exact-pair rate 1.00) and were **correct**.
- The soft claims (C3, C4) **diverged** (exact-pair rate 0.33) and carried lower
  self-confidence (0.67, 0.58 vs 0.99).
- Confidence co-varied with stability — both signals agreed on which claims were solid.

So as a **confabulation *detector*** — "is this claim solid enough to trust, or soft
enough that it needs external grounding?" — the method worked perfectly on this set.

## The crucial lesson (pre-registered falsification condition partially fired)

**On C4, the stable majority was WRONG and the divergent minority was RIGHT.** Two of
three resamples converged on the same slightly-off paraphrase ("...favourable variations
and the rejection of injurious variations, I call...") while the lone divergent resample
reproduced the actual 6th-edition text. Two independent samples agreed on the same
imperfect rendering.

This is exactly the failure I pre-registered ("a confabulated/wrong passage coming back
stable"). It fired partially — at the 2-of-3 level, not 3-of-3, so the **claim** was still
flagged SPLIT (soft) rather than STABLE. But the lesson is sharp and important:

> **Byte-stability is a confabulation DETECTOR, not a truth SELECTOR.**
> A high-stability claim is strong evidence the model is *retrieving rather than
> inventing* — but the retrieved bytes can still be a stable *approximation* of the
> source, not the exact source. And within a soft claim, you must NOT take the majority
> passage as the truth: here the majority was the wrong one.

## Bonus: the backtracking the user predicted

On the C4 trap (the phrase "survival of the fittest" is Spencer's, adopted by Darwin only
in the 5th edition), **all three resamples spontaneously surfaced the misattribution** —
each noted Darwin borrowed it from Herbert Spencer and added it from the 5th edition
onward, rather than naively asserting Darwin coined it. The byte-provenance demand forced
the model to confront the provenance and it partially backtracked on the trap premise on
its own — the dynamic predicted in the ADJ72 design discussion.

## Honest deviations from pre-registration

- Predicted stability ordering was C1≈C2 > C3 > C4. Actual at exact-match: C1=C2 (1.0)
  > C3=C4 (0.33, tied). At graded Jaccard, **C4 (0.722) > C3 (0.515)** — the reverse of
  the predicted C3 > C4. C4's stable pair shares most tokens, inflating its graded
  similarity, which is exactly why graded similarity alone is misleading and the
  exact-match rate is the safer signal.
- C3's instability turned out NOT to be confabulation: both divergent passages are *real*
  sentences from the paper (the introduction's postulate vs the §2 restatement). So
  "SPLIT" here means "genuine ambiguity about which real sentence answers the question,"
  not "the model is making it up." The detector flags the soft spot correctly but does
  not, by itself, distinguish *ambiguity* from *invention* — that distinction needs the
  external grounding step.

## How this composes with the existing framework

The method is a cheap, closed-book **pre-filter** for the open-book grounding spider:

1. Run the closed-book resample (K independent draws). Cost: K forward passes, no web.
2. **STABLE + high-confidence claims** → strong retrieval signal → trust provisionally, or
   pass to a **one-time CAS confirmation** (ADJ71) to upgrade `claimed_from_model_memory`
   → `grounded`. Cheap.
3. **SPLIT / UNSTABLE claims** → soft spot → mandatory external grounding (ADJ66 spider /
   ADJ64 underdetermination), and **never** take the majority passage as truth.

It does not replace open-book grounding; it **triages** which claims need it, closed-book,
for free. That is the honest, useful result: stability tells you *where you can skip the
web and where you cannot*.

## What a bigger run would establish

n=4 is an existence proof, not a measurement. The real number is the correlation between
exact-pair-rate and verbatim-correctness across many claims, ideally with K≥5 and a
cross-model arm (different family resamples — ADJ05 independence), on less-contaminated
items. The C4 result predicts the key refinement: score correctness at the **3-of-3
identical** bar, not 2-of-3 majority, since a 2-of-3 majority can be a shared
approximation.
