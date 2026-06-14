# Dynamic Topology

## Overview

There are no predefined channels in the agent system. Every
agent-to-agent connection is established at runtime through
agent-discovery. The orchestrator owns the provider lifecycle:
**spawn-on-discovery** brings a provider to life when first
needed, **idle-shutdown** retires it when no consumer is using
it, and **pool sizing** keeps the right number of instances
alive based on load.

The single carve-out: a parent agent's manifest may declare
which **sub-agents** it will spawn. Those sub-agents start when
the parent starts and stop when the parent stops. Their
existence is statically known, but **even their channels go
through agent-discovery**. The pre-definition is structural
(the supervisor tree shape), not topological (who talks to
whom).

This rule unlocks several capabilities that pre-wired pipelines
cannot offer:

1. **Lazy spawning of expensive providers.** An OAuth broker
   that holds long-lived refresh tokens, a TLS-handshake-heavy
   HTTPS client, a vault-leasing helper — none of these need to
   run when no one is using them. They spawn on first discovery
   and shut down after a configurable idle period.

2. **Provider pooling.** Multiple consumers can be bridged to
   the same provider instance. The orchestrator keeps the pool
   sized to load, scaling up under demand and back down on
   quiescence.

3. **Hot replacement.** Update a provider's package, register
   the new version in the agent-registry, and the next
   discovery returns the new instance. Existing consumers keep
   their bridges to the old version until they disconnect; new
   bridges go to the new version. No restart of the consumers,
   no rewiring of any topology config.

4. **Smaller manifests.** Instead of declaring "I write to
   channel X and read from channel Y," agents declare
   semantically: "I provide role A; I discover role B." The
   orchestrator decides what channels exist and who has access
   to them.

5. **Bounded blast radius.** A misbehaving consumer cannot
   poison a pre-wired channel that other consumers also read,
   because no such channel exists. Each consumer-provider
   bridge is a fresh ratcheted channel; compromising one tells
   you nothing about the others.

This spec is a **rule** that constrains the rest of the system,
similar in spirit to `read-write-separation.md` and
`first-party-parity.md`. Several existing specs need amendments
to remove pre-wired pipeline configs and replace them with
agent-discovery + provides/discover declarations. Those
amendments are tracked in this spec's Migration section.

---

## Where It Fits

```
   Every agent
        │
        │  speaks only host.* APIs
        ▼
   host-runtime-rust
        │
        │  channel creation goes through one path:
        │  host.discovery.find_and_connect(...)
        ▼
   agent-discovery (this is the ONLY channel-creation path)
        │
        ▼
   Orchestrator
        │  decides:
        │   - which providers to spawn now (spawn-on-discovery)
        │   - which idle providers to retire (idle-shutdown)
        │   - how many of each provider to keep alive (pool sizing)
        │  AND wires every requested bridge.
        ▼
   secure-host-channel
        │  ratcheted bridges everywhere
```

**Depends on:**
- `agent-discovery` — the only channel-creation path.
- `agent-registry` — the source of truth for which packages
  provide which roles, used at spawn-on-discovery time.
- `orchestrator` — manages provider lifecycle.
- `supervisor` — sub-agent supervision (the one carve-out).
- `secure-host-channel` — every bridge is a ratcheted channel.

**Used by:** Every agent. Every host. The orchestrator. Every
manifest.

---

## Design Principles

1. **One way to create a channel.** Agent-discovery is the only
   public API for establishing a channel between two agents.
   No `[channel.foo]` section in any TOML, no programmatic
   pre-wiring, no `Channel::new(originator, receivers)` outside
   the orchestrator's discovery handler.

2. **Sub-agents are structural, not topological.** A parent
   declares its children in the manifest. The supervisor knows
   to spawn them. They communicate (with the parent and with
   anyone else) via discovery. The parent does not pre-wire
   channels to its children.

3. **Providers are lifecycle-managed.** The orchestrator
   decides when providers run. Defaults favor "running"; idle-
   shutdown is opt-in via the manifest.

4. **Spawn-on-discovery is implicit.** A consumer calling
   `find_and_connect` for a role with no active provider does
   not get `NoProvidersFound` immediately. The orchestrator
   first checks the agent-registry: is any registered package
   able to provide this role? If yes, spawn it. Only if no
   registered package can satisfy the role does the consumer
   get `NoProvidersFound`.

5. **No magic; all decisions auditable.** Every spawn,
   shutdown, bridge creation, and bridge teardown produces an
   audit record. A reviewer can reconstruct the entire topology
   over time from the audit log alone.

