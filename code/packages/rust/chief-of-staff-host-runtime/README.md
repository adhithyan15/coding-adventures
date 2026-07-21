# chief-of-staff-host-runtime

`chief-of-staff-host-runtime` turns reviewed host and orchestrator profiles into
active D18D tool runtimes. Each host names its privilege ceiling, capability
surface, and exact tool ids. The orchestrator profile requires one owner per
tool and routes calls only to that host. Activation fails unless every
allowlisted tool has been registered and every definition fits those bounds.

This is the first production-shaped replacement for wiring an unrestricted
`InMemoryToolRuntime` directly inside each Chief job. Profiles can now activate
either that in-process runtime or a supervised external-host runtime. The
external path adapts the repo-owned `generic-job-runtime` stdio process pool,
requires exactly one process specification per profile host, routes each RPC to
the sole owner of its tool id, exposes worker health snapshots, applies bounded
restart policy after crashes, and supports orchestrator shutdown.

The child protocol is the versioned `generic-job-protocol` envelope carrying a
`HostRpcRequest` and `HostRpcResponse`. This proves the process and RPC boundary
without weakening the profile gate. A deny-all Deno worker can implement the
same JSON-lines protocol in the next slice; it does not need a parallel process
manager or a second tool-routing policy.

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
