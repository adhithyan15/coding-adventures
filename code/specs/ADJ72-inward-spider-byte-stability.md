# ADJ72 — The Inward Spider: Byte-Stability as a Closed-Book Confabulation Detector, and Why Layers Are Required

> **Headline.** Can a model's own consistency tell it whether it is *retrieving* a fact
> or *inventing* one — closed-book, no web? ADJ72 tests "byte-stability": ask a model to
> answer AND emit the exact verbatim source bytes its answer rests on, resample K times
> with independent fresh instances, and measure whether the cited passages converge
> (memorized) or diverge (confabulated). Across an n=4 probe and a 50-claim blind run, the
> result is sharp: **stability cleanly separates retrieval from invention (0 fabrications
> among 29 byte-identical claims; 11/11 apocryphal traps consistently flagged as having no
> source) — but it does NOT certify that a retrieved passage is *relevant* or *correct*.**
> A real, stably-recalled passage can be misapplied (the "closing sentence" that isn't),
> and a *stable error* (a systematically wrong reading) sails straight through. The fix is
> Adhithya's recursive-byte-provenance thesis, demonstrated end-to-end on two real
> Humanity's Last Exam questions: a **second layer** that demands the cited bytes *entail*
> the claim — and, where the closed-book layer is uncertain, an **open-book spider** that
> grounds the flagged atom in a primary source — catches exactly the failures a single
> layer cannot. Finally, the whole pipeline is shown working with **Haiku** as the worker
> model: it never ships a confident-wrong answer, getting a web-groundable question right
> and honestly abstaining on a PDF-bound one. Artifacts:
> [`code/specs/data/adj72/`](data/adj72/).

## 1. The idea under test

The framework's `claimed_from_model_memory` authenticity class (ADJ70) is a lower-trust
tier than fetched-and-verified source bytes. ADJ72 asks whether we can *promote* a
closed-book claim toward "grounded" using only the model's own behavior — turning the
model into an **inward-pointing spider** that "fetches from its own memory" and exposes a
detectable signal when it is fabricating.

The signal: **genuinely memorized content is stable under resampling** (a sharply peaked
distribution — the model reproduces the same bytes), while **confabulated content varies**
(each draw is fresh fiction). This is self-consistency applied at the byte level, and it
is the positive-polarity twin of ADJ67's discrimination gate (which keys on near-zero
*verdict* variance to refuse degenerate small-model output).

## 2. The probe (n=4) and the 50-sample run

**Probe.** Four claims spanning canonical → confabulation-prone. Canonical lines (Pride &
Prejudice, UDHR Art. 1) came back byte-identical across three resamples and correct; the
soft cases diverged. But on the Darwin "survival of the fittest" trap, **two of three
resamples agreed on a slightly-wrong paraphrase while the lone divergent one was correct**
— the first sign that stability is a detector, not a selector.

**50-sample blind run.** 50 claims (17 canonical / 16 translation-or-edition-variable /
17 traps), 3 independent blind closed-book resamples each, scored at the strict **3-of-3
identical** bar. Subagents were allowed to answer "no verbatim source exists" so honest
backtracking was captured, not forced. Results
([`50sample/FINDINGS.md`](data/adj72/50sample/FINDINGS.md)):

| stability bucket | canonical | variable | trap | total |
|---|--:|--:|--:|--:|
| STABLE (identical ×3) | 17 | 9 | 3 | 29 |
| SPLIT (2 agree) | 0 | 7 | 1 | 8 |
| UNSTABLE (all differ) | 0 | 0 | 2 | 2 |
| STABLE "no source exists" | 0 | 0 | 11 | 11 |

**Headline: 0 fabrications among the 29 byte-identical claims; 16/17 traps handled
correctly** (11 consistently rejected as apocryphal — "Let them eat cake," "Elementary my
dear Watson," misattributed Einstein/Gandhi/Voltaire/Sun-Tzu quotes, "Eppur si muove";
2 flagged unstable; 1 split; 2 answered correctly). Exactly one trap produced a dangerous
result: the Watson–Crick "closing sentence" came back STABLE + confident — and **wrong for
the question**, because the model reliably retrieves the famous "It has not escaped our
notice…" line, which *is real* but *is not the closing sentence*.

