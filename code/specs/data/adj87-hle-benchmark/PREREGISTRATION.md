# ADJ87 — HLE defensibility benchmark (model scale × framework depth): pre-registration

**Status: DRAFT for sign-off. Nothing run yet.**

## The design (Adhithya's, 2026-06-08)

On a held-out set of Humanity's-Last-Exam (HLE) items, cross **2 models × 3 framework
depths**, judged by a **blind adjudicator on defensibility** AND scored on **accuracy vs the
real HLE answers**:

| arm | model | framework | book |
|---|---|---|---|
| **A1 bare** | Opus, Haiku | none (one-shot) | closed |
| **A2 framework, closed-book** | Opus, Haiku | the real decompose→ground→chain pipeline, grounding from the model's **own recall** (each step flagged grounded/assumed; no retrieval) | closed |
| **A3 framework + spider + CAS** | Opus, Haiku | the real pipeline, grounding each step by **spidering sources into the CAS** (verbatim citations) | open |

- **Blind adjudicator:** a separate model role, given the arms' outputs **unlabeled and in
  randomized order**, judges *which is more defensible* (pairwise) — every claim traceable to
  a source/given/flagged-assumption — NOT which is correct.
- **Accuracy** scored separately against the real HLE gold answer.

> NOTE — framework shape: HLE items are **knowledge/reasoning QA**, so the framework here is
> the **decompose → ground → chain → verify** pipeline (ADJ66/68/69 lineage), *not* the ADJ84
> adjudication engine (that is for rule-application). "Real framework, not a single prompt"
> still holds: A2/A3 must expose the decomposed claim-IR, the grounded facts (recall-flagged
> for A2, CAS-cited for A3), and the chain/proof — multi-stage, not one prompt.

## Pre-registered hypotheses

- **H1 (defensibility depth):** defensibility A3 > A2 > A1, within each model. Grounding the
  chain (even from recall, A2) beats bare prose; spidered citations (A3) beat recall-flagged.
- **H2 (defensibility-parity across scale — the headline):** under the framework the
  **Opus↔Haiku defensibility gap collapses** — Haiku-A3 ≈ Opus-A3 on defensibility, even
  though Opus-A1 ≫ Haiku-A1. Defensibility is a property of the discipline, not the scale.
- **H3 (accuracy is a different axis — honest boundary):** accuracy is driven by **retrieval,
  not discipline** — A3 (open-book) > A1≈A2 (closed-book, recall-bounded). The framework does
  NOT close the Opus↔Haiku *accuracy* gap on closed-book arms; where a model lacks the
  knowledge it **abstains/flags** rather than fabricating. Defensibility ↑ does not imply
  accuracy ↑.
- **H4 (no fabrication):** citation-fabrication = 0 in A3 (deterministic byte-anchor on the
  CAS); A2 flags assumptions explicitly; A1 fabricates confidently.

## Metrics

- **Defensibility:** (a) blind pairwise adjudication (win-rate A3>A2>A1, Haiku-A3 vs Opus-A3);
  (b) defensibility fraction (verifiable claims / total) per arm.
- **Accuracy** vs HLE gold (exact/auto-graded where possible).
- **Abstention/underdetermination rate** (A2/A3 flag missing-knowledge instead of guessing).
- **Citation-fabrication count** (A3 must be 0).

## Staging + cost

- HLE items: a held-out, **less-contaminated** subset (avoid the famous public samples). N
  pilot = 10, then scale.
- 2 models × 3 arms = 6 generations/item + 1 adjudication; A3 adds spider web-research
  (the expensive part). Pilot first to measure per-item cost (the ADJ86 pilot was ~90k
  tokens/item for a 4-stage adjudication; A3 with web research will be higher).
- **Prerequisite builds:** (1) the QA grounding-chain pipeline as a reusable multi-stage
  workflow (A2 = recall-grounded, A3 = spider+CAS); (2) HLE item loader (needs HuggingFace
  access — `cais/hle`); (3) the blind pairwise defensibility adjudicator.

## Open decisions for sign-off

- [ ] HLE source: the gated `cais/hle` dataset (needs HF token) vs a hand-curated
      less-contaminated subset?
- [ ] Pilot N=10 first (recommended) to validate the 3-arm pipeline + adjudicator + cost?
- [ ] A2 "closed-book framework" definition above (recall-grounded, flagged) — confirm.
