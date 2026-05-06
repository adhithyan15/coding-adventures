# Weather Classifier Host

## Overview

`weather-classifier-host` is the **trust-laundering middle agent**
of the v1 PoC pipeline. It discovers a `weather-snapshot-source`
provider (the weather-fetcher), reads one raw forecast per tick,
applies a hard-coded rule to decide whether the user should carry
a jacket and/or an umbrella, and publishes a structured
**enum-only recommendation** as a `weather-recommendation-source`
provider for the file-writer to consume.

This host is the keystone of the pipeline's RWS analysis. The
fetcher reads attacker-influenceable bytes from the network. The
file-writer commits actuation (writes to disk). Without the
classifier, RWS would force them into one giant agent that we
cannot allow. With the classifier in the middle and a
schema-pinned recommendation channel, the file-writer reads only
trust-laundered bytes — a fixed enum and a few clamped numbers —
no attacker-controlled string ever crosses the boundary.

The classifier:

- **Has no actuation capability.** No `fs:write`, no
  `net:connect`, no `proc:exec`. Pure compute on bytes that
  arrive from one channel and bytes that go out on another.
- **Has untrusted input** (the raw forecast bytes). RWS-clean
  because no actuation pairs with it.
- **Provides a schema-pinned role** so its consumers can opt
  into trust-laundering when their schemas match.
- **Is per-tick.** One snapshot in, one recommendation out, exit
  Stop. The supervisor's `Transient` restart policy honors the
  clean exit.

The rule itself is deliberately simple — nine lines of code. The
v1 PoC tests the substrate, not the classifier's intelligence.
A future version (or a competitor's host package providing the
same role) can apply LLM reasoning over the raw forecast and
emit the same enum.

---

## Where It Fits

```
   weather-fetcher  (provides weather-snapshot-source)
        │
        │  schema-pinned channel established by orchestrator
        │  via agent-discovery when classifier discovers fetcher
        ▼
   weather-classifier-host (this spec)
        │
        ├── consumes:  agent-discovery (find_and_connect)
        │              host-runtime-rust (Tier 1 SDK)
        │              capability-cage-rust (manifest enforcement)
        │              json-parser, json-value (parse forecast)
        │              json-serializer (emit recommendation)
        │
        ├── manifest:  discover: weather-snapshot-source
        │              provides: weather-recommendation-source
        │              (NO net:connect, NO fs:*, NO proc:*)
        │
        ├── publishes: one Recommendation per tick to whoever
        │              the orchestrator bridged to
        │
        └── exits Stop on success, fails the tick on any error
                     │
                     ▼
   file-writer (discovers weather-recommendation-source)
```

**Depends on:**
- `host-runtime-rust` — the Tier 1 SDK; provides
  `host::discovery::find_and_connect`, `host::channel::read`,
  `host::channel::write`, and the entrypoint plumbing.
- `agent-discovery` — for the dynamic peer lookup.
- `dynamic-topology` — the model under which this spec is written.
- `capability-cage-rust` — manifest enforcement; particularly
  the new `discover:role:*` and `provides:role:*` capability
  taxonomy entries.
- `read-write-separation` — defines the trust-laundering
  contract this host fulfills.
- `json-parser`, `json-value`, `json-serializer` — for parsing
  the raw forecast JSON and emitting the recommendation JSON.
- `time` — for the recommendation's `fetched_at_ms` passthrough.

**Used by:**
- `weather-agent` (the program) declares this host as a
  `spawn_children` entry.
- `file-writer-host` discovers `weather-recommendation-source`
  and is bridged to this host by the orchestrator.

---

## Design Principles

1. **No actuation, period.** The manifest contains zero
   actuation capabilities. The host's only effects are
   in-process (parsing, computing) and channel writes (an
   internal action under RWS).

2. **Defensive parsing of the snapshot.** Every field optional.
   Unexpected types return errors. Numeric values clamped to
   the schema's allowed ranges before being included in the
   recommendation. The classifier treats the channel content
   as adversarial even though the schema-pinned upstream is
   nominally trusted.

