# Weather Agent (v1 PoC)

## Overview

`weather-agent` is the first end-to-end proof of concept for the
agent substrate. Its single observable behavior:

> **Every few minutes, fetch the current Seattle forecast from
> `api.weather.gov`, decide whether the user should carry a jacket
> or umbrella, and append a structured line to `./weather-log.txt`.**

The PoC deliberately avoids OAuth, SMTP, and Gmail. Those primitives
are specced (`oauth.md`, future `smtp-transport.md`, future
`email-host.md`) and useful for later agents, but they add credential
setup, third-party API behavior, and TLS-handshake complexity that
would make the first end-to-end test a multi-day debugging exercise.
A file write keeps the loop fast and visible: open the file in any
editor and watch new lines appear.

What this PoC validates is *not* the weather rule. It is the
**substrate** — every primitive we have specced converging in one
running system:

| Primitive                                  | Exercised by             |
|--------------------------------------------|--------------------------|
| `actor` — channels, mailboxes              | inter-host messaging     |
| `supervisor` — OTP-style tree              | orchestrator's child set |
| `capability-cage-rust` + manifests         | every host enforces it   |
| `read-write-separation` (RWS)              | the three-host split     |
| `secure-host-channel` — X3DH, ratchet, DOS | every host ↔ orchestrator|
| `host-protocol` — JSON-RPC dispatcher       | every host.* call        |
| `host-runtime-rust` — Tier 1 native        | each host's process      |
| `tls-platform-windows` (Schannel)          | the HTTPS fetch          |
| `https-transport` — http1 + tls-platform   | api.weather.gov call     |
| `os-job-runtime` + Windows Task Scheduler  | every-few-minutes trigger|
| `vault` (already built)                    | (not used in PoC)        |

If a tick of the agent successfully writes a line to the log, the
substrate works end to end. Subsequent agents (Gmail, GitHub,
calendar, smart home) reuse the same primitives — only their hosts
and channel schemas differ.

This spec is the **wiring document**. It specifies the three host
packages, the two channels between them, the supervision tree, the
job that triggers it, the manifest for each host, the on-disk file
format, and the success criteria. The three host packages get their
own focused specs (`weather-fetcher-host.md`,
`weather-classifier-host.md`, `file-writer-host.md`) that reference
this one for shared schemas and channel names.

---

## Where It Fits

```
   Windows Task Scheduler                                       (the trigger)
        │
        │  every N minutes (default: 5), runs:
        │  weather-agent.exe --once
        ▼
   weather-agent (the program)                                  (this spec)
        │  starts the orchestrator with the three-host tree
        │  waits for one full pipeline tick to complete
        │  exits 0 on success, non-zero on any host failure
        ▼
   Orchestrator                                                 (orchestrator.md)
        │
        ├── Host: weather-fetcher                               (weather-fetcher-host.md)
        │     fetches api.weather.gov via https-transport
        │     publishes one Snapshot to weather-snapshots channel
        │
        ├── Host: weather-classifier                            (weather-classifier-host.md)
        │     reads weather-snapshots
        │     applies the rule (precip > threshold → umbrella; etc.)
        │     publishes one Recommendation enum to
        │     weather-recommendations channel
        │
        └── Host: file-writer                                   (file-writer-host.md)
              reads weather-recommendations
              appends one structured line to ./weather-log.txt
```

**Depends on every spec listed above.**

**Used by:** the user (who sees the file growing).

---

## Design Principles

1. **No new primitives.** This spec invents nothing. It composes
   primitives that already exist in their own specs.

2. **RWS-clean by construction.** The three-host split is exactly
   the canonical safe pattern from `read-write-separation.md`:
   ingester → trust-launderer → actuator, with schema-pinned
   channels between them.

3. **Minimum viable everything.** Every host is the smallest
   possible implementation that exercises the substrate. The
   classifier's rule is hard-coded; the file format is a single
   line per tick; the schedule defaults to every 5 minutes.

4. **Reproducibly deterministic given input.** Same forecast
   bytes from `api.weather.gov` → same recommendation → same log
   line. No randomness in the rule.

5. **Visible on disk.** The success signal is a file the user can
   see growing. No hidden state, no UI, no notifications.

