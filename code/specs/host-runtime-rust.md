# Host Runtime (Rust, Tier 1)

## Overview

The Host Runtime is the OS process that the orchestrator spawns for
each agent. It is the **personal security escort** in the
congressional analogy: it accompanies one staffer (the agent), reads
its badge (manifest), opens doors on its behalf, fetches what it asks
for, and logs every room entered. It does this for exactly one agent
and one channel; it never crosses staffers, never shares state with
its peers, and never trusts the orchestrator that spawned it.

This spec defines the **Tier 1 (native Rust)** runtime — the host
binary used when the agent code itself is written in Rust and
compiled into the same process. It is the simplest tier and the one
we will build first. Tier 2 (WebAssembly inside the host) and
Tier 3 (a separate subprocess running Deno / Python / Ruby / Elixir)
share the same architecture but with different inner agent
execution; their specs build on this one.

The host runtime's responsibilities are:

1. **Bootstrap** the secure channel to the orchestrator (child side
   of X3DH).
2. **Verify** the agent package signature a second time (does not
   trust the orchestrator's verification).
3. **Load** the agent manifest and construct the capability cage.
4. **Build** the in-process actor supervision tree with the agent's
   declared structure.
5. **Implement** the host.* protocol — every method an agent can
   call corresponds to a handler that checks the manifest, runs
   middleware, performs the operation (often via an ephemeral
   sub-agent), and returns the result over the secure channel.
6. **Forward** lifecycle events and panic signals to the orchestrator.

The host runtime is the **third-most-trusted process in the system**,
after the vault and the orchestrator. A bug here lets a single
agent escape its own cage; it does not let an agent escape into other
agents' data. The independent vault, the per-host channel keys, and
the orchestrator's process supervision contain the blast radius.

---

## Where It Fits

```
   Orchestrator (parent OS process, supervisor of this host)
        │
        │  spawns; owns orchestrator-side of secure channel
        ▼
   Host Runtime (this spec)        ← runs as one OS process per agent
   ┌──────────────────────────────────────────────────────────────┐
   │  channel bootstrap (child side of X3DH)                       │
   │  signature re-verification                                    │
   │  manifest loader (capability-cage-rust)                       │
   │  in-process actor supervisor (supervisor crate)               │
   │  host.* protocol server                                       │
   │     ├── network.* dispatcher                                  │
   │     ├── fs.*      dispatcher                                  │
   │     ├── proc.*    dispatcher                                  │
   │     ├── vault.*   dispatcher  (forwards to vault host channel)│
   │     ├── channel.* dispatcher  (forwards to peer hosts)        │
   │     └── system.*  dispatcher                                  │
   │  ephemeral sub-agent pool                                     │
   │  middleware chain (cap check + tier + rate limit + audit)     │
   │  audit forwarder                                              │
   │  panic detector  → emits PanicSignal upward                   │
   └──────────────────────────────────────────────────────────────┘
        │
        │  hosts the agent's actor tree
        ▼
   Agent Code (native Rust)
        │  uses host.* SDK (in-process function calls)
        │  cannot import std::fs / std::net / std::process directly
        │  every OS access flows through host.* dispatchers
        ▼
   ─── (no further outward access) ───
```

**Depends on:**
- `supervisor` — the host runtime IS an in-process supervisor.
- `capability-cage-rust` — manifest loader, secure wrappers, audit
  envelope, the `Backend` trait for in-process delegation.
- `secure-host-channel` — child-side bootstrap and message I/O.
- `actor` — channels, mailboxes, message types.
- `vault-secure-channel` — the cryptographic primitives (transitively
  via `secure-host-channel`).
- `json-parser`, `json-value`, `json-serializer` — host.* message
  parse/encode.
- `process-manager` — for ephemeral sub-agents that need to spawn
  external commands (subject to manifest).
- `time` — monotonic clock.

**Used by:**
- The orchestrator (spawns instances of this binary).
- The agent code (links against this crate as a library and exposes
  its `run()` entrypoint).

---

## Design Principles

1. **One host, one agent, one channel.** A host runtime instance
   serves exactly one agent, holds exactly one secure channel to the
   orchestrator, and never multiplexes. Multiple agents = multiple
   host processes.

2. **Re-verify everything.** The host does not trust the orchestrator
   to have verified the package's signature, the manifest's
   well-formedness, or the capability bounds. Every check the
   orchestrator does, the host does again with its own logic.

3. **Manifest is enforcement, not policy.** The agent's manifest
   bounds what the host can possibly do for it. The host adds no
   policy of its own; it only refuses requests outside the manifest.
   Tier escalation, rate limits, and middleware are configured from
   the manifest, not from host code.

4. **Symmetric cage.** The host enforces the cage on the agent's
   side (R1b in the defense-in-depth model). The orchestrator's
   policies and the OS sandbox enforce the cage on the host's side
   (R3 + R4). The host trusts neither and is trusted by neither.

5. **Ephemeral sub-agents for OS-touching work.** Network, file,
   and process operations spawn an actor that performs exactly one
   request and dies. No long-lived OS handles inside the host
   process; no connection pools that survive a single request.

6. **Crash recovery is the orchestrator's job.** If the host
   crashes, it does so loudly: no panic recovery, no `catch_unwind`,
   no silent restart loops. The orchestrator's supervisor strategy
   decides what happens next.

---

## Key Concepts

### Channel Bootstrap (Child Side)

When the orchestrator spawns the host runtime, it passes:

- `argv[1]` — path to the signed agent package directory
- `env CHANNEL_BOOTSTRAP_FD=3` — file descriptor of the bootstrap pipe
- `fd 3` — readable pipe carrying the X3DH bundle and session metadata
- `fd 0/1/2` — stdin/stdout/stderr (stdout is the channel's transport
  for the post-handshake stream)

The host runtime's bootstrap sequence:

```
1. Read CHANNEL_BOOTSTRAP_FD from env. Validate it is "3".
2. Read the entire bootstrap blob from fd 3 (single read; pipe is
   closed by the parent immediately after writing).
3. Parse the JSON:
   { protocol_version  : "1.0",
     orch_prekey_bundle: { OIK_pub, OEK_pub, signature },
     child_session_id  : <uuid>,
     channel_aad_prefix: "host://<host_name>/<session_id>" }
4. Verify the orchestrator's signature on OEK_pub against OIK_pub.
   On failure → exit 1; the parent will see the failure as immediate
   child death. (We do not try to recover; the orchestrator may have
   been compromised between bundle generation and our read.)
5. Generate child identity key (CIK) and child ephemeral key (CEK).
6. Construct ChannelInitiator with our CIK and the orchestrator's
   bundle; produce the first wire message containing CEK_pub plus
   the X3DH-derived first chain ciphertext.
7. Write the first wire message to stdout (fd 1) using the
   length-prefixed format from secure-host-channel.md.
8. Subsequent messages use the established ratcheted channel.
9. CIK lives in this process's memory only. It is never written to
   disk or shared. It is zeroized on host shutdown.
```

The host's identity key CIK is **per-spawn** for Tier 1. Unlike the
orchestrator, the host has no long-term identity to persist. If the
host is restarted, a new CIK is generated and a fresh channel is
bootstrapped.

### Signature Re-Verification

Before reading the agent's manifest, the host:

1. Reads `argv[1]/PUBKEY_ID`.
2. Reads the trusted-keys file from a path the orchestrator gave it
   in the bootstrap (the host has no vault access of its own; the
   orchestrator passes the relevant trusted public keys as part of
   the bootstrap blob, signed by the orchestrator's identity key).
3. Computes the SHA-256 hash of the rest of the package using the
   same deterministic file-set ordering D18 specifies.
4. Verifies the package's Ed25519 SIGNATURE against the hash with
   the trusted public key.
5. On failure → exit 2 with a structured error sent to the
   orchestrator over the channel before exit (so the orchestrator
   logs the reason).

This verification is **independent** of the orchestrator's
verification. If the package on disk has been tampered with between
the orchestrator's check and ours, we catch it. If the
orchestrator's logic has a bug, ours catches the bug.

### Manifest Loader and Capability Cage

After signature verification:

```rust
let manifest_json = std::fs::read_to_string(
    package_path.join("manifest.json"))?;
let manifest = capability_cage::Manifest::load_from_str(&manifest_json)?;
```

The manifest is now the source of truth for every capability check
the host will perform. A copy is sent to the orchestrator over the
channel during the post-bootstrap handshake (so the orchestrator
knows what the host has registered as its policy), but the host's
own copy is what enforces every call.

The host installs `HostInternalBackend` as the capability-cage
backend for in-process actors. This backend implements the
`Backend` trait by calling the host's own dispatcher functions
(which themselves perform the manifest check, then call the OS).
Result: every secure call from agent code goes through two manifest
checks — once via the cage's `Manifest::check`, once via the
dispatcher — before reaching the OS.

### In-Process Supervision Tree

The host runtime is itself a supervisor (root of the *in-process*
tree, distinct from the *OS-process* tree the orchestrator runs).
At startup it constructs the agent's supervision tree from the
manifest and any auxiliary `agent.toml` configuration:

```
HostRuntime (root in-process supervisor)
├── HostProtocolServer        (always present, talks to channel)
├── EphemeralSubAgentPool      (spawns ON DEMAND for OS work)
├── AuditForwarder            (always present, forwards audit lines)
└── Agent's own actor tree     (declared by the agent code)
    ├── ...
    └── ...
```

The host runtime sets sensible defaults for the always-present
actors (`Permanent` restart policy, `Graceful(2s)` shutdown).
The agent code defines its own actors via the standard supervisor
API; the host runtime registers them under itself.

### Host Protocol Server

A single-actor dispatcher that:

1. Reads decrypted Host Protocol messages from the inbound side of
   the secure channel.
2. Looks up the method name in a static dispatch table:
   ```
   "network.fetch"        → handlers::network_fetch
   "fs.read"              → handlers::fs_read
   "vault.requestLease"   → handlers::vault_request_lease
   "channel.read"         → handlers::channel_read
   "system.now"           → handlers::system_now
   ...
   ```
3. Invokes the handler, which:
   a. Validates the params against the method's JSON schema.
   b. Runs the middleware chain (capability check, tier check,
      rate limit, audit).
   c. Spawns an ephemeral sub-agent if the operation requires
      OS access; awaits its result.
   d. Constructs the result or error.
4. Sends the response over the secure channel with the same `id`.

The dispatcher is single-threaded by default. Concurrent requests
are processed serially; the agent code is expected to not depend
on out-of-order responses for v1. A future revision may add
per-request worker tasks if real workloads require it.

### Middleware Chain

Every dispatched method goes through this chain in order:

```
1. Capability check (R1b)
   manifest.check(category, action, target)?
   On denial → return CapabilityDenied error immediately.

2. Tier check
   if method.tier > 0:
     send TierChallenge to orchestrator over the channel
     await response with timeout = tier-specific
     On denial → return TrustBoundaryDenied.

3. Rate limit
   if !rate_limiter.try_acquire(method.cost):
     return RateLimited error.

4. Audit (open envelope)
   audit_record_start(name=method, props={url, path, ...})

5. Dispatch
   actual handler runs; spawns ephemeral sub-agent if needed.

6. Audit (close envelope)
   audit_record_end(success/failure, elapsed, error_kind)

7. Response
   serialize result/error; send over secure channel.
```

The middleware chain is the same for every method; only the
dispatch step differs. This uniformity makes the host easy to audit
and easy to extend (new methods plug into the same chain).

### Ephemeral Sub-Agents

Any method that touches the OS spawns a sub-agent that performs the
work and dies. The sub-agent is itself a tiny actor:

```rust
pub struct NetworkProxyAgent {
    target_host:  String,
    target_port:  u16,
    /* ... */
}

impl Actor for NetworkProxyAgent {
    fn handle(&mut self, msg: Message, ctx: &mut Context) -> ActorResult {
        // open socket to target_host:target_port
        // perform request
        // emit result on the reply channel
        // return Stop in the result
    }
}
```

The sub-agent:
- Is created with **only** the parameters it needs for one request
  (the URL, the path, the bytes — never broader handles).
- Has its own narrow capability cage: a manifest with exactly the
  one capability needed for this one operation.
- Lives in milliseconds. By the time an attacker could exploit it,
  it no longer exists.
- Never holds OS handles across requests. No connection pools, no
  cached file descriptors.

The sub-agent pool is a `DynamicSupervisor` (per `supervisor.md`)
under the host runtime root. Sub-agents are `Restart::Temporary`:
if they die, they stay dead, and the dispatcher returns an error
to the agent.

### Vault Forwarding

The host has no vault access of its own. Every `vault.*` request
is forwarded over a separate, pre-wired channel from the host to
the vault process. The vault performs its own manifest check (the
host's manifest, signed by the orchestrator and presented at vault
channel setup), then either:

- Returns a lease (lease mode) — the host forwards it to the agent.
- Sends the secret directly to a peer host (direct mode) — the
  host receives only an acknowledgment.

The host never sees direct-mode secrets in plaintext. The vault
is a sibling, not a child; it has its own supervisor, its own
secure channel, and its own crash policy.

### Channel Forwarding

Channels in the agent's pipeline (per D18 pipeline composition) are
pre-wired by the orchestrator at host launch time. The host knows
which channels it can read and which it can write to; the agent
calls `channel.read` / `channel.write` and the host validates the
agent is the registered originator/receiver before forwarding.

The channels themselves are `actor::Channel` instances — encrypted,
append-only logs with offset tracking — described in D19.

### Audit Forwarder

Every audit record produced by the capability-cage envelope is
written to a bounded in-memory queue. A dedicated `AuditForwarder`
actor drains this queue and sends batches over the secure channel
on a low-priority namespace (`_internal.audit`). The orchestrator
relays audit records to its own audit log.

If the secure channel is unavailable (rare; happens only during
re-handshake), audit records spool to memory up to a cap (10 MiB
default), then drop the oldest. Dropped records are themselves
audit-recorded.

### Panic Detector

The host runs a small detector that watches for the conditions
defined in `secure-host-channel.md` (circuit-breaker fires, AEAD
failures, replay storms, mailbox saturation, scheduling latency
spikes) and emits `PanicSignal` notifications upward over the
control-priority channel. The signal carries the host's own ID as
`origin` — if the orchestrator forwards the signal further, the
chain is preserved.

### Shutdown

When the orchestrator sends a `Terminate` notification (or the
secure channel is severed), the host runtime:

1. Stops accepting new host.* calls.
2. Cancels all pending sub-agents with `Cancelled` errors returned
   to the agent.
3. Lets the agent's actor tree wind down per its own shutdown
   policies (graceful with the configured timeout).
4. Flushes the audit queue.
5. Zeroizes channel keys.
6. Exits with code 0 if shutdown was clean, else with a code
   indicating the abnormal reason.

---

## Public API

### As a binary

```
host-runtime <package_path>
   env: CHANNEL_BOOTSTRAP_FD=<fd>
   fd <fd>: read end of orchestrator's bootstrap pipe
   fd 0/1/2: stdin/stdout/stderr (stdout is the channel transport)
```

### As a Rust library (linked by Tier 1 agents)

Tier 1 agents are statically linked with this crate and call its
`run()` from their `main()`:

```rust
use host_runtime_rust::{HostRuntime, AgentEntrypoint};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    HostRuntime::run(AgentEntrypoint {
        package_path: std::env::args().nth(1).unwrap().into(),
        boot_agent:   Box::new(|host| {
            // The agent's own actor tree:
            host.start_child(ChildSpec {
                id:       ChildId::new("weather-rule-engine"),
                kind:     ChildKind::Worker,
                start:    Box::new(|| start_weather_rule_engine()),
                restart:  Restart::Permanent,
                ..Default::default()
            })?;
            Ok(())
        }),
    })
}
```

The `boot_agent` closure runs after the channel is established and
the supervisor tree's defaults are in place. Agent code uses the
host's SDK functions (described next) to call host.* methods.

### Agent SDK (in-process function calls for Tier 1)

```rust
pub mod host {
    /// Manifest the agent code is bound by. Loaded from the package's
    /// required_capabilities.json at startup. Read-only.
    pub fn manifest() -> &'static Manifest;

    pub mod network {
        pub fn fetch(url: &str, opts: FetchOpts) -> Result<Response, HostError>;
    }

    pub mod fs {
        pub fn read(path: &Path) -> Result<Vec<u8>, HostError>;
        pub fn write(path: &Path, data: &[u8]) -> Result<(), HostError>;
        // ...
    }

    pub mod vault {
        pub fn request_lease(name: &str, ttl: Duration)
            -> Result<Lease, HostError>;
        pub fn request_direct(name: &str, consumer: AgentId)
            -> Result<(), HostError>;
    }

    pub mod channel {
        pub fn read(id: &ChannelId) -> Result<Option<Message>, HostError>;
        pub fn write(id: &ChannelId, payload: &[u8]) -> Result<(), HostError>;
        pub fn ack(id: &ChannelId, msg_id: &MessageId) -> Result<(), HostError>;
    }

    pub mod system {
        pub fn now() -> u64;             // monotonic ns
        pub fn unix_time() -> u64;       // ms since epoch
        pub fn random_bytes(n: usize) -> Vec<u8>;
        pub fn log(level: LogLevel, message: &str);
    }
}

pub enum HostError {
    CapabilityDenied   { requested: Capability, reason: String },
    TrustBoundaryDenied{ tier:      PrivilegeTier },
    RateLimited        { retry_after: Duration },
    NotFound           { kind:      ResourceKind },
    Expired            { lease:     LeaseId },
    Cancelled,
    Timeout,
    Conflict           { message:   String },
    Upstream           { source:    Box<dyn std::error::Error + Send + Sync> },
}
```

For Tier 1, `host::network::fetch(url, opts)` is implemented as a
direct in-process call into the `HostProtocolServer` (no
serialization, no transport hop). The middleware chain still runs.
The same call from a Tier 2 (WASM) or Tier 3 (subprocess) agent
takes the same path through the chain after JSON deserialization.

---

## Test Strategy

### Unit Tests

1. **Bootstrap parsing.** Valid blob → parsed. Missing fields →
   error. Tampered signature → reject.
2. **Signature re-verification.** Valid package + trusted key →
   accept. Tampered file → reject. Unknown key → reject.
3. **Middleware chain.** Each step's denial is observable in the
   audit record; the chain short-circuits at the first denial.
4. **Dispatcher.** Each method routes to its handler; unknown
   method returns `MethodNotFound`.
5. **Ephemeral sub-agent lifecycle.** Spawned per request; dies
   after one response; no surviving handles.

### Integration Tests

6. **End-to-end host.network.fetch.** Spawn a real host runtime
   pointed at a test orchestrator harness; from agent code call
   `host::network::fetch("http://test.local/...")`; verify the
   request reaches a mock upstream and the response is decoded
   correctly.
7. **CapabilityDenied.** Same flow but the manifest does not
   include `net:connect:test.local:80`; verify the call is denied
   before reaching the network and the audit record reflects the
   denial.
8. **Vault forwarding.** From agent code call
   `host::vault::request_lease("foo", 60s)`; verify the request
   reaches the vault host (mocked), the lease is returned, and the
   agent receives it.
9. **Channel forwarding.** Two host runtimes, one channel between
   them; one writes, the other reads; verify in-order delivery and
   offset tracking.
10. **Crash on bad bootstrap.** Inject a bootstrap blob with a
    tampered orchestrator signature; verify the host exits with
    code 1 and emits no host.* messages.
11. **Graceful shutdown.** Orchestrator sends Terminate; verify
    pending sub-agents are cancelled, audit is flushed, channel
    keys are zeroized, exit code is 0.
12. **Panic detection.** Inject AEAD failures via a fault-injecting
    transport; verify a PanicSignal is emitted upward within 100 ms.

### Coverage Target

`>=95%` line coverage. The host runtime is the in-process security
boundary for every agent; bugs here let one agent escape its cage.

---

## Trade-Offs

**Single-threaded dispatcher.** Concurrent host.* calls from one
agent are processed serially. Real workloads (LLM call latency
dominates by ~3 orders of magnitude over dispatch) make this fine
for v1. A future version may add per-request task spawning if
high-throughput agents demand it.

**No panic recovery.** A panic in the host runtime crashes the
process. The orchestrator restarts it per its supervisor policy.
This is intentional: silent panic-and-continue can leave the host
in a corrupted state that violates its security invariants. Loud
crashes are safer than quiet bugs.

**Per-spawn child identity key.** The host has no long-term
identity. Every restart is a fresh CIK. This means audit logs
that correlate a host across restarts must do so via the host
name and package hash, not via key identity. We accept the
loss because long-term per-host keys would need to be persisted
(more attack surface) and rotated on restart anyway (no
practical security gain).

**Ephemeral sub-agents have spawn overhead.** Each network /
fs / proc call spawns a sub-actor. In-process this is microseconds.
For agents that issue thousands of small requests per second this
might dominate; for agents that issue one request per second
(typical AI agent rate) it is invisible. We optimize for the
common case.

**Audit spool is bounded.** If the audit forwarder cannot drain
to the channel (channel down, orchestrator slow), audit records
spool to a 10 MiB in-memory cap and then drop the oldest. Drops
are themselves recorded so a forensic reviewer sees the gap. The
alternative (block on audit queue full) would let a stuck
audit channel halt the agent, which is worse.

**Vault is a separate process, not in-host.** Every vault.* call
crosses a process boundary. The latency is microseconds (Unix
socket / named pipe), but it is not zero. The benefit — the host
cannot read vault state via memory bugs — is worth it.

**Tier 1 agents are statically linked.** A Tier 1 agent and its
host runtime are one binary. Updating the host runtime means
recompiling every Tier 1 agent. We accept this for v1 because
Tier 1 is reserved for first-party agents we ship; later, a
dynamic-linking story (or moving these agents to Tier 2 WASM)
removes the coupling.

---

## Future Extensions

- **Concurrent dispatch** with per-request tasks for high-throughput
  agents.
- **Pluggable middleware** so first-party hosts can install
  domain-specific checks (e.g., a coding-agent host that adds a
  "no destructive Bash without explicit approval" middleware).
- **Hot reload** of the agent's code via dynamic linking, without
  killing the host process. Requires solving the "how does the
  manifest update affect in-flight calls" question.
- **In-process Tier 2 (WASM) hosting** so a single host runtime
  can switch between Tier 1 and Tier 2 agents at runtime.

These are deliberately out of scope for V1.