**The sharpened lesson:** byte-stability reliably certifies *retrieved-not-invented*. It
does **not** certify *relevant* or *correct*. The residual failure mode is precision, not
hallucination — which is precisely the gap a second provenance layer must close.

## 3. Why layers are required — two real HLE questions

### 3a. Palmyrene RIB 1065 — the *stable error* ([`hle-twolayer/`](data/adj72/hle-twolayer/))

The inscription `RGYNᵓ BT ḤRY BR ᶜTᵓ ḤBL`. Plain Claude failed this in ADJ67 by reading
`BT … BR …` as the filiation formula "daughter of … son of …", dropping the correct
"freedwoman of Barates" reading.

Run blind through the pipeline:
1. **IR-decompose** → 6 byte-accounted tokens.
2. **Confabulation gate** (3 closed-book resamples) → **all six glosses STABLE** — and the
   stable reading was WRONG ("Regina, daughter of Ḥari, son of ʿAttā. Alas!"). **The gate
   alone failed: it cannot catch a stable error.**
3. But the gate's **confidence channel** flagged the two content tokens glossed as names —
   ḤRY (0.61) and ᶜTᵓ (0.65) — as the weak links.
4. **Open-book justification layer** grounded the flagged tokens: ḤRY = "freedwoman" (root
   Ḥ-R-R), ᶜTᵓ = the name Barates → **CONTRADICTED** the closed-book reading; identified
   RIB 1065.
5. **Compose** (trust rule: open-book grounding > closed-book memory) — the correction
   *propagated*: grounding the flagged low-confidence token overturned the
   high-confidence-but-wrong `BT`/`BR` parse → **"Regina, the freedwoman of Barates,
   alas."**
6. **Blind judge:** gate-only = INCORRECT; full pipeline = CORRECT.

This is the recursive-byte-provenance thesis made concrete: a single grounding layer
(does the model stably retrieve this?) is necessary but insufficient; the **layer above**
(do authoritative bytes *entail* the claim?) catches the stable error, using the
framework's core asymmetry — the checking layer needs no recall, only the ability to see
that the offered evidence does not support the claim.

**Honest caveats:** it was the *confidence* channel, not byte-stability, that flagged where
to ground; the actual error locus (`BT`/`BR`, high confidence) was never flagged and was
fixed only by *propagation* from the grounded neighbor — which worked here because the
tokens are tightly coupled but is not guaranteed in general.

### 3b. Hummingbird sesamoid — the *unstable gap* + web contamination ([`hle-hummingbird/`](data/adj72/hle-hummingbird/))

The canonical HLE flagship question (bare models fail it: GPT-4o said 3, Gemini said 2).

- **Confabulation gate** (3 closed-book resamples): **8 / 7 / 7**, no verbatim passage,
  self-flagged `exists=false`, confidence ~0.15. The gate caught the ungroundedness; the
  provenance demand flipped the model from "confidently guess" to "I'm guessing."
- **Open-book spider** found the PRIMARY source — Zusi & Bentz (1984), *Myology of the
  Purple-throated Carib and Other Hummingbirds*, Smithsonian Contributions to Zoology
  No. 385 — extracted the verbatim passage (p. 22, tendinous fasciculi "to the bases of
  all but the medial rectrix"), reasoned 5 rectrix pairs − the medial = **4**, and
  explicitly **rejected the uncited "2"** that every casual web source parrots.
- **Blind judge:** bare closed-book = INCORRECT + undefensible; framework = CORRECT +
  defensible.

Three failure populations get this wrong (bare frontier models; bare Claude; naive RAG
trusting the contaminated "2"); only the grounding discipline gets **4**, defensibly.

## 4. Running it on Haiku ([`haiku-test/`](data/adj72/haiku-test/))

Worker phases routed through **Haiku** (not a frontier model in disguise):

| question | Haiku alone | Haiku + framework | truth |
|---|---|---|---|
| Palmyrene | filiation trap; committed nothing | **"Regina, the freedwoman of Barates, alas"** — found RIB 1065, cited it | ✓ |
| Hummingbird | abstained (conf 0.0) | **UNDERDETERMINED** — found the right primary source, could not extract the 80-page PDF, **refused the contaminated "2"** | 4 (not reached) |

**Headline: zero confident-wrong answers, either arm.** On the web-groundable question
Haiku + framework went wrong → correct-and-cited, matching the frontier-model run. On the
PDF-bound question Haiku's *judgment* was sound (right source identified, contaminated "2"
refused) but it hit a **document-retrieval-and-extraction ceiling** and correctly landed
at honest "underdetermined" — strictly better than the bare frontier models that
confidently answered wrong. The capability gap is **tool-shaped, not intelligence-shaped**:
pair Haiku with a deterministic PDF fetch-extract tool (or the ADJ71 CAS cache) and its
already-correct judgment would likely carry it to "4."