6. **Per-tick lifecycle.** Each scheduled run launches the
   orchestrator, completes one pipeline tick, and exits cleanly.
   The orchestrator is not a long-running daemon for the PoC —
   that simplification means we don't need to worry about
   long-term restart behavior, panic-broadcast escalation, or
   cross-tick state. Per-tick startup is ~50-200 ms; the human
   eye won't notice.

---

## The Pipeline

Three hosts, two channels, one job. Every arrow is exactly one
message per tick.

```
                         (per-tick, every 5 min by default)

  ┌─────────────────┐    Snapshot          ┌─────────────────┐
  │ weather-fetcher │ ─────────────────►   │  weather-       │
  │                 │                       │  classifier     │
  │ caps:            │     channel:         │                 │
  │  net:connect:    │    weather-snapshots │ caps:            │
  │   api.weather    │    schema-pinned     │  channel:read:   │
  │   .gov:443       │    no string fields  │   weather-       │
  │   (ingestion,    │    no actuation      │   snapshots      │
  │    untrusted)    │    flavor            │  channel:write:  │
  │  channel:write:  │                      │   weather-       │
  │   weather-       │                      │   recommendations│
  │   snapshots      │                      │   (internal)     │
  └─────────────────┘                       └─────────────────┘
                                                     │
                                            Recommendation
                                                     │
                                                     ▼
                                            ┌─────────────────┐
                                            │  file-writer    │
                                            │                 │
                                            │ caps:            │
                                            │  channel:read:   │
                                            │   weather-       │
                                            │   recommendations│
                                            │  fs:write:        │
                                            │   ./weather-     │
                                            │   log.txt        │
                                            │   (actuation)    │
                                            └─────────────────┘
                                                     │
                                                     ▼
                                            ./weather-log.txt
                                            (one line appended
                                             per tick)
```

### RWS analysis

- `weather-fetcher`:
  - inputs: `net:connect:api.weather.gov:443` (untrusted)
  - outputs: `channel:write:weather-snapshots` (internal channel)
  - actuation? No — internal channel writes are not actuation per
    `read-write-separation.md`.
  - **Verdict:** RWS-clean. One untrusted input, no actuation.

- `weather-classifier`:
  - inputs: `channel:read:weather-snapshots` (originator is
    `weather-fetcher`, which read untrusted bytes — so the channel
    content is transitively untrusted UNLESS the channel is
    schema-pinned for trust laundering).
  - outputs: `channel:write:weather-recommendations` (internal)
  - actuation? No.
  - **Verdict:** RWS-clean even before schema laundering, because
    the classifier has no actuation. (The schema pinning matters
    for the *next* hop.)

- `file-writer`:
  - inputs: `channel:read:weather-recommendations` (originator
    is `weather-classifier`).
  - outputs: `fs:write:./weather-log.txt` (actuation).
  - **Verdict:** RWS-clean **iff** `weather-recommendations` is
    schema-pinned for trust-laundering. The schema (defined below)
    has no string fields, only a fixed enum and numbers, so no
    attacker-controlled bytes can cross it. The classifier
    structurally cannot inject a payload into the recommendation;
    it can only emit one of four enum values plus two clamped
    numbers.

---

## Channel Schemas

### `weather-snapshots`

The raw forecast as we received it from `api.weather.gov`. We do
**not** schema-pin this channel — its content is by definition
attacker-influenceable. Anything reading from it is treated as
having read untrusted bytes.

```
Schema: application/json
Originator: weather-fetcher
Receivers:  weather-classifier
Trust:      untrusted (no laundering)
Schema is informational only:
{
  "raw_response_body":  string  // the verbatim JSON from
                                 //  api.weather.gov; opaque to
                                 //  the channel layer
  "fetched_at_ms":      number  // Unix ms the response arrived
  "http_status":        number
  "endpoint_url":       string
}
```

Only `weather-classifier` reads this channel. The classifier
parses `raw_response_body` defensively (every field optional, all
strings clamped to a safe length, all numbers clamped to safe
ranges).

### `weather-recommendations`

The schema-pinned trust-laundered channel. **No string fields.**
The classifier emits only fixed values; the writer can only
write fixed values.