3. **Schema-pinned emit.** The recommendation conforms to a
   strict JSON schema with no string arms. The schema is the
   contract; consumers that opt in (file-writer with a matching
   `schema_accept`) can treat the bridge as trust-laundered.

4. **One snapshot per tick.** The classifier doesn't accumulate
   state. It reads one message, emits one message, exits. No
   averaging across ticks, no smoothing, no derivative.
   Stateless makes it cheap to spawn, easy to test, easy to
   reason about.

5. **Hard-coded rule.** The v1 PoC's intelligence is the
   substrate's correctness, not the classifier's wisdom. Five
   `if` statements. A future Smart Classifier with LLM-driven
   reasoning is a different package providing the same role.

6. **Per-tick lifecycle.** Spawn, discover fetcher, read,
   classify, emit, exit. No long-running state. Idle-shutdown
   is moot because the host exits Stop on success.

---

## Manifest

```json
{
  "version": 1,
  "package": "rust/weather-classifier-host",
  "capabilities": [],
  "discover": [
    {
      "role":            "weather-snapshot-source",
      "qualifier_query": { "location": "seattle" },
      "schema_accept":   "schemas/weather-snapshot.schema.json",
      "schema_emit":     "schemas/weather-snapshot-ack.schema.json",
      "max_outstanding": 1,
      "justification":   "Read one raw forecast per tick"
    }
  ],
  "provides": [
    {
      "role":             "weather-recommendation-source",
      "qualifier":        { "location": "seattle" },
      "schema_emit":      "schemas/weather-recommendation.schema.json",
      "schema_accept":    "schemas/weather-recommendation-ack.schema.json",
      "trust_laundering": true,
      "max_concurrent":   3,
      "discoverable_by": [
        { "agent_id": "file-writer" },
        { "agent_id": "weather-agent" }
      ],
      "justification":    "Provide schema-pinned umbrella/jacket recommendation"
    }
  ],
  "justification": "Trust-laundering middle: read raw forecast (untrusted internal channel), emit schema-pinned enum (no string fields, no attacker-controllable bytes can cross). RWS-clean: no actuation."
}
```

Things conspicuously absent:

- **No `net:*` capability.** This host never talks to the
  network. The fetcher does that.
- **No `fs:*` capability.** This host never reads or writes
  files. The file-writer does that.
- **No `proc:*` capability.** This host never spawns processes.
- **No explicit channel names.** Under dynamic-topology, the
  classifier doesn't know in advance which channel it will read
  from or write to. Discovery returns channel ids at runtime.

This is the smallest manifest of any host in the pipeline. The
`provides` and `discover` sections (added by
`dynamic-topology.md` and `agent-discovery.md`) are the entire
public surface.

---

## The Recommendation Schema

(Restated from `weather-agent.md` for convenience; the source of
truth is the schema file shipped with the package.)

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title":   "Weather Recommendation",
  "type":    "object",
  "additionalProperties": false,
  "required": ["kind", "high_temp_f", "precip_pct", "fetched_at_ms"],
  "properties": {
    "kind": {
      "type": "string",
      "enum": ["NoAction", "JacketOnly", "UmbrellaOnly", "Both"]
    },
    "high_temp_f": {
      "type":     "number",
      "minimum":  -50,
      "maximum":  130
    },
    "precip_pct": {
      "type":     "integer",
      "minimum":  0,
      "maximum":  100
    },
    "fetched_at_ms": {
      "type":     "integer",
      "minimum":  0
    },
    "tick_id": {
      "type":     "string",
      "pattern":  "^[0-9a-f]{32}$"
    }
  }
}
```

**Why this is trust-laundering:**

- `kind` is a string but `enum`-restricted to four exact
  values. Cannot carry attacker text.
- `high_temp_f` is a number but range-clamped to plausible
  Earth-surface temperatures.
- `precip_pct` is an integer 0-100.
- `fetched_at_ms` is a non-negative integer.
- `tick_id` is a hex pattern, 32 chars exactly.
- `additionalProperties: false` — the channel layer rejects
  any message with extra fields before delivery.

No field can carry an attacker-controlled string of arbitrary
content. The downstream file-writer therefore reads only data
shapes the substrate has structurally validated.

---

## The Rule

For the v1 PoC, the classifier's logic is:

```
let high_temp_f = period_today.temperature
                  if temperatureUnit == "F"
                  else fahrenheit_from_celsius(...)
