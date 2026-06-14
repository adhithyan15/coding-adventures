# Orchestrator

## Overview

The Orchestrator is the long-running root process of the agent system.
It is the **Chief of Staff** in the congressional analogy: it knows who
is in the building, makes sure the right offices exist, decides who
opens the door for whom, and notices when an office goes quiet — but it
does not draft speeches, read emails, or hold any agent's secrets.

Its design principle is unusual and deliberate: **the orchestrator
should be as worthless as possible to compromise.** It holds no
agent manifests, decrypts no agent traffic, knows no capability
specifics, and stores no secrets beyond its own long-term identity
keys. An attacker who fully compromises the orchestrator gains the
power to denial-of-service the system (stop and start hosts) but not
to read any user data, impersonate any agent, or escalate to the OS
in ways the host processes themselves are not already permitted to.
Everything sensitive lives below the orchestrator: hosts hold
manifests, the vault holds secrets, channels hold ciphertext.

The orchestrator's responsibilities are exactly four:

1. **Verify package signatures** before any host is launched.
2. **Supervise host processes** as the root of the OS-process
   supervision tree, applying restart strategies and capability
   inheritance.
3. **Maintain a host registry** mapping host names to running
   process handles, so the orchestrator can resolve discovery
   queries and report status.
4. **Coordinate the panic-broadcast network** as the root authority,
   triggering tree-wide quarantine when the global panic rate
   exceeds threshold and paging a human.
5. **Manage provider lifecycle** for every discoverable role:
   spawn-on-discovery when no provider is active, idle-shutdown
   when no consumer is using one, and pool sizing per the
   role's `min_instances` / `max_instances` / `idle_shutdown`
   declarations. (Per `dynamic-topology.md` and
   `agent-discovery.md`.)
6. **Bridge agent-discovery requests** by creating fresh
   ratcheted channels via `secure-host-channel`, applying the
   RWS analysis on the post-bridge topology, and refusing
   bridges that would violate the rule.

Plus one supporting role: **bootstrap secure channels** to each
host using the per-spawn X3DH dance defined in `secure-host-channel.md`,
holding the orchestrator's long-term identity key in memory after the
vault unlocks at startup.

The orchestrator does **not**:

- read agent manifests (the host reads its own manifest)
- generate Deno flags or runtime configuration (the host does)
- decrypt host ↔ agent traffic (it cannot — it has no key)
- hold vault secrets (the vault is a sibling actor, not a child)
- understand what any agent does
- pre-wire channel topology — per `dynamic-topology.md`, all
  agent-to-agent channels are established at runtime via
  agent-discovery; the orchestrator never declares
  `[channel.*]` in its config

This spec defines what the orchestrator *does* do: signature
verification, supervision, registry, panic root, channel
bootstrap, **provider lifecycle management**, and
**discovery-bridge creation**. Everything else is delegated.

---

## Where It Fits

```
   User (CLI / mobile / web)
        │
        │  startup commands, queries, control
        ▼
   Orchestrator  ← THIS SPEC
   ┌─────────────────────────────────────────────────────────────┐
   │  signature verifier   ──► trusted keyring (vault-key-custody)│
   │  host supervisor      ──► supervisor crate                  │
   │  service registry     ──► in-memory + persistent on disk    │
   │  trust checker        ──► privilege tier escalation         │
   │  channel bootstrap    ──► secure-host-channel               │
   │  panic root           ──► tree-wide quarantine              │
   └─────────────────────────────────────────────────────────────┘
        │
        │  spawns OS processes (each is a host)
        ▼
   Hosts (OS processes, supervised by orchestrator)
        │
        │  each speaks host.* over secure-host-channel
        ▼
   Agents (Tier 1 native / Tier 2 WASM / Tier 3 BYO)
```

**Depends on:**
- `supervisor` — the orchestrator IS a top-level supervisor.
- `capability-cage-rust` — the orchestrator's own manifest is the
  envelope of what any host can request.
- `secure-host-channel` — bootstrap and operate one channel per host.
- `vault-sealed-store`, `vault-auth`, `vault-key-custody` — for
  unlocking the orchestrator's identity key at startup.
- `actor` — for the in-process actors that implement the four roles.
- `process-manager` — for `fork`/`exec`/`wait` of host processes.
- `json-parser`, `json-value` — for service-registry persistence.