## 5. What ADJ72 establishes

1. **Byte-stability is a real, closed-book confabulation *detector*** — 0 fabrications in
   29 stable claims; 11/11 apocryphal traps flagged as sourceless. It is necessary
   infrastructure for promoting `claimed_from_model_memory` toward grounded.
2. **It is not a truth selector.** Stable-but-wrong (Watson–Crick relevance miss; Palmyrene
   stable misparse) passes the single layer. Stability needs the confidence channel to
   triage and, above it, a justification/entailment layer to verify relevance.
3. **Recursive byte-provenance across layers closes the gap**, demonstrated end-to-end:
   the layer above grounds the flagged atom and checks entailment; open-book grounding
   supersedes closed-book memory; corrections propagate. Both real HLE failures were
   recovered, blind, and confirmed by an independent blind judge.
4. **Open-book lifting HLE is established prior art** (OpenAI Deep Research 26.6% vs
   single-digit base models; Gemini Deep Research's climb). ADJ72's distinctive
   contribution is **not** the score lift — it is **defensibility and resistance to web
   contamination** (the hummingbird "2 vs 4" case) plus **graceful degradation on small
   models** (Haiku never ships confident-wrong).

## 6. Honest limitations

- Single model family (Claude); the stronger test adds a different-family resample arm
  (cross-model byte agreement) — ADJ05 independence at the byte level.
- The 50-sample resamples were batched per round (12 subagent calls); a fully clean design
  is one subagent per (claim, resample).
- "Confident-wrong with no flagged neighbor" remains the open hole: the Palmyrene fix
  relied on a low-confidence neighbor existing to trigger grounding. A high-confidence
  error with no uncertain neighbor could still slip the gate.
- Correctness scoring used verified ground truth + spot web-verification, not a full
  per-claim open-book verifier.

## 7. What this opens up

- **The PDF/long-document fetch-extract tool** for the spider — the specific capability
  floor a small model hits. Tool-shaped, cheap, and would let Haiku clear document-bound
  questions.
- **Cross-model byte agreement** as a stronger confabulation gate (different families must
  reproduce the same bytes).
- **The Haiku screening harness**: 20–30 real HLE items, blind Haiku-alone controls vs
  Haiku + full pipeline, scored on correctness AND defensibility AND model cost — the real
  numbers behind "cheap model + grounding discipline, competitive and auditable."
- **The confident-wrong-with-no-flagged-neighbor case** — the remaining hole — needs its
  own layer (e.g., adversarial re-reading, ADJ05/ADJ42, applied to every high-confidence
  atom regardless of its neighbors' confidence).

## See also

- [ADJ65](ADJ65-uncertainty-primitive.md) — decision sensitivity / "which assumed weight
  to ground first"; ADJ72 is the byte-level analog for source claims.
- [ADJ66](ADJ66-spider-rulebook-grounding.md) — the open-book spider ADJ72's second layer
  invokes.
- [ADJ61](ADJ61-justification-gate.md) / [ADJ64](ADJ64-underdetermination-gate.md) — the
  justification and underdetermination gates the layered runs instantiate.
- [ADJ67](ADJ67-grounding-discipline-headtohead.md) / [ADJ68](ADJ68-defensibility-audit.md)
  — the HLE head-to-heads and the defensibility axis ADJ72 extends.
- [ADJ70](ADJ70-byte-provenance-experiment-results.md) — the `claimed_from_model_memory`
  authenticity class ADJ72 operationalizes.
