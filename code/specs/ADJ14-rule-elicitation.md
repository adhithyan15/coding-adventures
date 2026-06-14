# ADJ14 — Rule Elicitation: Bootstrapping a Rulebook from the LLM's Own Weights

## Overview

[ADJ09](ADJ09-rule-compilation-pipeline.md) specifies the rulebook
compilation pipeline: rulebook text → IR Rule nodes → engine. The
input to that pipeline is **rulebook text** — a TSA regulation, a
clinical guideline, a license clause. ADJ09 is silent on where that
text comes from, implicitly assuming an authoritative document exists.

ADJ14 fills that gap. It defines a new **Stage 0 — Rule Elicitation**
that runs *before* ADJ09's Stage 1 segmentation:

```text
                ┌──────────────────────────────┐
   domain hint  │  Stage 0 — Rule Elicitation  │  ← this document
   ───────────▶ │  (ADJ14)                     │
                └──────────────┬───────────────┘
                               │ rule text
                               ▼
                ┌──────────────────────────────┐
                │  ADJ09 Stages 1–6            │
                │  segmentation → compilation  │
                │  → ADJ02–05 checks           │
                └──────────────┬───────────────┘
                               │ IR Rule nodes + audit trail
                               ▼
                          typed Rulebook
```

The core claim:

> Large language models have ingested enormous amounts of regulatory,
> clinical, legal, and operational text. The TSA carry-on rulebook is
> in their weights. So is HIPAA, the IFRS Conceptual Framework, the
> AMA CPT manual, GAAP, the GDPR, the IRC, every major clinical
> guideline. We can ask for it. **We just cannot trust the raw
> answer.** ADJ14's contract is: the LLM volunteers the rules; the
> existing checker discipline (ADJ02–05) and the existing clarification
> loop (ADJ06) audit them. The output is a typed `Rulebook` whose
> defensibility comes from the audit trail, not from the LLM's
> confidence.

## Why This Is Not Just Asking the LLM

A naive "what are the TSA rules?" prompt at answer time would
**recursively hallucinate** — the same model that will answer the
compliance question gets to invent the rules it'll be judged against.
ADJ14 breaks this by:

1. **Eliciting rules in a separate phase** with its own prompt-version
   constant (`ELICIT_RULES_PROMPT_VERSION`), its own call record, its
   own document_id namespace (`rulebook-<domain>-<timestamp>`).
2. **Routing the elicited text through `decompose_text`** — the same
   primitive that handles factual input. The model that elicits the
   rules is not trusted to also structure them; the structure comes
   from the IR grammar.
3. **Running ADJ02–05 against the resulting IR** with the same
   discipline as input extraction. Coverage forces every byte of the
   elicited text to be accounted for. Polarity catches "shall" /
   "shall not" / "may" inversions. Round-trip catches renderings that
   drift. Adversarial finds alternative readings.
4. **Looping on ADJ06 retries until clean.** If any check fires, the
   clarification dialogue retries the elicitation (or the
   decomposition) up to `max_attempts` times. A rulebook that exhausts
   ADJ06 retries is marked **Tentative** and surfaced for expert review
   per ADJ09's review workflow — the framework does not silently ship
   an unsound rulebook.

The recursion is the point. The rulebook acquisition pipeline is the
same pipeline as input extraction. Same primitives. Same checks. Same
audit trail. Same self-correction. The only difference is **what the
IR represents**: rules instead of facts.

## Trust Tiers

A `Rulebook` carries one of three trust tiers in its metadata:

```text
RulebookTrust := Tentative | Reviewed | Authoritative
```

| Tier | How obtained | Default for |
|---|---|---|
| **Tentative** | ADJ14 Stage 0 — LLM-elicited, ADJ02–05 passed, no human review | The result of every fresh `acquire_rulebook(domain)` call. |
| **Reviewed** | Tentative rulebook that an authorized domain expert has signed off (per ADJ09 §"Expert Review Workflow") | Persisted artefacts in `code/specs/rulebooks/<domain>-<version>.json`. |
| **Authoritative** | Compiled from a published regulatory document (not an LLM elicitation) | Future work — when external rulebook ingestion lands. |

A `Tentative` rulebook can be **used** by the answer-time pipeline but
the resulting adjudication carries a `tentative_rulebook` flag in its
audit trail. Downstream consumers (a real TSA system, a clinical
decision-support system) can decide whether to surface answers backed
only by Tentative rules, gate them behind disclaimers, or require
Reviewed-or-better.

The framework's default for the demos in this repository is to use
Tentative rulebooks freely — the demos exist to exercise the
framework, not to ship adjudications. Real deployments configure their
own minimum tier.

## Stage 0 — The Elicitation Primitive

A new LLM primitive, `elicit_rules`, joins the existing set in
[`llm-primitives`](../packages/rust/llm-primitives/src/lib.rs). Its
contract:

```rust
pub struct ElicitRulesRequest {
    /// Domain hint (`"tsa-declaration"`, `"clinical-triage"`,
    /// `"contract-review"`, …). Drives the prompt's framing.
    pub domain_hint: String,
    /// Optional scope refinement. For TSA: `"carry-on baggage"`.
    /// For clinical: `"emergency triage acuity scoring"`. When
    /// absent, the model produces a broad rulebook for the domain.
    pub scope_hint: Option<String>,
    /// Stable identifier the framework attaches to this rulebook.
    /// Convention: `"rulebook-<domain>-<YYYYMMDD>-<short-hash>"`.
    pub document_id: String,
}

pub struct ElicitRulesResponse {
    /// The raw natural-language rulebook text the model produced.
    /// This is the input to ADJ09 Stage 1 segmentation. Not yet IR.
    pub rule_text: String,
    /// The primitive's audit-trail record.
    pub call_record: LlmCallRecord,
}
```

The primitive's system prompt instructs the model to:

1. **Be exhaustive.** List every rule it knows for the domain.
2. **Number each rule.** Numbering supports later segmentation.
3. **Be precise about exceptions and conditional logic.**
4. **Cite sources where possible.** "Per 49 CFR § 1540.111…",
   "Per UpToDate, Emergency Severity Index Algorithm…". If the
   citation cannot be produced with confidence, omit rather than
   fabricate.
5. **State limits of knowledge.** "I am uncertain whether the
   2024 amendment to X is included" — explicit uncertainty flags
   get extracted as `Uncertainty` nodes by `decompose_text`.

The role is `Role::RuleExtractor` (already declared in
`llm-primitives::Role`). Deployments default this role to the same
client as `Role::Extractor`; specializing is a deployment knob.

## Stage 0 — Orchestrator

A new crate `adjudication-rulebook` provides the orchestrator:

```rust
pub struct AcquireRulebookRequest {
    pub domain_hint: String,
    pub scope_hint: Option<String>,
    pub as_of: chrono::NaiveDate,  // see ADJ09 § "Time-Varying Rules"
    pub max_clarification_attempts: usize,
}

pub struct Rulebook {
    pub domain: String,
    pub document_id: String,
    pub ir_document: serde_json::Value,  // the audited IR
    pub source_text: String,             // raw elicited text
    pub trust: RulebookTrust,            // Tentative on Stage-0 output
    pub elicit_prompt_version: String,
    pub decompose_prompt_version: String,
    pub model_identity: ProviderIdentity,
    pub as_of: chrono::NaiveDate,
    pub audit_trail: Vec<LlmCallRecord>, // every call from elicit → decompose → ADJ02–05 → ADJ06
}

pub fn acquire_rulebook(
    req: &AcquireRulebookRequest,
    gateway: &GatewayConfig,
) -> Result<Rulebook, AcquireRulebookError>;
```

The orchestrator's flow:

```text
1. elicit_rules(domain, scope) -> raw_text + elicit_call_record
2. decompose_text(
       source_text = raw_text,
       document_id = "rulebook-<domain>-<as_of>-<hash>",
       domain_hint = "<domain>/rulebook",  // namespaced
   ) -> ir_document + decompose_call_record
3. for each check in [coverage, polarity, round_trip, adversarial]:
       result = check.run(ir_document, raw_text)
       if result.fails:
           outcome = adj06::retry_<check>_on_<failure>(...)
           if outcome.exhausted:
               return Err(Acquire::ChecksExhausted { trail })
           else:
               ir_document = outcome.corrected_ir  // or rendering
4. assemble Rulebook { trust: Tentative, audit_trail: full_trail, ... }
5. return Rulebook
```

The orchestrator's audit trail concatenates every `LlmCallRecord` from
every step. Replay against `(audit_trail, model_identity)` reproduces
the exact rulebook bit-for-bit (modulo cache state).

## Persistence and Reuse

The `adjudication-rulebook` crate persists each successfully-acquired
rulebook to disk under `code/specs/rulebooks/<domain>-<as_of>.json`
(gitignored for Tentative; committed once promoted to Reviewed).

Subsequent calls to `acquire_rulebook(req)` consult this cache first.
The cache key is `(domain, scope, elicit_prompt_version, model_identity, as_of)`.
A miss triggers fresh elicitation; a hit returns the persisted
rulebook with its full audit trail intact.

The existing `llm-cache` handles caching of the underlying LLM calls.
The rulebook-level cache is a thin wrapper that records "this whole
rulebook came out clean" so the entire pipeline can be skipped, not
just individual calls re-played.

## Cache Invalidation

A rulebook is invalidated when any of:

- The elicit prompt-template version bumps (`ELICIT_RULES_PROMPT_VERSION`).
- The decompose prompt-template version bumps (currently
  `decompose-text-v3`).
- A new `as_of` date is requested.
- The model identity registered for `Role::RuleExtractor` changes.

Each invalidation triggers fresh elicitation. Old rulebooks are
**retained** with their `as_of` and prompt-version metadata so prior
adjudications remain replayable per ADJ09 § "Time-Varying Rules" and
ADJ08 § "Replay Tooling".

## How Answer-Time Uses a Rulebook

ADJ14 produces the Rulebook; it does not specify how answers consume
it. That belongs to the per-demo orchestrator. Two consumption
patterns are anticipated; both are listed here for context.