6. **The substrate prefers liveness over efficiency.** When the
   orchestrator must choose between "spawn an extra provider
   instance" and "block a consumer's discovery," it chooses to
   spawn. Resource caps prevent unbounded spawning, but within
   the caps the default is to satisfy the request.

---

## What Pre-wiring Is Forbidden

The previous draft of `weather-agent.md` and the orchestrator's
`orchestrator.toml` schema referenced sections like:

```toml
# FORBIDDEN under dynamic topology

[channel.weather-snapshots]
originator       = "weather-fetcher"
receivers        = ["weather-classifier"]
trust_laundering = false

[channel.weather-recommendations]
originator       = "weather-classifier"
receivers        = ["file-writer"]
schema_path      = "schemas/weather-recommendation.schema.json"
trust_laundering = true
```

These do not exist in the new model. The orchestrator config
loses its `[channel.*]` sections entirely.

The `[host.*]` sections also lose any topology-related fields
(no more "this host is a receiver on these channels"). They
keep only structural / lifecycle fields:

```toml
# Allowed under dynamic topology

[host.weather-fetcher]
package        = "./agents/weather-fetcher.agent"
restart        = "permanent"
shutdown       = { graceful = "5s" }
idle_shutdown  = "5m"               # NEW: opt-in lifecycle
min_instances  = 0                   # NEW: lifecycle hint
max_instances  = 1                   # NEW: lifecycle hint
```

Channel topology lives nowhere in static config. It exists only
as the result of discovery calls at runtime.

---

## What Pre-launching Is Allowed

A parent agent's manifest may declare sub-agents:

```json
{
  "version": 1,
  "package": "rust/weather-agent",
  "spawn_children": [
    {
      "child_id":      "weather-fetcher",
      "package":       "./agents/weather-fetcher.agent",
      "restart":       "permanent",
      "shutdown":      { "graceful": "5s" }
    },
    {
      "child_id":      "weather-classifier",
      "package":       "./agents/weather-classifier.agent",
      "restart":       "permanent",
      "shutdown":      { "graceful": "5s" }
    },
    {
      "child_id":      "file-writer",
      "package":       "./agents/file-writer.agent",
      "restart":       "permanent",
      "shutdown":      { "graceful": "5s" }
    }
  ],
  "capabilities": [/* ... */],
  "provides":     [/* ... */],
  "discover":     [/* ... */],
  "justification": "..."
}
```

Effects:

- The parent's supervisor includes these as `ChildSpec`s.
- They start when the parent starts and stop in reverse order
  when the parent stops.
- Their `discoverable_by` is automatically extended to include
  the parent's role (so the parent can discover its own
  children).
- They appear in the agent-registry only if individually
  registered there (sub-agent declarations are not
  registrations); but their package_path is checked at parent-
  registration time to confirm the packages exist and are
  signed.

Sub-agents' channels — even to the parent — go through
discovery. The parent calls
`host.discovery.find_and_connect("file-writer-role")` and the
orchestrator returns a bridge to its own child. From the
parent's perspective there is no special path; from the
orchestrator's perspective the providers list happens to
include a child whose discoverable_by lists the parent's role.

---

## Spawn-on-Discovery

When a consumer calls `find_and_connect` and no instance of the
requested role is currently running:

```
1. agent-registry lookup:
   list packages whose manifests declare provides:role:<role>
   filter by qualifier match against the requester's
     qualifier_query
   If no matching package exists → NoProvidersFound.

2. Selection:
   Choose one package. Default policy: stable order
   (alphabetical by agent_id). Future: weighted by previous
   success/failure.

3. Spawn:
   Use the orchestrator's normal launch path
   (orchestrator.launch_host) with all the usual checks:
   - registry hash pin
   - signature verify
   - manifest load
   - tier challenge if effective_tier > 0
   - secure-host-channel bootstrap
   On any failure → return the structured error to the
   consumer; do not retry within this discovery call.

4. Wait for handshake completion.
   Configurable timeout, default 10 seconds.
   On timeout → NoProvidersFound (and audit log records the
   spawn-and-timeout for forensic review).

5. Proceed with bridging:
   Continue from step 3 of the agent-discovery bridging
   algorithm (the new provider is now active and counted
   toward providers_returned).

6. Audit:
   { kind: "discovery.spawn_on_discovery",
     requester, role, qualifier, spawned_provider_id,
     elapsed_ms_to_handshake, ... }
```