```
Schema: application/json
Originator: weather-classifier
Receivers:  file-writer
Trust:      LAUNDERED via schema pin; receivers may treat as trusted

JSON schema (strict; declared in the channel config):
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "additionalProperties": false,
  "required": ["kind", "high_temp_f", "precip_pct",
               "fetched_at_ms"],
  "properties": {
    "kind": {
      "type": "string",
      "enum": ["NoAction", "JacketOnly", "UmbrellaOnly", "Both"]
    },
    "high_temp_f":     { "type": "number", "minimum": -50,  "maximum": 130 },
    "precip_pct":      { "type": "integer", "minimum": 0,    "maximum": 100 },
    "fetched_at_ms":   { "type": "integer", "minimum": 0 },
    "tick_id":         { "type": "string", "pattern": "^[0-9a-f]{32}$" }
  }
}
```

Notes:
- `kind` is the only string but is restricted by `enum` to four
  exact values. No injection surface.
- `tick_id` is a 32-hex-char string but pattern-restricted to
  hex; no injection surface.
- `additionalProperties: false` — the channel rejects any
  message with extra fields.

The orchestrator's pipeline config marks this channel
`trust_laundering: true` and the schema as the validating
contract:

```toml
[channel.weather-recommendations]
originator       = "weather-classifier"
receivers        = ["file-writer"]
schema_path      = "schemas/weather-recommendation.schema.json"
trust_laundering = true
```

Per `read-write-separation.md` v1, the orchestrator accepts the
declared schema; v2 will mechanically check that the schema has
no `string` arms without `enum` or `pattern` (the schema above
satisfies the v2 check by construction).

---

## Manifests

### `weather-fetcher`

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
      "justification": "Fetch current Seattle forecast"
    },
    {
      "category":      "channel",
      "action":        "write",
      "target":        "weather-snapshots",
      "flavor":        "internal",
      "justification": "Publish raw forecast for the classifier"
    }
  ],
  "justification": "Ingestion-only host; reads weather, publishes raw to channel; no actuation."
}
```

### `weather-classifier`

```json
{
  "version": 1,
  "package": "rust/weather-classifier-host",
  "capabilities": [
    {
      "category":      "channel",
      "action":        "read",
      "target":        "weather-snapshots",
      "trust":         "untrusted",
      "justification": "Read raw forecast from fetcher"
    },
    {
      "category":      "channel",
      "action":        "write",
      "target":        "weather-recommendations",
      "flavor":        "internal",
      "justification": "Publish enum recommendation"
    }
  ],
  "justification": "Trust-laundering middle; reads untrusted, emits schema-pinned enum; no actuation."
}
```

### `file-writer`

```json
{
  "version": 1,
  "package": "rust/file-writer-host",
  "capabilities": [
    {
      "category":      "channel",
      "action":        "read",
      "target":        "weather-recommendations",
      "trust":         "trusted",
      "justification": "Read schema-pinned (trust-laundered) recommendation"
    },
    {
      "category":      "fs",
      "action":        "write",
      "target":        "./weather-log.txt",
      "flavor":        "actuation",
      "justification": "Append one structured line per tick"
    }
  ],
  "justification": "Actuator; reads trust-laundered enum from internal channel, appends to log file."
}
```

The `trust: "trusted"` annotation on the file-writer's read of
`weather-recommendations` is what the orchestrator validates
against the channel's `trust_laundering: true` flag. The agent
author is asserting "this channel's schema is restrictive enough
that I treat its content as trusted." The orchestrator confirms
that `trust_laundering: true` is set in the pipeline config and
that the channel schema exists.

### Orchestrator manifest (this PoC's `.orchestrator/orchestrator.toml`)

```toml
state_dir = "./.orchestrator"

[panic_thresholds]
global_alert_count_window = "60s"
global_alert_count_max = 5

[manifest]
capabilities = [
  # Spawn host processes (we're at the top of the supervision tree).
  { category = "proc", action = "fork" },
  { category = "proc", action = "exec", target = "*" },
  # Read agent packages.
  { category = "fs", action = "read", target = "./agents/*" },
  # Write our own state.
  { category = "fs", action = "write", target = "./.orchestrator/*" },
]

