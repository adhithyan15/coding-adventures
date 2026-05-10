# Weather Fetcher Host

## Overview

`weather-fetcher-host` is the first of three host packages that
make up the v1 PoC weather agent. Its single job: every tick,
fetch the current Seattle forecast from the U.S. National Weather
Service's public API and publish the verbatim response on the
`weather-snapshots` channel for the classifier to interpret.

This host is **ingestion-only**. It has one network capability
(reaching `api.weather.gov:443`), one channel write capability
(internal), and nothing else. It cannot write files, cannot reach
any other network host, cannot exec processes, cannot read or
write the vault. Its capability cage is the smallest possible
manifest that still does useful work.

The choice of api.weather.gov is deliberate:

- **Free.** No registration, no API key, no quota negotiation.
- **HTTPS-only with proper certs.** Exercises the full
  `tls-platform-windows` + `https-transport` stack including
  certificate validation against the Windows trust store.
- **Standards-based.** GeoJSON output, well-documented, stable
  endpoints. We can point the classifier at known fixtures
  derived from real responses.
- **US-government-operated.** No tracking, no rate limits we will
  ever hit, no retroactive ToS changes.
- **Two-step lookup.** A first request (`/points/{lat},{lon}`)
  returns a metadata document including a forecast URL; a second
  request fetches the forecast itself. This exercises the
  `https-transport` API twice per tick — including the redirect
  handling, since the first response sometimes returns a Location
  header.

The fetcher does the two-step internally and publishes only the
**second** response (the actual forecast) on the channel. The
classifier should not have to know about the points-to-forecast
indirection.

---

## Where It Fits

```
   weather-agent program
        │
        │  declares this host as a spawn_children entry
        │  (per dynamic-topology; sub-agents are the only
        │   thing that can be predefined)
        ▼
   weather-fetcher-host (this spec)
        │
        ├── consumes: https-transport (Http1OverTls)
        │             tls-platform-windows (Schannel)
        │             host-runtime-rust (the Tier 1 SDK)
        │
        ├── manifest: net:connect:api.weather.gov:443 (ingestion)
        │             provides: weather-snapshot-source
        │             (NO channel:write — channels created at
        │              runtime via agent-discovery)
        │
        ├── publishes: one Snapshot per tick on whichever
        │             channel the orchestrator bridged to us
        │             when a consumer (the classifier) called
        │             find_and_connect("weather-snapshot-source")
        │
        └── exits Stop on success, fails the tick on any error
```

**Depends on:**
- `host-runtime-rust` — the in-process Tier 1 host runtime;
  provides `host::network::fetch`, `host::channel::write`, and
  the entrypoint plumbing.
- `https-transport` — the Tier 1 host runtime uses this for
  `host::network::fetch`. We do not use it directly; we just
  call the host SDK.
- `capability-cage-rust` — for the `Manifest` type and
  `secure_*` wrappers (transitively via the host runtime).
- `json-parser`, `json-value` — to parse the points-response and
  extract the forecast URL.
- `time` — for `fetched_at_ms` timestamping and tick deadlines.

**Used by:**
- `weather-agent` (the program) wires this host as one of three
  in the orchestrator's supervision tree.
- `weather-classifier-host` reads what we publish.

---

## Design Principles

1. **Ingestion-only.** No actuation capability of any kind. RWS
   classifies the host as a pure consumer of untrusted external
   bytes; the only way out is the internal channel.

2. **Smallest possible manifest.** One network target, one channel
   write. Nothing else.

3. **Verbatim publish.** We do not interpret the forecast; we
   publish the raw response body for the classifier to parse.
   The fetcher's correctness boundary is "the bytes I send are
   the bytes the API returned"; nothing more.

4. **Per-tick lifecycle.** Spawn, fetch, publish, exit. No
   long-running state. The points-to-forecast URL is
   re-resolved on every tick (cached for ~10 ms inside the tick,
   not across ticks).

