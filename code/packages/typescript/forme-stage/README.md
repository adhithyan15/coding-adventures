# @coding-adventures/forme-stage

The heart of the Forme kernel — the `Stage<In, Out>` contract every pipeline package implements, plus the `StageContext` that gets handed to every `Stage.run` invocation.

See [code/specs/FM01-forme-kernel.md](../../../specs/FM01-forme-kernel.md) §3-4 for the design.

## Two surfaces

### Stage authors

```typescript
import { defineStage } from "@coding-adventures/forme-stage";
import { Kinds } from "@coding-adventures/forme-types";

export default defineStage({
  name:        "@forme/parse-markdown",
  version:     "0.1.0",
  apiVersion:  1,
  description: "Parses CommonMark + GFM into a ContentNode.",
  consumes:    Kinds.ContentSource,
  produces:    Kinds.ContentNode,
  capabilities: [],          // declare every capability you'll touch
  configSchema: null,        // JSON Schema or null for no config
  async run(source, config, ctx) {
    ctx.logger.info("parsing", { path: source.path });
    // ...
    return parsedContentNode;
  },
});
```

### Orchestrator authors

The orchestrator builds a `StageContext` per invocation by composing the in-memory facilities (logger, cancellation, clock, cache, telemetry, event bus) with capability-gated APIs (`StorageApi`, `NetworkApi`, `EnvApi`, `FilesystemApi`, `ShellApi`). For each capability the stage **didn't** declare, the orchestrator plugs in the matching `denied*Api()` so a method call throws `CapabilityError` with the missing capability embedded.

```typescript
import {
  consoleLogger, createCancellationTokenSource, systemClock, inMemoryCache,
  inMemoryEventBus, noOpTelemetryEmitter,
  deniedStorageApi, deniedNetworkApi, deniedEnvApi,
  deniedFilesystemApi, deniedShellApi,
  type StageContext,
} from "@coding-adventures/forme-stage";

const cancelSrc = createCancellationTokenSource();
const ctx: StageContext = {
  logger: consoleLogger().child({ stage: "parse-markdown", instance: "p1" }),
  cancellation: cancelSrc.token,
  time: systemClock(),
  cache: inMemoryCache(),
  telemetry: noOpTelemetryEmitter(),
  storage: realStorageApi,        // ← provided by the orchestrator
  network: deniedNetworkApi(),    // ← stage didn't declare network:*
  env: deniedEnvApi(),
  filesystem: deniedFilesystemApi(),
  shell: deniedShellApi(),
  events: inMemoryEventBus(),
};
```

## What's exported

| Group              | Exports                                                                                                       |
| ------------------ | ------------------------------------------------------------------------------------------------------------- |
| Stage contract     | `Stage`, `defineStage`, `StageOutput`, `JsonSchema`                                                           |
| Context shapes     | `StageContext`, `StageInitContext`                                                                            |
| Logger             | `Logger`, `LogLevel`, `LOG_LEVELS`, `consoleLogger()`, `silentLogger()`                                       |
| Cancellation       | `CancellationToken`, `CancellationTokenSource`, `createCancellationTokenSource()`, `neverCancelledToken()`     |
| Clock              | `Clock`, `systemClock()`, `frozenClock({timestamp, monotonicStart?, monotonicTickMs?})`                       |
| Cache              | `Cache`, `inMemoryCache()`                                                                                    |
| Telemetry          | `TelemetryEmitter`, `noOpTelemetryEmitter()`, `callbackTelemetryEmitter(sink)`                                |
| Event bus          | `EventBus`, `inMemoryEventBus()`                                                                              |
| Capability APIs    | `StorageApi`, `NetworkApi`, `EnvApi`, `FilesystemApi`, `ShellApi` — all interfaces                            |
| Denied wrappers    | `deniedStorageApi()`, `deniedNetworkApi()`, `deniedEnvApi()`, `deniedFilesystemApi()`, `deniedShellApi()`       |

## Design notes

- **Stages are values, not classes.** `defineStage` is the identity function at runtime; it exists purely for TypeScript inference. Stage objects are inspectable with no hidden state.
- **Frozen clock** for reproducible builds (FM03 §8). `frozenClock({timestamp})` makes `nowMs`/`nowIso` return a fixed instant; `monotonicMs` advances by `monotonicTickMs` per call (default 0).
- **In-memory cache** coalesces concurrent misses on the same key into one computation. A rejection drops the entry so retries can recover.
- **Event bus** is for *coordination*, not data flow — data flows along the pipeline's typed edges. Handler errors are swallowed so one bad subscriber can't break the whole bus.
- **Denied API wrappers** name the missing capability in the error message so stage authors know exactly what to add to their manifest.

## Coverage

```bash
npm install
npx vitest run --coverage
```

Targets 100% line + branch on every executable file.
