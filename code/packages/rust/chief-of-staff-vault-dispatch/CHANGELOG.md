# Changelog

All notable changes to `chief-of-staff-vault-dispatch` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
  schema, so the "no secret return channel" property is enforced structurally
  rather than by handler discipline. A test registers a deliberately leaking
  handler under the real definition and asserts the runtime rejects it — and
  counts handler invocations, so the test cannot pass vacuously on arguments
  that failed input validation before the handler ran.
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

### Specification

- Implements D18D section 7.1, "Host dispatch binding", added in the same
  branch. The spec was written first and states the four invariants normatively
  so an implementation can be reviewed against a rule rather than against the
  author's intent.
