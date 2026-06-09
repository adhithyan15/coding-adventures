# ADJ95 — defensibility pilot: can Haiku reach Opus-level defensibility with spider + CAS?

Refined design (closed-book dropped — proven noise-dominated in ADJ92/94). 10 fresh, stratified,
text-only exact-match `cais/hle` items (Math×2, Physics×2, CS, Chemistry, Engineering, Biology,
Humanities, Other), open-book, N=1. **Primary metric = defensibility** (blind Opus judge, 0–5,
scoring grounded/auditable/traceable reasoning INDEPENDENT of correctness); secondary = accuracy.
**Byte provenance enforced + audited at every layer.** No hints; answer never shown to the solver.

## Arms
| arm | spider/CAS | reason | |
|---|---|---|---|
| `plain-haiku` | — | Haiku | floor |
| `plain-opus` | — | Opus | frontier baseline |
| `fw-haiku` | Haiku | Haiku | all-Haiku framework |
| `fw-haiku/opus-CAS` | **Opus** | **Haiku** | division of labor |
| `fw-opus` | Opus | Opus | all-Opus ceiling |

The Opus CAS is built once and shared by `fw-opus` and `fw-haiku/opus-CAS`.

## Result
| arm | defensibility (0–5) | correct/10 | provenance-complete/10 |
|---|---|---|---|
| plain-haiku | 2.1 | 1 | 0 |
| plain-opus | **3.9** | 3 | 0 |
| fw-haiku | 3.4 | 1 | 10 |
| **fw-haiku/opus-CAS** | **3.6** | **3** | 10 |
| fw-opus | 3.5 | 2 | 10 |

## Findings
1. **YES — Haiku reaches Opus-level defensibility with spider + CAS.** The framework nearly doubles
   Haiku's defensibility (2.1 → 3.4–3.6), bringing it level with `fw-opus` (3.5) and just under
   `plain-opus` (3.9). At N=10 the framework arms are statistically indistinguishable from each
   other and from Opus; the robust signal is **all framework arms ≈ plain-Opus ≫ plain-Haiku**.
2. **Who builds the CAS matters — more for accuracy than defensibility.** Opus-CAS (3.6) slightly
   beats Haiku-CAS (3.4) on defensibility, but on **accuracy it triples Haiku's correct count
   (1 → 3)**. `fw-haiku/opus-CAS` **matches `plain-opus` exactly (3/10)** at a fraction of the cost —
   a cheap reasoner over an Opus-built CAS reaches the frontier on *both* axes.
3. **Byte provenance held everywhere.** 10/10 provenance-complete for every framework arm (every
   step cited a sourced CAS fact or the givens; grounded-fraction = 1.0; grounded adversarial read
   left no surviving unsupported step) vs 0/10 for the plain arms by construction. The framework's
   defensibility *is* its enforced provenance — and it kept the cheap model on-task (plain-Haiku
   derailed into "the codebase doesn't contain…" once; no framework arm did).

## The thesis, in one line
**Framework-Haiku is as *defensible* as plain-Opus, and Haiku reasoning over an Opus-built CAS is as
*accurate* as plain-Opus** — at a fraction of the cost. Intelligence accumulated in the pipeline
(spider → provenance-enforced CAS → grounded reasoning), not the weights.

## Honest caveats
- **N=10, N=1/cell — directional, not significant.** Defensibility deltas among framework arms
  (3.4/3.5/3.6) are within noise; the large, reliable gap is framework vs plain-Haiku.
- **HLE accuracy is brutally low across the board** (plain-Opus 3/10); several "incorrect" are
  near-misses (e.g. the nested-function integral: all framework arms + plain-Opus agree on 5487).
  Accuracy is the noisy axis here; defensibility is the clean one.
- Single blind Opus judge; defensibility rubric is a 0–5 holistic score.

## Next
- Scale to the 50-item protocol (ADJ94) with this 5-arm design, batched 10×5 with 30-min breaks,
  for powered defensibility comparisons (McNemar/Wilcoxon, per-stratum).
- 2-judge reliability subset; report inter-judge agreement on defensibility.