**Used by:**
- The user's shell (start/stop, list hosts, install agent packages).
- Hosts (only via the secure channel, for lifecycle messages).
- Future automations (the iPaaS-style tools that wire pipelines).

---

## Design Principles

1. **Worthless when compromised.** The orchestrator's blast radius is
   denial-of-service. It holds no agent secrets, reads no user data,
   and has no privilege the hosts do not also have. Compromise costs
   the user uptime, not data.

2. **All policy lives in manifests.** The orchestrator does not bake
   policy into code. Every decision (does this host launch? what is
   its privilege tier? can it talk to that other host?) is read from
   a signed manifest and a checked capability.

3. **Single point of supervision, never of communication.** The
   orchestrator decides when hosts start and stop. It does not route
   their messages. Channels go directly host-to-host (or
   host-to-vault), pre-wired at supervisor build time, encrypted and
   ratcheted independently of the orchestrator.

4. **Idempotent and crash-safe.** Every operation the orchestrator
   performs is idempotent and survives a mid-operation crash. On
   restart, the orchestrator reads the persistent registry, queries
   for surviving hosts (some may have outlived a brief orchestrator
   restart), and reconciles.

5. **Verifies twice on different evidence.** The orchestrator
   verifies a package's signature before launching the host. The
   host verifies the same signature again before reading the
   manifest, because it does not trust the orchestrator. Two
   independent checks on the same artifact.

6. **Logs every supervision event.** Start, stop, restart, panic,
   tier escalation, signature failure — every event becomes an
   audit record on a dedicated audit channel that no host can
   write to or suppress.

---

## Key Concepts

### Signature Verifier

Every agent package has the shape defined in `D18-chief-of-staff.md`:

```
my-agent.agent/
├── manifest.json
├── code/
│   └── ...
├── launch.sh                (generated at build time)
├── SIGNATURE                (Ed25519 over the rest)
└── PUBKEY_ID                (which key signed; not the key itself)
```

The orchestrator's signature verifier:

1. Reads `PUBKEY_ID` from the package.
2. Looks up the public key in the **trusted keyring**, a vault
   namespace at `vault://orchestrator/trusted-keys/`.
3. Computes the SHA-256 hash of the rest of the package
   (deterministic file-set ordering as specified in D18).
4. Verifies the Ed25519 `SIGNATURE` against the hash with the
   trusted public key.
5. Returns Ok or `SignatureError` (UnknownKey | InvalidSignature |
   PackageMalformed).

**Three trusted-key tiers** (mirrors D18:880):

```
Production key (CI YubiKey)        → grants Tier 0-3 launches
Developer key (local dev)          → grants Tier 0-1 launches only
Third-party key (community agent)  → grants tier capped at user's approval
```

The trust tier of the signing key bounds the maximum tier of any
host launched from a package signed by it. A package signed with a
developer key cannot launch a Tier 2 host even if its manifest says
so; the orchestrator rejects with `InsufficientTier`.

The trusted keyring is itself in the vault, which means modifying it
requires Tier 3 (hardware key) per the privilege tier policy.
Adding a new trusted key is one of the rare actions that pages a
human even in an autonomous deployment.

### Host Supervisor

The orchestrator is a `Supervisor` (from `supervisor` crate) at the
root of the OS-process tree. Every host is a `ChildKind::HostProcess`
child. Strategies and restart policies are defined per-host in the
agent package's `manifest.json` and copied into the corresponding
`ChildSpec` at registration time:

```rust
let host_spec = ChildSpec {
    id:               ChildId::new("comms-host"),
    kind:             ChildKind::HostProcess,
    start:            Box::new(|| spawn_host_process(&package)),
    restart:          manifest.restart_policy,    // permanent | transient | temporary
    shutdown:         manifest.shutdown_policy,   // graceful(d) | brutal | infinity
    manifest:         manifest.capability_manifest,
    vault_namespace:  manifest.vault_namespace,
    channel_scope:    manifest.channel_scope,
};
```

The supervisor's capability-inheritance check (defined in
`supervisor.md`) ensures the host's manifest is a subset of the
orchestrator's own. The orchestrator's manifest is broad
(`supervise`, `proc:fork`, `proc:exec:*`, `vault:admin`,
`fs:read:./agents/*`) because it spawns processes; hosts have
narrower manifests appropriate to their job.

