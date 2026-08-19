# Changelog

## Unreleased

- Compose the config-backed exact privilege resolver and Trust Checker into the
  production daemon. Fully declared Tier 0 channel and pipeline mutations are
  executable; missing mappings and interactive tiers remain fail-closed.
- Compose an optional shell-free notification helper for exact Tier 1 approval
  while preserving unavailable Tier 1 defaults and closed Tier 2/3 gates.
- Compose an independently optional shell-free native biometric helper for exact
  Tier 2 approval while preserving timeout-as-denial and a closed Tier 3 gate.
- Compose an independently optional shell-free native hardware-key helper for
  exact Tier 3 approval while preserving timeout-as-denial.

- Add optional Chief-owned Reolink pairing over the shared durable controller.
  One complete owner-only configuration tuple binds credentials and a pinned
  network target to an exact bridge, while worker failure joins coordinated
  shutdown and only an opaque Vault reference enters durable state.
- Add optional Chief-owned ZoneMinder pairing over the shared durable
  controller. One complete owner-only configuration tuple binds credential
  input to an exact NVR, startup restores transaction state, and worker failure
  joins coordinated shutdown while API session tokens remain process-local.
- Add optional Chief-owned Axis VAPIX pairing over the shared durable
  controller. One complete owner-only configuration tuple binds credential
  input to an exact bridge, startup restores transaction state, and worker
  failure joins coordinated shutdown without exposing raw credentials.
- Add optional Chief-owned ONVIF pairing over the shared durable controller.
  One complete owner-only configuration tuple binds credential input to an
  exact bridge, startup restores transaction state, and worker failure joins
  the coordinated shutdown path without exposing credentials in durable state.
- Add optional Chief-owned Hue pairing over the shared durable controller. An
  owner-only injected KEK explicitly enables in-process Vault custody; pending
  sessions retain their principal and exact revision, transaction recovery runs
  before serving, and worker failure participates in coordinated shutdown.
- Add optional Chief-owned Hue mDNS discovery on the shared durable smart-home
  controller. Worker setup is idempotent, its lifecycle is explicitly
  start/stop/join managed with both listeners, and clock or actor failure stops
  the composed daemon.
- Preserve Home Assistant request-clock failure through the shared HTTP
  runtime instead of substituting timestamp zero. Each request now samples one
  Unix-millisecond value and returns 503 before authorization or mutation when
  production wall time is unavailable.
- Add an optional Chief-owned Home Assistant-compatible HTTP listener backed by
  the exact same restored durable D23 controller as model tools. Both listeners
  bind before serving and stop together; local HTTP authority provisioning is
  durable, idempotent, and fail-closed.
- Provision operator-declared Chief-host smart-home tool grants through a
  serialized central D23 transaction before serving. Exact unchanged records are
  idempotent, stable grant IDs support durable revocation, and unknown tools,
  future issuance times, persistence failures, or unavailable wall-clock time
  fail startup closed.
- Evaluate model-selected smart-home tools at their real Unix-millisecond
  invocation time. The injected clock now drives grant expiry, controller
  transactions, and durable authorization audit timestamps, and fails closed
  before dispatch when production time is unavailable.
- Restore the central Smart Home controller for model-enabled deployments and
  inject a bounded core D18D catalog into authenticated host tool dispatch.
- Expose the exact production host data-plane composition boundary so the real
  Level 1 child can be exercised against file-provisioned keys, durable encrypted
  channels, and the configured Ollama adapter in one end-to-end test.
- Provision non-empty typed data-plane declarations into the production daemon's
  exact channel-key and Ollama authorities, with UUID-v7/process-monotonic publish
  metadata and no startup network probe. Empty declarations remain unavailable.
- Compose durable per-request host data-plane authorization.
- Compose the storage-backed durable pipeline launch-binding provider. Host
  starts now require an exact registered package plus current immutable channel
  claims, active membership, and bounded persisted model settings.

## 0.1.0 - 2026-08-03

- Add the concrete cross-platform Chief daemon executable.
- Compose strict configuration, owner-only local authentication, trusted package
  keys, durable registry storage, verified host supervision, authenticated
  WebSocket serving, periodic reconciliation, and cooperative process shutdown.
- Bound and race-check configuration-file loading without following a final
  symlink.
- Apply the configured restart-intensity bound to the reconciler, and pin the
  crate's default against the reconciler's own so the two cannot drift apart.
- Derive the reconciler's boot id from the daemon's wall-clock start time.
- Derive the reconciler's boot id from random bytes mixed with the wall clock,
  rather than a clock reading alone that two runs can share.
