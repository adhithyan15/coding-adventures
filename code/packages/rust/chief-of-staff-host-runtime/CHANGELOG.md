# Changelog

## Unreleased

- Add `SupervisedOrchestratorRuntime::spawn_deno_from_package`, which reads the
  agent manifest from `package.manifest_bytes()` after verifying the package
  and derives the whole orchestrator profile from it. One package, one
  manifest, one host.
- Add `OrchestratorProfile::from_manifests`. It is stronger than `from_json` --
  it runs `AgentManifest::validate` per host, including the version-to-field
  binding `HostProfile::validate` knows nothing about, and then
  `OrchestratorProfile::validate`, which `from_json` never calls at all -- but
  it does **not** by itself establish integrity, and its doc comment says so.
  `AgentManifest`'s fields are all `pub`, so binding a manifest to a verified
  package is the caller's job; `spawn_deno_from_package` is the entry point
  that does it.
- `validate` refuses two hosts claiming the same tool, which matters on this
  path: two agents independently declaring `artifact.write` is an ordinary
  authoring mistake and must not resolve to whichever host was seen last.
- Note that on the supervised path the tier ceiling is aggregate: the runtime
  keeps `tool_owners` and discards per-host `max_tier`, which only
  `HostProfileRuntime::check_registration` reads. With one package per tree the
  max-check is equivalent, but per-host tier enforcement does not exist here.

- Enforce D18S S-I7 in `HostProfileRuntime::check_registration`: a tool whose
  schema names another agent cannot be registered into an agent host. New
  `HostRuntimeError::ToolNamesAnotherAgent` names the tool and the offending
  property. `tools_naming_another_agent` had existed as a conformance test
  since the S-I7 work and nothing consulted it.
- Checked at REGISTRATION, on the definition in hand, not at profile
  construction. A first attempt gated `HostProfile::from_manifest`, which has
  no production callers -- every profile reaching a live agent comes from
  `OrchestratorProfile::from_json`, which builds `HostProfile` struct-literally,
  or from `HostProfileRuntime::new`. A boundary on a path nothing takes repeats
  the error it was meant to fix.
- Resolving ids through `builtin_tool_definition` would have covered 34
  built-ins and silently skipped everything else, including the ten
  `smart_home.*` tools this repo already pins as naming a peer through
  `principal_id`. A registration sees the real definition whatever catalog it
  came from, so the resolver and its gap disappear.
- Only the named-identity half of S-I7 is enforced. Eleven built-ins declare a
  `JsonSchema::Any` position that `tools_with_unverifiable_schema` reports as
  unverifiable, and refusing those would reject `job.install`,
  `context.append_entry` and nine others. Closing those schemas is tracked
  separately; until then "the agent cannot supply one" holds only for tools
  absent from that list.
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
