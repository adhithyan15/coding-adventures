# adjudication-polarity-modality (Rust)

Reference implementation of [`ADJ03`](../../../specs/ADJ03-polarity-modality-checker.md).
NegEx / ConText-style scope detection over IR node source spans.
Catches the canonical *"denies chest pain"* → `symptom(chest_pain)`
failure class deterministically, before any LLM-heavy checker runs.

## What It Catches

| Source | Wrong IR | Right IR |
|---|---|---|
| "Patient denies chest pain" | `chest_pain(patient)` Affirmed | `chest_pain(patient)` Denied |
| "No history of asthma" | `asthma(patient)` Affirmed | `asthma(patient)` Denied Past |
| "Father had MI at 50" | `mi(patient)` Past | `mi(patient)` FamilyHistory |
| "PE ruled out by CT angio" | `pe(patient)` Denied | `pe(patient)` Affirmed RuledOut |
| "Possibly pneumonia" | Fact pneumonia(patient) | Uncertainty pneumonia(patient) |

## API

```rust
use adjudication_polarity_modality::{
    check_polarity_modality, TriggerTaxonomy, ViolationSet,
};

let taxonomy = TriggerTaxonomy::clinical_default();
let violations = check_polarity_modality(&doc, &ir_doc, &taxonomy);
if violations.is_empty() {
    // pass
} else {
    // each violation has node_id, trigger_class, required, actual —
    // ready to surface as ADJ06 clarification.
}
```

## Trigger Taxonomy

Each trigger is `(class, surface, direction, scope_rule)`:

- **class**: Negation / Hedge / TemporalPast / TemporalPresent /
  TemporalFuture / Hypothetical / FamilyHistory / RuleOut / Subject
- **surface**: the literal text (`"denies"`, `"history of"`,
  `"ruled out by"`)
- **direction**: which side of the trigger its scope applies to
- **scope_rule**: how far the scope extends
  (UntilSentenceEnd, UntilPunctuation, UntilTermination(["but",
  "however", ...]), UntilTokenCount(n))

`TriggerTaxonomy::clinical_default()` ships a small but reasonable
English clinical taxonomy. Domains add their own with
`TriggerTaxonomy::add`.

## RuledOut vs. Denied

A specific note: the spec is emphatic that `RuledOut` is **modality**
and `Denied` is **polarity** — they are not synonyms. *"Denies"* is
the patient's claim; *"ruled out"* is the clinician's adjudication.
Billing, malpractice review, and downstream reasoning treat them
distinctly. This crate enforces the distinction at check time: a
`Ruled out by CT angio` trigger requires modality `RuledOut` with
polarity still `Affirmed`.

## Status

Experimental. Covers the high-impact scope-detection cases. Compound
multi-word triggers and the cross-node scope check live in `ADJ03a`,
a planned follow-up.
