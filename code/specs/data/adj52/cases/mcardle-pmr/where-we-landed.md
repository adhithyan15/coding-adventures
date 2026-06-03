# ADJ52 — novel case end-to-end: McArdle disease mimicking PMR

First fully-novel case run through the entire pipeline (ingest → derive →
compile → counterfactuals → plain-Claude control → blind judge).

- **Source:** PMC11724029 (2025, open access). **Ground truth:** late-onset
  McArdle disease (GSD V, PYGM), initially misdiagnosed as polymyalgia
  rheumatica. The discriminator: markedly elevated CK is incompatible with
  PMR; lifelong exertional intolerance is the metabolic-myopathy clue.
- **Sanitisation:** prose stripped of the diagnosis; ground truth held by the
  orchestrator only. Artifacts: `00`–`07` + this file.

## Pipeline

1. **Domain-blind ingester** (`02-ingestion.json`) — inferred the domain,
   captured 40 facts, and flagged the central tension (metabolic vs
   inflammatory vs PMR) as a typed uncertainty + raised the CK-disproportion
   query — without being told the answer. (NOTE: an earlier ingester run was
   discarded because the Agent description leaked the case name; see
   `lessons.md`. This was the clean re-run.)
2. **Recursive rulebook deriver** (`03-derived-rulebook.adj`) — built a
   6-candidate differential from the IR alone, recursing into subcategories,
   with real citations (GeneReviews, EULAR/ACR, AFP, MSD, AMBOSS) and a
   `% rationale` on every clause. Modeled the discriminator honestly: CK as
   LR<1 against PMR, exertional features as strong LRs for McArdle. (6
   numeric/unit term-args normalized to qualitative atoms for grammar
   compliance — see finding 3.)
3. **Engine** (`05-run-output.txt`):

   | Query | Posterior | |
   |---|---:|---|
   | diagnosis(mcardle_disease_gsd_v) | **100.0%** | ✓ true diagnosis, top |
   | diagnosis(coexisting_pmr_and_metabolic_myopathy) | 99.5% | two-hit reading |
   | diagnosis(polymyalgia_rheumatica) | 97.5% | trap; CK fired −1.61 against |
   | diagnosis(seronegative_inflammatory_myopathy) | 3.4% | ✓ rejected |
   | diagnosis(diabetic_amyotrophy) | 0.5% | ✓ rejected |
   | next_step(pygm_molecular_genetic_testing) | **83.3%** | ✓ correct disposition |

4. **Plain-Claude control** (`06-plain-claude.json`) — *also correct*: McArdle /
   metabolic myopathy, forearm test + biopsy + PYGM, at "Moderate" confidence.
5. **Blind judge** (`07-judge.json`), framework arm presented WITH its real
   audit trail — **winner: plain Claude (B).**

## Where we landed

The framework's deterministic engine **independently reached the correct
diagnosis and the correct disposition, with a fully cited audit trail** that
makes the PMR-vs-McArdle discrimination explicit (CK −1.61 against PMR). That
is a real positive on a genuinely novel case.

But the blind judge **still preferred plain Claude**, for honest reasons that
now repeat across both cases run (lymphoma + McArdle):

1. **Over-saturated, internally-incoherent posteriors — CONFIRMED, not a
   rendering artifact.** McArdle 100%, coexisting 99.5%, *and* PMR 97.5% all
   read near-certain simultaneously. The LR engine scores each query as an
   independent binary `P(dx | evidence)`; the candidates never compete, so
   they don't form a coherent ranked differential and the numbers read as
   false precision. **This is the #1 thing standing between the framework and
   demonstrable value over a strong base model.** Fix: normalize across the
   mutually-exclusive candidates (a softmax-style differential that sums to
   ~1 and ranks coherently) and temper extreme posteriors. Directly serves the
   "uncertainty at the core" goal.
2. **Inert VOI markers.** The deriver declared `uncertain { test(...) } for
   diagnosis(...)` but wrote no `contributes` clauses *from* those test
   results, so the counterfactual panel reported VOI range 0.0 — the
   confirmatory test cannot move the posterior. The panel correctly surfaced
   this modeller gap. Fix: the deriver must emit `contributes` from the
   test-result terms to the conclusion.
3. **Term normalization.** The deriver emitted numeric/unit args
   (`age(64_years)`, `creatine_kinase_later_peak(3473_IU_per_L)`) that violate
   the adj-lang IDENT grammar (no leading digit / uppercase); 6 were
   normalized by hand. Fix: an automated normalization pass, or instruct the
   deriver to emit grammar-valid qualitative atoms.

**Net:** the pipeline works end-to-end on a novel case and gets the answer
right — but to *beat* a strong base model it must add calibration and a
coherent differential, not just citations. When the base model already
answers correctly with graded confidence, defensibility alone doesn't win;
calibrated, coherent uncertainty does.
