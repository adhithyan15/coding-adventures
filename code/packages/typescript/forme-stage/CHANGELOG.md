# Changelog — @coding-adventures/forme-stage

## 0.1.0 — 2026-05-15

Initial release. Implements FM01 §3-4 — the Stage contract and the
StageContext bundle every stage receives.

### Added — Stage contract (FM01 §3)

- `Stage<In extends KindDescriptor, Out extends KindDescriptor>` —
  the universal contract every pipeline package implements. Generic
  over input/output kind descriptors so TypeScript infers `run`'s
  parameter types from `consumes`/`produces`.
- `StageOutput<Out>` — union of `KindPayload<Out>`, `Promise<KindPayload<Out>>`,
  and `AsyncIterable<KindPayload<Out>>`. Stages match their declared
  output shape (Stream-declared = AsyncIterable; otherwise single value).
- `JsonSchema` type alias.
- `defineStage(s)` — type-narrowing identity helper. Runtime no-op;
  preserves the precise generic parameters TypeScript would otherwise
  widen.
- Optional `init(config, ctx)` and `dispose(ctx)` lifecycle hooks.

### Added — StageContext (FM01 §4)

- `StageContext` interface with all 11 named members.
- `StageInitContext` for `init`/`dispose` (omits per-invocation
  cancellation/cache; adds validated `config`).

### Added — un-gated facilities (default impls)

- **Logger**: `consoleLogger({level?, write?, now?})` writes structured
  JSON lines; `silentLogger()` drops everything; `child(fields)` for
  scoped loggers. Five-level ladder (`LOG_LEVELS` frozen tuple).
- **Cancellation**: `createCancellationTokenSource()` returns a
  read-side `token` and a write-side `cancel(reason?)`. Token exposes
  `cancelled`, `reason`, `throwIfCancelled()`, `onCancel(cb)`, and a
  standard `AbortSignal` for fetch interop. Errors from `onCancel`
  callbacks are swallowed so one bad subscriber can't break others.
  `neverCancelledToken()` for tests/sync stages.
- **Clock**: `systemClock()` wraps `Date.now()` + `performance.now()`
  with a graceful fallback. `frozenClock({timestamp, monotonicStart?,
  monotonicTickMs?})` for reproducible builds.
- **Cache**: `inMemoryCache()` is a Map-backed `Cache`. Coalesces
  concurrent misses by memoising the *promise*; drops entries on
  rejection so retries succeed.
- **Telemetry**: `noOpTelemetryEmitter()` and `callbackTelemetryEmitter(sink)`.
- **Event bus**: `inMemoryEventBus()` — single-process pub/sub,
  handler errors swallowed, idempotent unsubscribe, snapshot-iteration
  so handlers can safely unsubscribe themselves.

### Added — capability-gated APIs (interfaces + denied wrappers)

- `StorageApi`, `NetworkApi`, `EnvApi`, `FilesystemApi`, `ShellApi`
  interfaces matching FM01 §4.8 verbatim.
- `deniedStorageApi()`, `deniedNetworkApi()`, `deniedEnvApi()`,
  `deniedFilesystemApi()`, `deniedShellApi()` — each method throws
  `CapabilityError` with the exact missing capability and the
  attempted operation embedded in the message. The orchestrator
  hands these to stages that didn't declare the matching capability.

### Spec adherence

No deliberate divergences from FM01 §3-4.

### Notes

- The denied AsyncIterable cases (`storage.list`, `storage.watch`)
  throw on the first `next()` call rather than at subscription time —
  the alternative would let stages start an iteration loop they
  couldn't actually consume.
- `EnvApi.get`/`getOrThrow` and `FilesystemApi.homeDir`/`tempDir` are
  synchronous per the spec, so the denied wrappers throw directly
  instead of returning rejected promises.
