# Changelog

## Unreleased

- Add `check_registration` to `HostProfileRuntime` and
  `OrchestratorProfileRuntime`: reports whether a definition would be accepted
  without registering it, so a caller wiring several related tools can pre-flight
  the whole set instead of leaving a host half-wired when a later one is refused.
  `register_handler` now calls it rather than repeating the checks, so the dry
  run and the real thing cannot drift apart.

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
