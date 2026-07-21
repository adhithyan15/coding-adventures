# Changelog

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
