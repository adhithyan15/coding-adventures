# ADJ72 two-layer HLE run — results

End-to-end run of the assembled pipeline on a real HLE question (Palmyrene
inscription RIB 1065, the one plain Claude got WRONG in ADJ67), blind, ground truth
withheld from every phase except the judge.

## The phases and what each produced

| phase | what it did | output |
|---|---|---|
| 1. IR decompose | tokenized the inscription, byte-accounted | 6 tokens (RGYNᵓ, BT, ḤRY, BR, ᶜTᵓ, ḤBL); query `translate(inscription)`; framing left untokenized |
| 2. rule derivation + confabulation gate | 3 independent closed-book resamples, per-token gloss + confidence | all 6 glosses **STABLE** across resamples |
| 2.5 open-book justification | grounded the gate-flagged low-confidence tokens against real sources | ḤRY and ᶜTᵓ glosses **CONTRADICTED**; RIB 1065 identified; corrected reading found |
| 3. compose | applied trust rule (open-book > closed-book) and propagated corrections | "Regina, the freedwoman of Barates, alas." |
| 4. blind judge | scored gate-only vs full-pipeline answers against the key | gate-only = **INCORRECT**; full-pipeline = **CORRECT** |

## The headline: one layer fails, two layers succeed — exactly as predicted

**Confabulation gate alone (byte-stability) → stable WRONG answer.** All three
closed-book resamples independently produced the same reading: *"Regina, daughter of
Ḥari, son of ʿAttā. Alas!"* — the filiation misparse, glossing ḤRY as a personal name
and reading BT/BR as "daughter of / son of." Because the error is systematic in the
model's latent knowledge, resampling reproduced it identically. **Byte-stability cannot
catch a stable error** (the 50-sample boundary, confirmed on a real reasoning case).

**The layer above caught it.** The gate's confidence channel flagged the two content
tokens glossed as names — ḤRY (0.61) and ᶜTᵓ (0.65) — as the weak links. The open-book
justification layer grounded them and found:
- ḤRY is **"freedwoman"** (root Ḥ-R-R "free"), heading the construct `BT ḤRY` =
  "freedwoman of" — **not** a father's name. Verbatim-sourced from RIB 1065 / the
  Palmyrene corpus, with the Latin parallel using *liberta*.
- ᶜTᵓ is the name **Barates** (`BR ᶜTᵓ`), the dedicator — **not** a grandfather "ʿAttā."

**The correction propagated.** Grounding the flagged low-confidence token (ḤRY)
overturned the *high*-confidence-but-wrong structural reading of its neighbors (BT, BR
were 0.91 and 0.95 — never flagged). Re-reading `BT ḤRY` as "freedwoman of" and `BR ᶜTᵓ`
as the name collapsed the spurious three-generation genealogy. The blind judge scored
the result CORRECT and the gate-only baseline INCORRECT, citing the grading rule
(dropping the freedwoman-of-Barates relationship = wrong).

## What this validates (and what it doesn't)

**Validates Adhithya's "layers of recursive byte provenance" thesis, concretely:**
- A single grounding layer (does the model stably retrieve this?) is **necessary but
  insufficient** — it green-lit a stable error.
- The layer above (do authoritative bytes *entail* the proposed reading?) is what
  caught and corrected it. The relevance/justification failure that byte-stability
  could not see was visible the moment a higher layer demanded the proposed gloss be
  *entailed by grounded source bytes* rather than merely *stably recalled*.
- The mechanism is the framework's core asymmetry: the open-book layer did not need to
  already know the answer — it only had to ground the flagged token and check whether
  the closed-book gloss survived contact with the source. It didn't.

**Honest caveats:**
- It was the **confidence** channel, not byte-stability, that flagged where to ground.
  Pure byte-stability said "all stable, ship it" — wrong. The gate needs both channels:
  stability (catches invention) AND uncertainty (flags what to ground); and even both
  together only *triage* — the open-book layer does the actual correcting.
- The error locus (BT/BR, high confidence) was **never flagged**; it was fixed only by
  *propagation* from the grounded neighbor. Propagation worked here because the tokens
  are tightly coupled; it is not guaranteed in general. A confident-wrong claim with no
  flagged neighbor could still slip through.
- The open-book layer did, in effect, retrieve the published reading of a famous
  inscription. That is legitimate open-book grounding — and the point is that **the gate
  is what triggered the retrieval**. A pure closed-book pipeline would have shipped the
  stable wrong answer; the layered pipeline knew it had a soft spot worth grounding.

## Bottom line

On the exact HLE question plain Claude failed, the assembled pipeline —
IR-decompose → closed-book rule derivation + confabulation gate → open-book
justification on the flagged tokens → compose with open-book-wins trust rule —
produced the correct answer, blind, and an independent blind judge confirmed it.
The result is a clean existence proof that **recursive byte provenance across layers
catches the stable-error failure mode that any single layer (including the confabulation
gate) cannot.**
