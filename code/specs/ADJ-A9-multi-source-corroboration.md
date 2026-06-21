# ADJ-A9 — multi-source corroboration (`cites … locator …`)

## Motivation

ADJ46's awkwardness catalogue listed **A9 — source-disagreement aggregation
(multiple citations per `(conclusion, evidence)`)** as a language-layer
follow-up. This spec implements the *corroboration* half of A9: a single clause
may carry **more than one citation** that all support the **same** likelihood
ratio / edge.

This is distinct from the engine's existing `source_disagreements` machinery,
which handles the case where *different* `contributes` clauses (one LR per
source) **disagree** about magnitude. A9 is the complementary case: the *same*
LR is **corroborated** by several independent primary sources. The primary-
source-first grounding discipline (MYCIN-2026) wants exactly this — a grounded
fact is stronger when two or more re-fetchable spans say the same thing, and the
audit trail should record every one of them, not just the first.

Today the lowerer rejects a second `source`/`locator` on a clause with
`LowerError::DuplicateAnnotation`. A9 keeps that rule for the **primary**
citation (one `source` + one `locator` per clause, unchanged) and adds a new,
*repeatable* annotation for **corroborating** citations.

## Surface syntax

A new repeatable annotation, valid wherever `source`/`locator`/`trust` are:

```
contributes 2.5 from neutrophil_predominance to bacterial_meningitis
    source  "Tunkel et al., IDSA practice guidelines 2004"
    locator "https://academic.oup.com/cid/article/39/9/1267"
    trust   authoritative
    cites   "van de Beek et al., NEJM 2006" locator "https://www.nejm.org/doi/full/10.1056/NEJMra052116"
    cites   "Brouwer et al., Clin Microbiol Rev 2010" locator "https://journals.asm.org/doi/10.1128/CMR.00070-09"
```

- `cites STRING locator STRING` — a corroborating citation: a source span and
  the locator (URL / page / §) where it can be re-fetched. **Both are
  required** — a corroboration with no locator is not re-checkable, so the
  grammar mandates the pair.
- The `locator` keyword is **reused** (no new `at` keyword) to avoid reserving a
  short, common word that rulebooks might want as an identifier. Only one new
  keyword, `cites`, is added.
- Corroborations are **repeatable** (zero or more per clause). The primary
  `source`/`locator`/`trust` annotations remain **at-most-once** (unchanged).
- Corroborations inherit the clause's `trust` tier — they are co-equal citations
  for the same fact, not independent evidence at a different weight.

## Semantics

- Corroborations are **documentary only**: they do **not** change the engine's
  arithmetic. They are NOT additional evidence and must not be double-counted as
  extra LR weight — that would inflate posteriors. They ride inside the clause's
  `Provenance` so every proof step that cites the clause can also list its
  corroborating sources.
- The proof DAG is unaffected structurally: corroborations are a field on the
  existing `Provenance` value already threaded through every `DerivationOrigin`.

## Data model (logic-engine)

`Provenance` gains one additive field:

```rust
pub struct Provenance {
    pub source: String,
    pub locator: Option<String>,
    pub trust_tier: TrustTier,
    pub corroborations: Vec<Citation>,   // NEW — default empty
}

pub struct Citation {       // NEW
    pub source: String,
    pub locator: String,    // required: a corroboration must be re-fetchable
}
```

The field defaults to empty, so every existing constructor (`new`, `cited`,
`consensus`, `empirical`, `unattributed`, `Default`) and every existing caller
is source-compatible. A builder `with_corroboration(source, locator)` appends
one.

## Lowering (adj-lang)

`annotations_to_provenance` accumulates `Annotation::Cites { source, locator }`
into `Provenance::corroborations` in source order. The at-most-once checks for
`source`/`locator`/`trust` are unchanged; `cites` has no duplicate check (it is
inherently repeatable).

## Rendering (adj-lang-cli)

The clause-provenance JSON gains a `"corroborations":[{ "source":…,
"locator":… }]` array (empty when none). Existing `"source"/"locator"/"trust"`
fields are unchanged, so existing recall/proof consumers keep working.

## Out of scope (clean follow-ups)

- Per-corroboration trust tiers (today they inherit the clause tier).
- Surfacing corroborations in the differential `source_disagreements` report
  (the disagreement case is already handled across separate clauses).
