# adjudication-coverage (Rust)

Reference implementation of [`ADJ02`](../../../specs/ADJ02-coverage-checker.md):
**every meaningful input span must belong to the source spans of at
least one IR node — either as a Fact, Query, Uncertainty, or an
explicit Discarded node citing a reason.**

This is the first and cheapest of the four ADJ checker passes. It
runs before any LLM call at check time; the tagger may itself be an
LLM call, but its output is treated as data.

## Where It Fits

```text
   IR document (ADJ01)
        │
        ▼
   adjudication-coverage   ← this crate (ADJ02)
        │
        ▼
   ADJ03 polarity/modality
        │
        ▼
   ADJ04 round-trip entailment
        │
        ▼
   ADJ05 adversarial verifier
```

## API

```rust
use adjudication_coverage::{
    check_coverage, CoverageResult, Document, RuleBasedTagger, StrictnessMode,
};

let doc = Document {
    id: doc_id.clone(),
    normalized_text: "I am not bringing matches".to_string(),
};
let tagger = RuleBasedTagger::with_clinical_defaults();

let result = check_coverage(&doc, &ir_document, &tagger, StrictnessMode::Strict);

match result {
    CoverageResult::Pass => { /* downstream passes can run */ }
    CoverageResult::Fail { uncovered } => {
        // surface uncovered ranges as clarification questions (ADJ06)
        for span in uncovered {
            // ...
        }
    }
}
```

## The Tagger

The `Tagger` trait abstracts over the token classifier. The default
`RuleBasedTagger` recognizes:

- **Stopwords** (`the`, `a`, `of`, `is`, ...): configurable list.
- **Punctuation**: `[ \t.,;:!?()[]{}'"]`.
- **Filler tokens** (`umm`, `you-know`): configurable list.
- **Always-meaningful overrides** (e.g., domain terms that look like
  stopwords in some languages): configurable list.

Domains can replace this with a classifier-model or LLM tagger by
implementing `Tagger`.

## Strictness Modes

Per ADJ02 §"Configuration Surface":

- `Strict` — any uncovered meaningful byte fails coverage.
- `Permissive` — uncovered `Filler`/`Determiner` tokens tolerated.
- `AuditOnly` — never fails; uncovered ranges still reported for
  telemetry.

## What's Enforced

- Every meaningful byte must be inside the union of some IR node's
  `source_spans`.
- A `Discarded` node with reason `Unparseable` is **always** a
  coverage failure (ADJ01's hard rule).
- Multiple IR nodes covering the same span are permitted.
- Empty IR with meaningful input fails (the framework requires an
  explicit `Discarded(NonDomainContent)` node, not silence).

## Status

Experimental. The default tagger is rule-based; an LLM-driven
tagger living in a separate crate is planned as a follow-up to
exercise high-complexity clinical text.