When a host crashes, the supervisor's restart strategy decides what
happens. The orchestrator does not invent its own logic — it consumes
what `supervisor` provides.

### Service Registry

A registry of running hosts:

```rust
pub struct ServiceRegistry {
    entries: HashMap<HostName, HostEntry>,
}

pub struct HostEntry {
    pub host_name:        HostName,
    pub package_path:     PathBuf,
    pub package_hash:     [u8; 32],     // SHA-256 of the signed package
    pub pid:              u32,
    pub status:           HostStatus,
    pub started_at:       SystemTime,
    pub last_heartbeat:   SystemTime,
    pub channel_id:       ChannelId,    // the secure-host-channel for this host
}

pub enum HostStatus {
    Starting,
    Running,
    Restarting,
    Stopping,
    Stopped,
    Quarantined { until: SystemTime, reason: String },
}
```

The registry is persisted to disk under the orchestrator's data
directory (default `./.orchestrator/registry.json`) on every change
and re-read at startup. Persistence uses atomic write (write to
`registry.json.tmp`, fsync, rename) so a crash during write never
leaves a corrupt registry.

The registry is **not authoritative** about what is actually
running; the supervisor's child set is. The registry is a cache for
fast lookup and for reconstructing intent on restart.

### Trust Checker

For any operation that requires a privilege tier above 0, the trust
checker challenges the user according to the tier policy:

```
Tier 0  →  No challenge
Tier 1  →  Notification with 5-second auto-approve window
Tier 2  →  Biometric (Face ID, Touch ID, fingerprint, passphrase)
Tier 3  →  Hardware key (YubiKey, FIDO2)
```

The challenge is delivered via a side channel (the user's phone,
desktop notification, or hardware key prompt) and not through any
host. A timeout or denial cancels the operation; an approval
proceeds.

The trust checker is consulted by:
- The signature verifier (when launching a host whose effective
  tier is >= 1).
- The pipeline-wiring logic (when connecting hosts whose channels
  cross tier boundaries).
- The vault (when releasing high-tier secrets — the vault calls back
  into the trust checker via a dedicated channel).

The trust checker itself does not store user credentials; it
delegates to platform-specific authenticators (Touch ID, Hello,
fingerprint, etc.) and to the vault's auth subsystem
(`vault-auth`).

### Channel Bootstrap

For each host the orchestrator spawns, it must establish a
`secure-host-channel` (per `secure-host-channel.md`). The
orchestrator owns the orchestrator-side of every such channel.

Bootstrap sequence (orchestrator side):

```
1. Generate per-spawn ephemeral X25519 key pair (OEK).
2. Construct a PreKeyBundle:
   { OIK_pub  : orchestrator long-term identity public key,
     OEK_pub  : per-spawn ephemeral public key,
     signature: OIK signs OEK_pub
                (proves bundle came from this orchestrator) }
3. Open a fresh OS pipe; write the bundle plus session metadata
   to the write end:
   { protocol_version  : "1.0",
     orch_prekey_bundle: { OIK_pub, OEK_pub, signature },
     child_session_id  : new uuid v7,
     channel_aad_prefix: "host://<host_name>/<session_id>" }
4. Spawn the host process with:
   - argv: [path-to-host-runtime, path-to-agent-package]
   - env:  { CHANNEL_BOOTSTRAP_FD = "3" }
   - fd 3: read end of the bootstrap pipe
   - fds 0/1/2: stdin/stdout/stderr piped to orchestrator's logging
5. Wait for the host's first wire message on its stdout
   (the response to X3DH).
6. Construct the orchestrator-side ChannelResponder; verify the
   handshake succeeds.
7. Add the channel to the registry; mark the host as Running.
```

The OEK is rotated on every spawn; OIK is constant for the
orchestrator's lifetime (until the user rotates it via Tier 3).
The bootstrap pipe is closed and its fd freed immediately after
the host reads from it; the bootstrap window is microseconds.

### Panic-Broadcast Root

The orchestrator is the root of the panic-broadcast network defined
in `secure-host-channel.md`. It:

1. **Receives** panic signals from every host (and forwarded panic
   signals from sub-orchestrators in deeper trees).
2. **Aggregates** signals across the tree; tracks
   panic-rate-per-minute per origin.
3. **Decides** when to escalate to **tree-wide quarantine**:
   - 5 distinct alerts in 60 seconds → tree-wide quarantine
   - any Critical-severity panic → tree-wide quarantine
   - 3 forge-detection failures (from the chain-of-forwarders check) → tree-wide quarantine
