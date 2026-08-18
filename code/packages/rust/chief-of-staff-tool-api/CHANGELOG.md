# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Security

- The terminal event's payload no longer carries `error.details`. Those details
  reach the immediate caller on `ToolResult`, but the event stream is a
  broadcast, and on an output-validation failure `details` holds one `path` per
  offending field built from the handler's own output object -- so a handler
  whose output was refused could broadcast arbitrary text to every sink by
  naming its fields carefully. Same covert channel as the one below, different
  field.
- A call that fails output validation no longer publishes the handler's events.
  They were assembled before validation and forwarded on the rejection path, so
  the runtime declared a call invalid and shipped that call's side effects
  anyway -- a handler whose output was refused could still say whatever it liked
  to every event sink. The runtime's own framing events are unaffected: they
  describe the call, not its result.
- Registering a definition under a built-in tool id now requires it to match the
  catalog entry, with a new `ToolApiError::BuiltinDefinitionMismatch`. Without
  it, the schema a tool is validated against was a property of whichever
  `ToolDefinition` reached `register`, not of the tool id -- so anything holding
  a registry could register `vault.request_direct` with `output_schema: None`
  ahead of the vault binding -- `DuplicateToolId` already blocked replacing an
  entry, so the hole was getting in first, not overwriting -- silently
  disabling the output validation the vault binding's "no secret return
  channel" rests on, while the registry looked entirely normal. Only ids
  the catalog claims are constrained; smart-home and host-local tools are
  untouched.

  This is a **behaviour change for anything that registered a look-alike under a
  real built-in id.** One existed: this crate's own test fixture reused
  `artifact.create` with a different description and output schema. It is now
  `test.artifact_create`, which is what it always should have been -- a
  synthetic tool testing the runtime had no business claiming a catalogued id.

### Added

- Added the canonical Tier-2 `vault.request_lease` built-in with strict opaque
  receipt schemas and the D18D `vault:lease` policy capability.
- Added the canonical Tier-2 `vault.request_direct` built-in with strict
  consumer targeting, a null acknowledgment, and no secret return channel.
- Added canonical approval challenges and explicit-consent, biometric, and
  hardware-key assurance levels. Tier-aware policy can now require approval at
  a privilege threshold, and Tier 2+ grants must match the active challenge
  before a handler can run.

## [0.1.0] - 2026-05-08

### Added

- Initial Chief of Staff Tool API core package.
- Canonical tool definition, invocation request, event, result, and metric types.
- Tool catalog query helpers for filtering by family, side effects, tier,
  capability, tag, stability, and limit.
- JSON-schema-like input validation for model-facing tool arguments.
- First-phase built-in tool catalog definitions for context, artifact, memory,
  and job store/runtime tools.
- SkillStore read and lifecycle built-ins for listing, manifest/asset reads,
  install, activation, deactivation, and uninstall.
- Deterministic in-memory tool registry with duplicate detection and call validation.
- Deterministic in-memory runtime that pairs definitions with handlers, validates
  invocations before execution, emits canonical events, and returns `ToolResult`
  records.
- Policy decision hooks and deterministic policy profiles for permission, tier,
  side-effect, and approval gates before handler execution.
- Expanded D18D first-party catalog parity for ContextStore snapshots,
  ArtifactStore revision/list/retention tools, MemoryStore lifecycle tools, and
  Job runtime list/status/run/uninstall tools.
- Explicit `ToolApprovalGrant` support so approval-required calls can be replayed
  through the same validated runtime path while stale or mismatched grants are
  rejected before handler execution.
- Handler output validation against advertised tool output schemas before the
  runtime emits completed results.
- Storage-neutral read-side query helpers for tool invocation requests, call
  lifecycle records, event streams, and terminal results.
- Provider-neutral JSON-schema-shaped projection for tool input/output schemas
  and `ToolSchemaDocument` exports for future model-gateway adapters.
- Catalog export snapshots with schema documents, validation state, and summary
  counts for model gateway adapters and portability checks.
- Catalog summary maps and helpers for required-capability and tag coverage.
- Expanded catalog summary counts for idempotency, concurrency, capability
  gates, lock scopes, timeouts, and output-schema coverage.
- Schema-light `ToolDefinitionSummary` rows plus built-in and registry summary
  query helpers for read-side catalog listings.
- Payload-free `ToolExecutionTraceSummary` rows for per-invocation runtime
  health, reference, terminal-event, and follow-up checks.
- Append-only `ToolExecutionJournal` with count-only summaries and query helpers
  for recorded invocation requests, call records, events, and terminal results.
- Payload-free `ToolExecutionJournalHealthSummary` rows for active-call,
  approval, terminal-event, reference, and follow-up coverage across a journal.
- Payload-free `ToolAuditRecord`, audit query helpers, and an in-memory audit
  sink so runtimes can persist the D18D minimum audit shape without exposing
  arguments, outputs, or event payloads.
