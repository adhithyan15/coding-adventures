# ADJ51 — Byte-Recursive Provenance: Contracts at Every Pipeline Layer

> **Headline.** ADJ50 quietly violated the framework's own byte-accounting
> principle: at the ingestion layer, four facts from the source prose
> (hyperlipidemia, former smoker, CACS, progressive worsening) were
> silently dropped because the rulebook had no rule for them. The
> engine never saw them. The failure was invisible. ADJ51 closes this
> by recognising that **byte-accounting is the framework's load-bearing
> architectural primitive, and it must apply recursively at every layer
> of the pipeline** — ingestion (prose → typed facts + queries),
> derivation (rationale → adj-lang clause), retrieval (claim → source
> bytes), and reasoning (clause → posterior shift). Two end-to-end
> experiments on real published cases (PMC12750962 and PMC12914605)
> demonstrate the contracts in action with sub-agent-driven pipelines
> and a domain-agnostic runner. The PMC12914605 experiment — a
> "lymphoma masquerading as infection" case — produced the correct
> disposition (admit / biopsy) at 99.9% confidence and correctly
> rejected the actinomyces red herring at 10%, while honestly tagging
> two language limitations the deriver could not encode.

## The principle

The framework's contract has always been: **every byte of source is
accounted for, no assertion moves the posterior without provenance.**
ADJ48–ADJ50 enforced this rigorously on the **reasoning** step:
contributes / joint / prior clauses all carry citations; the engine
refuses to fire a clause without a `source ".."` annotation.

It was never enforced on the **other layers**: prose-to-observations
(ingestion), or rationale-to-clause (derivation). ADJ50's vignette
comment documents an ingestion-layer violation in plain sight:

> "Inputs that the rulebook has no rule for (hyperlipidemia, former
> smoker, CACS, progressive worsening) are omitted here and discussed
> in the ADJ50 spec as missing rules."

That is the same confident-hallucination failure mode the framework
was designed to eliminate, just one layer earlier than where we'd
been enforcing it. The fix is to apply the byte-accounting contract
**recursively** — at every layer, every byte of every artifact in
the pipeline carries a disposition; nothing is silently dropped.

| Layer | Artifact | Contract |
|---|---|---|
| Source | papers, guidelines, court opinions, filings | every byte indexable; structured claims extracted with source byte-spans |
| Ingestion | typed observations + queries | every byte of input prose tagged (`extracted` / `discarded_as_non_factual` / `below_extraction_threshold` / `ambiguous_but_flagged`) |
| Derivation | adj-lang rulebook | every clause has a rationale block; rationale must mechanically match the clause's behaviour; if it cannot, an `intent_not_encoded` framing block names the gap |
| Reasoning | posterior + audit trail | every probability shift traces to a fired clause whose evidence term traces to an observation whose source span traces to source bytes |

## Experiment 1 — PMC12750962 (cardiology stress test)

**Case.** 47-year-old male with exertional chest pain, normal ECG
and serial troponins, but 100% pRCA occlusion confirmed at
catheterization. The same case ADJ50 used.

**Pipeline.**

1. Generic ingester subagent: prose (2360 bytes) → 58 typed
   observations + 3 queries (`diagnosis(acute_coronary_syndrome)`,
   `diagnosis(stable_angina)`, `disposition(admit_for_further_cardiac_workup)`),
   100% byte coverage.
2. Generic deriver subagent: observations + queries → 61-clause
   adj-lang rulebook with real peer-reviewed citations (Diamond-Forrester,
   Braunwald, Twerenbold NEJM, Greenwood CE-MARC, Detrano MESA,
   Amsterdam AHA/ACC, Gulati 2021 AHA/ACC chest-pain guideline).
3. Engine via `cargo run --bin adj51` on the assembled
   rulebook + vignette.

**Results.**

| Query | Posterior | Decision | Ground truth |
|---|---:|---|---|
| `diagnosis(acute_coronary_syndrome)` | 3.8% | DISCHARGE | ❌ patient had ACS |
| `diagnosis(stable_angina)` | ~100% | ABOVE | ✓ chronic CAD substrate confirmed |
| `disposition(admit_for_further_cardiac_workup)` | ~100% | **ADMIT** | ✓ **matches actual clinical decision** |

The framework got the **decision** right (admit) even though it got
the **diagnostic label** wrong on Q1. That divergence is itself the
signal.

**Why Q1 was wrong.** ACS is an umbrella over NSTEMI, STEMI, and
unstable angina. The serial-troponin rule fired at -2.0/-2.5 logits
across the board for ACS, but unstable angina is troponin-negative
by definition. The deriver's prose rationale *said* it intended to
bound the rule for UA; the clause it actually wrote applied
uniformly. **The rulebook layer had no contract requiring the
rationale to match the clause.** That gap is the load-bearing finding
of experiment 1.