4. **Acts** on quarantine:
   - Refuse all new host spawns.
   - Refuse all new manifest installs.
   - Refuse all new channel wirings.
   - Continue serving existing children (their channels keep
     operating, possibly with tightened thresholds).
   - Page a human via every configured channel
     (push notification, email, SMS, on-call rotation hook).
5. **Awaits explicit clearance** from the user before exiting
   tree-wide quarantine. The clear command is itself a Tier 3
   operation (hardware key required).

The panic root is also responsible for **persisting the panic
log** to a write-only audit file under
`./.orchestrator/panic-log.jsonl`, append-only, fsync on every
write, with file permissions that prevent any host from reading
it (it is a forensic record, not telemetry).

---

### Provider Lifecycle Management

Per `dynamic-topology.md`, the orchestrator owns the lifecycle
of every discoverable provider. There are no predefined
channels; every channel is created at runtime via
`agent-discovery.md`'s `find_and_connect`. The orchestrator
must therefore decide:

- **When to spawn a provider** (no active instance, but a
  consumer wants one)
- **When to retire one** (no active bridges and the configured
  idle threshold has passed)
- **How many to keep alive** (load-driven scaling within the
  `min_instances` / `max_instances` envelope each provider
  declares)

The full semantics live in `agent-discovery.md`'s "Provider
Lifecycle" section and `dynamic-topology.md`'s rules. The
orchestrator's responsibilities are:

1. **Spawn-on-discovery.** When a `find_and_connect` request
   arrives for a role with no active provider, the orchestrator
   consults the agent-registry, selects a registered package
   that provides the role and matches the qualifier, and
   launches it through the normal `launch_host` path
   (including the registry hash pin, signature verify, and
   tier challenge). The consumer's discovery call blocks until
   the new host completes its X3DH handshake (default timeout
   10 s) or returns `NoProvidersFound`.

2. **Idle-shutdown.** Per provider, the orchestrator tracks
   time-since-last-active-bridge. When it exceeds the
   provider's declared `idle_shutdown` AND no current bridges
   exist AND `min_instances` would not be violated, the
   orchestrator gracefully terminates the provider (sends
   Terminate, waits for shutdown timeout per supervisor rules,
   reaps the OS process, removes from the registry).

3. **Pool sizing.** The orchestrator scales a role's instance
   count between `min_instances` and `max_instances` based on
   load. When the average `current_load` across active
   instances exceeds 75% of `max_concurrent`, spawn another
   (up to the cap). When average drops below 25% and at least
   one instance has been idle past `idle_shutdown`, retire
   that one. `min_instances` is always honored: if a crash
   drops the count below it, spawn a replacement immediately
   without waiting for a discovery.

4. **Bridge creation.** Per `agent-discovery.md`'s bridging
   sequence, after providers are resolved the orchestrator
   creates two ratcheted channels via `secure-host-channel`,
   pushes endpoint handles to both sides via control-priority
   messages, and updates the bridge ledger.

5. **Bridge teardown.** When either side disconnects (clean
   close or process exit), the orchestrator notifies the peer,
   reclaims the channel slots, decrements the provider's
   `current_load`, and (if applicable) starts the
   idle-shutdown timer for that provider.

The orchestrator's bridge ledger is persisted to disk
alongside the registry (`./.orchestrator/bridges.json`,
atomic write) so a mid-bridge orchestrator crash can
reconstruct active bridges on restart and avoid duplicating
or orphaning them.

The default policy (`min_instances: 0`, `max_instances: 1`,
`idle_shutdown: never`) yields "exactly one instance once
discovered, kept alive forever" — operationally identical to
a pre-spawned model, with no extra config burden on the
agent author.

**There is no `[channel.*]` section in `orchestrator.toml`.**
Channels exist only as the result of discovery calls at
runtime. The TOML config retains `[host.*]` for per-host
lifecycle hints (`restart`, `shutdown`, `idle_shutdown`,
`min_instances`, `max_instances`) and `[panic_thresholds]`
for the panic root, but channel topology is invisible at the
config level — it is a runtime-only concept.

---

## Lifecycle

### Startup

