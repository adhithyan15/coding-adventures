# Agent Discovery

## Overview

Agent Discovery is **the only way** an agent can establish a
channel to another agent in this system. Per
`dynamic-topology.md`, there are no predefined channels: every
agent-to-agent connection is established at runtime through
this API. The orchestrator owns the discovery, the bridge
creation, and the lifecycle of the providers being discovered.

The pattern in plain words:

> "I'm a coding agent. I need a file-reader for the workspace
> directory. Orchestrator, find me one and put a channel between
> us."

The orchestrator looks up which agents declare they provide
that role (spawning one if necessary; see Spawn-On-Discovery
below), applies authorization (does the requester's manifest
permit discovering this kind of peer? does the provider permit
being discovered by this kind of requester?), creates two new
ratcheted channels via `secure-host-channel`, hands one endpoint
to each agent, and goes back to its supervisory job.

This is the agent-system equivalent of Erlang's `:global` or
Kubernetes service discovery, but with four properties those
systems don't have:

1. **Capability-cage gated on both sides.** Discovery is itself a
   capability declared in the manifest. An agent can only
   discover peers it has explicitly declared a need for. A peer
   can only be discovered if it has explicitly declared the
   capability it provides.

2. **RWS analysis applied at bridge time.** When the orchestrator
   bridges A to B, it runs the read/write-separation analysis
   against the new channel topology. If the bridge would
   produce an agent that has both untrusted-input reads and
   actuation writes, the bridge is refused.

3. **Schema-pinning supported per discovered channel.** A
   provider declares the schema of what it emits; a consumer
   declares the schema it expects; the orchestrator verifies
   compatibility (and, when both sides agree, marks the channel
   as `trust_laundering: true` so RWS treats it as a trust
   boundary).

4. **Provider lifecycle is orchestrator-managed.** Discovery
   triggers spawn when no provider is active. Idle providers
   are gracefully shut down. Pool size scales with load. See
   the Provider Lifecycle section.

For the v1 PoC, the weather agent uses discovery for every
channel including those between sibling sub-agents. The
`weather-agent` parent declares its three children via
`spawn_children` (the only thing that can be predefined per
`dynamic-topology.md`); the children find each other at
runtime via this API.

---

## Where It Fits

```
   Agent A (requester)
        │
        │  host.discovery.find_and_connect("file-reader",
        │                                   { workspace: "..." })
        ▼
   host-runtime-rust (Tier 1 SDK)
        │
        │  forwarded as a host.* RPC
        ▼
   Orchestrator
        │
        │  1. Check requester's manifest: discover:role:file-reader?
        │  2. Look up active providers of "file-reader"
        │  3. Check provider's manifest: providable to A?
        │  4. RWS analysis on the new bridged topology
        │  5. Create two new ratcheted channels
        │  6. Hand one endpoint to A's host, one to B's host
        │  7. Return channel-ids to both agents
        ▼
   Agent A and Agent B can now talk via the new channel(s)
```

**Depends on:**
- `orchestrator` — owns the agent-name → host-process registry,
  resolves the discovery query, performs the bridging.
- `supervisor` — the orchestrator is a top-level supervisor;
  bridging is part of its supervisory operations.
- `secure-host-channel` — the new channels created by the bridge
  use the same X3DH+ratchet+DOS-protection machinery as every
  other channel.
- `capability-cage-rust` — gates `discover:role:*` and
  `provides:role:*` capabilities at manifest validation.
- `read-write-separation` — applied at bridge time to the
  combined topology.
- `host-protocol` — defines the new `discovery.*` methods
  (`find`, `connect`, `find_and_connect`, `disconnect`).

**Used by:**
- Future coding agent (finds file-reader, file-writer, command-
  runner peers on demand).
- Future smart-home controller (finds zigbee-coordinator,
  hue-bridge, mqtt-broker hosts).
- Future iPaaS workflow runtime (finds connectors per workflow
  step).
