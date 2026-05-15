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

The primary tests write a real `umbrella-today.txt` file through the capability
cage, assert that the supervised agent says to bring an umbrella for the rainy
fixture, prove the supervisor recreates a killed child before the next tick, and
lower the Weather Agent capability manifest into Linux, macOS, Windows, FreeBSD,
OpenBSD, and portable host-broker sandbox primitive plans.

Run the live HTTPS smoke manually when network access is acceptable:

```bash
cargo test -p weather-agent-e2e umbrella_today_agent_fetches_live_weather_over_tls -- --ignored --nocapture
```
