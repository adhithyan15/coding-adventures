# Paper 1 · E3 — cross-domain consolidation

> **Work item W3** (analysis; no new LLM runs). Consolidates the scattered cross-domain runs into
> one results artifact supporting the claim: **the same machinery runs across many adjudicative
> domains with no per-domain code — only the rulebook/corpus changes.** Honest about where it wins,
> ties, and the two named failure modes. Plan: [`PAPER1-WORKPLAN.md`](PAPER1-WORKPLAN.md).

## 1. The claim E3 supports

The byte-provenance pipeline is **rulebook-parameterized, not domain-coded**: to move to a new
domain you supply a different grounded corpus + rulebook; the stage contract, coverage/entailment
gates, CAS, and engine are unchanged. E3 is the breadth evidence for that claim — and a deliberately
*honest* one (the framework's value is auditability/correctability, not out-diagnosing experts).

## 2. The domains exercised (no per-domain code)

| domain | source run | what was grounded / decided |
|---|---|---|
| Clinical — ACS chest pain | ADJ48, [ADJ50](ADJ50-nejm-stress-test-on-acs-rulebook.md) | ACS rulebook; stress-tested on a real published NEJM case |
| Clinical — bacterial meningitis | [ADJ44](ADJ44-mycin-2026-meningitis.md) | recursive rulebook derivation; CSF differential (the MYCIN-2026 anchor) |
| Finance — M&A deal completion | [ADJ49](ADJ49-ma-deal-completion-in-adj-lang.md) | deal-completion criteria in adj-lang — "the investment-banker objection answered," no code change from clinical |
| Legal — aviation/FSIA briefs | [ADJ38](ADJ38-cross-domain-framework-validation.md) (Mata v. Avianca anchor) | sentence/phrase-level tiling; citations as entities; 7-domain gap inventory |
| Coding / legal / memory | [ADJ70](ADJ70-byte-provenance-experiment-results.md) | byte-provenance shakedown; "code is input" correction; `claimed_from_model_memory` class |
| Naturalization law (8 U.S.C. 1427) | [ADJ71](ADJ71-cas-program-cache-experiment.md) | CAS program-cache: ground once → compiled library → held-out case runs with **zero answer-time model calls** |
| 3 stress sub-domains (n>1) | [ADJ56](ADJ56-cross-domain-stress-test.md) | three grounded corpora, case-blind, no per-domain code |
| 6 non-medical domains | [ADJ59](ADJ59-cross-domain-validation.md) | head-to-head vs plain Claude, blind judge, held-aside ground truth |
| **10 regulatory/legal domains × 10 (corpus-1)** | [**run100**](../data/adj101-defensibility-100crossdomain/run100/) | the pre-registered 100-item benchmark; byte-accounting 100/100, no hallucinated rules 100/100 |
| **10 *new* regulatory/legal domains × 10 (fresh, gold-vetted)** | [**run100b**](../data/adj101-defensibility-100crossdomain/run100b/) | unseen domains; byte-accounting 98/100, no hallucinated rules 100/100, abstention 26/30 |
| Methodology / breadth survey | [ADJ19](ADJ19-cross-domain-empirical-bench.md), ADJ38 | the cross-domain bench shape; 7-domain catalog + gaps |

Spanning **clinical, financial, legal, regulatory, coding, and general knowledge-work** domains on
one unchanged pipeline — now anchored by a **pre-registered 200-item benchmark across 20 distinct
regulatory/legal domains** (run100 + run100b), the largest and most rigorous breadth evidence in the
program. The 20 domains: tax filing, employment leave, insurance claims, benefits eligibility,
building permits, academic appeals, consumer credit, contract disputes (corpus-1); plus
healthcare-billing, environmental-permitting, consumer-warranty, securities-compliance, food-safety,
rental-housing, professional-licensing, customs-import, data-privacy, workers-compensation
(corpus-2, **never seen by the framework before the run**).

## 3. The honest results (this is the point — not a clean sweep)

- **Breadth holds.** Across all of the above, moving domains required **only a new corpus + rulebook**,
  never a code change. That is the load-bearing breadth claim and it survives.
- **Head-to-head (ADJ59, 6 non-medical domains, blind judge).** First three domains the framework
  **lost 0–3** — auditability *without commitment* got correctly penalized. A single fix, the
  **qualitative verdict** (report the feature-derived answer, not just the audit trail), flipped it to
  **4 wins / 1 tie / 1 loss, correct in all 6**, vs plain Claude's 3 correct / 2 partial / 1 wrong.
  The lesson: grounding must *commit to a decision*, not just expose structure.
- **Two named failure modes (ADJ56) — the most useful results.**
  1. **Grounded extrapolation:** a grounded corpus can drive a confident "correct" answer that is
     actually **correct-by-extrapolation** (0.88 → 0.999), i.e. the chain over-extends beyond what the
     bytes support.
  2. **Ungrounded over-penalization:** without grounding the reasoner runs hot (0.95–0.98) and can be
     badly wrong by over-weighting a single feature (e.g. age → 0.002).
  Honest framing carried into the paper: **the framework does not out-diagnose the expert**; it makes
  the reasoning auditable and the errors localizable/correctable.
- **Derive-once across domains (ADJ71).** The naturalization-law CAS program-cache demonstrates the
  reuse payoff in a *non-clinical* domain: ground once, compile a library, and execute a held-out case
  with **zero answer-time model calls** — the same mechanism E2 measures (this is the cross-domain
  evidence for the derive-once claim, and the bridge to paper 2's MYCIN proof).
- **The 200-item benchmark (run100 + run100b) — breadth + measurement validity at scale.** The
  byte-provenance *spine replicates across 20 domains and two independent corpora*: rulebook
  byte-accounting **100/100 (corpus-1)** and **98/100 (corpus-2, the 2 misses being the input gate
  firing, not leaking)**, **no hallucinated rules 100/100** in both. Abstention discipline holds out
  of sample: underdetermined items → INDETERMINATE (abstain, not fabricate) **24/30** and **26/30**;
  the combined adjudication gate (N-reader entailment × decision-sensitivity × precedence) leaves
  **zero confident-wrong verdicts** — every residual is a safe abstention, an input-gate flag, or a
  genuine conflict. Crucially, the **same adversarial reading reflexively audited the benchmark's own
  gold**: it caught **3 mislabeled-determinate items (corpus-1)** and **4 mislabeled-underdetermined
  items (corpus-2)** *before* scoring — the ADJ99 measurement-validity theme recurring on fresh data.
  This is the strongest single piece of E3 breadth evidence and ties the cross-domain claim to the
  paper's measurement-validity contribution.

## 4. How this maps to the paper's contributions

- Supports the **stage-contract / rulebook-parameterized** claim (C1): one machinery, 20+ domains,
  no per-domain code — with the 200-item benchmark as the quantitative anchor (byte-accounting and
  no-hallucinated-rules hold across both corpora).
- The ADJ59 commitment fix and the ADJ56 failure modes are **honesty ballast** for the discussion:
  they bound the claim to *auditability/correctability*, not accuracy supremacy — exactly the
  goal-shift the paper argues for.
- ADJ71's zero-model-call cross-domain reuse foreshadows E2's recurring-cost result and paper 2.
- The run100/run100b **reflexive gold audit** is direct cross-domain support for the
  **measurement-validity** thread (the ADJ99 rescore's lesson): the framework's adversarial reading
  catches errors in its own benchmark's gold, at scale, on unseen domains.

## 5. What E3 is NOT
Not a new benchmark and not an accuracy leaderboard. It is a breadth-and-honesty consolidation:
*same machinery, many domains, here is where it helps, where it merely matches, and the two ways it
fails.* Any accuracy numbers quoted are secondary and inherit each source run's scoring caveats.

## 6. Consolidation status
- **Primary quantitative anchor — DONE:** the 200-item benchmark (run100 + run100b) supplies the
  domain × {ran-with-no-code-change, byte-accounting, no-hallucinated-rules, abstention,
  gold-audit-hits} table directly from `run100*/run100_results.json` + `FINDINGS.md`. Every number
  here traces to a committed artifact (byte-provenance applied to the paper itself; W6).
- **Legacy runs (ADJ49/56/59/70/71) — qualitative ballast:** retained for the breadth narrative and
  the two named failure modes; their accuracy numbers inherit each source run's scoring caveats and
  are secondary to the pre-registered benchmark.
- **Remaining (optional polish):** one publication figure rendering the 20-domain table; confirm the
  ADJ59 numbers cited are the post-ground-truth-leak-fix ones (§5 of ADJ59). Neither blocks the
  manuscript — the load-bearing E3 evidence is the 200-item benchmark, which is complete and merged.
