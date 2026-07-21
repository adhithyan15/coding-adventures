# chief-of-staff-host-runtime

`chief-of-staff-host-runtime` turns reviewed host and orchestrator profiles into
active D18D tool runtimes. Each host names its privilege ceiling, capability
surface, and exact tool ids. The orchestrator profile requires one owner per
tool and routes calls only to that host. Activation fails unless every
allowlisted tool has been registered and every definition fits those bounds.

This is the first production-shaped replacement for wiring an unrestricted
`InMemoryToolRuntime` directly inside each Chief job. It deliberately does not
spawn Deno yet; process supervision and host RPC can wrap the same active
runtime without changing catalog policy.

An orchestrator profile has this shape:

```json
{
  "profile_id": "umbrella_today_v1",
  "hosts": [
    {
      "host_id": "weather_fetcher",
      "max_tier": "tier1",
      "allowed_tools": ["weather.fetch_current"],
      "capabilities": ["weather_api_read"]
    },
    {
      "host_id": "file_writer",
      "max_tier": "tier1",
      "allowed_tools": ["file.write_text"],
      "capabilities": ["filesystem_write"]
    }
  ]
}
```

The Weather Agent end-to-end crate loads an orchestrator profile containing
separate fetcher, classifier, and writer hosts. It registers each host catalog,
activates the orchestrator runtime, and only then executes the scheduled
pipeline. This preserves read/write separation instead of placing untrusted
network ingestion and filesystem actuation in one host.