5. **Fail fast on any error.** A network failure, a non-200
   response, a malformed points document — any of these fails
   the tick. The supervisor's `OneForAll` strategy then aborts
   the other hosts and the agent exits non-zero. The next tick is
   a fresh attempt.

6. **No retry inside the fetch.** Retries belong to a higher
   layer (the operator, who decides whether to bump the
   schedule's retry policy or accept gaps). For the PoC, a
   missed tick just means a missed log line.

---

## The Two-Step Fetch

The National Weather Service's API is documented at
`https://www.weather.gov/documentation/services-web-api`.
The relevant endpoints:

```
1. GET https://api.weather.gov/points/{lat},{lon}
   Returns: GeoJSON document with metadata for the lat/lon point,
   including a "properties.forecast" URL specific to that grid
   square.

   Example response (truncated):
   {
     "properties": {
       "gridId":   "SEW",
       "gridX":    124,
       "gridY":    68,
       "forecast": "https://api.weather.gov/gridpoints/SEW/124,68/forecast",
       ...
     }
   }

2. GET <the forecast URL from step 1>
   Returns: GeoJSON document with a "properties.periods" array;
   each period describes a future time window (typically twelve-hour
   blocks).

   Example response (truncated):
   {
     "properties": {
       "periods": [
         {
           "name":             "Today",
           "temperature":      58,
           "temperatureUnit":  "F",
           "probabilityOfPrecipitation": { "value": 70, "unitCode": "wmoUnit:percent" },
           "shortForecast":    "Rain Likely",
           ...
         },
         {
           "name":             "Tonight",
           ...
         },
         ...
       ]
     }
   }
```

For Seattle, lat/lon hard-coded for the v1 PoC:
**`47.6062, -122.3321`** (downtown Seattle).

We **do not** parse the forecast in the fetcher; we publish the
verbatim second response. The classifier extracts what it needs.

The fetcher does parse the **first** response (the points
document) just enough to extract `properties.forecast` — and that
parsing is defensive: every field optional, the value is
validated as an HTTPS URL pointing to api.weather.gov before we
follow it.

---

## Snapshot Publication

The Snapshot we publish to `weather-snapshots` matches the schema
defined in `weather-agent.md`:

```json
{
  "endpoint_url":      "https://api.weather.gov/gridpoints/SEW/124,68/forecast",
  "http_status":       200,
  "fetched_at_ms":     1747382400123,
  "raw_response_body": "{\"properties\":{\"periods\":[...]}}"
}
```

Notes:
- `endpoint_url` is the URL of the **second** request (the
  forecast URL), not the points URL.
- `http_status` is the status of the second request. Always 200
  for successful publishes; non-200 fails the tick before
  publishing.
- `fetched_at_ms` is the wall-clock time at which the second
  response arrived, in Unix milliseconds.
- `raw_response_body` is the verbatim response body as a string
  (the response is JSON; the channel transport encodes the
  string itself as JSON, double-encoded). Body is bounded to
  100 KiB; larger responses fail the tick.

The classifier deserializes `raw_response_body` and parses it as
a NWS forecast document.

---

## Manifest

Per `dynamic-topology.md`, channels are not declared in
manifests; they're created at runtime via `agent-discovery`.
The fetcher publishes its output by **providing** a role
(`weather-snapshot-source`); whoever needs raw forecasts
discovers that role and the orchestrator bridges them.

```json
{
  "version": 1,
  "package": "rust/weather-fetcher-host",
  "capabilities": [
    {
      "category":      "net",
      "action":        "connect",
      "target":        "api.weather.gov:443",
      "flavor":        "ingestion",
      "trust":         "untrusted",
      "justification": "Fetch current Seattle forecast from NWS public API"
    }
  ],
  "discover": [],
  "provides": [
    {
      "role":             "weather-snapshot-source",
      "qualifier":        { "location": "seattle" },
      "schema_emit":      "schemas/weather-snapshot.schema.json",
      "schema_accept":    "schemas/weather-snapshot-ack.schema.json",
      "trust_laundering": false,
      "max_concurrent":   3,
      "min_instances":    0,
      "max_instances":    1,
      "idle_shutdown":    "10m",
      "discoverable_by": [
        { "agent_id": "weather-classifier" },
        { "agent_id": "weather-agent" }
      ],
      "justification":    "Publish raw NWS forecast bytes for downstream classification"
    }
  ],
  "justification": "Ingestion-only host. One network endpoint, one provided role (no actuation, no other capabilities). RWS-clean: untrusted input + internal channel writes only via the discovered bridge."
}
```

Notes on the new fields:

- **No `channel:write` capability.** Under `dynamic-topology.md`,
  the channel id this host writes to is not known at config
  time; it's created when a consumer (the classifier) calls
  `find_and_connect("weather-snapshot-source", ...)` and the
  orchestrator bridges them. The host's authority to write to
  that channel comes from being the registered provider of
  the role, not from a capability-cage entry.
- **`trust_laundering: false`** on this `provides` entry —
  the snapshot the fetcher emits is *not* trust-laundered.
  Its content includes a `raw_response_body` string that came
  from `api.weather.gov`, which is by definition
  attacker-influenceable. The classifier reads this snapshot
  knowing it must defensively parse. The trust laundering
  happens one hop downstream at the
  `weather-recommendation-source` boundary.
- **`idle_shutdown: "10m"`** — if no consumer discovers this
  fetcher for 10 minutes, the orchestrator gracefully retires
  it. Per-tick agents (the v1 PoC pattern) will respawn it on
  the next tick's discovery; long-running consumers keep it
  alive.
- **`max_instances: 1`** — only one fetcher instance at a
  time. A bursty load that needed two would hit
  `BridgeQuotaExceeded { which: PoolSaturated }` rather than
  spawn a second; for the v1 PoC this is fine since the
  classifier consumes one snapshot per tick.
- **`discoverable_by`** restricts who can discover us. The
  classifier, plus the parent `weather-agent` (for inspection),
  are the only permitted discoverers.

The `flavor: ingestion` annotation on the network capability
is what RWS uses to decide this host is *not* an actuator on
its network.

The capability cage's manifest loader, the supervisor's
registration check, and the orchestrator's bridge-time RWS
analysis all independently verify this manifest is RWS-clean.

---

## Code Sketch

The host is small. Below is the structure (not the final code).

```rust
//! weather-fetcher-host: ingest the Seattle forecast and
//! publish a Snapshot on the weather-snapshots channel.

use host_runtime_rust::{HostRuntime, AgentEntrypoint, host};
use json_parser::Value;
use std::time::SystemTime;

const SEATTLE_LAT: &str = "47.6062";
const SEATTLE_LON: &str = "-122.3321";
const POINTS_URL_BASE: &str = "https://api.weather.gov/points/";
const USER_AGENT: &str = "weather-agent/0.1 (coding-adventures; https://github.com/adhithyan15/coding-adventures)";
const MAX_BODY_BYTES: u64 = 100 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    HostRuntime::run(AgentEntrypoint {
        package_path: std::env::args().nth(1).unwrap().into(),
        boot_agent:   Box::new(boot),
    })
}

fn boot(host: &Host) -> Result<(), HostError> {
    // Step 1: resolve points → forecast URL
    let points_url = format!("{}{},{}", POINTS_URL_BASE,
                              SEATTLE_LAT, SEATTLE_LON);
    let points_resp = host::network::fetch(&points_url, FetchOpts {
        method:           HttpMethod::Get,
        headers:          vec![
            ("User-Agent", USER_AGENT),
            ("Accept",     "application/geo+json"),
        ],
        max_body_size:    MAX_BODY_BYTES,
        connect_timeout:  Some(Duration::from_secs(10)),
        handshake_timeout: Some(Duration::from_secs(10)),
        read_timeout:     Some(Duration::from_secs(15)),
        ..Default::default()
    })?;

    if points_resp.status != 200 {
        return Err(HostError::Upstream(format!(
            "points request returned {}", points_resp.status).into()));
    }

    let points_json = json_parser::parse(&points_resp.body)?;
    let forecast_url = extract_forecast_url(&points_json)?;
    validate_https_to_weather_gov(&forecast_url)?;

    // Step 2: fetch the forecast itself
    let forecast_resp = host::network::fetch(&forecast_url, FetchOpts {
        method:           HttpMethod::Get,
        headers:          vec![
            ("User-Agent", USER_AGENT),
            ("Accept",     "application/geo+json"),
        ],
        max_body_size:    MAX_BODY_BYTES,
        connect_timeout:  Some(Duration::from_secs(10)),
        handshake_timeout: Some(Duration::from_secs(10)),
        read_timeout:     Some(Duration::from_secs(15)),
        ..Default::default()
    })?;

    if forecast_resp.status != 200 {
        return Err(HostError::Upstream(format!(
            "forecast request returned {}", forecast_resp.status).into()));
    }

    // Step 3: serve discovery requests for our role.
    //
    // Under dynamic-topology, the channel id we write to is not
    // known at config time; the orchestrator created it at the
    // moment the classifier called find_and_connect on our
    // weather-snapshot-source role. The host runtime hands us
    // back a channel handle for each accepted bridge. For the
    // v1 PoC's per-tick model, we serve exactly one bridge per
    // tick and exit.
    let bridge = host::provide::accept_one(
        "weather-snapshot-source",
        Duration::from_secs(5),    // wait up to 5s for a consumer
    )?;

    let snapshot = json::object! {
        "endpoint_url":      forecast_url,
        "http_status":       forecast_resp.status,
        "fetched_at_ms":     unix_ms_now(),
        "raw_response_body": String::from_utf8(forecast_resp.body)?,
    };

    host::channel::write(&bridge.outbound_channel,
                         snapshot.to_string().as_bytes())?;

    // Step 4: clean exit; supervisor sees Transient + Normal exit and
    // does not restart us within this tick.
    Ok(())
}

fn extract_forecast_url(points: &Value) -> Result<String, HostError> {
    points.get("properties")
          .and_then(|p| p.get("forecast"))
          .and_then(|f| f.as_str())
          .map(|s| s.to_string())
          .ok_or_else(|| HostError::Parse(
              "points response missing properties.forecast".into()))
}

fn validate_https_to_weather_gov(url: &str) -> Result<(), HostError> {
    let parsed = url_lite::parse(url)
        .map_err(|e| HostError::Parse(format!("invalid forecast URL: {e}")))?;
    if parsed.scheme != "https" {
        return Err(HostError::Parse("forecast URL is not HTTPS".into()));
    }
    if parsed.host != "api.weather.gov" {
        return Err(HostError::Parse(format!(
            "forecast URL host {} not allowed", parsed.host)));
    }
    Ok(())
}

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
```

This is ~80 lines of agent code on top of the host runtime SDK.
The substrate handles the security, the channels, the
supervision, and the OS — the agent code just describes the
business logic.

---

## Defensive Parsing

The points response is **the only thing the fetcher parses**, and
even then minimally — only enough to extract the forecast URL.
The defensive checks:

- `properties.forecast` may be absent → `Parse` error.
- The value may be any JSON type → only `String` is accepted.
- The string must parse as a URL → `Parse` error otherwise.
- The URL's scheme must be `https` → `Parse` error otherwise.
- The URL's host must be exactly `api.weather.gov` → `Parse`
  error otherwise. (We deliberately do not allow other hosts even
  if they are also government-operated; the manifest's
  `net:connect` is targeted to `api.weather.gov:443` and a redirect
  elsewhere would fail at the network layer anyway.)

The forecast response's body is **not parsed by the fetcher at
all**. We just convert it to a UTF-8 String and stuff it into the
Snapshot. The classifier owns all the JSON parsing for the
forecast itself.

---

## Test Strategy

### Unit Tests

1. **Snapshot construction.** Given a fixed forecast response
   body and a fixed `fetched_at_ms`, the published Snapshot has
   the exact expected JSON.
2. **Forecast URL extraction.**
   - Valid points response → URL extracted.
   - Missing `properties` → `Parse` error.
   - Missing `properties.forecast` → `Parse` error.
   - `properties.forecast` is a number/array/object → `Parse` error.
   - URL is non-HTTPS → `Parse` error.
   - URL host is not `api.weather.gov` → `Parse` error.
3. **Status handling.**
   - Points returns 404 → tick fails before second request.
   - Points returns 200, forecast returns 503 → tick fails.

### Integration Tests (mocked HTTPS)

4. **Two-step happy path** with a mock https-transport that
   serves a captured points response and a captured forecast
   response; verify a Snapshot is published with the expected
   `endpoint_url`, `http_status`, `raw_response_body`.
5. **Body too large.** Mock returns a >100 KiB forecast body;
   verify `BodyTooLarge` from https-transport propagates as
   tick failure.
6. **Network failure on points.** Mock returns
   `HttpsError::TcpConnect`; verify clean tick failure.
7. **Capability denied.** Run with a manifest that doesn't
   include `net:connect:api.weather.gov:443`; verify
   `CapabilityDenied` from the cage.

### Real-Network Tests (gated)

8. **Live api.weather.gov.** With explicit opt-in, run a single
   tick against the real API; verify the published Snapshot
   `http_status` is 200 and `raw_response_body` contains a
   `"periods"` array. Run rarely; the classifier's tests are
   the regression-detector for upstream-format changes.

### Coverage Target

`>=90%` line coverage on the agent code. The host runtime and
https-transport have their own coverage targets in their specs.

---

## Trade-Offs

**Hard-coded Seattle lat/lon.** The PoC targets one location.
Generalization is a configuration field added later; not part of
the substrate test.

**No caching across ticks.** The points-to-forecast URL mapping
is re-resolved every 5 minutes even though the answer almost
never changes. Caching would save one HTTPS round-trip per tick
(~100 ms). We accept the cost because per-tick processes have
no place to cache, and persistent state would complicate the
substrate test.

**No retry on transient failure.** A flaky network kills a tick.
The agent exits non-zero; the next scheduled tick is a fresh
attempt. For a 5-minute cadence, missing one tick is a 5-minute
gap in the log, not a problem.

**Body cap is conservative.** 100 KiB is far larger than any
real NWS forecast response. We pick it for safety against an
unexpected upstream change; tightening it (or making it
configurable) is fine if real responses approach the cap.

**The User-Agent string identifies the project.** NWS asks API
clients to send a descriptive User-Agent. We comply with a
project URL so they can reach us if there's a problem. Some
operators may prefer a more anonymous UA; that's a configuration
choice they can override after the PoC.

**Verbatim publish, not interpreted.** The fetcher does no
interpretation of the forecast. This means the channel content
includes a lot of fields the classifier won't use. We accept the
extra channel bandwidth (a forecast response is ~10-30 KiB) for
clean separation of concerns: the fetcher fetches, the
classifier classifies.

---

## Future Extensions

Out of scope for the v1 PoC:

- **Multi-location.** A configuration file maps location names
  to lat/lon; one fetcher instance per location, or a single
  fetcher publishing to multiple channels.
- **Caching the points→forecast URL** with a TTL across ticks
  (requires persistent state in the host).
- **Alternate weather sources.** Open-Meteo, OpenWeatherMap,
  Pirate Weather. Each gets its own fetcher host with its own
  manifest target.
- **Streaming responses.** For radar imagery and other large
  payloads, switch to https-transport's streaming API once it
  exists.
- **Conditional requests.** `If-Modified-Since` to reduce
  bandwidth and load on the upstream.
