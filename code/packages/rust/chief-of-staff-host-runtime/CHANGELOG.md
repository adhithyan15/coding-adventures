# Changelog

## Unreleased

- Enforce D18S S-I7 in `HostProfile::from_manifest`: a manifest declaring a
  tool whose schema names another agent is refused. `tools_naming_another_agent`
  had existed as a conformance test since the S-I7 work, and a test is not a
  boundary -- nothing consulted it, so a manifest declaring
  `vault.request_direct` produced a working profile that let the agent name a
  peer.
- Deliberately NOT applied to `from_json`. That is the operator-config path for
  supervisor-side profiles, where naming a consumer is the point and
  `request_direct` is correct. S-I7 governs what an agent may be offered.
- Tool ids this crate has no definition for are skipped rather than rejected:
  `activate()` already refuses a host whose allowed tools were not all
  registered, and this check must not quietly become a second, weaker existence
  check.
- Add `HostProfile::from_manifest`, deriving a host profile from a verified
  agent manifest so the surface a supervisor enforces comes from bytes inside
  the integrity boundary rather than operator config.
- `HostProfile::capabilities` is fed from `manifest.tool_capabilities`, never
  from `manifest.capabilities`: the first holds D18D scopes like
  `smart_home:read`, the second holds spec-13 operating-system triples like
  `fs:read:/x`. Crossing them would grant tool access on the strength of an
  unrelated operating-system declaration.

- Add `check_registration` to `HostProfileRuntime` and
  `OrchestratorProfileRuntime`: reports whether a definition would be accepted
  without registering it, so a caller wiring several related tools can pre-flight
  the whole set instead of leaving a host half-wired when a later one is refused.
  `register_handler` now calls it rather than repeating the checks, so the dry
  run and the real thing cannot drift apart. The walk is co-total with
  registration: it repeats the registry's own `InvalidDefinition` and
  `DuplicateToolId` checks as well as the three profile checks, returning the
  same error variants. A pre-flight that checks fewer things than the real call
  is worse than none — it converts "this will fail" into "this will succeed"
  immediately before it fails anyway.

- Accept canonical colon-delimited D18D capability scopes in reviewed host
  profiles so built-in service policy can be registered without aliases.
- Support a distinct signed, code-free `SKILL.md` package layout alongside the
  canonical deny-all Deno layout.
- Add no-overwrite package signing and retain authenticated Level 1 source bytes
  for race-free trusted loading.
- Retain authenticated manifest bytes for race-free package discovery.
- Expose read-only trusted public-key lookup for daemon composition and audits.
- Add a canonical deny-all Deno launch plan shared by package generation,
  verification, and process activation.
- Route subprocess host RPC calls through profile-gated D18D handlers in Rust.
- Serve agent-originated `host.*` calls over the live deny-all Deno stdio
  session, returning typed Rust handler results and policy rejections.
- Allow reviewed host profiles to install centralized D18D policy engines and
  route scoped approval grants through active host ownership boundaries.
- Enforce challenge binding and biometric-strength assurance for Tier 2 host
  calls before dispatching their profile-owned handlers.

## Unreleased

- Added JSON host profiles with exact D18D tool allowlists, privilege ceilings,
  capability coverage checks, catalog completeness validation, and an active
  runtime wrapper for executable Chief jobs.
- Added supervised external host activation over the shared stdio process pool,
  including profile-complete process catalogs, owner-routed host RPC, bounded
  crash restart, health snapshots, and shutdown.
- Added deterministic SHA-256 package sealing, Ed25519 verification against a
  typed trusted keyring, tamper and symlink rejection, signer privilege ceilings,
  and a verified-only supervised activation API.
- Added launch-time package re-verification and literal deny-all Deno worker
  commands, with executable RPC proof that network, file reads and writes,
  environment access, and subprocess execution remain unavailable.
