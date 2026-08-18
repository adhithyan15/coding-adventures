# Changelog

All notable changes to `chief-of-staff-vault-dispatch` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `VaultToolBridge::register_lease_only_into_host`, for deployments with no
  trusted `VaultDirectDelivery`. `request_direct` without one has nowhere to
  deliver, and registering it against a permissive stub would be strictly worse
  than leaving it out — the agent would see a working direct-delivery tool while
  the secret went nowhere. This is not a hole in the pair registration's
  all-or-nothing rule: that rule is about partial FAILURE, while this is a
  deliberate subset, chosen up front and complete in itself. They are separate
  named operations so a reader can tell which one a call site meant. D18D 7.1 V4
  amended to match.

### Changed

- The crate now has its first consumer outside its own test suite: the
  `weather-agent-e2e` end-to-end harness. That is a CI exercise, not a
  deployment -- see the note on the daemon below. It registers
  through the bridge instead of its own `vault.request_lease` handler, which had
  drifted in two ways that mattered: it accepted any TTL the lease layer would
  take (ninety days, versus the bridge's fifteen-minute agent ceiling) and built
  its error with `format!`, which V2 forbids.
- No *shipping* process registers these tools yet. The daemon does not assemble
  a host-profile tool runtime at all, so the tier gate, the capability checks,
  and every built-in definition other than smart-home are inert in production.
  Tracked separately; it needs an architectural decision rather than a patch.

## [0.1.0] — 2026-08-17

Initial release. Closes the gap between the D18D vault tool *declarations* and
the D18 vault *runtime*: the two tools were catalogued but had no handlers, so
`vault.request_lease` returned `ToolNotFound` at dispatch.

### Added

- `VaultToolBridge`, binding `vault.request_lease` and `vault.request_direct`
  to a `ChiefVaultRuntime` and a trusted `VaultDirectDelivery` adapter. Cheap to
  clone; both fields are `Arc`.
- `VaultToolBridge::register_into_host` — the D18D section 7.1 V4 path, which
  registers through the host runtime's checked `register_handler` so the
  tool-allowed, `Tier2` ceiling, and `vault:lease` / `vault:direct` capability
  checks all apply. Registration failures are returned, not swallowed.
- `VaultToolBridge::register_all` — the same handlers against a bare
  `InMemoryToolRuntime`, for local dispatch and tests. Documented as *not*
  applying the host checks, because the tool registry has no host profile to
  check against.
- `VaultToolBridge::definitions`, which reads both definitions from the built-in
  catalog rather than reconstructing them, so this crate cannot register the
  vault tools at a tier the rest of the system does not validate against.
- The `errors` module: every static message the crate can produce, collected in
  one place so that "bounded, secret-free errors" is checkable by reading one
  screen instead of auditing every `Err` site.
- `MAX_SECRET_NAME_BYTES` (512), bounding the caller-controlled bytes hashed on
  every vault lookup.

### Security properties

- `vault.request_direct` returns JSON `null`. Its declared `output_schema` is
  `null` and the tool runtime validates handler output against the declared
  schema, so for the `output` field the "no secret return channel" property is
  enforced structurally rather than by handler discipline. A test registers a
  deliberately leaking handler under the real definition and asserts the runtime
  rejects it — and counts handler invocations, so the test cannot pass vacuously
  on arguments that failed input validation before the handler ran.
- The runtime validates `output` and only `output`. `artifact_refs` and
  `memory_refs` are copied through unchecked, and `events` are assembled before
  validation and published even when validation rejects the call. Both handlers
  therefore return all three empty, with a test pinning it, and the spec and
  docs state the boundary explicitly rather than implying the runtime covers
  everything.
- Leak tests assert over the whole `ToolExecutionTrace` rather than the
  `ToolResult`, because `ToolResult` has no `events` field — a leak test built
  on it would silently skip the one channel the runtime does not validate.
- The trusted delivery adapter receives a `VaultDirectRequest` carrying the
  requesting agent, session, and secret name alongside the destination. Given
  only `(consumer, payload)` an adapter can express nothing stronger than a
  global destination allowlist, under which a caller cleared to send one secret
  to a consumer is equally cleared to send every secret to it — a confused
  deputy holding the authority but not the facts.
- Handler errors carry `&'static str` messages with nothing interpolated, and
  `details` stays JSON `null`. The one value forwarded from a lower layer is
  `LeaseError::InvalidParameter`, whose payload is itself typed `&'static str`
  and so cannot carry runtime data by construction.
- A refused delivery maps to `ToolPermissionDenied` rather than
  `ToolExecutionError`: the adapter refusing is the adapter exercising authority
  the caller does not have, and telling the caller to retry would be wrong.
- Arguments are re-validated inside each handler. A `ToolHandler` is a trait
  object that anything can invoke directly, so "the layer above validated it"
  stops being true as soon as someone wires the handler up differently.
- Duplicate argument keys resolve to the first occurrence, matching what the
  schema validator reads. Taking the last would let a caller show one value to
  validation and a different one to the vault.

- `MAX_AGENT_LEASE_TTL_MS` (15 minutes) caps the TTL an *agent* may request.
  The lease layer permits 90 days and bounds its table at 4096 rows; together
  those are a squat — fill the shared table at the maximum TTL and every other
  consumer of that manager, trusted host paths included, is locked out for a
  quarter. Capping the agent-facing TTL well under the sweep horizon makes the
  attack self-healing rather than durable.
- Repeated argument keys are rejected rather than resolved. `JsonValue::Object`
  preserves duplicates and the schema validator checks every occurrence, so
  nothing type-invalid slips past — but the schema constrains types, not values.
  Since D18D V4 puts per-call authorization in the policy engine, a policy
  parsing arguments with map or last-wins semantics would authorize one name
  while the handler fetched another, both components individually correct.
  Rejecting removes the category rather than betting on two parsers agreeing.
- `VaultDirectRequest` carries `requesting_user_id` as well as the agent, since
  one vault instance may serve several users and an adapter otherwise cannot
  tell whose session asked.

### Known gaps as of 0.1.0

These describe the state at 0.1.0, and are kept as a record of what that
release did not do rather than as a current statement.

The first has since been *partly* closed: `allowed_agents` and `allowed_mode`
ship in the vault runtime (VLT06, PR #11982). `privilege_tier` is still recorded
and read by nothing, and the tier and capability checks are still per-tool and
run once at registration -- so a `privilege_tier` on a registered secret is not
a control.

- There is no per-secret policy. `privilege_tier`, `allowed_agents`, and
  `allowed_mode` are unimplemented, and the tier and capability checks are
  per-tool and run once at registration. Any caller clearing the tool gate can
  request any registered secret in either mode, including leasing one intended
  to be direct-delivery only. Recorded in D18D §7.2.
- `requesting_agent_id` is read from the execution context and is only as
  trustworthy as whatever populated it. A binding relying on it must first
  establish that its host attests the field.
- A `vault_ref` is a bearer capability, and a successful lease necessarily puts
  it into the tool output, whence the runtime copies it into the terminal event
  payload and the execution journal. Sinks that persist journals persist live
  capabilities.

### Specification

- Implements D18D section 7.1, "Host dispatch binding", added in the same
  branch. The spec was written first and states the four invariants normatively
  so an implementation can be reviewed against a rule rather than against the
  author's intent.
