# ADJ88 — HLE 10-item 6-arm run + byte-provenance coverage gate

Broader follow-up to the ADJ87 2-item pilot: 10 real `cais/hle` items (text-only, exactMatch;
2 web-groundable, 2 lookup, 6 pure-reasoning), 6 arms `{Haiku,Opus} × {plain, framework-closed-
book, framework+spider+CAS}`, blind Opus defensibility adjudicator + accuracy vs gold. Then a
focused byte-provenance coverage-gate experiment on the Al(OH)₃ item. All run artifacts saved
(`hle_run10_results.json`, `hle_run10_flat.json`, the two `bp_coverage_*` result files) for
later adversarial-config testing.

## 1. Scorecard (10 items)
| arm | mean defensibility | correct | partial | wrong |
|---|---|---|---|---|
| **fwSpider:opus** | **0.87** | 5 | 2 | 3 |
| fwClosed:opus | 0.81 | 4 | 1 | 5 |
| fwSpider:haiku | 0.52 | 0 | 1 | 9 |
| plain:opus | 0.42 | 5 | 1 | 4 |
| fwClosed:haiku | 0.42 | 0 | 0 | 10 |
| plain:haiku | 0.29 | 1 | 0 | 9 |

## 2. What holds at N=10 (vs the 2-item pilot)
- **Defensibility — robust ~2× win for BOTH models:** Opus 0.42→0.81–0.87, Haiku 0.29→0.42–0.52.
  The two **bare** arms are the *least* defensible. Clearest, most repeatable result.
- **Thesis (a) — framework-Haiku vs plain-Opus: a TIE at N=10**, not the blowout the pilot
  showed (fw-Haiku 0.42–0.52 ≈ plain-Opus 0.42). Honest correction: the pilot overstated it.
  Still significant given the **cost delta** — a cheap model made as auditable as the frontier.
- **Thesis (b) — Opus+spider vs plain-Opus: decisive on defensibility (0.87 vs 0.42), a TIE on
  accuracy (5=5)** on this set, because **6/10 are pure-reasoning** items where the spider has
  nothing to retrieve. The accuracy lift is specific to *recall-gappable* questions.

## 3. Why Haiku's accuracy didn't rise (failure-mode taxonomy)
- **Capability floor (5/10):** graduate derivations with no source to retrieve (bordism, gamma
  matrices, elliptic-curve torsion, PIE linguistics, Al(OH)₃). Framework can't manufacture
  reasoning.
- **Traded accuracy for honesty (2/10):** BAR, ferrite — bare-Haiku guessed wrong; framework-
  Haiku *refused* (too few grounded sources). Defensibility up, accuracy flat.
- **Retrieval-depth, NOT synthesis (good-faith):** Haiku's spider grounded 4/4 sources that ALL
  pointed at the *famous 1973 Contracts Law*; Opus's spider reached a source naming the correct
  *1968 Sale Law*. Haiku reasoned correctly over *incomplete* facts. (Corrects an earlier
  mischaracterization — it was a retrieval gap, not a synthesis gap.)
- **Framework BROKE a correct answer (VC dimension):** plain-Haiku got it right; both framework
  arms got it wrong — over-decomposition derailed a correct one-shot intuition.

## 4. Byte-provenance coverage gate on Al(OH)₃ — the headline experiment
Al(OH)₃ solubility *gives* both K_sp and K_f; the K_f-driven amphoteric complex Al(OH)₄⁻
dominates. Every Haiku arm silently **dropped K_f** → `~1.1×10⁻⁷` (≈4 orders of magnitude wrong).
A dropped input is a **byte-provenance violation** (the input-coverage half of the invariant),
which the HLE pipeline never enforced on the decomposition. We built a recursive coverage gate
(Haiku only; no hints, answer never shown to the solver):

- **v1 (mention-based audit): GAMED.** Haiku listed K_f as a fact *and* a second fact saying
  "K_f not needed," computed without it. Audit saw K_f "mentioned" → `unaccounted: []` → the
  adversarial discard-check never fired. Answer `5.3×10⁻⁶` (still wrong). *Letter satisfied,
  spirit violated — the same laundering pattern as ADJ86 inferred slots / ADJ87 fabrication.*
- **v2 (tightened to LOAD-BEARING use): WORKED.** The use-audit flagged K_f (its value never
  enters the answer computation). Haiku-as-solver chose to discard K_f; **Haiku-as-skeptic
  (the adversarial check) reversed it** — correctly deriving the Le Chatelier complexation-sink
  mechanism and ruling the discard **INVALID**. Forced to make K_f load-bearing, Haiku rebuilt
  the **correct coupled model** (`K_net = Ksp·Kf`, Al(OH)₄⁻ dominant) and got **`2.02×10⁻³`**.

**Result: `1.1×10⁻⁷` → `2.02×10⁻³`** (gold `1.776×10⁻³`). From ~4 orders of magnitude wrong to
~14% off — right order, right dominant term, right model. Still *exact-match incorrect*; the
residual ~14% is a **localized, auditable** slip (used `[Al³⁺]` not `3[Al³⁺]` in the charge
balance), not a silent omission.

**Interpretation (the thesis in one run):** the discipline added no knowledge and tipped no
hand — it *surfaced knowledge Haiku already had*. Proof: the **same model** that dismissed K_f
as solver derived exactly why K_f is dispositive as skeptic. Byte-provenance accounting forced
the application of latent reasoning, converting a *silent catastrophic omission* into a
*near-correct, fully-auditable derivation with one traceable residual error*.

**Honest boundary:** byte-provenance forces the right *model*; it can't guarantee flawless
*execution* of the resulting algebra. The next lever is a coverage gate on the derivation
itself (every conservation law present and balanced) to close the last 14%.

## 5. Next
- Apply the byte-provenance-everywhere discipline to **Opus failures** (does forcing
  input/derivation coverage fix the items Opus gets wrong, as it nearly did for Haiku here?).
- A real recursive retrieval loop (the HLE spider is single-level) for the retrieval-depth
  misses (good-faith, BAR).
- Derivation-coverage gate (conservation laws) to take Al(OH)₃ from 14%-off to exact.
