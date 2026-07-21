# weather-agent-e2e

`weather-agent-e2e` is the first end-to-end Chief of Staff substrate exercise for
the umbrella-today agent described in `code/specs/weather-agent.md`.

The crate keeps a deterministic Seattle weather fixture for CI and also exposes
an ignored live mode that fetches real Weather.gov data through `tls-platform`
and `http1`. The live operation module is generated at build time from
`required_capabilities.json` and statically linked as Rust source, and its HTTP
client refuses undeclared domains before opening TLS sockets. The fixture keeps
CI stable while still forcing the fetch, classify, supervise, write, journal,
store, and capability boundaries to run as one pipeline.

The D18D tools are loaded from `orchestrator_profile.json` through
`chief-of-staff-host-runtime`. Fetch, classification, and file writing belong to
three isolated host profiles with independent capability sets. The profile must
be complete and active before the first tool invocation. The writer host applies
a centralized policy that requires a call-scoped user approval before filesystem
output; an absent grant leaves the call pending and the report unwritten.

A successful run now emits a validated D18C `JobRunReceipt` plus a compact
`UmbrellaUserReport`. The receipt points at the stored report artifact, while the
user report carries the recommendation, completion time, approval state, and
journal invocation count. This makes umbrella-today a reusable Chief job with a
terminal product rather than only an architecture harness.

Every run also copies the canonical payload-free D18D audit rows into
`chief-of-staff-tool-audit-store` over the D18A local-folder backend before actor
errors are returned. The job then reopens that store as a fresh reader and emits
an `UmbrellaDurableAuditSummary` keyed to the job, run, session, user, and host
profile. Approved runs and approval-blocked writes are therefore both durable
without persisting tool arguments, outputs, or credentials. Durable call IDs are
scoped by scheduler tick, allowing successive runs to share one audit root while
preserving duplicate-delivery conflicts inside a tick.

The primary tests write a real `umbrella-today.txt` file through the capability
cage, assert that the supervised agent says to bring an umbrella for the rainy
fixture, prove the supervisor recreates a killed child before the next tick, and
lower the Weather Agent capability manifest into Linux, macOS, Windows, FreeBSD,
OpenBSD, and portable host-broker sandbox primitive plans.

Run the live HTTPS smoke manually when network access is acceptable:

```bash
cargo test -p weather-agent-e2e umbrella_today_agent_fetches_live_weather_over_tls -- --ignored --nocapture
```