**Five architectural takeaways, domain-independent:**

1. **Categories with sub-types are a structural failure mode.** A
   rule that correctly weights evidence for sub-type A, applied
   uniformly to the parent category, reads as correct but isn't.
   Generic: M&A "clean financial audit" is strong evidence against
   the *fraud* sub-type of deal failure, weak evidence against the
   *regulator-block* sub-type.
2. **Cross-query disagreement is itself an audit signal.** Q1 said
   3.8%, Q3 said 99.9%; coupled queries should move together; the
   disagreement flags a miscalibrated rule even without domain
   knowledge.
3. **There is a translation gap between deriver-intent and
   deriver-output.** LLMs producing structured artifacts from intent
   often lose nuance at the encoding step. The fix is mechanical
   verification that the artifact reflects the intent.
4. **Multi-query is load-bearing for resilience.** A single-query
   framework would have collapsed here; the disposition query
   recovered the correct clinical action.
5. **Value-deepening was the wrong tool.** The troponin *value* was
   unambiguous (5.9 ng/L well below URL); the *rule scope* was
   wrong. The framework needs **rule-applicability deepening** as a
   distinct mechanism alongside value-deepening.

## The rulebook byte-accounting contract

The fix follows from generalising the ingestion contract one layer
down. Every byte of the deriver's output is tagged by structure. A
single, mechanically checkable rule:

**Every clause (`prior` / `contributes` / `interacts`) must be
immediately preceded by a rationale block — one or more `% ...`
comment lines — that states in plain prose what the clause is
supposed to encode.**

A semantic verifier later reads each `(rationale, clause)` pair and
checks alignment. If a rationale claims "X" and the clause does "Y",
the verifier flags the mismatch.

This means: **do not write a rationale that says something the
clause doesn't enforce.** When a nuance is recognised but
inexpressible in adj-lang's current shape, the deriver has three
honest options:

1. **Use term granularity to scope the rule.** Split a query into
   sub-queries with independent priors so the rule attaches to the
   correct scope.
2. **Use `interacts` to encode conditional behaviour.** A multi-term
   joint clause can encode "this contribution applies only when these
   other terms co-occur."
3. **Emit an `intent_not_encoded` framing block AND reduce the
   magnitude.** When the nuance is genuinely inexpressible, the
   deriver tags the prose statement explicitly and writes the clause
   at a magnitude defensible across the broader scope, not the
   magnitude that would be appropriate for the unscoped intent.

`code/specs/data/adj51/experiment2/validate_rulebook.py` is the
structural validator. The semantic verifier is future work
(envisioned as a separate verifier subagent reading each
`(rationale, clause)` pair and returning alignment / mismatch
verdicts).

## Experiment 2 — PMC12914605 (lymphoma masquerading as infection)

**Case.** 26-year-old African-American female with facial swelling,
cough with hemoptysis, leukocytosis (WBC 44k→55k with myelocyte
precursors), mediastinal mass on CT, positive Actinomyces sputum
culture, failed broad-spectrum antibiotics. Final diagnosis at
biopsy: Primary Mediastinal Large B-cell Lymphoma (PMBCL).

The prose was sanitised to exclude any byte that named the final
diagnosis or named expert recommendations. The framework had to
derive the right answer from raw clinical observations alone.

**Pipeline.**

1. Generic ingester (no clinical hints, no example queries):
   prose (1510 bytes) → 47 observations + 4 queries
   (`diagnosis(underlying_pulmonary_pathology)`,
   `diagnosis(pulmonary_malignancy)`,
   `diagnosis(pulmonary_actinomycosis)`,
   `next_diagnostic_step(biopsy_or_advanced_workup)`).
   100% byte coverage.
2. Sandboxed deriver subagent (file access restricted to the
   ingestion JSON only; WebSearch / WebFetch / Bash / Write
   permitted): observations + queries → 77-clause adj-lang
   rulebook with rationale blocks for every clause. Structural
   validator PASS.
3. Deriver recursed: introduced 2 additional sub-queries
   (`diagnosis(pulmonary_tuberculosis)`,
   `diagnosis(hematologic_malignancy_myeloid)`) because the
   literature distinguishes them as separate categories with
   different evidence profiles.
4. Deriver wrote 2 `intent_not_encoded` framing blocks honestly
   tagging language limitations (self-attributed weight loss as
   ambiguous self-report; Actinomyces as ambiguous
   colonizer-vs-pathogen).

**Results.**