```
1. Read configuration: orchestrator.toml in the working directory.
   Configured items: vault location, default tier policy,
   panic-broadcast thresholds, persistence paths, log-stream sinks.

2. Vault unlock:
   prompt user for unlock factor (passphrase / biometric / hw key)
   open vault-sealed-store with the resulting master key
   load orchestrator's long-term identity keypair
     (vault://orchestrator/identity/)
   load trusted keyring
     (vault://orchestrator/trusted-keys/)

3. Reconstruct from registry:
   read ./.orchestrator/registry.json
   for each entry:
     if pid is alive AND package_hash matches what's on disk:
       reattach to the host process
       wait for the host to re-handshake on its existing channel
       mark as Running
     else:
       mark as Stopped
       if restart_policy is Permanent or Transient:
         relaunch the host (same flow as a fresh launch)

4. Start the audit log writer (a dedicated actor that appends to
   ./.orchestrator/audit.jsonl with fsync per record).

5. Start the panic root actor (subscribes to the well-known panic
   channel that every host publishes to).

6. Start the user-facing control surface (the CLI / RPC server
   that takes commands from the user shell; details out of scope).

7. Mark orchestrator as Ready.
```

### Launching a New Host

```
launch_host(package_path: &Path) -> Result<HostName, LaunchError>

1. Read package manifest.json (orchestrator does NOT inspect any
   other file in the package; only the manifest).
2. Verify signature (signature_verifier.verify(package_path))
   On failure → return SignatureError.
3. Determine effective tier from manifest.
   If effective tier > tier of signing key → return InsufficientTier.
4. If effective tier > 0 → trust_checker.challenge(tier)
   On denial or timeout → return TrustChallengeDenied.
5. Construct ChildSpec from manifest.
   Verify manifest is subset of orchestrator's manifest (supervisor
   does this on start_child).
   On capability escalation → return CapabilityEscalation.
6. supervisor.start_child(child_spec)
   This in turn calls our spawn_host_process closure, which:
   a. Generates the per-spawn channel bootstrap.
   b. Forks and execs the host runtime with the bootstrap fd.
   c. Completes the X3DH handshake.
7. Add to ServiceRegistry.
   Persist registry.
8. Emit AuditRecord {kind: "host.launched", host_name, tier, ...}
9. Return Ok(host_name).
```

### Stopping a Host

```
stop_host(name: HostName, mode: ShutdownMode) -> Result<(), StopError>

1. Look up host in registry. If absent → return NotFound.
2. supervisor.terminate_child(child_id)
   Honors the host's shutdown policy (Graceful(d) | Brutal | Infinity).
3. The supervisor closes the secure channel; both sides zeroize keys.
4. Remove from registry; persist.
5. Emit AuditRecord {kind: "host.stopped", host_name, mode, ...}
6. Return Ok.
```

### Handling a Host Crash

The supervisor detects the crash (via `wait()` returning a non-zero
status or signal) and applies its restart strategy. The orchestrator's
involvement is:

```
on_host_exit(host_name, exit_reason):
  registry.set_status(host_name, Restarting | Stopped)
  emit AuditRecord {kind: "host.crashed", host_name, exit_reason}
  // supervisor handles the actual restart via its strategy.
  // If it restarts, our spawn_host_process closure runs again,
  // including a fresh channel bootstrap.
  // If it gives up (max_restarts exceeded), supervisor escalates;
  // orchestrator marks status Stopped and emits a critical audit
  // record.
```

### Panic Reception

```
on_panic_signal(signal: PanicSignal):
  panic_log.append(signal)         // forensic record
  panic_rate_tracker.record(signal)

  // The supervisor may have already acted (brutal-killed the
  // suspect) by the time we see this; that's fine, it's idempotent.

  if panic_rate_tracker.exceeds_global_threshold():
    enter_tree_wide_quarantine(reason: signal)
```

### Tree-Wide Quarantine

```
enter_tree_wide_quarantine(reason):
  set state.quarantined = true
  refuse all launch_host / install_manifest / wire_pipeline calls
  emit AuditRecord {kind: "tree.quarantined", reason, ...}
  page_human(reason)                // every configured channel

clear_quarantine():
  // Tier 3 challenge required (hardware key).
  trust_checker.challenge(Tier3)?
  set state.quarantined = false
  emit AuditRecord {kind: "tree.cleared", ...}
```

### Shutdown

