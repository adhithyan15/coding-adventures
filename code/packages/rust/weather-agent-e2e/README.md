# weather-agent-e2e

`weather-agent-e2e` is the first end-to-end Chief of Staff substrate exercise for
the umbrella-today agent described in `code/specs/weather-agent.md`.

The crate deliberately starts with a deterministic Seattle weather fixture. The
current repository substrate has tool/runtime/capability/store/job primitives,
but not a native HTTPS client for `api.weather.gov`. The fixture keeps CI stable
while still forcing the fetch, classify, supervise, write, journal, store, and
capability boundaries to run as one pipeline.

The primary test writes a real `umbrella-today.txt` file through the capability
cage and asserts that the supervised agent says to bring an umbrella.
