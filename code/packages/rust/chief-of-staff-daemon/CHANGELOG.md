# Changelog

## Unreleased

- **`[hosts.broker]` gives each agent its own channel broker** (D18S
  P2.6d-2b; #13980). The broker's key table is `[data_plane] channel_keys`,
  slot for slot, with home-relative paths resolved. The daemon opens no key
  file for it: the launcher opens each agent's files when it launches that
  agent's broker.
  - The binary is verified against its pinned digest at startup, so a wrong
    binary stops the daemon instead of failing every launch. It is checked
    again before each launch, through the descriptor that is executed.
  - Off Linux, `[hosts.broker]` is refused at startup
    (`BrokerUnsupported`): there is no verified launch there yet.
  - New errors: `BrokerExecutable`, `BrokerUnsupported`, `BrokerKeys`.
  - Without the table, nothing changes. In this step the daemon still loads
    the channel keys for its own path as well; P2.6d-2c removes that.
- **The daemon suppresses its own core dumps before it starts** (D18S S-I5;
  #13980 P2.6a). The daemon holds every agent's channel keys. `main` now
  calls `chief_of_staff_process_hardening::suppress_core_dumps` before
  anything else, and refuses to start if that fails:
  - `RLIMIT_CORE` is set to zero, soft and hard;
  - on Linux, `PR_SET_DUMPABLE` is set to 0;
  - on macOS, `PT_DENY_ATTACH` is set.

  The smoke test reads the running daemon's `/proc/<pid>/limits`, and, when
  not run as root, the ownership of `/proc/<pid>/mem`.
- **Every vault the daemon opens is now anchored** (VLT01 F11; #13980
  P1.20c). The six smart-home pairing vaults (Hue, ONVIF, Axis, ZoneMinder,
  Reolink, Synology) share the Chief vault's storage root, but they used to
  open it with `SealedStore::new`. So a restored snapshot of the root was
  `Tamper` for the Chief namespaces, yet still loaded the pairing
  credentials it held.
  - All seven openers now go through `open_anchored_vault`. It keeps the
    anchor in `<kek_path>.freshness/`, refuses an anchor inside the storage
    directory, and runs only after the KEK file has been read.
  - An anchor failure on a pairing vault is reported as
    `ChiefDaemonError::ChiefVaultAnchor`. It is the same storage root.
  - New `ChiefDaemonError::ChiefVaultAnchorInsideStorage`. It replaces the
    misleading `AnchorError::InsecureDirectory` for an anchor placed inside
    the storage directory, or storage placed inside the anchor.
    - The check now resolves symlinks once the storage directory exists, and
      on Unix it compares device and inode numbers.
    - Before this, a symlinked `storage_path` put the anchor inside the
      storage, and the check passed.
  - A source-level test pins that no production opener calls
    `SealedStore::new`. It also counts the eight `open_anchored_vault`
    sites: the definition, the Chief vault and six pairing services.
  - The new test restores a snapshot of a pairing vault after a credential
    rotation. It loads the old credential without the anchor, and is
    `Tamper` through the daemon's opener.
- `open_chief_vault` now anchors the vault (VLT01 F11) in
  `<kek_path>.freshness/`, created owner-only next to the KEK. A consistent
  snapshot of the storage directory put back, records and index together, is
  `Tamper` at startup. The index alone could not catch that. A new
  `ChiefDaemonError::ChiefVaultAnchor` reports an anchor directory that
  cannot be opened, or one that is writable by others.
- New `compose_host_data_plane_with_fetcher`. It is the production composition
  with the `net.fetch` resolver and transport supplied as a `NetFetch<R, T>`,
  so the whole pipeline still runs. `compose_host_data_plane` calls it with
  `NetFetch::production()`. The agent
  tool source is no longer generic over a resolver and transport. This is the
  seam the P1.5 weather reference agent's end-to-end test uses.
- **P1.4c: the daemon now serves `net.fetch` and `vault.request_lease`** (D18V
  "Daemon composition"). They come from a second model-tool source,
  `agent_tools::AgentModelTools`, composed beside smart home. Unlike smart
  home, it gives each host a different surface:
  - **Surface (V-D2).** It is derived from the host's own package, which is
    verified against the daemon keyring and pinned to the registration's
    `package_hash`. The manifest's tier may not exceed the signing key's
    ceiling. Every tool goes through the host runtime's `check_registration`
    (allowed tools, tier, tool capabilities). `net.fetch` also needs a
    `net:connect` capability. `vault.request_lease` also needs a vault and a
    `vault_access` mode of `leased` or `both`. Surfaces are cached under host
    name, package path and package hash, so editing a package after it is
    registered cannot widen it.
  - **Identity (V-D3).** Every call runs as the registration's host name.
  - **Leases (V-D4).** The manifest's `vault_access` narrows a lease request
    (which secrets, and the longest TTL) before the vault's own policy sees
    it.
  - **Credentials (V-D5).** They are redeemed through `consume_for`, so a lease
    is bound to the host it was issued to and the secret to its provisioned
    destinations. Every refusal reads `credential_refused`.
  - **Errors (V-D6).** Failures are tool results carrying
    `details.reason`, not transport failures.
- **Startup loads the vault (V-D1).** New `load_chief_vault_runtime` opens the
  vault and registers every sealed record, all or nothing. A corrupt record
  stops startup with the new `ChiefDaemonError::ChiefVaultLoad`. `run` loads
  it before anything serves. `compose_host_data_plane` loads the keyring and
  the vault the same way.
- `required_capabilities.json` declares the new `net:dns` and `net:connect`
  egress, both limited to targets named by verified manifests.

- Add `open_chief_vault`. It returns `None` when `[vault] kek_path` is absent.
  Otherwise it reads the owner-only KEK and unseals the vault storage root,
  initializing it on first use. This is the single way to open the Chief
  vault: `chief-of-staff vault put` uses it now, and startup registration
  (P1.4c) will use it next. Adds `ChiefDaemonError::ChiefVaultSecret` and
  `ChiefVault`.
- `ChiefDaemonError` now reports a `source()` for `Config`, `ChiefVaultSecret`
  and `ChiefVault`. Operators can then see which config field is wrong or why
  the vault would not unseal, rather than only "configuration unavailable".
- Compose the model tool surface through `CompositeModelToolDispatcher`, with
  the smart-home dispatcher as one source rather than the whole surface. No
  behaviour change today -- it is a list of one -- but adding the second source
  is now an addition rather than a rewrite of the daemon's composition.
- Memoize `D18dSmartHomeModelTools::definitions`. It ignores the binding and
  rebuilt the whole 322-entry smart-home catalog once per production tool id,
  measured at 17ms. `ListModelTools` and `CompleteWithTools` already paid that;
  routing a composite by tool name would have made `ExecuteTool` pay it too, on
  a path whose rate a child controls and where the previous gate was a
  ten-element string scan.

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
- Derive the reconciler's boot id from random bytes mixed with the wall clock,
  rather than a clock reading alone that two runs can share.