let precip_pct  = period_today.probabilityOfPrecipitation.value
                  if present and <= 100
                  else 0

let need_jacket   = (high_temp_f < 60.0)
let need_umbrella = (precip_pct > 30)

let kind = match (need_jacket, need_umbrella) {
    (false, false) => "NoAction",
    (true,  false) => "JacketOnly",
    (false, true ) => "UmbrellaOnly",
    (true,  true ) => "Both",
}

emit Recommendation {
    kind,
    high_temp_f: clamp(high_temp_f, -50.0, 130.0),
    precip_pct:  clamp(precip_pct, 0, 100),
    fetched_at_ms: snapshot.fetched_at_ms,
    tick_id: random_32_hex(),
}
```

**Choice of thresholds:**

- **60°F** for the jacket — this is the user's preference (per
  Seattle weather and reasonable comfort). Configurable in a
  later revision via the manifest's `provides` qualifier.
- **30%** for the umbrella — National Weather Service
  documentation suggests 30% is the typical threshold for
  "carry an umbrella" advice. Configurable later.

Both thresholds are hard-coded in v1. A future version can
make them configurable per location or per user.

---

## Defensive Parsing

The snapshot's `raw_response_body` field is the raw JSON the
fetcher received from `api.weather.gov`. Even though the
fetcher's `provides` is nominally schema-pinned (a Snapshot
schema, not the NWS API schema), the inner `raw_response_body`
is opaque text that the channel layer cannot validate. The
classifier parses it defensively:

```
1. Parse raw_response_body as JSON. Reject on parse error.
2. Verify shape: object with "properties" key whose value is
   an object with "periods" key whose value is a non-empty
   array. Reject on any deviation.
3. Extract periods[0]. Reject if missing.
4. Extract temperature: must be number; reject if not.
5. Extract temperatureUnit: must be string "F" or "C"; reject
   anything else.
6. Convert to Fahrenheit if needed.
7. Extract probabilityOfPrecipitation: optional object.
   If present, value must be number 0-100 (or null, which
   we treat as 0).
8. Reject if extracted high_temp_f is outside (-50, 130) —
   this is the schema's clamp range; values outside indicate
   either a unit-conversion bug or an attacker-influenced
   response.
```

Any rejection fails the tick. The `OneForAll` supervisor stops
the other hosts; the agent exits non-zero; the next scheduled
tick is a fresh attempt.

---

## Per-Tick Lifecycle

```
T=0    Parent (weather-agent program) spawns this host.
T=10ms Host runtime completes channel bootstrap with orchestrator.
T=20ms Agent calls
       host::discovery::find_and_connect(
           role: "weather-snapshot-source",
           qualifier: { location: "seattle" },
           schema_accept: weather-snapshot.schema.json,
           schema_emit:   weather-snapshot-ack.schema.json,
       )
       Orchestrator finds the running fetcher (sibling under same
       parent), bridges with a schema-pinned ratcheted channel,
       returns channel ids.
T=40ms Agent calls
       host::channel::read(inbound_channel) — blocks for the
       Snapshot.
T=~250ms Snapshot arrives (fetcher took ~200ms to call NWS).
       Agent parses raw_response_body defensively, applies the
       rule, constructs Recommendation.
T=~252ms Agent calls
       host::channel::write(outbound_channel, recommendation_json).
       (The orchestrator is responsible for ensuring the file-writer
       has discovered us and the channel is ready by now; if it
       hasn't, our write blocks briefly.)
T=~255ms Agent returns Stop.
       Supervisor sees Transient + Normal exit; does not restart
       within this tick.
```

Total: ~250 ms wallclock dominated by the fetcher's network
RTT. The classifier itself runs in <5 ms.

---

## Code Sketch

```rust
//! weather-classifier-host: the trust-laundering middle of the
//! v1 PoC pipeline.

use host_runtime_rust::{HostRuntime, AgentEntrypoint, host};
use json_parser::Value;
use json_serializer::to_string;
use std::time::SystemTime;

const JACKET_THRESHOLD_F:   f64 = 60.0;
const UMBRELLA_THRESHOLD_PCT: i64 = 30;
const TEMP_MIN_F: f64 = -50.0;
const TEMP_MAX_F: f64 =  130.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    HostRuntime::run(AgentEntrypoint {
        package_path: std::env::args().nth(1).unwrap().into(),
        boot_agent:   Box::new(boot),
    })
}