```
shutdown(mode):
  refuse new commands
  supervisor.terminate()
    // walks the OS-process tree in reverse-start order;
    // each host gets its shutdown policy honored.
  flush audit log; close panic log
  zeroize identity keys in memory
  release vault
  exit
```

---

## Storage Layout

```
.orchestrator/
├── registry.json              service-registry snapshot
├── audit.jsonl                append-only structured audit log
├── panic-log.jsonl            append-only panic-signal forensic log
├── orchestrator.toml          configuration (read-only after start)
└── pids/
    ├── comms-host.pid          one file per running host
    └── ...
```

`registry.json` and `audit.jsonl` are written via atomic
write-then-rename; `panic-log.jsonl` is fsync-on-write to ensure
no signal is ever lost in a crash. The pids directory exists for
external observability (a sysadmin can `cat .orchestrator/pids/*`
to see what is running).

All four files are inside the orchestrator's vault namespace
(`vault://orchestrator/state/`) when the user opts into encrypted
state-on-disk; otherwise they are world-readable plain JSON for
debuggability.

---

## Public API

### CLI Surface (sketch)

```
orchestrator start [--config ./.orchestrator/orchestrator.toml]
orchestrator stop [--mode graceful|brutal] [--timeout 30s]
orchestrator status
orchestrator list-hosts
orchestrator launch <package-path>
orchestrator stop-host <host-name>
orchestrator restart-host <host-name>
orchestrator install-manifest <package-path>
orchestrator clear-quarantine               # Tier 3 challenge
orchestrator audit-tail                     # stream audit log
orchestrator panic-tail                     # stream panic log
orchestrator add-trusted-key <key-file>     # Tier 3
orchestrator remove-trusted-key <key-id>    # Tier 3
```

The CLI is a thin client over an internal RPC; the orchestrator
process exposes a Unix domain socket / Windows named pipe whose ACLs
grant only the user that started it.

### Rust API (for embedding the orchestrator in custom binaries)

```rust
pub struct Orchestrator { /* opaque */ }

pub struct OrchestratorConfig {
    pub vault_path:               PathBuf,
    pub state_dir:                PathBuf,
    pub identity_key_namespace:   VaultNamespace,
    pub trusted_keys_namespace:   VaultNamespace,
    pub panic_thresholds:         PanicThresholds,
    pub default_shutdown_timeout: Duration,
}

pub struct PanicThresholds {
    pub global_alert_count_window:   Duration,    // default 60s
    pub global_alert_count_max:      u32,         // default 5
    pub forge_detection_max:         u32,         // default 3
}

impl Orchestrator {
    pub fn start(config: OrchestratorConfig)
        -> Result<Self, StartError>;

    pub fn launch_host(&mut self, package: &Path)
        -> Result<HostName, LaunchError>;
    pub fn stop_host(&mut self, name: HostName, mode: ShutdownMode)
        -> Result<(), StopError>;
    pub fn restart_host(&mut self, name: HostName)
        -> Result<(), StopError>;

    pub fn list_hosts(&self) -> Vec<HostEntry>;
    pub fn host_status(&self, name: &HostName)
        -> Option<HostStatus>;

    pub fn add_trusted_key(&mut self, key: PublicKey,
        tier_cap: PrivilegeTier) -> Result<(), TierError>;
    pub fn remove_trusted_key(&mut self, key_id: KeyId)
        -> Result<(), TierError>;

    pub fn enter_quarantine(&mut self, reason: String);
    pub fn clear_quarantine(&mut self) -> Result<(), TierError>;

    pub fn audit_tail(&self) -> impl Iterator<Item = AuditRecord> + '_;
    pub fn panic_tail(&self) -> impl Iterator<Item = PanicSignal> + '_;

    pub fn shutdown(self, mode: ShutdownMode)
        -> Result<(), ShutdownError>;
}

pub enum LaunchError {
    SignatureError(SignatureError),
    InsufficientTier { required: PrivilegeTier, have: PrivilegeTier },
    TrustChallengeDenied,
    CapabilityEscalation { missing: Vec<Capability> },
    ManifestParse(String),
    Spawn(io::Error),
    BootstrapFailed(ChannelError),
    QuarantineActive,
}

pub enum SignatureError {
    UnknownKey(KeyId),
    InvalidSignature,
    PackageMalformed(String),
}
```

---

## Test Strategy

### Unit Tests

1. **Signature verification** — known good, unknown key, tampered
   signature, malformed package, key tier capping.
