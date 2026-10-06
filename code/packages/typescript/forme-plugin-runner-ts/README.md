# @coding-adventures/forme-plugin-runner-ts

The reference TypeScript plugin-side runtime for Forme's FM02 subprocess
protocol. It turns an ordinary `Stage` into a bounded Content-Length-framed
JSON-RPC peer, mirrors lifecycle and typed streams, and exposes a wire-backed
`StageContext` without giving plugin code an ambient host API.

```ts
import { runPlugin } from "@coding-adventures/forme-plugin-runner-ts";
import { defineStage } from "@coding-adventures/forme-stage";
import { KERNEL_API_VERSION, Kinds } from "@coding-adventures/forme-types";

await runPlugin(defineStage({
  name: "@example/uppercase",
  version: "1.0.0",
  apiVersion: KERNEL_API_VERSION,
  description: "Uppercase one content value",
  consumes: Kinds.ContentNode,
  produces: Kinds.ContentNode,
  capabilities: [],
  configSchema: null,
  async run(input) { return input; },
}));
```

The sandbox launcher supplies the selected stage id and optional exact config
schema hash as bounded bootstrap arguments. The runner then validates the host
handshake, announces the authored contract, accepts one run at a time, and
translates single values, async streams, cancellation, `StageError`, binary
values, and reserved-looking objects.

Storage, network, environment, wall-clock, and filesystem operations are RPCs
mediated by the host's active run and grants. Their asynchronous SDK methods
must be awaited. `storage.watch()` remains live across the boundary through a
bounded stream handle; returning early cancels that handle, and run teardown
releases any watcher the plugin retained. Cancellation and monotonic time
remain local and synchronous.
Stage-local cache and event-bus state never cross the process boundary;
telemetry remains a no-op until FM02 assigns it a wire method. `system:shell`
still crosses the host boundary and is rejected for third-party plugins.

Frames, headers, stream queues, decoded bytes, metadata, and capability arrays
are bounded. Malformed envelopes, unknown or out-of-phase methods, inactive
streams, concurrent runs, invalid identities, and oversized values fail closed.
On `stage.dispose`, the runner replies, detaches its input, and lets the plugin
process exit without busy-waiting.

## Development

```bash
sh BUILD
```

The suite drives the real framed peer through lifecycle, complete context
mediation, binary values, streams, cancellation, typed errors, resource bounds,
and a host-launched end-to-end SDK fixture.
