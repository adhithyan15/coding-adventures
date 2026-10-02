# Changelog

## Unreleased

- Let product callers load `grants.toml` for every discovered plugin in the
  same snapshot pass, granting nothing for missing or stale manifest hashes
  and rejecting malformed authority before any stage can resolve. Discovery
  and authority loading accept an `AbortSignal` and check it throughout their
  bounded filesystem walks.
- Exported the verified config-schema snapshot type used by production sandbox
  process factories.

- Add strict bounded codecs and fail-closed filesystem persistence for the
  user trust store and manifest-bound per-plugin capability grants.
- Publish authority files through restrictive same-directory temporary files
  and atomic rename while rejecting symlinked, multiply-linked, oversized, or
  non-canonical inputs.
- Make discovery and ignored-signal escalation coverage deterministic across
  POSIX and Windows hosts, and accept a clean natural child exit as successful
  bounded abandoned-stream cleanup without requiring a redundant kill signal.
- Give the discovery candidate-limit filesystem fixture enough time on
  contended Windows CI while keeping the production limit unchanged.

## 0.1.0 — 2026-09-27

- Add deterministic manifest discovery and `StageRef` resolution.
- Add bounded Content-Length JSON-RPC framing and bidirectional requests.
- Add handshake/announcement parity, lifecycle, streaming, diagnostics,
  capability mediation, cancellation escalation, and crash isolation.
- Require an injected process launcher to attest sandbox isolation and the
  exact verified manifest/entry identity; no unsandboxed production fallback
  is provided.
- Bind host APIs to the active run ID and the intersection of manifest,
  policy, and per-instance grants; authorize every network redirect.
- Bound discovery snapshots, wire allocations, mediated byte transfers, and
  response bodies; retire sessions on cancellation, protocol failure, crash,
  timeout, or early stream abandonment.
- Accept trusted bounded capability backends through `capabilityApis` for
  end-to-end orchestrator execution.
- Bridge stream inputs for both streaming and single-output stages, and
  round-trip `Uint8Array` payloads with a collision-safe bounded wire codec.
- Defer live storage-watch mediation to FM-B015 rather than materializing an
  unbounded iterator; finite storage listings remain bounded snapshots.
- Validate config schemas before they reach the permissive runtime validator,
  cancel redirect bodies, and fail closed on cross-origin or replay-sensitive
  redirects.
- Recheck opened plugin files against their quiescent host-managed discovery
  root; FM-B015 owns atomic immutable installation roots.
- Snapshot config schemas during discovery, hash exact bytes, bind the digest
  into stage identity and launcher attestation, and reject unsigned auxiliary
  schema claims from signed plugins until FM-B015 package signatures exist.
- Bound decoded output queues by bytes as well as item count, discard buffered
  values on failure, and enforce lifetime plugin-log byte and entry budgets.
- Keep iterator and process cleanup bounded when upstream `return()` throws or
  an abandoned-stream cancellation write stalls on plugin stdin.
- Wire the host's manifest/signature prerequisite chain into downstream
  standalone BUILD recipes so clean CI plans remain reproducible.
- Parse `Content-Length` headers with a bounded linear scanner, avoiding
  attacker-controlled regular-expression backtracking in the trusted host.