| Query | Posterior | Decision | Ground truth alignment |
|---|---:|---|---|
| `diagnosis(underlying_pulmonary_pathology)` | 82.4% | ABOVE | ✓ something IS going on |
| **`diagnosis(pulmonary_malignancy)`** | **100.0%** | **ABOVE** | ✓ **true diagnosis** |
| `diagnosis(pulmonary_actinomycosis)` | 10.5% | BELOW | ✓ red herring correctly rejected |
| **`next_diagnostic_step(biopsy_or_advanced_workup)`** | **99.9%** | **ABOVE** | ✓ **correct disposition** |
| `diagnosis(pulmonary_tuberculosis)` | 35.0% | (marginally ABOVE) | ~ correctly low rank; nudged above threshold by shared imaging pattern |
| `diagnosis(hematologic_malignancy_myeloid)` | 99.4% | ABOVE | ~ honest gap: leukemoid reaction can't be disambiguated as paraneoplastic vs. primary without immunophenotype, which the sanitised prose excludes; framework correctly flags that biopsy will resolve |

**The framework got the load-bearing decisions right:** malignancy
high, red herring rejected, biopsy required. The audit trail makes
the reasoning visible end-to-end. The two sub-queries the deriver
introduced via recursion correctly surfaced the hematologic
differential rather than collapsing it into "infection vs. cancer."
The `intent_not_encoded` blocks made the rulebook's limits legible
inside the rulebook itself.

## The Google-style indexed-corpus architecture (sketched)

The path from experiment to deployment runs through one
architectural shift: **the deriver should retrieve from a
pre-indexed source corpus, not derive from web searches per case.**

Every paper, guideline, court opinion, SEC filing — every source —
is pre-ingested through the same byte-accounting IR. Each claim in
each source becomes an indexed structured assertion:

```
indexed_claim {
  predicate_term:    troponin(below_url)
  query_target:      diagnosis(myocardial_infarction)
  likelihood_ratio:  0.10
  applicable_scope:  ["adult_ed_chest_pain", "0_1h_pathway"]
  source:            Reichlin T et al., Arch Intern Med 2012;172:1211-1218
  source_byte_span:  [bytes 4823 to 5042 of the indexed source]
  trust_tier:        authoritative
  index_timestamp:   2026-05-12
}
```

At runtime, the deriver becomes a retrieval + assembly job (~10
seconds) instead of a web-research job (~8 minutes). Hallucinated
citations become impossible — the deriver can only emit clauses
backed by indexed claims, or tag `intent_not_encoded(no_indexed_source)`
visibly. Provenance is end-to-end byte-traceable: physician asks
"why this LR?" → indexed claim → source paper → exact bytes.

This is the deployment path. The indexed corpus is the framework's
compounding asset and the reason the framework gets better with
every paper added — auditable, citable, with no model retraining.

## What the framework is selling

The architecture's defining property is **not correctness**.
Correctness comes and goes with rulebook quality. The architecture's
defining property is **diagnosable wrongness**: when the framework
is wrong, the failure mode is mechanically discoverable from the
audit trail.

Status-quo LLMs sell confident answers wrapped in prose. This
framework sells **probabilistic outputs with byte-traceable audit
trails such that the failure mode of a wrong answer is locatable
to a specific clause, specific rationale, specific source byte-span.**
That is the value proposition for any domain where adjudicative
work is subject to oversight, audit, liability, or learning loops.

## Thesis

Adjudicative knowledge work — work that produces a defensible
judgment from evidence under stated criteria — decomposes into
four artifacts: **facts, uncertainties, queries, and a rulebook.**
All four can be expressed in a probabilistic provenance-encoded
programming language (adj-lang as the candidate) under a recursive
byte-accounting contract: every byte of every artifact, at every
layer of the pipeline, is tagged with a disposition, and no byte is
silently dropped. The framework produces either a posterior with
full audit trail or a kickback to humans naming specific resolution
targets. When the framework is wrong, the failure mode is
mechanically diagnosable from the audit trail; this is the
architecture's defining property.

Scope: **adjudicative** knowledge work — a large fraction of
professional knowledge work (clinical, legal, financial, audit,
compliance, scientific evaluation, regulatory, peer review,
investigative, due diligence, claims processing, eligibility
determinations). Explicitly not all knowledge work (excludes
generative creative work, open-ended hypothesis generation,
negotiation, interpersonal labour, embodied real-time skill).

## What ADJ51 ships

- This spec.
- `code/specs/data/adj51/experiment/` — full PMC12750962 run with
  generic prompts, captured ingestion, derived rulebook, vignette,
  and engine output.
- `code/specs/data/adj51/experiment2/` — full PMC12914605 run with
  the rulebook byte-accounting contract, sandboxed deriver,
  validators, captured artifacts.
- `code/specs/data/adj51/validate_ingestion.py` (and its variants in
  the experiment dirs) — the structural byte-accounting validator
  for ingestion.