# Pipeline definition

[host.weather-fetcher]
package = "./agents/weather-fetcher.agent"
restart = "transient"        # one-shot per tick
shutdown = { graceful = "5s" }

[host.weather-classifier]
package = "./agents/weather-classifier.agent"
restart = "transient"
shutdown = { graceful = "5s" }

[host.file-writer]
package = "./agents/file-writer.agent"
restart = "transient"
shutdown = { graceful = "5s" }

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

---

## Supervision Tree

```rust
let orchestrator = SupervisorSpec {
    strategy:     Strategy::OneForAll,    // tightly-coupled: a tick is all-or-nothing
    max_restarts: 0,                      // do not retry within a tick
    max_seconds:  60,
    manifest:     orchestrator_manifest(),
    children: vec![
        ChildSpec {
            id:       ChildId::new("weather-fetcher"),
            kind:     ChildKind::HostProcess,
            start:    Box::new(|| spawn_host_process("./agents/weather-fetcher.agent")),
            restart:  Restart::Transient,
            shutdown: Shutdown::Graceful(Duration::from_secs(5)),
            manifest: load_manifest(".../weather-fetcher.agent/required_capabilities.json"),
            ..Default::default()
        },
        ChildSpec {
            id:       ChildId::new("weather-classifier"),
            kind:     ChildKind::HostProcess,
            start:    Box::new(|| spawn_host_process("./agents/weather-classifier.agent")),
            restart:  Restart::Transient,
            shutdown: Shutdown::Graceful(Duration::from_secs(5)),
            manifest: load_manifest(".../weather-classifier.agent/required_capabilities.json"),
            ..Default::default()
        },
        ChildSpec {
            id:       ChildId::new("file-writer"),
            kind:     ChildKind::HostProcess,
            start:    Box::new(|| spawn_host_process("./agents/file-writer.agent")),
            restart:  Restart::Transient,
            shutdown: Shutdown::Graceful(Duration::from_secs(5)),
            manifest: load_manifest(".../file-writer.agent/required_capabilities.json"),
            ..Default::default()
        },
    ],
    ..Default::default()
};

let root = Supervisor::start(orchestrator)?;
```

`Strategy::OneForAll` because the three hosts coordinate via
channels; if one dies mid-tick the others' state is meaningless,
restart them together. `Restart::Transient` means a clean exit
(the host completed its single message) is honored — the host is
not restarted within the same tick.

`max_restarts = 0` because we don't want to re-attempt within a
single scheduled invocation; a failed tick exits non-zero and
the next scheduled tick is a fresh process.

---

## Per-Tick Lifecycle

```
T=0    Windows Task Scheduler launches weather-agent.exe --once
T+5ms  weather-agent main():
       - parse args
       - start orchestrator with the three-host SupervisorSpec
T+50ms orchestrator:
       - verifies signatures of three packages
       - generates per-spawn ephemeral X25519 keys
       - launches three host processes (fork+exec each)
       - completes X3DH handshake with each (in parallel)
T+200ms hosts running:
       - weather-fetcher's agent logic begins:
          - call host.network.fetch("https://api.weather.gov/...")
          - on response, call host.channel.write("weather-snapshots", ...)
          - on success, return Stop (clean exit)
       - weather-classifier:
          - host.channel.read("weather-snapshots") — blocks until message
          - on message, parse, apply rule, host.channel.write(
              "weather-recommendations", recommendation)
          - return Stop
       - file-writer:
          - host.channel.read("weather-recommendations") — blocks
          - on message, format line, host.fs.write("./weather-log.txt", line)
          - return Stop
T+~700ms all three hosts have returned Stop.
       supervisor sees all transient children exited normally.
       weather-agent.main() observes all-children-stopped, exits 0.
T+~750ms Windows Task Scheduler records exit 0; logs the tick.
```

Total wallclock: ~700 ms - 2 s depending on api.weather.gov RTT.

---

## File Format

`./weather-log.txt` is plain UTF-8, one line per tick, append-only:

