# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Add `InMemoryToolRuntime::as_agent_surface`, which extends the S-I7 identity
  walk to tool OUTPUTS. This closes the first clause -- the agent's view
  contains no agent identity -- where the registration gate could not reach it:
  that gate inspects DECLARED schemas, and an `Any` output declares nothing, so
  `context.read_entries` could return `{"entries": [{"agent_id": "peer-7"}]}`
  and pass every check.
- Scoped by RUNTIME rather than by a per-tool exemption list. The exemption is
  a property of who is looking, not of the tool: the smart-home audit and
  access-review readers exist to report on principals and must keep doing so,
  and they simply cannot be registered into an agent surface because
  `check_registration` refuses a tool that names a peer in its schema. A
  per-tool list would have to be maintained forever and would drift.

- Add `JsonSchema::validate_supplied_value`, which validates shape and
  additionally refuses an agent identity smuggled through a position no schema
  describes. Tool invocation now validates arguments with it.
- This **narrows** S-I7's second clause -- *the agent cannot supply one* -- for
  the eleven built-ins `tools_with_unverifiable_schema` reports. It does not
  close it, and `tools_with_unverifiable_schema` remains the authoritative
  statement of where the clause does not hold. `job.install`'s `spec: Any`
  accepted `{"run_as_agent_id": "peer-7"}` and reported clean; it no longer
  does.
- The check reads KEY NAMES, never values, so an identity carried as a value
  still passes: `{"spec": {"args": ["--to", "peer-7"]}}`, an identity used as a
  map key under an innocuous parent, or one base64-encoded inside a string.
  That is inherent -- an `Any` position is uninterpreted by definition, and
  only the handler knows which string is an id. Real closure needs the identity
  to come from supervisor-side wiring rather than from a blob the agent
  authors.
- The peer vocabulary is derived from stems plus plural and id/name suffixes
  rather than hand-kept. The hand-kept version had `from_agent`, `for_agent`
  and `target_agent` but not `to_agent`; the gaps were systematic.
- Normalization strips every non-alphanumeric, not just `_` and `-`. Value keys
  are arbitrary JSON strings, so `"agent id"`, `"agent.id"` and a zero-width
  space inside `"agent\u200b_id"` all reached the handler. A non-ASCII key in
  an undescribed position is refused outright rather than folded, because
  chasing homoglyphs is a losing game.
- A separate, narrower vocabulary applies to values than to schema properties.
  A schema property named `agent` is a tool asking the caller to name one; a
  key named `agent` inside an opaque blob is usually the blob's own identity --
  `agent` is a first-class frontmatter key in this repo's SKILL.md manifest
  format, so treating them alike would have made `skill.install` unable to
  accept any real manifest. `host_id`/`hostname` are excluded for the same
  reason: in a job spec they name a machine.
- Fully schematizing those positions was rejected: a job `spec` and a
  `metadata` bag are open by design, and a schema enumerating their keys would
  be wrong the first time someone added one. The check moves from the declared
  shape to the actual value, so structure stays open and identity names do not.
- Deliberately NOT applied to outputs, because doing so broke the smart-home
  audit and access-review readers, which exist to report on principals.
- **This leaves a real residual on the output side.** The registration gate
  refuses tools that DECLARE a peer identity, which is precisely not the hole
  case: `tools_naming_another_agent` uses only the named half and discards the
  holes, so all eleven hole-carrying tools remain in `v1_agent_tool_catalog`
  and an `Any` output can return `{"entries": [{"agent_id": "peer-7"}]}`
  unchallenged. S-I7's first clause is unenforced wherever an output has an
  `Any` position. Tracked separately; the fix is either to exclude
  hole-carrying tools from the V1 catalog or to scope the
  principals-are-legitimate exemption per tool instead of globally.

- Add `tools_naming_another_agent`, `tools_with_unverifiable_schema` and
  `v1_agent_tool_catalog`, making D18S S-I7 checkable: in V1 an agent's view
  contains no agent identity and the agent cannot supply one. These are
  conformance checks, not a runtime gate -- the enforcement points that decide
  a real agent's surface are the signed manifest and host registration, and
  wiring them is tracked separately. The check walks nested objects and arrays, because a peer
  identity in `delivery.recipients[].agent_id` authorizes as much as one at the
  top level and is easier to miss in review.
- Exclude `vault.request_direct` from the V1 agent surface: it requires a
  `consumer_agent_id`, naming a peer the agent cannot know exists.
  `vault.request_lease` is unaffected. The tool stays correct on the
  supervisor-side path, where the consumer comes from wiring.
- Report violations rather than silently filtering them, so a newly added tool
  taking a peer identity fails a test instead of quietly joining the surface.
- Report `JsonSchema::Any` and `allow_unknown_fields` positions separately as
  *unverifiable* rather than clean. Eleven built-ins declare one, including
  `job.install`'s `spec: Any`, which accepts `{"run_as_agent_id": "peer-7"}`
  and reports no violation. S-I7's "the agent cannot supply one" is established
  only for tools absent from that list, and conflating a hole with a pass would
  have let "no violations" read as "the rule holds".
- Walk output schemas too. Checking only inputs establishes "cannot supply one"
  while leaving "the view contains none" unchecked; a tool returning
  `{agent_id, status}` puts a peer in the agent's view just as surely.
- Match on normalized names covering the identity synonyms this repo actually
  uses -- `host_id` and `host_name` (the host name IS the agent id),
  `principal_id`, `originator_id`, `recipient` -- and normalize case and
  separators, since `validate_schema_key` permits `agentId` and nothing
  enforces snake_case.

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
  to every event sink. The runtime's own framing events remain — not because
  they "describe the call rather than its result", which is false of the
  terminal event, but because a caller has to learn the outcome, and the
  terminal payload is now bounded to the error kind and a runtime-chosen
  message.
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

- `forbidding_side_channels`, a `ToolHandler` combinator that fails any call
  whose handler populated `artifact_refs`, `memory_refs`, or `events`. The
  runtime cannot know which tools must leave those empty; this is how a binding
  says so once, at registration, and has it checked on every call.
- `ToolApiError::BuiltinDefinitionMismatch`. Adding an enum variant is
  technically breaking for exhaustive matches on `ToolApiError`; all 18
  in-repo dependents compile unchanged.

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