- The orchestrator CLI (`orchestrator discover` for inspection).

---

## Design Principles

1. **Orchestrator-mediated only.** Agents do not discover each
   other directly. Every discovery goes through the
   orchestrator, which is the only place with the full agent
   registry and the authority to create channels.

2. **Capability declarations on both sides.** A provider
   declares what it provides; a consumer declares what it may
   discover. Both sides must explicitly opt in. The
   orchestrator never bridges agents that haven't opted in.

3. **Discovery is bridging.** A successful discovery returns
   channel ids, not agent identities. The requester and
   provider may not even learn each other's names; they learn
   only that they have a channel to a peer that satisfies the
   query.

4. **Per-bridge channels.** Each bridge creates fresh channels
   keyed independently of every other channel. A compromised
   channel does not reveal others.

5. **RWS-checked at bridge time.** The same RWS rule from
   `read-write-separation.md` applies. If the bridge would
   produce a violating agent topology, the bridge is refused.

6. **Schema-pinning is opt-in trust laundering.** When both
   sides agree on a schema, the channel is marked
   `trust_laundering: true` and RWS treats its content as
   trusted (as long as the schema has no string-arm injection
   surface — same rule as for pre-wired channels).

7. **Quotas to prevent enumeration / DOS.** Each agent has a
   per-window cap on discovery calls and on outstanding bridges.
   The orchestrator's audit log records every discovery
   attempt, success or denial.

8. **Local to one orchestrator.** v1 discovery is in-process to
   one orchestrator on one machine. Cross-machine federation
   (so agent A on host X can discover provider B on host Y) is
   explicit future work.

---

## Roles vs. Agent IDs

Agents can be discovered in two distinct ways:

### By role

A **role** is a logical name for a capability some agent provides
(e.g., `"file-reader"`, `"file-writer"`, `"oauth-broker:google"`,
`"zigbee-coordinator"`). Multiple agents may provide the same
role; the orchestrator picks one per discovery call.

The role is a **string** in a flat namespace, conventionally
`<verb>` or `<verb>:<qualifier>`. A provider declares roles in
its manifest:

```json
{
  "provides": [
    {
      "role":          "file-reader",
      "qualifier":     { "workspace": "/home/user/projects/*" },
      "schema_emit":   "schemas/file-reader-emit.schema.json",
      "schema_accept": "schemas/file-reader-accept.schema.json",
      "max_concurrent": 4,
      "trust_laundering": true,
      "justification": "Reads files inside the workspace dir on request"
    }
  ]
}
```

The `qualifier` is a JSON object that further narrows the role.
Discovery queries can match against the qualifier — e.g., a
requester looking for `file-reader` with workspace
`/home/user/projects/myapp` matches this provider because the
provider's qualifier glob covers it.

### By agent ID

Sometimes the requester knows the exact agent it wants to talk
to (e.g., "the vault" — there is exactly one). It uses the
agent's stable name from `agent-registry.md`:

```rust
host::discovery::find_and_connect_by_id("vault", QueryOpts::default())?;
```

ID-based discovery is mostly for singletons (vault, audit-sink,
panic-broadcast root). Role-based discovery is the normal case.

The orchestrator authorizes both forms identically: the
requester's manifest must include either `discover:id:<name>` or
`discover:role:<role>` for the lookup to succeed.

### Why not just ID?

Two reasons:

1. **Multi-instance.** A coding agent might want to talk to
   "any file-reader for the current workspace" without caring
   about its specific name. Role-based discovery permits this.

2. **Replaceable providers.** If `file-reader-v1.agent` is
   revoked and `file-reader-v2.agent` registered, role-based
   consumers transparently switch over. ID-based consumers
   would break.

Both are useful; both are supported.

---

## Capability Declarations

### Provider side: `provides`

In the provider's manifest:

```json
{
  "version": 1,
  "package": "rust/file-reader-host",
  "capabilities": [
    {
      "category":      "fs",
      "action":        "read",
      "target":        "/home/user/projects/**/*",
      "trust":         "untrusted",
      "justification": "Read source code on request"
    }
  ],
  "provides": [
    {
      "role":            "file-reader",
      "qualifier":       { "workspace": "/home/user/projects/*" },
      "schema_emit":     "schemas/file-reader-emit.schema.json",
      "schema_accept":   "schemas/file-reader-accept.schema.json",
      "trust_laundering": true,
      "max_concurrent":  4,
      "discoverable_by": [
        { "role": "coding-agent" }
      ],
      "justification":   "Provide read-only file access to coding agents inside workspace"
    }
  ]
}
```

The `discoverable_by` field is a constraint list: only
requesters whose manifest declares `provides:role:coding-agent`
themselves (i.e., they are coding agents) may discover this
provider. Empty list means "any agent with the matching
`discover:` capability."

### Consumer side: `discover`

In the consumer's manifest:

```json
{
  "version": 1,
  "package": "rust/coding-agent",
  "capabilities": [
    /* ... */
  ],
  "provides": [
    {
      "role":            "coding-agent",
      "discoverable_by": [],
      "justification":   "Identify as a coding agent for peers' discoverable_by checks"
    }
  ],
  "discover": [
    {
      "role":            "file-reader",
      "qualifier_query": { "workspace": "/home/user/projects/myapp" },
      "schema_accept":   "schemas/file-reader-emit.schema.json",
      "schema_emit":     "schemas/file-reader-accept.schema.json",
      "max_outstanding": 1,
      "justification":   "Need to read source files of the user's project"
    }
  ]
}
```

The schema fields cross-validate: the consumer's
`schema_accept` must match (or be a strict superset of) the
provider's `schema_emit`, and vice versa. The orchestrator
checks compatibility at bridge time and refuses if they don't
match.

### Capability-cage interpretation

The cage extends its taxonomy with two new (category, action)
pairs:

```
discover    role     <role-name>     - "may discover providers of <role>"
discover    id       <agent-id>      - "may discover by exact id"
provide     role     <role-name>     - "may register as provider of <role>"
```

The `discover` and `provide` capabilities are validated at
manifest load time alongside the existing taxonomy. The cage's
RWS classifier treats them as **internal** by default
(neither untrusted-input nor actuation), but the bridge-time
RWS analysis re-evaluates the topology that results from the
bridge.

---

## The Discovery API

Three new methods in `host.discovery.*`:

```typescript
// Look up matching providers without connecting yet.
host.discovery.find(query: DiscoveryQuery) -> FindResult

// Connect to one of the providers returned by find().
host.discovery.connect(provider_handle: ProviderHandle) -> ConnectResult

// Combined: find one provider and connect in one call.
host.discovery.find_and_connect(query: DiscoveryQuery) -> ConnectResult

// Tear down a previously-established discovery channel.
host.discovery.disconnect(channel_id: ChannelId) -> ()
```

### `DiscoveryQuery`

```rust
pub enum DiscoveryQuery {
    /// Find providers of a specific role.
    Role {
        role:           String,
        qualifier:      JsonValue,                  // matched against provider's qualifier
        prefer_local:   bool,                       // hint, not authoritative
    },
    /// Find an agent by stable id (singleton lookup).
    Id {
        agent_id:       String,
    },
}
```

### `FindResult`

```rust
pub struct FindResult {
    pub providers:  Vec<ProviderHandle>,
}

pub struct ProviderHandle {
    /// Opaque token; do not parse.
    pub handle:                    [u8; 32],

    /// Public summary the requester is allowed to see.
    /// Does NOT include the provider's agent_id by default;
    /// see "Identity disclosure" below.
    pub role:                      String,
    pub qualifier_summary:         JsonValue,
    pub schema_emit_hash:          [u8; 32],
    pub schema_accept_hash:        [u8; 32],
    pub trust_laundering:          bool,
    pub current_load:              u32,            // active connections
    pub max_concurrent:            u32,
}
```