```
2026-05-06T14:00:01.234Z  tick=8a7b... kind=Both          high_f=58 precip_pct=72
2026-05-06T14:05:00.987Z  tick=2c0e... kind=JacketOnly    high_f=55 precip_pct=15
2026-05-06T14:10:00.456Z  tick=4f1a... kind=Both          high_f=53 precip_pct=80
2026-05-06T14:15:00.012Z  tick=9d3e... kind=NoAction      high_f=68 precip_pct=10
```

Format details:
- ISO 8601 UTC timestamp with millisecond precision
- two-space separator between fields
- `tick=<32-hex>` — the tick_id from the recommendation
- `kind=<enum>` left-padded to 12 chars for column alignment
- `high_f=<int>` and `precip_pct=<int>`

The format string lives **inside the file-writer's code**, never
crosses any channel. The values come from the schema-pinned
recommendation (enum + clamped numbers + hex tick_id). No
attacker-controlled byte can reach this file.

---

## The Job

A single `JobSpec` registered with `os-job-runtime`'s Windows
backend at install time:

```rust
JobSpec {
    job_id:      "weather-agent",
    name:        "Weather Agent",
    description: "Fetch Seattle forecast and update weather-log.txt",
    action: JobAction::Command {
        program: PathBuf::from("./weather-agent.exe"),
        args:    vec!["--once".to_string()],
        input:   None,
    },
    trigger: JobTrigger::Interval {
        every:  Duration::from_secs(5 * 60),    // every 5 minutes
        anchor: None,                            // anchor=None means start ASAP
    },
    concurrency_policy: ConcurrencyPolicy::Skip,    // if a tick is still running, skip
    retry_policy:       RetryPolicy::default(),
    timeout_seconds:    Some(60),                   // a tick should never take >1 min
    env:                vec![],
    working_directory:  Some(PathBuf::from(".")),
    output_policy:      OutputPolicy::default(),
    enabled:            true,
}
```

Install once at PoC setup:

```
$ ./weather-agent.exe --install-schedule
[ok] Registered Task Scheduler job "weather-agent" — runs every 5 minutes.
```

Uninstall when done:

```
$ ./weather-agent.exe --uninstall-schedule
[ok] Removed Task Scheduler job "weather-agent".
```

---

## CLI Surface

```
weather-agent.exe --once               Run one pipeline tick now and exit.
                                       Used by Task Scheduler and for manual testing.

weather-agent.exe --install-schedule   Register the every-5-minutes job with
                                       Windows Task Scheduler.

weather-agent.exe --uninstall-schedule Remove the registered job.

weather-agent.exe --status             Print the current schedule status,
                                       last tick result, and tail of weather-log.txt.

weather-agent.exe --tail               tail -f the log file (convenience).

weather-agent.exe --help               Help.
```

`--once` is the only mode the substrate test really needs;
everything else is operator ergonomics.

---

## Success Criteria

The PoC is successful when **all** of the following are true:

1. `./weather-agent.exe --once` exits 0 and appends one new line
   to `./weather-log.txt`.
2. `./weather-agent.exe --install-schedule` registers a Task
   Scheduler entry with no warnings.
