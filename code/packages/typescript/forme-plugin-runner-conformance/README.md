# @coding-adventures/forme-plugin-runner-conformance

The reusable FM02 conformance harness for Forme plugin-side runtimes. It
spawns a runner as an ordinary subprocess and drives only the public bounded
Content-Length/JSON-RPC protocol, so TypeScript, Python, Rust, or a future
runtime can prove the same behavior without importing TypeScript SDK code.

```ts
import {
  CONFORMANCE_STAGE,
  runRunnerConformance,
} from "@coding-adventures/forme-plugin-runner-conformance";

await runRunnerConformance({
  executable: "python3",
  args: ["fixture.py", CONFORMANCE_STAGE.id, CONFORMANCE_STAGE.configSchemaHash],
  modeArgument: true,
  expectedRunner: "forme-plugin-runner-py",
  expectedRunnerVersion: "0.1.0",
});
```

The command receives one final mode argument: `single`, `stream`,
`stream-single`, or `single-stream`. Each fixture exposes the canonical
`@forme/conformance` stage metadata in `CONFORMANCE_STAGE` and implements the
small operation vocabulary used by the suite. The public constants are the
single source of truth for future SDK fixtures.

The corpus covers lifecycle negotiation, binary and reserved-looking JSON
values, every host-mediated `StageContext` capability, logger notifications,
single/stream/hybrid shapes, cooperative cancellation, typed stage errors,
live storage-watch start/value/cancel lifecycle, malformed frame rejection,
and frame resource bounds. Driver input, output,
stderr capture, aggregate stdout, message and notification counts, queued
work, headers, payloads, request IDs, and timeouts are bounded. Outbound
values are size-preflighted before binary expansion or JSON allocation. A
malformed, flooding, or silent peer fails closed and is retired with bounded
TERM-to-KILL escalation.

The harness intentionally declares broad `proc:exec` because the caller
selects the SDK command under test. Its only signaling authority is the exact
child it starts. Merging a new non-empty capability profile remains subject to
the repository's hardware-key branch-protection approval.

The harness deliberately does not certify an OS sandbox. FM-B052 owns process,
filesystem, network, descriptor, CPU, and memory containment on each supported
platform; this package certifies only the language runner's wire behavior.

## Development

```bash
sh BUILD
```

The package tests stage the built TypeScript SDK and its exact compiled
dependencies into a temporary fixture root, then pass the complete public
corpus through a real child process. Coverage gates the driver and suite.
