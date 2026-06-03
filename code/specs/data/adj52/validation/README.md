# ADJ52 validation — domain-blind ingester

`experiment2-ingester-domain-blind.json` is the captured output of a
**domain-blind generic ingester subagent** run on the ADJ51
experiment-2 sanitised prose (PMC12914605 — PMBCL masquerading as
infection). The subagent received the prose inline only, was **not told
the domain**, and was instructed not to read local files or look up the
case outcome (sandbox v1 — prompt-level).

This run validates the single most failure-prone constraint in the
pipeline (a domain-blind ingester that classifies ambiguity instead of
guessing). Assessment against the held-out ground truth:

| Criterion | Result |
|---|---|
| Inferred domain unaided | ✓ "clinical_medicine… (hematology_oncology / infectious_disease differential)" — caught the case's defining tension |
| Byte coverage / exhaustiveness | ✓ 51 facts (ADJ51's hand-built reference: 47); every clause mapped; connectives discarded *with* reasons |
| **Ambiguity → uncertainty, not a guess** | ✓✓ flagged the load-bearing ambiguities as typed uncertainties: `u3` Actinomyces colonizer-vs-pathogen (ground truth: "actinomyces was colonization, not pathogen"), `u6` weight-loss intentionality, `u2` leukocytosis interpretation, `u4` mass nature |
| Did not hallucinate the answer | ✓ never asserted a diagnosis; ground truth notes the naive-LLM trap is to confidently answer "infection" — the ingester refused and surfaced the crux |
| Raised sensible queries | ✓ unifying diagnosis, actinomyces causal-vs-incidental, next diagnostic step (biopsy) |

## Honest issues surfaced

1. **Sandbox is prompt-level only.** The subagent made (and abandoned)
   an accidental `WebFetch`. True isolation (run the ingester in a
   worktree containing only its input, with no repo access) is a
   hardening follow-up before scaled/unsupervised runs.
2. **Ingester queries are human-readable, not engine-ready.** They come
   out as `what_is_the_unifying_diagnosis` rather than
   `diagnosis(pulmonary_malignancy)`. A normalization pass (or a tighter
   query-term instruction) is needed between ingestion and the
   deriver/vignette stage so queries lower cleanly to engine terms.
3. **Minor:** the subagent prepended one sentence of prose before the
   JSON despite "JSON only"; the orchestrator must parse the JSON object
   out rather than assume the whole message is JSON.