- `code/specs/data/adj51/experiment2/validate_rulebook.py` — the
  structural byte-accounting validator for the rulebook layer.
- `code/specs/data/adj51/src/main.rs` — domain-agnostic runner that
  compiles rulebook + observations + queries and prints per-query
  posteriors, fired clauses, and coverage reports.
- `code/specs/data/adj51/README.md` — reproduction instructions.

## What ADJ51 does not yet do

- The **semantic verifier** that reads each `(rationale, clause)`
  pair and confirms alignment is not yet built (structural
  validator only). Next milestone.
- The **indexed source corpus** is sketched architecturally but not
  built. The deriver still uses live WebSearch for citations rather
  than retrieval from a pre-indexed corpus.
- The **counterfactual sensitivity panel** as a first-class output
  is not yet wired into the runner. The engine has
  `counterfactual()`; the runner doesn't surface it. Next
  milestone.
- The **kickback taxonomy** (value-deepening / rule-applicability
  deepening / coupled-query disagreement / coverage gap / decision
  sensitivity) is partially implemented across ADJ47's
  `suggest_kickback`. A unified kickback request structure is
  future work.
- **Cross-domain demonstration.** Two experiments are both
  clinical. The architectural claim is domain-agnostic but a
  legal / financial / regulatory experiment is needed to demonstrate
  it.
- **Scale.** Two cases. The path forward is the chronicle-producing
  pipeline that processes cases continuously, fine-tunes the
  contracts as failure modes surface, and accumulates lessons into a
  publicly-auditable record.

## What this opens up

- **The chronicle.** A long-running case-processing pipeline that
  runs each case end-to-end through the contracts and produces a
  structured + readable per-case entry (audit trail, sensitivity
  panel, coverage report, issues observed, fixes applied, new
  indexed claims). After ~10 cases there's a stable shakedown;
  after ~50 there's a story; after ~500 there's a paper; after
  ~5000 there's a discipline.
- **The indexed corpus loop.** Build the indexer that pre-decomposes
  papers into structured claims with source byte-spans; build the
  retrieval-assembly path; measure latency and citation grounding
  against the slow path.
- **The semantic verifier.** Build the subagent that reads each
  `(rationale, clause)` pair and flags mismatches. Demonstrate it
  would have caught experiment 1's troponin failure.
- **Cross-domain validation.** Apply the contracts to a legal case
  (court opinion adjudication) and a financial case (SEC enforcement
  action) to show the architecture is genuinely domain-agnostic.

## Cost summary

| Metric | Value |
|---|---|
| Experiments end-to-end | 2 (clinical, both real published cases with documented ground truth) |
| Generic ingester runtime per case | ~1–2 min |
| Generic deriver runtime per case (with web search) | ~5–10 min |
| Engine runtime per case | < 1 sec |
| Total pipeline runtime per case (live) | ~10 min |
| Experiment 1 — Q3 disposition decision | correct (admit) at 99.9% |
| Experiment 2 — load-bearing decisions | malignancy 100%, biopsy 99.9%, actinomycosis 10% (rejected) |
| Byte-accounting contracts demonstrated | 2 (ingestion, rulebook) |
| `intent_not_encoded` framing blocks in experiment 2 | 2 |
| Sub-queries introduced by deriver via recursion (exp 2) | 2 |
| Cited sources in derived rulebooks | ~80 across both runs (all real peer-reviewed) |

## Status

- 2026-06-02: ADJ51 spec + experiments + code on branch
  `adj51-byte-accounted-ingestion`.

## See also

- [ADJ50](ADJ50-nejm-stress-test-on-acs-rulebook.md) — the stress
  test that exposed the silent-drop failure mode this PR makes
  impossible.
- [ADJ48](ADJ48-mycin-2026-in-adj-lang.md) — the original adj-lang
  ACS rulebook; the language ADJ51's contracts apply to.
- [ADJ47](ADJ47A-logic-engine-typed-engine.md) — the engine that
  ADJ51's runner consumes; provides `counterfactual` for the
  unbuilt sensitivity panel.
- [ADJ45](ADJ45-three-way-blind-judge-experiment.md) — the
  blind-judge empirical proof that the resolution loop earns its
  keep; ADJ51's contracts extend the same defensibility story to
  the ingestion and derivation layers.
- [ADJ44](ADJ44-mycin-2026-meningitis.md) — the recursive
  rulebook-derivation pipeline that the indexed-corpus architecture
  generalises.
- [ADJ19](ADJ19-confident-hallucination.md) — the original
  confident-hallucination failure-mode catalogue; ADJ51 closes the
  ingestion-layer and derivation-layer variants of the same
  pattern.
