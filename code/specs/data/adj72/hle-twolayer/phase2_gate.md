# Phase 2 — confabulation gate result

Per-token gloss across 3 independent closed-book resamples:

| token | surface | gloss (all 3 rounds agree) | stability | mean conf |
|---|---|---|---|---|
| t1 | RGYNᵓ | "Regina" (the deceased) | STABLE | 0.80 |
| t2 | BT | **"daughter of"** | STABLE | 0.91 |
| t3 | ḤRY | **"Ḥari / Ḥairan — father's name"** | STABLE | **0.61 (low)** |
| t4 | BR | **"son of"** | STABLE | 0.95 |
| t5 | ᶜTᵓ | "ʿAttā — grandfather's name" | STABLE | **0.65 (low)** |
| t6 | ḤBL | "Alas!" | STABLE | 0.84 |

## The negative result (stability alone fails)

**Every token gloss is STABLE across all three resamples.** The confabulation gate
keyed on byte-stability therefore PASSES all six rules — and the composed reading is:

> "Regina, daughter of Ḥari, son of ʿAttā. Alas!"

This is the **stable wrong answer** — the exact ADJ67 failure, reproduced. The error
(reading `BT ... BR ...` as the filiation formula "daughter of … son of …" and glossing
ḤRY as a personal name rather than the Ḥ-R-R "freed" root) is *systematic in the model's
latent knowledge*, so resampling reproduces it identically. **Byte-stability cannot catch
a stable error.** This confirms, on a real case, the boundary found in the 50-sample run:
stability certifies *retrieved-not-invented*, not *correct*.

## What the gate's second channel did flag

Confidence is bimodal: the structural connectors (BT 0.91, BR 0.95), the name Regina,
and the lament ḤBL came back high; the two **content tokens glossed as names — t3 ḤRY
(0.61) and t5 ᶜTᵓ (0.65) — came back low.** The model's weakest link, by its own
confidence, is exactly ḤRY: the token it (wrongly) glossed as the father's name.

## Decision → recurse open-book on the flagged tokens

Per the layered design: the gate-flagged low-confidence content tokens (t3 ḤRY, t5 ᶜTᵓ)
are sent to an **open-book justification layer** that grounds their actual lexical
meaning in authoritative sources, and checks whether the proposed closed-book reading is
*entailed* by what the sources say. The hypothesis under test (Adhithya's "layers"
claim): grounding the flagged token ḤRY open-book will reveal the Ḥ-R-R "freed" root,
contradict the "father's name" gloss, and — because open-book byte-grounding outranks
closed-book memory — propagate back to correct the confident-but-wrong `BT`/`BR`
filiation parse.