fn boot(host: &Host) -> Result<(), HostError> {
    // Discover the fetcher; orchestrator bridges via
    // schema-pinned ratcheted channel.
    let conn = host::discovery::find_and_connect(
        DiscoveryQuery::Role {
            role:      "weather-snapshot-source".into(),
            qualifier: json::object! { location: "seattle" },
            prefer_local: true,
        },
    )?;

    // Read one snapshot.
    let snapshot_msg = host::channel::read(&conn.inbound_channel)?
        .ok_or(HostError::Upstream("no snapshot received".into()))?;
    let snapshot = json_parser::parse(&snapshot_msg.payload)?;

    // Parse raw_response_body defensively.
    let raw = snapshot.get("raw_response_body")
        .and_then(|v| v.as_str())
        .ok_or(HostError::Parse("snapshot missing raw_response_body".into()))?;
    let forecast = json_parser::parse(raw.as_bytes())?;

    let period = forecast
        .get("properties").and_then(|p| p.get("periods"))
        .and_then(|p| p.as_array())
        .and_then(|a| a.first())
        .ok_or(HostError::Parse("forecast missing properties.periods[0]".into()))?;

    let temp_raw = period.get("temperature")
        .and_then(|v| v.as_f64())
        .ok_or(HostError::Parse("temperature missing or non-number".into()))?;
    let unit = period.get("temperatureUnit")
        .and_then(|v| v.as_str())
        .ok_or(HostError::Parse("temperatureUnit missing".into()))?;
    let high_temp_f = match unit.as_str() {
        "F" => temp_raw,
        "C" => temp_raw * 9.0 / 5.0 + 32.0,
        _   => return Err(HostError::Parse(format!("unknown unit: {unit}"))),
    };
    if high_temp_f < TEMP_MIN_F || high_temp_f > TEMP_MAX_F {
        return Err(HostError::Parse(format!("temp out of range: {high_temp_f}")));
    }

    let precip_pct = period.get("probabilityOfPrecipitation")
        .and_then(|p| p.get("value"))
        .and_then(|v| v.as_i64())
        .map(|v| v.clamp(0, 100))
        .unwrap_or(0);

    let need_jacket   = high_temp_f < JACKET_THRESHOLD_F;
    let need_umbrella = precip_pct > UMBRELLA_THRESHOLD_PCT;
    let kind = match (need_jacket, need_umbrella) {
        (false, false) => "NoAction",
        (true,  false) => "JacketOnly",
        (false, true ) => "UmbrellaOnly",
        (true,  true ) => "Both",
    };

    let fetched_at_ms = snapshot.get("fetched_at_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let recommendation = json::object! {
        "kind":          kind,
        "high_temp_f":   high_temp_f,
        "precip_pct":    precip_pct,
        "fetched_at_ms": fetched_at_ms,
        "tick_id":       host::system::random_hex(32),
    };

    host::channel::write(
        &conn.outbound_channel,
        to_string(&recommendation).as_bytes(),
    )?;

    Ok(())  // clean Stop; supervisor honors Transient
}
```

About 70 lines on top of the host-runtime SDK. The substrate
handles channel bootstrapping, schema enforcement, encryption,
manifest checks, supervision, audit. Agent code is rule logic.

---

## Test Strategy

### Unit Tests

1. **Recommendation kinds.**
   - high=70°F, precip=10% → `NoAction`.
   - high=50°F, precip=10% → `JacketOnly`.
   - high=70°F, precip=80% → `UmbrellaOnly`.
   - high=50°F, precip=80% → `Both`.
   - high=60.0°F, precip=30% → `NoAction` (boundaries are exclusive
     on jacket, exclusive on umbrella).
   - high=59.9°F, precip=30.1% → `Both`.
2. **Unit conversion.**
   - 15°C correctly converts to 59°F → `JacketOnly` (precip 0).
3. **Defensive parsing.**
   - Snapshot missing `raw_response_body` → `Parse` error.
   - `raw_response_body` is not valid JSON → `Parse` error.
   - Forecast missing `properties` → `Parse` error.
   - Forecast missing `properties.periods` → `Parse` error.
   - `properties.periods` is empty array → `Parse` error.
   - First period missing `temperature` → `Parse` error.
   - First period `temperature` is a string → `Parse` error.
   - Unknown `temperatureUnit` → `Parse` error.
   - `temperature` value gives out-of-range Fahrenheit → `Parse` error.
   - Missing `probabilityOfPrecipitation` → treated as 0;
     `kind` derived from temperature alone.
4. **Recommendation schema validation.**
   - The emitted JSON validates against
     `weather-recommendation.schema.json`.
   - Mutating the emit to add an extra field violates
     `additionalProperties: false`.

### Integration Tests (mocked discovery)

5. **Happy path.** Mock orchestrator returns a discovery
   bridge to a mocked fetcher; mocked fetcher writes a fixture
   snapshot; assert the classifier writes the expected
   recommendation.
6. **Discovery failure.** Mock orchestrator returns
   `NoProvidersFound`; classifier exits non-zero.
7. **Channel close mid-read.** Mock fetcher closes its channel
   without writing; classifier exits with `Upstream`.
8. **Multiple snapshots in queue.** Mock fetcher writes 3
   snapshots; classifier reads only the first (per its
   per-tick contract); exits Stop.

### Coverage Target

`>=95%` line coverage on the agent code. The rule logic is
small but every defensive-parsing branch deserves a test.

---

## Trade-Offs

**Hard-coded thresholds.** 60°F and 30% are baked in. A user
who lives in Phoenix where 60°F is "freezing" would prefer
different thresholds. v1 ships one set; v2 makes them
configurable via the manifest's `provides` qualifier.

**Hard-coded Seattle qualifier.** The discover query asks for
location=seattle. A multi-location agent would want to
parameterize. v1 ships one location; future work generalizes.

**Stateless across ticks.** No moving averages, no temporal
smoothing. A spurious 100% precip at 1:00 PM would produce a
"Both" log line by itself. We accept this; the substrate is
the test, not the classifier's intelligence.

**Per-tick spawn.** Like the fetcher, this host spawns and
exits per tick. Spawn cost is ~50 ms. A long-running classifier
with `idle_shutdown` would amortize it but adds state to manage.
v1 stays simple.

**Pinned schema files.** The schema files ship inside the
package. A schema change is a package change is a re-
registration in the agent-registry (Tier 3 challenge). This is
the right friction for security-critical contracts.

**No retry on parse failure.** A malformed forecast fails the
tick. The next tick is a fresh fetch. If NWS is down or
returns garbage for an extended period, the agent will not
write any log lines until they recover.

**No advisory output for the user.** A failed tick just exits
non-zero. The user must read the orchestrator's audit log to
learn why a log line is missing. v2 might add a `failures.txt`
sidecar, but for the PoC the audit log is the truth.

---

## Future Extensions

Out of scope for the v1 PoC:

- **LLM-driven classification.** A different package providing
  the same `weather-recommendation-source` role could send the
  forecast text to a model and emit a richer recommendation
  (still constrained to the same enum).
- **Configurable thresholds** via the manifest qualifier or
  a separate config service.
- **Multi-location.** One classifier instance per location,
  spawned by the orchestrator on demand.
- **Trend detection** with persistent state across ticks
  (requires an `idle_shutdown` longer than tick interval and
  in-host state).
- **Alternative recommendation kinds.** UV warning, wind
  warning, etc. Each new kind extends the schema's enum;
  consumers that care upgrade their `schema_accept` and
  re-discover.