Tier-2+ launches still require the user's challenge. That
challenge fires synchronously within the requester's
discovery call — the consumer's `find_and_connect` blocks
until the challenge resolves. If the user denies, the consumer
receives the same `RwsViolation`/`CapabilityDenied`-style
error the orchestrator's launch path normally returns.

---

## Idle-Shutdown

Each provider entry in a host's manifest may declare:

```json
{
  "provides": [
    {
      "role":            "weather-snapshot-source",
      /* ... */
      "idle_shutdown":   "5m"       // optional; default = never
    }
  ]
}
```

When set:

- The orchestrator tracks the time since the provider's last
  active bridge.
- Once it exceeds `idle_shutdown` AND no current bridges are
  outstanding, the orchestrator gracefully terminates the
  provider.
- A subsequent discovery for that role triggers spawn-on-
  discovery (above). The next consumer pays the spawn cost.

`idle_shutdown` is a per-role property, not per-package. A
single provider package that lists multiple `provides` entries
gets shut down only when **all** of its roles have been idle
past their thresholds.

`min_instances` (a host-config field) overrides idle-shutdown:
if `min_instances >= 1`, idle-shutdown will not reduce below
that count.

This is the "spawn an agent that is expensive to maintain
forever" feature: a vault-leasing helper that opens a database
connection on startup and holds it for hours can opt into
`idle_shutdown: "10m"` and only run while consumers are
actually leasing.

---

## Pool Sizing

Each `provides` entry can specify:

```json
{
  "provides": [
    {
      "role":            "oauth-broker:google",
      "max_concurrent":  4,            // per provider instance
      "min_instances":   0,            // pool floor
      "max_instances":   3,            // pool ceiling
      "idle_shutdown":   "5m"
    }
  ]
}
```

The orchestrator's policy:

- **min_instances:** never let active count drop below this. If
  idle-shutdown would drop below `min_instances`, the
  orchestrator does not shut down. If a provider crashes and
  active count drops below `min_instances`, the orchestrator
  spawns a replacement immediately (independent of any pending
  discovery).
- **max_instances:** never spawn more than this. When all
  instances are at `max_concurrent` and a new bridge would
  exceed pool capacity, the consumer receives
  `BridgeQuotaExceeded { which: PoolSaturated }` with a
  `retry_after` hint.
- **load-driven scaling:** when the average current_load
  across active instances exceeds 75% of `max_concurrent`, the
  orchestrator spawns one more instance (up to `max_instances`).
  When average drops below 25% and there is at least one
  instance with zero active bridges past `idle_shutdown`,
  shut that one down (down to `min_instances`).

The default policy is `min_instances: 0`, `max_instances: 1`,
`idle_shutdown: never`. So the default behavior is "exactly
one instance once you discover me, kept alive forever" — the
same as today's pre-spawned model. Opting in to richer
lifecycle is a choice the agent's author makes.

---

## Bridging Sub-Agents to Parent

A parent declares its children in `spawn_children`. At parent
spawn time, the supervisor starts the children. The parent's
manifest also declares its `discover` entries as usual:

```json
{
  "spawn_children": [
    { "child_id": "file-writer", ... },
    { "child_id": "weather-fetcher", ... }
  ],
  "discover": [
    { "role": "file-writer-role", ... },
    { "role": "weather-snapshot-source", ... }
  ]
}
```

The children declare their `provides` with `discoverable_by`
that includes the parent:

```json
{
  "provides": [
    {
      "role":            "weather-snapshot-source",
      "discoverable_by": [{ "agent_id": "weather-agent" }]
    }
  ]
}
```

When the parent calls
`host.discovery.find_and_connect("weather-snapshot-source", ...)`,
the orchestrator's lookup finds the child (which is currently
running because the supervisor started it), checks that the
parent is in the child's `discoverable_by`, and bridges them.

There is **no special "parent-child channel" code path**. The
parent goes through discovery exactly like any other consumer.
The fact that the provider happens to be its own child is
visible only in the audit log.

---

## Migration

The dynamic-topology rule requires amendments to several
already-merged specs. The amendments are tracked here so
reviewers know what to expect:

| Spec                          | Amendment needed                                                                                          |
|-------------------------------|----------------------------------------------------------------------------------------------------------|
| `weather-agent.md`            | Drop `[channel.*]` sections. Replace pre-wired topology with `spawn_children` + `provides`/`discover` declarations. The 3-host pipeline becomes 1 parent + 3 sub-agents that discover each other. |
| `weather-fetcher-host.md`     | Drop `channel:write:weather-snapshots` capability. Replace with `provides: weather-snapshot-source`. Update sample code to show `host.discovery.find_and_connect` for the consumer side. |
| `agent-discovery.md`          | Add the spawn-on-discovery, idle-shutdown, and pool-sizing sections. Remove "discovery is alongside pre-wiring" wording. |
| `orchestrator.md`             | Drop `[channel.*]` from the orchestrator.toml example. Add provider-lifecycle responsibilities (spawn-on-discovery, idle-shutdown, pool sizing). |
| `weather-agent.md` (RWS)      | The RWS analysis still works; it now applies at discovery-bridge time per `agent-discovery.md`'s rules. The schema-pinned-channel example moves to a `provides`/`discover` declaration with matching schema fields. |
| `read-write-separation.md`    | Already mentions discovery-time RWS analysis. No structural change; possibly tighten language now that discovery is the only path. |

These amendments will land as separate follow-up PRs once this
spec merges. The newly-drafted specs (`weather-classifier-host.md`,
`file-writer-host.md`) will be written in the new model from
the start.

---

## Worked Example: Weather Agent After Migration

The v1 PoC weather agent under dynamic topology becomes:

```
weather-agent (the parent program)
  spawn_children:
    - weather-fetcher
    - weather-classifier
    - file-writer
  discover:
    - weather-snapshot-source       (to talk to fetcher)
    - weather-recommendation-source (to inspect at startup;
                                     not strictly needed, but
                                     useful for sanity check)
  capabilities:
    (none beyond the discover entries; this is a coordinator)

weather-fetcher
  capabilities:
    - net:connect:api.weather.gov:443 (ingestion, untrusted)
  provides:
    - role: "weather-snapshot-source"
      schema_emit:    schemas/weather-snapshot.schema.json
      discoverable_by:
        - { agent_id: "weather-classifier" }
        - { agent_id: "weather-agent" }
      idle_shutdown: "10m"
      max_concurrent: 3

weather-classifier
  discover:
    - role: "weather-snapshot-source"
      schema_accept: schemas/weather-snapshot.schema.json
  provides:
    - role: "weather-recommendation-source"
      schema_emit:    schemas/weather-recommendation.schema.json
      discoverable_by:
        - { agent_id: "file-writer" }
        - { agent_id: "weather-agent" }
      idle_shutdown: "10m"

file-writer
  capabilities:
    - fs:write:./weather-log.txt (actuation)
  discover:
    - role: "weather-recommendation-source"
      schema_accept: schemas/weather-recommendation.schema.json
```

Per-tick lifecycle:

```
T=0     Task Scheduler launches weather-agent.exe --once.
T=50ms  weather-agent main() starts orchestrator.
        Orchestrator launches weather-agent (the agent process).
T=200ms weather-agent's supervisor spawns its three children.
        All three handshake their secure-host-channel to
        the orchestrator.
T=300ms weather-classifier calls
        host.discovery.find_and_connect("weather-snapshot-source").
        Orchestrator finds the running fetcher; bridges them with
        a schema-pinned ratcheted channel; returns channel ids.
T=320ms file-writer calls
        host.discovery.find_and_connect("weather-recommendation-source").
        Orchestrator bridges file-writer to classifier.
T=350ms weather-fetcher fetches api.weather.gov; publishes Snapshot
        on the channel to classifier.
T=550ms classifier reads Snapshot, applies rule, publishes
        Recommendation on the channel to file-writer.
T=560ms file-writer writes one line to ./weather-log.txt.
        All three return Stop.
T=600ms weather-agent observes children completed; exits 0.
```

The observable behavior is the same as the pre-wired version.
The internal architecture is dynamic: every channel was created
at runtime by discovery; no `[channel.*]` config exists; the
schema-pinning happened automatically because the providers and
discoverers declared compatible schemas.

After this PoC works, the same substrate hosts a
calendar-host that the user spawns once a day and that idle-
shuts-down between uses; a smart-home-controller that bridges
on demand to whichever zigbee-coordinator is currently up; a
coding agent whose tool plugins (file-reader, command-runner)
spawn lazily as the agent invokes them.

---

## Test Strategy

### Unit Tests (orchestrator changes)

1. **Orchestrator config schema rejects `[channel.*]`.** Loading
   an orchestrator.toml that contains a channel section returns
   a parse error referencing this spec.
2. **`spawn_children` validation.** A parent with valid
   sub-agent declarations passes; with cycles or missing
   packages fails with structured errors.

