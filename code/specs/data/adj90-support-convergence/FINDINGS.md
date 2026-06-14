# ADJ90 — convergence control on the support loop + batch validation on Opus's failure set

Two ADJ89 follow-ups: (1) **convergence control** for the support-loop oscillation that cost 1/3
on both models; (2) **scaling** the convergence-controlled inference-support gate to the FULL set
of Opus failures from the ADJ88 10-item HLE run. All-Opus; no hints; the answer is never shown to
the solver. Workflows + result JSON saved.

## 1. Convergence control (single item, Al(OH)₃, N=3)
Two fixes to the ADJ89 support loop:
- **Missing-standard-constant → explicit assumption.** The auditor now categorizes each
  unsupported item; a well-known constant the problem omits (K_w = 1e-14 at 25 °C) is surfaced as
  a flagged *assumption* instead of being treated as a fatal hole that makes the problem
  "underdetermined" (the ADJ89 destabilizer).
- **Stop when going in circles.** The loop halts once the auditor stops surfacing genuinely-new
  issues, instead of re-reasoning until it destabilizes.

**Result: mechanism fixed, headline flat.** K_w was cleanly surfaced as an assumption in all 3
samples; loops converged in 1–2 rounds with no oscillation; **sample 1 — which oscillated →
wrong in ADJ89 — now converges → correct.** But gated stayed **2/3** (plain 1/3), because the
failure *moved*: sample 3's auditor passed a latent approximation in one round and the solve took
it (`5.8×10⁻³`) while plain happened to nail that one. **New bottleneck = auditor recall** (does
the support check flag the approximation?), not loop dynamics.

## 2. Batch: convergence-gated gate on ALL 5 Opus failures (N=2 each)
| item | kind | gold | plain | gated |
|---|---|---|---|---|
| Spin bordism | graduate derivation | `Z+Z+Z+Z+Z` | 0/2 | **2/2** |
| Al(OH)₃ | reasoning over givens | `1.776×10⁻³` | 0/2 | **1/2** |
| ferrite chart | chart/lookup | `10` | 0/2 | **1/2** |
| BAR unit | pure recall | `Shuriken` | 2/2 | 2/2 |
| PIE linguistics | specific knowledge | `hereth` | 0/2 | 0/2 (1 partial) |
| **TOTAL** | | | **2/10** | **6/10** |

**The inference-support gate roughly TRIPLED Opus's correctness on its own failure set (2→6/10).**

### Where it helped — and the prediction it broke
The prior hypothesis was "the gate only helps reasoning *over givens* (Al(OH)₃-class)." **Wrong —
too narrow.** The standout is **Spin bordism: 0/2 → 2/2.** Plain-Opus *guessed a rank* (`Z²`, `Z`);
gated-Opus, forced to make every inference survive a support check, **actually carried out the
Atiyah–Hirzebruch spectral-sequence computation** (filtration cells on the p+q=12 diagonal, d₂/d₃
differentials killing the Z/2 cells via `Sq²a₄=a₆, Sq¹a₆=a₇`) and landed `Z⁵`. The real pattern:
**the gate helps wherever Opus has the capability but *shortcuts* it one-shot** — graduate-math
derivation (bordism), reasoning over givens (Al(OH)₃), chart reasoning (ferrite). It adds no
knowledge; it forces the model to *apply* the capability it already has. This is the same
"surface latent reasoning" mechanism as ADJ88's Haiku/K_f, now lifting a *frontier* model on a
*graduate* problem.

### Where it didn't, and an honest confound
- **PIE linguistics (0/2):** needs specific Old English sound-change knowledge. The gate produced
  a well-structured, defensible answer and even surfaced the correct form (`hēreth`) as one fork
  in sample 2 — but mis-selected `heweth` as primary. The gap was *selection*, not generation.
- **CONFOUND — this batch is OPEN-BOOK, not closed-book.** The `general-purpose` agent has
  WebSearch, so both arms are web-capable. **BAR is 2/2 in both arms because plain-Opus searched
  the web** (in ADJ88's constrained pipeline it had no search and got it wrong). So BAR is
  gate-neutral (pure recall, solved by retrieval), and the gate's lift (2→6) is correctly isolated
  as *reasoning discipline on top of a web-capable solver* — the fair within-batch comparison.

## Synthesis
- **Convergence control** makes the gate robust (handles missing givens as assumptions, no
  oscillation) but exposes **auditor recall** as the next lever — the support check must reliably
  flag latent approximations (candidate fix: multiple auditor votes / union of flags).
- **The batch is the strongest evidence yet for the core thesis:** support-checked derivation
  converts a frontier model's one-shot *guess* into a correct multi-step *computation* — `Z²`→`Z⁵`
  on graduate algebraic topology, no added knowledge. Across Opus's failure set the discipline
  ~tripled correctness (2→6/10), concentrated exactly on items where the model had the capability
  but shortcut it, and neutral where the failure was retrieval (BAR) or specific recall (PIE).

## Caveats
- **N=2/item is noisy** — directional, not definitive. Al(OH)₃'s 1/2 is the known auditor-recall
  variance; one bordism run hit the iteration cap but still landed `Z⁵`.
- Grading is blind Opus vs the HLE gold string (literal match); we grade reproduction of the gold,
  not independent correctness of the gold itself.

## Next
- **Auditor recall**: multi-vote support check (union of flags) to close the Al(OH)₃-class 1/2 →
  2/2 and the sample-3 regression.
- A closed-book vs open-book split of the batch to cleanly separate reasoning-discipline lift
  (bordism, Al(OH)₃) from retrieval lift (BAR).
- Companion: the ADJ89 Haiku inference-support run (`adj89-opus-bp-coverage/bp_inference_support_
  haiku*`) — setup-discipline works for both models, execution separates them.