The handle is bound to the requester's session and expires after
30 seconds (configurable). It is the only thing the requester
can pass to `connect`.

### `ConnectResult`

```rust
pub struct ConnectResult {
    /// Channel id for messages from requester → provider.
    pub outbound_channel:  ChannelId,
    /// Channel id for messages from provider → requester.
    pub inbound_channel:   ChannelId,
    /// True if the bridge was schema-pinned (trust-laundered).
    pub trust_laundered:   bool,
    /// Agent the orchestrator chose to fulfill the request.
    /// Disclosure subject to the provider's `disclose_identity`
    /// setting; defaults to false.
    pub provider_id:       Option<String>,
}
```

### `disconnect`

Tears down both channels of a bridge. Idempotent. Either side
can call it. The orchestrator notifies the other side via a
control-priority message.

---

## Bridging Sequence

When `find_and_connect` is invoked on the orchestrator:

```
1. Validate requester's manifest:
   - discover:role:<role> (or discover:id:<id>) exists
   - schema_accept_hash matches the provider's schema_emit_hash
   - schema_emit_hash matches the provider's schema_accept_hash
   On any mismatch → CapabilityDenied / SchemaMismatch.

2. Resolve providers:
   - List active hosts whose manifests declare provides:role:<role>
   - Filter by qualifier match (provider's qualifier covers
     requester's qualifier_query)
   - Filter by discoverable_by (requester must satisfy provider's
     allowed-roles list)
   - Filter by current_load < max_concurrent
   - If empty → NoProvidersFound.
   - Apply selection policy (default: round-robin among matches;
     prefer_local hint can override if there is a local-vs-remote
     distinction in the future).

3. RWS analysis on the post-bridge topology:
   - Compute requester's effective inputs/outputs after gaining
     the new inbound channel and the new outbound channel.
   - Compute provider's effective inputs/outputs after gaining
     the new outbound and new inbound channel.
   - If trust_laundered (matching schemas), treat the channels
     as trust-laundering boundaries (per RWS spec).
   - If RWS would be violated on either side after the bridge,
     refuse with RwsViolation.

4. Per-agent quota check:
   - Requester's outstanding-bridges count + 1 ≤ requester's
     max_outstanding (per their manifest's discover entry).
   - Provider's current_load + 1 ≤ provider's max_concurrent.
   - On violation → BridgeQuotaExceeded.

5. Create two new ratcheted channels:
   - outbound: originator=requester, receiver=provider
   - inbound:  originator=provider, receiver=requester
   - Both use secure-host-channel's X3DH+ratchet machinery,
     just like every other channel in the system.
   - The channels are scoped to the orchestrator (not under
     either host's supervisor scope), so the orchestrator can
     reclaim them on either-side termination.

6. Push channel handles to both sides:
   - Send a control-priority "BridgeEstablished" message to
     each host's secure channel with the new channel ids.
   - The receiving host runtime registers the new channels
     in its in-process channel table.

7. Update the orchestrator's bridge ledger:
   - Record { bridge_id, requester_id, provider_id, outbound,
              inbound, schema_emit_hash, schema_accept_hash,
              created_at, trust_laundered }
   - Append to the persistent registry on disk.

8. Audit-log:
   - { kind: "bridge.established", requester, provider, role,
       qualifier, trust_laundered, outbound_channel,
       inbound_channel, ts }

9. Return ConnectResult to the requester.
```

The provider does not need to do anything proactive; the
control-priority message arrives on its existing channel and
the host runtime handles registration transparently. The
provider sees a new inbound channel ready for reads.

If the provider terminates, the orchestrator reclaims both
channels (sends close to the requester) and the bridge entry
is removed.

---

## Provider Lifecycle

Per `dynamic-topology.md`, the orchestrator manages the
lifecycle of every discoverable provider: when to spawn,
when to shut down idle, how many to keep alive. The semantics
below are the contract this spec implements.

