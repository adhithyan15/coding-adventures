# Grounded corpus — a provenance-first knowledge base for MYCIN-2026

This directory is the **product**: a growing, version-controlled, byte-provenanced
clinical knowledge base. The scripts that *build* it live in
[`../provenance/`](../provenance/); the design rationale is
[ADJ55](../../ADJ55-provenance-first-corpus.md).

## The goal — a modern MYCIN

A reconstruction of MYCIN's expert system on a 2020s footing, in three separable parts:

1. **LLM as a natural-language frontend.** The model reads free text — a case
   vignette, a paper — and maps it onto a controlled vocabulary. It never invents
   the reasoning weights; it ingests and routes.
2. **A framework that enforces byte provenance.** Every quantity in the knowledge
   base (a prior, a likelihood ratio) must trace, through a recursive citation
   chain, to a literal span of primary data — a published sensitivity/specificity,
   odds ratio, or prevalence. A number that cannot be grounded does not enter as a
   number; it is an explicit data-gap. **This is the antidote to hallucination:**
   the model cannot manufacture false precision, because precision must point at a
   datum. (Demonstrated: [ADJ54/55 PE proof](../provenance/pe/).)
3. **A deterministic, executable program.** Given the grounded corpus + a case's
   observations, the verdict is a pure function — a sequential Bayesian update that
   produces the same posterior every time, with every multiplier annotated by the
   study it came from. No model in the loop at decision time.

The split matters: the LLM (fallible, creative) is confined to the frontend; the
arithmetic (auditable, reproducible) is in the program; and the bridge between them
— the corpus — is held to the byte-provenance invariant.

## Corpus format

Each domain is a directory with a `corpus.json` (the grounded rulebook) and a
`SOURCES.md` (the human-readable provenance table). `corpus.json`:

```json
{
  "domain": "pulmonary_embolism",
  "target": "diagnosis(pulmonary_embolism)",
  "prior": { "lr": 0.192, "grounded": true,
             "provenance": { "study": "...", "values": "...", "byte_quote": "...", "formula": "..." } },
  "findings": [
    { "finding": "d_dimer", "state": "elevated", "lr": 1.64, "grounded": true,
      "verdict": "grounded",
      "provenance": { "study": "Blood Advances 2020 meta (34 studies, 22,849 pts)",
                      "values": "sens 0.97, spec 0.41", "formula": "LR+ = 0.97/(1-0.41) = 1.64",
                      "byte_quote": "The pooled estimates ... were 0.97 ... and 0.41 ..." } },
    ...
  ]
}
```

- `lr` is the likelihood ratio (or, for `prior`, the prevalence as a probability).
- `verdict` is one of `grounded` | `direction_only` | `fabricated`, assigned by the
  provenance spider. **Only `grounded` LRs are trusted by the deterministic
  evaluator's `grounded` mode**; the rest are visible data-gaps.
- `provenance` carries the byte-anchored chain to primary data.

## How to grow it

1. **Add a finding to an existing domain.** Add its `finding(state)` to the domain's
   skeleton and run the forward grounding spider
   ([`../provenance/pe/ground.workflow.js`](../provenance/pe/ground.workflow.js) is
   the template): crawl to primary data, compute the LR, byte-anchor it. Re-assemble
   with `build_corpus.py`.
2. **Add a domain.** Copy the PE skeleton pattern: enumerate the differential's
   discriminating findings *case-blind*, ground each, assemble. A domain where most
   links come back `direction_only` (e.g. the case-5 urology probe: 0/19 grounded) is
   a signal that the evidence base does not support a quantified rule — surface that,
   don't paper over it.
3. **Evaluate a case.** Map the case onto the domain vocabulary and run
   [`../provenance/pe/eval_case.py`](../provenance/pe/eval_case.py) — every step of
   the posterior prints its source.

## Domains

| domain | links grounded | notes |
|---|---|---|
| [`pulmonary_embolism`](pulmonary_embolism/) | 12/12 | first fully-grounded corpus; validated end-to-end on PMC11999957 ([ADJ55](../../ADJ55-provenance-first-corpus.md)) + n=4 stress test ([ADJ56](../../ADJ56-cross-domain-stress-test.md)) |
| [`streptococcal_pharyngitis`](streptococcal_pharyngitis/) | 9/9 | Centor/McIsaac + RADT; ADJ56. Caveat: prior is the *child* prevalence — see the infant-extrapolation failure mode in ADJ56 §3.1 |
| [`bacterial_meningitis`](bacterial_meningitis/) | 9/9 | CSF parameters; ADJ56. Caveat: the CSF findings are correlated — naive multiplication over-saturates (ADJ56 §3.2); needs the ADJ53 `mechanism` grouping |

## Generic tooling

- [`build.py`](build.py) — domain-agnostic assembler: `python build.py <domain> <grounding-results.json> <skeleton.json>`.
- [`eval.py`](eval.py) — domain-agnostic deterministic evaluator: `python eval.py <corpus.json> <case.json> [grounded|all]`.

Adding a domain is: a forward-grounding workflow (copy a `provenance/<domain>/ground.workflow.js`) + a `findings.json` skeleton → `build.py` → `eval.py`.