### Spawn-on-discovery

3. **Spawn fires when no provider is active.** A consumer
   calling `find_and_connect` for a role with no running
   instance triggers a spawn; the bridge is returned.
4. **Spawn timeout.** A package whose handshake never
   completes returns `NoProvidersFound` after the configured
   timeout; the audit log records the spawn-and-timeout.
5. **Spawn declined by tier challenge.** Tier-2 spawn for which
   the user denies the challenge returns the structured error;
   the consumer sees `CapabilityDenied`.

### Idle-shutdown

6. **Idle provider shuts down.** A provider with
   `idle_shutdown: 30s` and no active bridges shuts down 30s
   after the last disconnect.
7. **min_instances overrides idle-shutdown.** Same provider
   with `min_instances: 1` does not shut down even when idle.
8. **New discovery respawns.** After a provider has been
   shut down, a new discovery triggers spawn-on-discovery.

### Pool sizing

9. **Scale up.** With `max_instances: 3` and one running
   provider at 100% load, the next discovery spawns a second
   instance.
10. **Scale down.** With three providers all idle, the
    orchestrator shuts down two over `idle_shutdown` periods
    (down to `min_instances`).
11. **Saturation.** With `max_instances` providers all at
    `max_concurrent`, a new discovery returns
    `BridgeQuotaExceeded { which: PoolSaturated }` with a
    `retry_after`.

### Sub-agent integration

12. **Parent discovers child.** A parent with a child whose
    `discoverable_by` includes the parent's id can call
    `find_and_connect` and gets a bridge to the child.
13. **Sibling discovers sibling.** Two children of the same
    parent, where each declares the other's id in
    `discoverable_by`, can discover each other.

### Coverage Target

`>=90%` line coverage on the new orchestrator lifecycle paths.
Discovery's own coverage was set in `agent-discovery.md`.

---

## Trade-Offs

**Higher first-discovery latency.** Spawn-on-discovery costs
50-200 ms (the host launch path) on the first discovery for an
inactive provider. Subsequent discoveries hit the running
instance immediately. For low-rate use this is invisible; for
high-rate the user opts into `min_instances >= 1` to keep a
warm pool.

**Operator must reason about pool sizing.** Picking
`min_instances`, `max_instances`, `idle_shutdown`, and
`max_concurrent` requires the author to think about the role's
expected load. Defaults are conservative (pool of 1, never
idle-shut-down) so a missing answer doesn't fail-shut.

**Tier challenges in the consumer's call path.** A spawn-on-
discovery that needs a Tier 2 challenge blocks the consumer's
discovery call until the user responds. For interactive use
this is fine; for batch use this would be terrible. Future
work: an "always-warm" annotation that pre-launches Tier-2
providers at orchestrator startup so consumers never see the
challenge mid-flight.

**Sub-agent discovery still goes through orchestrator.** Even
parent-child traffic crosses the secure-host-channel via
discovery. Latency cost is ~100 microseconds per bridge
setup. We accept it because the alternative (a "fast path"
for parent-child) is exactly the kind of special-case that
`first-party-parity.md` rules out.

**No discovery cache.** Every `find_and_connect` re-resolves
providers from scratch. Caching the resolution result for a
window would reduce orchestrator load but adds staleness
risks (a provider that died between cached-resolve and use).
v1 stays simple.

**Migration cost.** Several specs need amendments. We pay it
now because the pre-wired model is wrong for the substrate
direction; deferring would mean writing more specs against a
model we know is going away.

**Audit volume increases.** Discovery-driven topology means
many more lifecycle events in the audit log (spawn,
idle-shutdown, scale-up, scale-down, bridge-establish,
bridge-teardown). This is a feature for forensics but does
mean log retention sizing needs more attention.

---

## Future Extensions

- **Cross-machine providers.** A consumer on machine A
  discovers a provider on machine B. The bridge crosses a
  network channel rather than a process-local one. Out of
  scope for v1.
- **Smart selection policies.** Round-robin in v1; future
  versions add least-loaded, sticky-session, geo-affinity,
  and load-aware spawn-rate control.
- **Pre-warming for predictable Tier-2 providers.** A way to
  declare "always keep one of these warm so consumers never
  see the challenge."
- **Discovery cache** with conservative TTL.
- **Provider draining.** During a graceful provider shutdown,
  refuse new bridges but let existing ones complete; only
  terminate the host process when all bridges are torn down.
  v1 has graceful shutdown but doesn't explicitly drain.

These are deliberately out of scope for v1.