### Spawn-On-Discovery

When `find_and_connect` (or `find`) is called for a role with
no currently-active provider:

```
1. Consult the agent-registry for any registered package whose
   manifest declares provides:role:<role>.
2. Filter by qualifier match against the requester's
   qualifier_query.
3. If no matching package exists in the registry → return
   NoProvidersFound.
4. Otherwise, select one (default: stable order by agent_id).
5. Launch via the orchestrator's normal launch_host path
   (registry hash pin, signature verify, manifest load,
   tier challenge if effective_tier > 0,
   secure-host-channel bootstrap).
6. Wait for the new host's handshake (default timeout: 10s).
7. Continue the bridging sequence from step 3 (the new
   provider is now active and counted toward
   providers_returned).
```

Tier challenges fire **synchronously** inside the requester's
discovery call. If the user denies, the consumer receives a
structured error (CapabilityDenied / TrustBoundaryDenied)
rather than a successful bridge.

### Idle-Shutdown

A provider's `provides` entry can declare:

```json
{
  "provides": [
    {
      "role":           "weather-snapshot-source",
      /* ... */
      "idle_shutdown":  "5m"
    }
  ]
}
```

When set, the orchestrator tracks the time since the
provider's last active bridge. When it exceeds
`idle_shutdown` AND no current bridges are outstanding, the
orchestrator gracefully terminates the provider. A
subsequent discovery for that role triggers
spawn-on-discovery again.

`idle_shutdown` is per-role, not per-package. A provider that
exports multiple roles is only shut down when **all** of its
roles have been idle past their thresholds.

`min_instances` (declared on the host's config in
`orchestrator.toml`, set per-package) overrides idle-shutdown:
if `min_instances >= 1`, the orchestrator will not reduce
below that count.

### Pool Sizing

```json
{
  "provides": [
    {
      "role":           "oauth-broker:google",
      "max_concurrent": 4,
      "min_instances":  0,
      "max_instances":  3,
      "idle_shutdown":  "5m"
    }
  ]
}
```

- **min_instances:** never let active count drop below this.
  If a provider crashes and active count drops below
  `min_instances`, the orchestrator spawns a replacement
  immediately (independent of any pending discovery).
- **max_instances:** never spawn more than this. When all
  instances are at `max_concurrent` and a new bridge would
  exceed pool capacity, the consumer receives
  `BridgeQuotaExceeded { which: PoolSaturated }` with a
  `retry_after`.
- **load-driven scaling:** when the average current_load
  across active instances exceeds 75% of `max_concurrent`,
  the orchestrator spawns one more instance (up to
  `max_instances`). When average drops below 25% and there is
  at least one instance with zero active bridges past
  `idle_shutdown`, that one is shut down (down to
  `min_instances`).

Default policy: `min_instances: 0`, `max_instances: 1`,
`idle_shutdown: never`. So out-of-the-box behavior is
"exactly one instance once you discover me, kept alive
forever" — matching the simplest possible operational model.
Opting in to richer lifecycle is a deliberate choice the
agent's author makes per role.

---

## Identity Disclosure

By default, the requester does not learn which agent the
orchestrator chose. This protects against enumeration attacks
and lets the orchestrator pool providers without revealing the
pool to consumers.

A provider can opt in to identity disclosure:

```json
{
  "provides": [
    {
      "role":               "vault",
      "disclose_identity":  true,    // tell requesters who I am
      "justification":      "There's only one vault; no point hiding"
    }
  ]
}
```

Even with disclosure, the provider's full manifest is not
shared — only its agent_id. This is enough for the requester
to know which audit records to read, not enough to enumerate
capabilities.

---

## RWS Treatment

When the orchestrator bridges A to B, it must consider the
post-bridge RWS of both:

### Without schema pinning

The new channels are treated as untrusted (the consumer reads
bytes the orchestrator did not validate the shape of). The RWS
classifier applies its normal rules:

- If A reads from new inbound channel (untrusted) and A has any
  actuation, A becomes RWS-violating after the bridge → refuse.
- Same for B.

This is why most useful bridges need schema pinning.

### With schema pinning

When both sides declare matching schemas (`schema_emit` of one =
`schema_accept` of the other) and the schemas are
restrictive (no string arms; only enums / numbers / fixed-shape
objects), the orchestrator marks the bridge `trust_laundered`.
The RWS classifier treats the channels as trust boundaries; an
agent reading from a trust-laundered channel does not become
transitively-untrusted.

The schema's restrictiveness is checked the same way as for
pre-wired channels (per RWS spec Phase 1: the orchestrator
trusts the author's declaration; Phase 2 will mechanically
verify no string arms).

---

## Quotas and Rate Limits

Per requester:
```
discover_calls_per_minute     30   (find / find_and_connect)
discover_calls_per_hour       300
outstanding_bridges            10  (per role; configurable per-role)
```

Per provider:
```
max_concurrent                 (declared in provides entry)
new_bridges_per_minute         60
new_bridges_per_hour           600
```

Per orchestrator (global):
```
total_bridges_active           1000
total_bridges_per_minute       100
```

Exceeding any quota returns a structured error:

```rust
pub enum DiscoveryError {
    CapabilityDenied { reason: String },
    NoProvidersFound,
    SchemaMismatch { detail: String },
    RwsViolation { detail: String },
    BridgeQuotaExceeded { which: QuotaKind, retry_after: Duration },
    UnknownProvider { handle: [u8; 32] },
    HandleExpired,
    OrchestratorBusy,
}

pub enum QuotaKind {
    DiscoverCallsPerMinute,
    DiscoverCallsPerHour,
    OutstandingBridges,
    ProviderConcurrent,
    ProviderNewBridgesPerMinute,
    OrchestratorTotalBridges,
    OrchestratorTotalBridgesPerMinute,
}
```

---

## Audit Records

Every discovery event produces an audit record:

```json
{
  "kind":          "discovery.find",
  "ts_ms":         ...,
  "requester":     "coding-agent",
  "query":         { "Role": { "role": "file-reader",
                                "qualifier": {...} } },
  "providers_returned": 2,
  "elapsed_ms":    3
}

{
  "kind":          "discovery.connect",
  "ts_ms":         ...,
  "requester":     "coding-agent",
  "provider":      "file-reader-host-v1",
  "role":          "file-reader",
  "outbound_channel": "...",
  "inbound_channel":  "...",
  "trust_laundered":  true,
  "elapsed_ms":    7
}

{
  "kind":          "discovery.disconnect",
  "ts_ms":         ...,
  "bridge_id":     "...",
  "initiated_by":  "coding-agent",
  "duration_ms":   12483
}
```

Refused requests are also recorded:

```json
{
  "kind":          "discovery.refused",
  "ts_ms":         ...,
  "requester":     "coding-agent",
  "reason":        "RwsViolation",
  "detail":        "..."
}
```

---

## CLI

```
orchestrator discover providers
    List all active providers (role, qualifier, current_load,
    discoverable_by) for inspection.

orchestrator discover bridges
    List active bridges (requester, provider, role, age,
    trust_laundered).

orchestrator discover audit-tail
    Stream discovery audit records live.
```

These are read-only; no Tier challenge required.

---

## Test Strategy

### Unit Tests

1. **Manifest validation.**
   - Provider with `provides:role:foo` accepted; without rejected
     when the role string is invalid.
   - Consumer with `discover:role:foo` accepted; without
     rejected.
   - Schemas missing on a `trust_laundering: true` provides
     entry → invalid manifest.
2. **Query matching.**
   - Role + qualifier match correctly with glob patterns.
   - `discoverable_by` filter applied correctly.
   - Selection policy (round-robin) selects fairly.
3. **Schema cross-validation.**
   - Compatible schemas accepted.
   - Incompatible schemas rejected with `SchemaMismatch`.
