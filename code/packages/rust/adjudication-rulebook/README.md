# adjudication-rulebook

Reference implementation of [ADJ14 — Rule Elicitation](../../specs/ADJ14-rule-elicitation.md).

Bootstraps a typed `Rulebook` from the LLM's own weights when no
authoritative regulatory document is on hand. The same audit
discipline (ADJ02–05 + ADJ06 retries) that protects extracted facts
in the framework now protects the rules under which those facts
will be judged.

## What this crate does

```text
   domain hint + scope
         │
         ▼
   elicit_rules        ─── raw rule text + LlmCallRecord
         │
         ▼
   decompose_text      ─── IR JSON + LlmCallRecord
         │
         ▼
   adjudication_ir::validate
         │
         ▼
   typed Rulebook { trust: Tentative, audit_trail: [..], ... }
```

The headline entry point is `acquire_rulebook(req, gateway)`. It
returns a `Rulebook` regardless of whether the IR validates — a
failing rulebook is still returned with `validation_passed = false`
and a diagnostic, so the caller can route to ADJ06, fall back to a
different model, or surface the issue for human review.

## Trust tiers

A `Rulebook` carries one of three trust tiers in its metadata:

| Tier | How obtained | Default for |
|---|---|---|
| `Tentative` | LLM-elicited, ADJ-validated, no human review | Every fresh `acquire_rulebook` output |
| `Reviewed` | Tentative rulebook signed off by a domain expert (ADJ09's review workflow) | Persisted artefacts after human sign-off |
| `Authoritative` | Compiled from a published regulatory document (not an LLM elicitation) | Future work — external rulebook ingestion |

Real deployments configure their minimum tier; the framework's
demos use Tentative rulebooks freely because they exist to exercise
the framework, not to ship adjudications.

## What v0.1 ships

- `Rulebook` typed container with full audit trail.
- `RulebookTrust { Tentative, Reviewed, Authoritative }`.
- `acquire_rulebook` orchestrator (elicit → decompose → validate).
- Hand-rolled JSON → `IRDocument` decoder so the validator can run
  against the LLM's response (the IR crate doesn't ship serde
  derives).
- 5 unit tests using a `ScriptedDual` mock that exercises both the
  text-completion path (`elicit_rules`) and the JSON-completion
  path (`decompose_text`) end-to-end without an LLM.

## What v0.1 does NOT ship yet

- ADJ02–05 checker passes against the rulebook IR. v0.1 runs only
  `adjudication_ir::validate` (the well-formedness gate). The full
  ADJ06 retry loop on rulebook checks follows in a subsequent PR.
- Disk persistence and caching of acquired rulebooks. Until that
  lands, the underlying `llm-cache` memoises individual LLM calls
  so repeated invocations are fast, but the orchestrator itself
  always runs end-to-end.
- Tentative → Reviewed promotion CLI per ADJ09's expert-review
  workflow.

## See also

- [ADJ14 spec](../../specs/ADJ14-rule-elicitation.md)
- [ADJ01 v3 IR grammar](../../specs/ADJ01-adjudication-ir-grammar.md)
- [`llm_primitives::elicit_rules`](../llm-primitives/src/elicit_rules.rs)
