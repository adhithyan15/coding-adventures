# File Writer Host

## Overview

`file-writer-host` is the **actuator** of the v1 PoC pipeline.
It discovers a `weather-recommendation-source` provider (the
classifier), reads one `Recommendation` per tick, and appends a
single structured line to `./weather-log.txt`. It is the only
host in the pipeline that has an `fs:write` capability — the
only host that produces an externally-visible side effect on
the user's machine.

Because the `weather-recommendation-source` channel is
schema-pinned (the classifier declares
`trust_laundering: true`, the file-writer's `schema_accept`
matches the classifier's `schema_emit`), RWS treats the bridged
channel as a trust-laundering boundary. The file-writer reads
**enum + clamped numbers + a hex tick id**, never an
attacker-controlled string. The literal text written to the
log file is constructed by the file-writer's own code from a
fixed format string substituting these validated values; no
byte from any external source flows into the file unmediated.

This host is the smallest possible actuator. Its manifest
declares one discovery and one filesystem write. The
implementation is ~50 lines of agent code on top of the
host-runtime-rust SDK.

---

## Where It Fits

```
   weather-classifier-host (provides weather-recommendation-source)
        │
        │  schema-pinned channel established by orchestrator
        │  via agent-discovery when file-writer discovers classifier
        ▼
   file-writer-host (this spec)
        │
        ├── consumes:  agent-discovery (find_and_connect)
        │              host-runtime-rust (Tier 1 SDK; host.fs.write)
        │              capability-cage-rust (manifest enforcement)
        │              json-parser, json-value (parse Recommendation)
        │              time (timestamp formatting)
        │
        ├── manifest:  discover: weather-recommendation-source
        │              fs:write: ./weather-log.txt (actuation)
        │              (NO net:*, NO proc:*, NO vault:*)
        │
        ├── actuates:  appends one line to ./weather-log.txt per tick
        │
        └── exits Stop on success, fails the tick on any error
```

**Depends on:**
- `host-runtime-rust` — Tier 1 SDK; provides
  `host::discovery::find_and_connect`, `host::channel::read`,
  `host::fs::write`, and the entrypoint plumbing.
- `agent-discovery` — for the dynamic peer lookup.
- `dynamic-topology` — model under which this spec is written.
- `capability-cage-rust` — manifest enforcement.
- `read-write-separation` — RWS-clean because the only
  untrusted-input source the file-writer reads (the
  recommendation channel) is a schema-pinned trust-laundering
  boundary.
- `json-parser`, `json-value` — to parse the Recommendation.
- `time` — to format the ISO-8601 timestamp.

**Used by:**
- `weather-agent` (the program) declares this host as a
  `spawn_children` entry.

This is the terminal node of the PoC pipeline; nothing
discovers from the file-writer.

---

## Design Principles

1. **Smallest possible actuator.** One discovery, one read, one
   formatted write, exit. No retries inside the tick, no
   buffering across ticks, no in-memory state.

2. **Pinned to one path.** The manifest's `fs:write` target is
   the literal `./weather-log.txt`. Not a glob, not a directory,
   not a configurable path. A future generalization could allow
   per-instance configuration; v1 is hard-coded.

3. **Format string lives in code.** The literal characters
   appended to the file (timestamps, separators, column widths)
   are baked into the writer's source. No part of the format
   crosses any channel.

4. **Append-only.** Open with O_APPEND (or its Windows
   equivalent — the host runtime's `fs.write` opens in append
   mode by default for actuators).

5. **Per-tick lifecycle.** Spawn, discover classifier, read,
   format, write, exit. Stateless across ticks.

6. **No retries on write failure.** If the disk is full or the
   file is locked, the tick fails. The next scheduled tick is
   a fresh attempt. Retries belong to a higher layer.

---

## Manifest

```json
{
  "version": 1,
  "package": "rust/file-writer-host",
  "capabilities": [
    {
      "category":      "fs",
      "action":        "write",
      "target":        "./weather-log.txt",
      "flavor":        "actuation",
      "trust":         "trusted",
      "justification": "Append one structured line per tick to the user's weather log"
    }
  ],
  "discover": [
    {
      "role":            "weather-recommendation-source",
      "qualifier_query": { "location": "seattle" },
      "schema_accept":   "schemas/weather-recommendation.schema.json",
      "schema_emit":     "schemas/weather-recommendation-ack.schema.json",
      "max_outstanding": 1,
      "justification":   "Read the schema-pinned recommendation from the classifier"
    }
  ],
  "provides": [],
  "justification": "Actuator: read schema-pinned (trust-laundered) recommendation from internal channel, append structured line to ./weather-log.txt. RWS-clean because the only untrusted-input source we'd otherwise have is the recommendation channel, and that channel is trust-laundered by schema."
}
```

The `trust: "trusted"` annotation on the recommendation channel
is what RWS uses to decide the channel content is laundered.
The orchestrator validates this at discovery time: the
classifier's `provides` entry has `trust_laundering: true`, the
schemas hash-match, and both sides agree → the bridge is
trust-laundered → the file-writer is permitted to combine that
read with `fs:write` actuation.

If the classifier didn't declare `trust_laundering`, or the
schemas didn't match, the file-writer's manifest would still
load (capability-cage doesn't know about other agents at load
time), but the orchestrator's bridge attempt would fail with
`RwsViolation` and the tick would exit non-zero before any
write happened.

---

## File Format

(Restated from `weather-agent.md` for convenience; the source
of truth is the writer's source code.)

```
2026-05-06T14:00:01.234Z  tick=8a7b1c0e... kind=Both          high_f=58 precip_pct=72
2026-05-06T14:05:00.987Z  tick=2c0e7d3f... kind=JacketOnly    high_f=55 precip_pct=15
2026-05-06T14:10:00.456Z  tick=4f1a92bd... kind=Both          high_f=53 precip_pct=80
2026-05-06T14:15:00.012Z  tick=9d3e4f5a... kind=NoAction      high_f=68 precip_pct=10
```

Per-line layout:

```
<timestamp>          ISO 8601 UTC, millisecond precision (24 chars)
<2 spaces>
"tick="<tick_id>     literal "tick=" + 32 hex chars from Recommendation.tick_id
<1 space>
"kind="<kind>        literal "kind=" + Recommendation.kind, left-padded
                     to 12 chars for column alignment
<1 space>
"high_f="<int>       literal "high_f=" + Recommendation.high_temp_f
                     rounded to nearest integer, 2-3 chars
<1 space>
"precip_pct="<int>   literal "precip_pct=" + Recommendation.precip_pct
<newline>            "\n"
```

Total per line: ~80-90 bytes. ~150 bytes per hour at the
5-minute schedule.

Format details:

- The timestamp comes from `host::system::unix_time` and is
  formatted by the writer's code into ISO 8601. It is not
  Recommendation.fetched_at_ms (which represents when the
  fetcher got the data); the file timestamp is the moment of
  writing. Both are useful, but for column alignment we use
  one — the time of write.
- The `tick_id` is verbatim from the Recommendation. It is a
  32-char hex string; the schema validates the pattern, so
  no other characters can appear here.
- `kind` is one of four literal strings; the writer's format
  string left-pads to 12 chars (length of "UmbrellaOnly")
  so columns align across rows.
- `high_f` is `Recommendation.high_temp_f` rounded to nearest
  integer for compactness.
- `precip_pct` is the integer from the Recommendation
  (already an integer in the schema).
- Newline is `"\n"` — Unix-style on every platform. Notepad
  on Windows handles it; modern text editors do.

---

## Per-Tick Lifecycle

```
T=0    Parent (weather-agent program) spawns this host.
T=10ms Host runtime completes channel bootstrap with orchestrator.
T=20ms Agent calls
       host::discovery::find_and_connect(
           role: "weather-recommendation-source",
           qualifier: { location: "seattle" },
           schema_accept: weather-recommendation.schema.json,
           schema_emit:   weather-recommendation-ack.schema.json,
       )
       Orchestrator finds the running classifier (sibling under
       same parent), bridges with a schema-pinned ratcheted
       channel marked trust_laundered, returns channel ids.
T=40ms Agent calls host::channel::read(inbound) — blocks until
       Recommendation arrives.
T=~252ms Recommendation arrives (classifier took ~250ms after
       the fetcher's network call).
       Agent constructs the line via local format string.
T=~253ms Agent calls
       host::fs::write("./weather-log.txt", line_bytes).
       Host runtime opens the file in O_APPEND, writes, closes.
T=~255ms Agent returns Stop. Supervisor honors Transient + Normal
       exit; does not restart within this tick.
```

Total: ~255 ms wallclock (dominated by upstream fetcher's
network RTT).

---

## Code Sketch

```rust
//! file-writer-host: the actuator of the v1 PoC pipeline.

use host_runtime_rust::{HostRuntime, AgentEntrypoint, host};
use json_parser::Value;
use std::time::SystemTime;

const LOG_PATH: &str = "./weather-log.txt";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    HostRuntime::run(AgentEntrypoint {
        package_path: std::env::args().nth(1).unwrap().into(),
        boot_agent:   Box::new(boot),
    })
}

fn boot(host: &Host) -> Result<(), HostError> {
    // Discover the classifier; orchestrator bridges via
    // schema-pinned trust-laundered channel.
    let conn = host::discovery::find_and_connect(
        DiscoveryQuery::Role {
            role:      "weather-recommendation-source".into(),
            qualifier: json::object! { location: "seattle" },
            prefer_local: true,
        },
    )?;

    // Read one Recommendation.
    let msg = host::channel::read(&conn.inbound_channel)?
        .ok_or(HostError::Upstream("no recommendation received".into()))?;
    let rec = json_parser::parse(&msg.payload)?;

    // Extract validated fields. The schema guarantees the
    // shapes; we still defensively unwrap for completeness.
    let kind        = rec.get("kind")
                         .and_then(|v| v.as_str())
                         .ok_or(HostError::Parse("kind missing".into()))?;
    let high_f      = rec.get("high_temp_f")
                         .and_then(|v| v.as_f64())
                         .ok_or(HostError::Parse("high_temp_f missing".into()))?;
    let precip_pct  = rec.get("precip_pct")
                         .and_then(|v| v.as_i64())
                         .ok_or(HostError::Parse("precip_pct missing".into()))?;
    let tick_id     = rec.get("tick_id")
                         .and_then(|v| v.as_str())
                         .ok_or(HostError::Parse("tick_id missing".into()))?;

    // Construct the line via local format string.
    // No byte from the channel reaches the file unmediated.
    let now_ms      = host::system::unix_time();
    let timestamp   = format_iso8601_ms(now_ms);
    let high_int    = high_f.round() as i64;
    let line = format!(
        "{}  tick={:32} kind={:<12} high_f={} precip_pct={}\n",
        timestamp,
        tick_id,             // schema-validated 32-char hex
        kind,                // schema-validated enum
        high_int,
        precip_pct,
    );

    // Append to the log file.
    host::fs::write(LOG_PATH, line.as_bytes())?;

    Ok(())  // clean Stop
}

fn format_iso8601_ms(unix_ms: u64) -> String {
    let secs    = unix_ms / 1000;
    let millis  = unix_ms % 1000;
    // Use the time crate or a small in-house formatter.
    let dt = format_unix_to_iso(secs);   // "2026-05-06T14:00:01"
    format!("{}.{:03}Z", dt, millis)
}
```

About 50 lines of agent code on top of the host-runtime SDK.
The host runtime handles channel bootstrap, secure-host-channel
encryption, manifest checks, fs:write enforcement, supervision,
audit. The agent's job is parsing the validated Recommendation
and applying the format string.

---

## Discovery and Trust Laundering

The orchestrator's bridge logic, when satisfying the file-
writer's `find_and_connect` call:

1. The file-writer's manifest has `discover: [{ role:
   "weather-recommendation-source", schema_accept:
   weather-recommendation.schema.json, ... }]`.
2. The classifier's manifest has `provides: [{ role:
   "weather-recommendation-source", schema_emit:
   weather-recommendation.schema.json, trust_laundering:
   true, ... }]`.
3. The orchestrator computes:
   - `schema_accept_hash(file-writer) == schema_emit_hash(classifier)` → match
   - `trust_laundering: true` on the provider side
   - therefore the bridge is **trust-laundered**.
4. RWS analysis on the post-bridge topology:
   - file-writer has actuation (`fs:write`).
   - file-writer reads from a trust-laundered channel.
   - therefore RWS-clean. Bridge is permitted.
5. Channels created. file-writer reads.

If step 3 fails (schema mismatch, or `trust_laundering: false`),
step 4 reclassifies the file-writer as having
untrusted-input-read + actuation → `RwsViolation` → bridge
refused → the discovery call returns `RwsViolation` → the
file-writer's tick fails. The user sees the error in the
orchestrator's audit log.

---

## Test Strategy

### Unit Tests

1. **Line format.**
   - Given a fixed Recommendation { kind: "Both",
     high_temp_f: 58.4, precip_pct: 72,
     tick_id: "8a7b1c0e9d3e4f5a..." } and a fixed timestamp,
     the produced line matches a captured fixture exactly.
   - Different kinds (NoAction, JacketOnly, UmbrellaOnly,
     Both) all produce correctly-aligned columns.
   - Negative temperatures round correctly.
2. **Defensive parsing.**
   - Recommendation missing `kind` → `Parse` error.
   - Recommendation missing `tick_id` → `Parse` error.
   - Recommendation with extra field → schema rejects at
     channel layer; the message never reaches the agent.
3. **fs.write behavior.**
   - Successful write returns Ok; line appears in the file in
     append mode.
   - fs.write returns CapabilityDenied if the manifest is
     mutated to remove the fs:write entry → the agent never
     opens the file.

### Integration Tests (mocked discovery)

4. **Happy path.** Mock orchestrator returns a discovery
   bridge to a mocked classifier; mocked classifier writes a
   fixture Recommendation; assert the file contains the
   expected line.
5. **Discovery failure.** Mock orchestrator returns
   `NoProvidersFound`; file-writer exits non-zero; no file
   created/modified.
6. **Bridge trust-laundering refusal.** Mock orchestrator
   returns a non-laundered bridge (e.g., classifier's manifest
   missing `trust_laundering: true`); file-writer's discovery
   call returns `RwsViolation`; agent exits non-zero; no
   write occurs.
7. **fs.write failure.** Mock fs backend returns IO error
   (disk full, permission denied); agent exits non-zero;
   tick failure recorded in audit log.

### Coverage Target

`>=95%` line coverage. The agent code is small; complete
coverage is feasible.

---

## Trade-Offs

**Hard-coded log path.** `./weather-log.txt`. A
multi-instance writer (one per location) would need to
parameterize the path. v1 is one location, one file.

**Hard-coded format.** A future user might want JSON-per-line
for machine consumption. v1 ships the human-readable text
format. JSON-line variant would be a different host package
providing the same role, with a different `fs:write` target
(`./weather-log.jsonl`).

**Per-tick spawn.** Like the other hosts, ~50 ms spawn cost
per tick. Acceptable at 5-minute cadence.

**No file rotation.** The log grows ~150 bytes per hour,
~3.5 KiB per day. After a year unattended that's ~1.3 MiB.
No rotation in v1; the user can manually truncate or
implement a sidecar rotator. Future work: a `fs:write` mode
that handles rotation, or a separate log-rotator host.

**Append-only, never delete.** If the user wants to start
fresh, they delete the file manually. The agent does not
implement clearing.

**Defensive parse despite schema validation.** The channel
layer validates the Recommendation against the schema before
delivery. The agent then re-extracts fields with `unwrap`-style
error handling. This is intentional belt-and-suspenders: a
bug in the channel layer that delivers a non-conforming
message to the agent should still surface as a clean Parse
error rather than a panic.

**No fsync after write.** The host runtime's `fs.write`
default does not fsync. A power loss between write and
flush could lose a tick's line. Acceptable for the PoC; a
production version might opt into fsync-per-write.

**Single line per tick, no batching.** Even if the agent had
multiple Recommendations queued (e.g., the classifier wrote
twice for some reason), this host reads exactly one. The
extra messages would sit in the channel until the next tick
spawn drains them — but per the per-tick lifecycle,
classifier produces exactly one per tick, so this is moot
in practice.

---

## Future Extensions

Out of scope for the v1 PoC:

- **JSON-per-line variant** (`file-writer-jsonl-host`)
  providing the same `weather-recommendation-source`
  discovery role with a different output format.
- **Configurable log path** via the manifest's qualifier.
- **Log rotation** — either built-in (size-based or
  time-based) or via a separate rotator host.
- **fsync after write** for durability under power loss.
- **Multi-line writes** (one Recommendation produces multiple
  lines, e.g., separate lines for jacket and umbrella
  warnings).
- **Format string in manifest** (parameterize the format
  string itself; reviewers must verify it has no injection
  surface).
- **Sidecar metadata file** with summary statistics over the
  log.