4. **Quota enforcement.**
   - Requester at quota receives `BridgeQuotaExceeded`.
   - Provider at `max_concurrent` is excluded from discovery
     results.
5. **Handle expiry.**
   - A `find` handle older than 30s returns `HandleExpired` on
     `connect`.

### Integration Tests

6. **End-to-end discovery.** Spawn a provider host and a
   consumer host; consumer calls `find_and_connect`; verify
   both get matching channel ids; verify a message sent on
   the outbound is received on the provider's inbound.
7. **RWS refusal.** Configure a bridge that would produce an
   RWS-violating consumer; verify `RwsViolation` and no
   channels created.
8. **Trust-laundered bridge.** Configure compatible schemas
   between an untrusted-input provider and an actuator
   consumer; verify the bridge is permitted because the
   schema-pinned channels launder the trust.
9. **Provider termination.** Establish a bridge; kill the
   provider host; verify the consumer receives a close
   notification on its inbound channel and a subsequent
   `discovery.connect` call to the same provider returns
   `NoProvidersFound`.
10. **Multiple providers, fair selection.** Three providers of
    the same role; 30 discovery calls; verify each provider
    receives ~10 (round-robin).

### Coverage Target

`>=90%` line coverage on the discovery logic. RWS analysis
already has its own coverage in `read-write-separation.md`;
integration with bridges is the new code path.

---

## Trade-Offs

**Local to one orchestrator.** Cross-machine discovery is
explicit future work. The substrate is one process; agents on
machine A cannot discover providers on machine B in v1.

**Default-deny on identity disclosure.** Requesters don't learn
provider names by default. This protects against enumeration but
means audit reviewers may need cross-references between
discovery records and the orchestrator's own bridge ledger to
trace which provider satisfied a given request.

**Schema cross-validation is hash-equality.** v1 just compares
SHA-256 of the schema files. This means a schema with extra
optional fields is *not* compatible with a schema without them
— even if logically they would be. v2 may add structural
schema-compatibility checks (provider's schema is a strict
superset, etc.).

**Round-robin selection.** v1 picks a provider naively. Smarter
policies (least-loaded, locality, sticky-session) come later.

**Per-call bridge cost.** Creating two ratcheted channels with
fresh X3DH key exchanges adds ~10-20 ms per discovery. For
discovery rates measured in calls-per-minute (not per-second),
this is invisible. For higher rates, a future optimization
might pool channels per (consumer, provider) pair across many
calls. v1 stays simple.

**No transitive discovery.** Agent A discovers B. B cannot then
"introduce" A to C; A must discover C itself. This is intentional
to keep the trust graph readable in audit logs.

**No load-balancing across providers.** v1 returns one provider
per call. An agent that needs to fan out to many providers in
parallel calls `find` to get the list, then issues multiple
`connect` calls. This is more verbose than a hypothetical
`find_and_connect_all` but keeps the per-call semantics simple.

**Manifest declarations duplicate intent.** The provider says
"I provide file-reader for workspace X." The consumer says "I
need a file-reader for workspace Y." The orchestrator matches
them. This duplication is by design: the manifest is the
contract; runtime negotiation would let agents lie to each other
about what they provide or want.

---

## Future Extensions

- **Cross-machine discovery** with cryptographic provenance for
  the discovery channel itself.
- **Provider weighting** for non-round-robin selection
  (least-loaded, sticky-session, geo-affinity).
- **Channel pooling** across many discovery calls between the
  same pair to amortize bridge setup.
- **Subscription-style discovery** ("notify me when a
  file-reader becomes available") — useful for hosts that
  spawn before their providers are ready.
- **Structural schema compatibility** instead of hash equality.
- **Discovery brokers** — third-party brokers that the
  orchestrator delegates discovery to (for very large
  topologies).
- **Transitive trust laundering** — a chain of schema-pinned
  bridges treated as one trust boundary.

These are deliberately out of scope for v1.