3. After 15 minutes of unattended running, `./weather-log.txt`
   contains at least 3 lines, each with a distinct `tick_id`,
   each with a sensible high_temp_f / precip_pct (within the
   schema's clamp ranges).
4. The orchestrator's audit log
   (`./.orchestrator/audit.jsonl`) contains, per tick:
   - one `host.launched` for each of the three hosts
   - three `host.* ` capability checks (the network fetch, the
     channel writes/reads, the file write)
   - one `host.stopped (clean)` for each of the three hosts
5. Stopping the schedule (`--uninstall-schedule`) removes the
   Task Scheduler entry; the log stops growing.
6. A manifest mutated to violate RWS (e.g., adding both
   `net:connect` to weather-fetcher AND `fs:write` to it)
   causes the orchestrator's signature-and-validation step to
   refuse the launch with `RwsViolation`.
7. The audit log shows zero panic signals during normal
   operation; an injected fault (kill the file-writer mid-write)
   shows the supervisor detecting the abnormal exit and the
   tick exiting non-zero.

---

## Test Strategy

### Per-host unit tests

Defined in each host's own spec; this spec only requires that
they exist and that they cover the contracts.

### Integration tests (this spec's responsibility)

1. **Single tick happy path.** Mock https-transport returns a
   fixture forecast; assert one line in the log with expected
   fields.
2. **Snapshot variants.** Walk through fixtures for high precip,
   low temp, both, neither; assert correct enum kind in each.
3. **Network failure.** Mock https-transport returns
   `TcpConnect`; tick exits non-zero; no log line written.
4. **Classifier crash.** Inject a panic in the classifier;
   supervisor's OneForAll restarts (within max_restarts=0 means
   it doesn't restart and bubbles up); tick exits non-zero.
5. **Schema violation.** Have a buggy classifier emit a
   recommendation with an extra string field; channel rejects
   the message; classifier sees the rejection; tick exits
   non-zero.
6. **RWS violation at install.** A modified
   `weather-fetcher.agent` manifest with both
   `net:connect:api.weather.gov:443` and `fs:write:...` is
   rejected at signature verify with `RwsViolation`; orchestrator
   refuses to start the pipeline.
7. **Concurrency.** Fire two `--once` invocations concurrently
   (simulating Task Scheduler overlap); the second observes
   the first's lock file and exits 0 with a "skipped" status.
8. **Schedule install/uninstall.** `--install-schedule` then
   `--uninstall-schedule` round-trips cleanly.
9. **15-minute unattended run.** Real Task Scheduler, real
   network. Assert ≥3 lines, no audit panics.

### Coverage Target

`>=85%` line coverage on the wiring (`weather-agent` itself).
The hosts have their own coverage targets in their specs.

---

## Trade-Offs

**No persistent orchestrator.** Per-tick startup is ~50-200 ms.
A long-running orchestrator could amortize that, but the PoC
does not need it, and per-tick keeps the model simple
(no cross-tick state, no panic-storm escalation logic to
exercise). Future agents that benefit from a persistent
orchestrator can opt in.

**Hard-coded Seattle.** The fetcher targets a literal
api.weather.gov endpoint for Seattle's grid. Generalizing the
location is configuration; not part of the PoC.

**Hard-coded rule.** The classifier's threshold logic
(precip > 30% → umbrella; high < 60°F → jacket) is hard-coded.
Tuning is an exercise for the reader.

**No retry on network failure.** A failed fetch fails the
entire tick; the next scheduled tick is a fresh attempt.
Real-world agents would want exponential backoff inside the
fetch; we defer that to keep the substrate test simple.

**Plain-text log file.** No JSON, no rotation, no compression.
The file grows ~150 bytes per tick = ~1.5 KiB per hour.
Acceptable for a PoC. Production agents would use a structured
log with rotation.

**Schedule cadence is conservative.** Every 5 minutes means
each tick is independent and clearly visible. We could go to
every 1 minute if we want noisier output. Per-tick cost
(spawn + TLS handshake + ~1 KB fetch) is bounded so cadence is
mostly about how much log we want to read.

**Single-machine, single-user.** The PoC runs on one Windows
machine with one user account. Multi-machine sync, multi-user
isolation, and remote orchestration are explicit future work.

---

## Future Extensions

After this PoC validates the substrate:

- Replace `file-writer` with `email-host` (uses smtp-transport +
  oauth) — restores the original "morning email" goal.
- Add a notification host that pings the user's phone instead.
- Add a `weather-history` host that aggregates recommendations
  over the day for a daily summary.
- Generalize to multiple cities by parameterizing the fetcher.
- Add a calendar host that adjusts the recommendation based on
  whether the user has outdoor meetings.

These all reuse the substrate; only new hosts are required.

---

## Where Each Fact Lives

| Decision                          | Where                              |
|-----------------------------------|------------------------------------|
| Pipeline shape                    | this spec                          |
| Channel names + schemas           | this spec                          |
| Job cadence                       | this spec                          |
| File format                       | this spec                          |
| Each host's manifest              | this spec (source of truth) + the host's spec (re-states for context) |
| `weather-fetcher` HTTP endpoint   | `weather-fetcher-host.md`          |
| Classifier rule                   | `weather-classifier-host.md`       |
| Writer's exact format-string code | `file-writer-host.md`              |
| Substrate behavior (every primitive used) | each primitive's own spec  |