2. **Registry persistence** — round-trip of every host status; atomic
   write semantics (kill mid-write must leave a valid file).
3. **Trust checker** — Tier 0 passes without challenge; Tier 1
   notify-and-auto-approve; Tier 2 biometric; Tier 3 hardware key;
   timeouts and denials propagate correctly.
4. **Capability inheritance** — host with a manifest exceeding the
   orchestrator's manifest is rejected before spawn.
5. **Panic-rate tracker** — global threshold logic; rate-window
   pruning; quarantine triggered at the configured thresholds.

### Integration Tests

6. **End-to-end launch** — spawn a real test host, complete the
   bootstrap, exchange a host.* call, verify the audit record exists.
7. **Crash and restart** — kill a permanent host; verify the
   supervisor restarts it via the same launch path; verify a fresh
   channel bootstrap.
8. **Crash storm** — inject a host that crashes immediately on start;
   verify the supervisor escalates within the configured intensity
   window; verify the orchestrator marks the host Stopped and audits.
9. **Panic-driven quarantine** — emit synthetic panic signals at the
   global threshold rate; verify tree-wide quarantine engages, new
   launches are refused, the human-page hook fires.
10. **Quarantine clearance** — clear quarantine with Tier 3 challenge;
    verify subsequent launches succeed.
11. **Restart reattachment** — start orchestrator, launch a host,
    kill the orchestrator without stopping the host, restart the
    orchestrator; verify it reattaches to the surviving host (or
    relaunches it) according to policy.
12. **Forge-detection** — inject a panic with a tampered chain of
    forwarders; verify rejection and forge-counter increment.

### Conformance Tests

13. **Audit log shape** — every supervision event produces a record
    matching the published JSON schema for AuditRecord.
14. **Registry shape** — registry.json matches the published schema.

### Coverage Target

`>=95%` line coverage. The orchestrator is the trust root for
package verification and the policy point for host lifecycle; bugs
here are systemic.

---

## Trade-Offs

**Persistent registry as a cache, not authority.** The supervisor's
child set is the source of truth for what is running. The registry
is a serialized cache that survives orchestrator restarts. This
means a crash during `launch_host` between "registry.add" and
"supervisor.start_child" leaves a registry entry that doesn't
correspond to a running process; the reconciliation logic at startup
detects this and either relaunches or removes.

**No automatic key rotation.** Long-term identity keys are rotated by
explicit user action (Tier 3). Automatic rotation would require a
secondary trust anchor (a backup key or attestation) that we do not
yet have a story for.

**Single orchestrator per machine.** v1 assumes one orchestrator
process per user account. Multi-orchestrator topologies (one for
work, one for personal) are achievable by isolating
`.orchestrator` directories and using different vault paths, but
the spec does not formalize them. Distributed orchestrators across
machines are explicitly future work.

**Tree-wide quarantine is brutal.** When the global panic threshold
is exceeded, all new spawns and pipeline wirings stop until a human
clears it. False positives during deployment would block real work.
We accept this: a panic storm is a strong signal, and pausing for
a human is the right default.

**Vault is a sibling, not a child.** The vault process is supervised
*peer to* the host processes, not under the orchestrator's manifest.
This means the orchestrator cannot kill the vault except via the
explicit `shutdown` flow; supervisor restart strategies for the
vault are configured separately. The reason: if the orchestrator
kills the vault (deliberately or by bug), every host loses access
to its leases, and the system enters cascading failure. Better to
treat the vault as ground truth.

**Two-channel bootstrap (pipe fd plus stdout reply).** A single
bidirectional channel from spawn would be simpler. We use a
write-only pipe for bootstrap and stdout for the X3DH reply because
the stdout is the natural transport for the post-handshake channel
anyway, and the bootstrap pipe is intended to be closed-and-forgotten
after one read.

---

## Future Extensions

- **Cross-machine orchestrators.** A federation of orchestrators
  on different machines, each owning a subtree, with the
  panic-broadcast network spanning the federation.
- **Automatic key rotation** with attestation-based recovery.
- **Hot configuration reload** without restart.
- **Live tier upgrade** for an already-running host (restart-free)
  with a per-call Tier 3 challenge.
- **Quarantine policies** per subtree (today the quarantine is
  global; a future version may quarantine just one branch).

These are deliberately out of scope for V1.