### Phase 1 — Rulebook in LLM Context (this PR sequence)

The answer-time prompt includes the rulebook's IR alongside the facts'
IR:

```text
SYSTEM:
  You are answering compliance questions. Rules to apply:
  <rulebook.ir_document>

USER:
  Facts:
  <facts.ir_document>
  Question:
  <query>
```

This is a low-effort first cut: the LLM does both rule-grounded
reasoning and answer synthesis. The audit trail records both
`(rulebook.audit_trail, query_call_record)`.

The expected immediate win is the elimination of the raw-arm rule
hallucinations measured in
[ADJ12](ADJ12-small-model-benchmarks.md) — "matches allowed if
unlit", "30-pound weight limit". With explicit rules in context, the
model is no longer free to invent them.

### Phase 2 — Programmatic Adjudication (future)

The Rulebook's `ir_document` is compiled to a Prolog/ProbLog program
by `adjudication-connector` ([ADJ11](ADJ11-problog-connector.md))
and `lp19-engine` answers the query programmatically. No LLM at
answer time. The LLM's role ends after rule elicitation; from there
on the system is deterministic and replayable byte-for-byte.

This is the long-term endpoint. Phase 1 ships first because it has
fewer moving parts and lets us measure the value of explicit
rulebooks in isolation from the engine work.

## Recursive Self-Application

A consequence worth stating: ADJ14 is **the framework applied to
itself**. The same primitives, the same checkers, the same retry loop
that handle factual input now handle the rules under which that input
will be judged. The audit trail records both. An adjudication's full
provenance becomes:

```text
provenance(answer) = (
    rulebook.audit_trail,    // how the rules came to exist
    facts.audit_trail,       // how the facts were decomposed
    answer_call_record,      // how the answer was produced
)
```

Anyone reviewing the system can replay any of the three independently.
A challenge to a specific rule is answerable by reproducing the
elicitation. A challenge to a specific fact is answerable by
reproducing the decomposition. A challenge to the answer is
answerable by reproducing both inputs and the answer call. The
framework's "intelligence accumulates in the pipeline, not the
weights" thesis becomes verifiable: every step has a record.

## Implementation Sequence

1. **This PR** (`adj14-spec`) — ADJ14 spec only.
2. **Next** (`adj14-primitive-and-crate`) — `elicit_rules` primitive
   in `llm-primitives`, `adjudication-rulebook` crate with
   `Rulebook` + `acquire_rulebook`, unit tests with scripted clients.
3. **After** (`adj14-tsa-demo-wiring`) — TSA demo acquires the
   rulebook before answering, raw arm now sees the rulebook in
   context. Measured comparison against the v0.7 baseline.
4. **Stretch** (`adj14-tentative-to-reviewed`) — CLI tooling that
   takes a Tentative rulebook, presents the ReviewableUnit surface
   (per ADJ09 § "Expert Review Workflow") to a human reviewer, and
   promotes the rulebook to Reviewed with the reviewer's signature.

## Open Questions

1. **How verbose should the elicit prompt be?** A short prompt elicits
   broad coverage but vague rules; a long prompt elicits precise rules
   but biases coverage toward what the prompt suggests. The
   benchmark in PR 3 above will inform the trade-off; the elicit
   prompt is versioned so iteration is safe.
2. **Cross-model consistency.** If `Role::RuleExtractor` is gemma4 and
   the same domain is elicited from llama3.1:8b, the two rulebooks
   will differ. The framework's response is the same as for any
   provenance question: both are valid Tentative rulebooks under
   their respective `(model_identity, elicit_prompt_version)`. ADJ09
   §"Conflicts Between Sources" already handles cross-source rule
   conflicts at answer time.
3. **Citation hallucination.** The elicit prompt asks the model to
   cite sources where possible. The model will sometimes hallucinate
   citations. ADJ04 round-trip will catch some of these; explicit
   citation-checking is a future extension (`ADJ14a`).
4. **Tabular and chart rules.** Some domains (drug dosing,
   tax brackets) are inherently tabular. Stage 0 currently elicits
   natural-language rules and depends on `decompose_text` to handle
   tabular structures; explicit table support remains ADJ09's open
   question #3.

## Limitations

1. **Knowledge cutoff.** A Tentative rulebook reflects the model's
   training-data cutoff. The `as_of` field is the *requested*
   cutoff; the actual freshness of the rules is bounded by the
   model. Real deployments should promote Tentative → Reviewed via
   expert review against the actual current rules.
2. **Domain coverage.** Domains the model has *not* seen produce
   thin or wrong rulebooks. The Stage 0 audit trail surfaces this
   indirectly via low rule count or high ADJ06 retry pressure; an
   explicit "coverage confidence" signal is `ADJ14b`.
3. **Single-model elicitation.** Stage 0 elicits from one model.
   Multi-model elicitation (eliciting from N models, taking the
   intersection / union / median rulebook) is a natural extension.

## Status

Draft. Sufficient to implement the `elicit_rules` primitive plus the
`adjudication-rulebook` crate. Implementation lands in follow-up PRs
per the sequence above.
