# FM03 — Forme Orchestrator: Pipeline Configuration and Execution

> **Status:** Code-ready specification. Read alongside FM00 (vision)
> and FM01 (kernel).
> **Scope:** The orchestrator runtime — pipeline configuration format,
> DAG construction, type checking, execution scheduling, streaming,
> caching, incremental rebuild, watch mode, reproducible builds,
> error handling, and observability. The packages
> `forme-orchestrator`, `forme-pipeline-config`, and `forme-cache`.
> **Out of scope:** The plugin host (FM02 — loading, sandboxing, the
> extension registry). FM03 assumes plugins are already loaded; a
> first-party-only configuration that imports stages directly works
> against FM03 today, with FM02 layering in third-party plugins later.

---

## 0. Preface

FM01 specified the kernel — the types every stage speaks, the
contracts every stage implements, the capability model every API
respects. FM03 specifies the runtime that takes a configuration of
those stages and *runs* them: builds the DAG, walks it in the right
order, threads each stage's `StageContext`, propagates errors, caches
results, and (optionally) re-runs only the slice affected by a change.

The orchestrator is the engine. The kernel is the type system. A
pipeline is the program written in that type system; the orchestrator
is its interpreter.

### 0.1 Relationship to other FM specs

- **FM00** sets the vision and the named-product configurations the
  orchestrator must support.
- **FM01** defines `Stage<In, Out>`, `StageContext`, `KindDescriptor`,
  the capability model, identity, errors. FM03 consumes all of these
  and adds nothing to their contracts.
- **FM02** (next, but separable) handles plugin loading and
  sandboxing. FM03 accepts stages as plain values; whether they came
  from a sandboxed plugin or a direct import is FM02's concern.
- **FM04 / FM05 / FM06** consume orchestrator services (Style IR,
  Interactivity IR, AOT compiler) but do not influence the
  orchestrator's contract.

### 0.2 What this spec pins down

1. **Pipeline configuration** — the shape of a `PipelineConfig`,
   the canonical TypeScript form, the declarative TOML form, and the
   compatibility rules between them.
2. **Orchestrator lifecycle** — five phases (resolve → typecheck → init
   → execute → dispose) with documented invariants at each boundary.
3. **DAG construction** — how a flat list of stage instantiations
   becomes a typed directed acyclic graph the executor can walk.
4. **Execution model** — topological scheduling, streaming, parallelism,
   backpressure, resource limits.
5. **Cache** — content-addressed key derivation, pluggable backends,
   invalidation, scope.
6. **Incremental rebuild** — revision-based dependency tracking and
   partial DAG re-execution.
7. **Watch mode** — source watching, debouncing, integration with the
   live preview server.
8. **Reproducible builds** — the determinism guarantees and the modes
   that enable them.
9. **Error handling** — recoverable vs fatal, batched diagnostics,
   continue-on-error semantics.
10. **Cancellation** — token plumbing, cleanup, partial-output handling.
11. **Observability** — structured logs, per-stage metrics, OpenTelemetry
    trace export.

### 0.3 Compatibility promise

The orchestrator's public API (the configuration type, the `run`
function, the result types) is stable within `KERNEL_API_VERSION` from
FM01. Breaking changes require a kernel API bump. Internals (the cache
format, the trace shape) follow semver but do not require a kernel
API bump unless they break a stage's externally observable contract.

---

## 1. Terminology

Reuses FM01's terminology. Adds:

- **Pipeline** — a directed acyclic graph of `StageInstance`s.
- **PipelineConfig** — the user-authored description that becomes a
  pipeline after resolution.
- **StageInstance** — a stage definition (`Stage<In, Out>`) plus
  configuration for *this particular use* of the stage. The same
  stage may appear multiple times in a pipeline with different
  configs — they are distinct instances.
- **Edge** — a typed connection from one instance's output to
  another's input.
- **Phase** — a step in the orchestrator's lifecycle (resolve,
  typecheck, init, execute, dispose).
- **Run** — a single end-to-end execution of a pipeline.
- **Cache key** — a deterministic identifier derived from a stage's
  name, version, configuration, and input revision; used to skip
  re-execution when nothing changed.
- **Watcher** — the component that surfaces source-side changes for
  incremental rebuild and live preview.
- **Driver** — a thin adapter that adapts the orchestrator to a host
  context (CLI, dev server, editor preview, CI).

---

## 2. Pipeline Configuration

The user-authored description of *what to build*. Two forms, one
underlying type.

### 2.1 The `PipelineConfig` type

The canonical in-memory shape:

```typescript
export interface PipelineConfig {
  /** Human-friendly identifier for this pipeline. Used in logs. */
  readonly name: string;

  /** Pipeline-wide settings. */
  readonly settings: PipelineSettings;

  /** Ordered list of stage instances. The orchestrator infers the
   *  DAG from declared types; this list is just declaration order
   *  for human readability and tie-breaking. */
  readonly stages: readonly StageInstanceSpec[];

  /** Explicit edges, when type inference is ambiguous. Empty in
   *  the common case (linear or simple-fan-out pipelines). */
  readonly wires?: readonly EdgeSpec[];

  /** Output destinations when more than one emitter is present. */
  readonly outputs?: readonly OutputSpec[];
}

export interface PipelineSettings {
  /** Storage root for the pipeline's StorageApi. */
  readonly storageRoot: string;

  /** Where the orchestrator's cache lives. */
  readonly cacheDir: string | null;

  /** Reproducible-build mode (§9). */
  readonly reproducibleBuild: boolean;

  /** Maximum stage-level parallelism. Default: hardware concurrency. */
  readonly maxConcurrency: number | null;

  /** Logging verbosity. */
  readonly logLevel: "trace" | "debug" | "info" | "warn" | "error";

  /** Continue past recoverable errors and report at the end. */
  readonly bestEffort: boolean;

  /** Maximum wall-clock for the entire run. Null = unlimited. */
  readonly deadlineMs: number | null;
}

export interface StageInstanceSpec {
  /**
   * The stage value itself. In TypeScript form, an imported stage
   * definition. In TOML form, a package reference resolved to a
   * loaded stage by the plugin host (FM02).
   */
  readonly stage: Stage<KindDescriptor, KindDescriptor> | StageRef;

  /** A stable, user-chosen ID for this instance. Optional;
   *  defaults to the stage's `name`. Required when the same stage
   *  appears more than once. */
  readonly id?: string;

  /** Configuration passed to the stage. Validated against
   *  `stage.configSchema` at typecheck time. */
  readonly config?: unknown;

  /** Capability grants for this instance. The plugin host may
   *  refuse if the stage's manifest does not declare these. */
  readonly capabilities?: readonly Capability[];
}

export interface StageRef {
  readonly kind: "stage-ref";
  /** Package name as known to the plugin host. */
  readonly packageName: string;
  /** Optional sub-export, e.g. "default" or "namedExport". */
  readonly export?: string;
}

export interface EdgeSpec {
  /** Source instance ID + optional output port. */
  readonly from: { id: string; port?: string };
  /** Sink instance ID + optional input port. */
  readonly to: { id: string; port?: string };
}

export interface OutputSpec {
  /** Which emitter instance produces this output. */
  readonly fromInstance: string;
  /** Friendly name (e.g. "web", "email", "pdf-handout"). */
  readonly name: string;
}
```

### 2.2 TypeScript form (canonical)

A pipeline is a TypeScript file that default-exports a
`PipelineConfig`. Stages are imported and referenced by value, so the
TypeScript compiler verifies type compatibility at edit time.

```typescript
// forme.config.ts
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";

import sourceFs        from "@forme/source-fs";
import parseMarkdown   from "@forme/parse-markdown";
import collectChrono   from "@forme/collect-chronological";
import renderStatic    from "@forme/render-static";
import emitFs          from "@forme/emit-fs";

const config: PipelineConfig = {
  name: "my-blog",
  settings: {
    storageRoot:       "./content",
    cacheDir:          "./.forme/cache",
    reproducibleBuild: false,
    maxConcurrency:    null,
    logLevel:          "info",
    bestEffort:        false,
    deadlineMs:        null,
  },
  stages: [
    { stage: sourceFs,      config: { glob: "**/*.md" } },
    { stage: parseMarkdown, config: { gfm: true } },
    { stage: collectChrono, config: { route: "/posts/:slug" } },
    { stage: renderStatic,  config: { theme: "default" } },
    { stage: emitFs,        config: { out: "./dist" } },
  ],
};

export default config;
```

The orchestrator is invoked via:

```bash
forme run                            # default forme.config.ts
forme run --config pipelines/blog.ts # explicit
forme watch                          # config + watcher
```

The CLI loads the file via dynamic import, validates the exported
object against the `PipelineConfig` type at runtime (using a
JSON-Schema-derived validator since TypeScript types are erased),
and hands it to `forme-orchestrator`.

### 2.3 TOML form (declarative)

For simpler pipelines a TOML file is equivalent. The orchestrator
parses it through `forme-pipeline-config` and produces the same
`PipelineConfig` value. Stages are referenced by package name; the
plugin host (FM02) resolves them at typecheck time.

```toml
# forme.config.toml
name = "my-blog"

[settings]
storage-root        = "./content"
cache-dir           = "./.forme/cache"
reproducible-build  = false
log-level           = "info"
best-effort         = false

[[stages]]
stage  = "@forme/source-fs"
config = { glob = "**/*.md" }

[[stages]]
stage  = "@forme/parse-markdown"
config = { gfm = true }

[[stages]]
stage  = "@forme/collect-chronological"
config = { route = "/posts/:slug" }

[[stages]]
stage  = "@forme/render-static"
config = { theme = "default" }

[[stages]]
stage  = "@forme/emit-fs"
config = { out = "./dist" }
```

The TS form is canonical; the TOML form compiles to the TS form. Any
TOML-only feature is a feature gap — both forms must be expressively
equivalent. Code reviewers hold this rule strictly.

### 2.4 Validation

A `PipelineConfig` passes validation when:

1. Every `StageInstanceSpec.stage` resolves to a loaded stage.
2. Every instance has a unique ID (auto-generated from `stage.name`
   when collisions don't occur; required when they do).
3. Every `config` validates against its stage's `configSchema`.
4. Every stage's `apiVersion` equals `KERNEL_API_VERSION`.
5. Every requested capability is declared in the stage's manifest.
6. Either `wires` is empty (DAG inferred from types) OR the explicit
   wires form a valid DAG that the inferred edges agree with.
7. Every emitter instance has a corresponding `OutputSpec` if more
   than one emitter is present.

A `ConfigError` carries the failing field path, the rule violated, and
remediation text.

---

## 3. The Orchestrator

### 3.1 Public API

```typescript
export interface Orchestrator {
  /** Build a pipeline from a config. Throws on invalid configs. */
  buildPipeline(config: PipelineConfig): Promise<Pipeline>;

  /** Run a pipeline once. */
  runOnce(pipeline: Pipeline, options?: RunOptions): Promise<RunResult>;

  /** Run a pipeline and re-run on source changes. */
  watch(pipeline: Pipeline, options?: WatchOptions): WatchSession;

  /** Tear down resources held by the orchestrator. */
  dispose(): Promise<void>;
}

export function createOrchestrator(options: OrchestratorOptions): Orchestrator;

export interface OrchestratorOptions {
  /** Cache backend. Defaults to filesystem at `settings.cacheDir`. */
  readonly cache?: CacheBackend;
  /** How telemetry events are surfaced. */
  readonly telemetry?: TelemetrySink;
  /** Plugin host for resolving `StageRef`s. Optional in
   *  TypeScript-only flows where stages are passed by value. */
  readonly pluginHost?: PluginHost;
  /** Logger sink. Defaults to a stderr console logger. */
  readonly logger?: Logger;
}

export interface Pipeline {
  /** Constructed DAG. */
  readonly dag: PipelineDag;
  /** Validated configuration. */
  readonly config: PipelineConfig;
}

export interface RunOptions {
  /** Cancellation token. Defaults to a fresh one tied to SIGINT. */
  readonly cancellation?: CancellationToken;
  /** Override `bestEffort` from the config. */
  readonly bestEffort?: boolean;
  /** Reuse cached results when available. Default: true. */
  readonly useCache?: boolean;
}

export interface RunResult {
  /** Outcome — success / partial / failed. */
  readonly outcome: "success" | "partial" | "failed" | "cancelled";
  /** Per-stage execution summaries. */
  readonly stages: readonly StageRunSummary[];
  /** Final emitter outputs, keyed by `OutputSpec.name`. */
  readonly outputs: ReadonlyRecord<string, DeployArtifact>;
  /** Any errors collected. */
  readonly errors: readonly StageError[];
  /** Wall-clock elapsed. */
  readonly elapsedMs: number;
  /** Hash of all inputs combined — useful for cache attribution. */
  readonly buildId: RevisionId;
}

export interface StageRunSummary {
  readonly instanceId: string;
  readonly stageName: string;
  readonly itemsConsumed: number;
  readonly itemsProduced: number;
  readonly elapsedMs: number;
  readonly cacheHits: number;
  readonly cacheMisses: number;
  readonly outcome: "success" | "skipped" | "failed";
  readonly errorCount: number;
}

export interface WatchSession {
  /** AsyncIterable of run results, one per build. */
  results(): AsyncIterable<RunResult>;
  /** Trigger a manual rebuild. */
  rebuild(): Promise<RunResult>;
  /** Stop watching. */
  stop(): Promise<void>;
}
```

### 3.2 Lifecycle phases

A run proceeds through five phases. Each phase has documented
invariants at its end.

```
   ┌──────────┐    ┌────────────┐    ┌──────┐    ┌─────────┐    ┌─────────┐
   │ Resolve  │───►│ Typecheck  │───►│ Init │───►│ Execute │───►│ Dispose │
   └──────────┘    └────────────┘    └──────┘    └─────────┘    └─────────┘
```

#### Resolve

Inputs: `PipelineConfig`. Outputs: a fully-loaded `ResolvedPipeline`
where every `StageRef` is replaced with a concrete `Stage<In, Out>`
loaded from the plugin host.

Invariant at exit: every stage in the pipeline is a loaded value
ready to call.

Failure modes: missing package; package found but does not export a
stage; capability mismatch between request and manifest.

#### Typecheck

Inputs: `ResolvedPipeline`. Outputs: a `TypedPipeline` with every
edge's I/O compatibility verified per FM01 §2.6.

Invariant at exit: for every edge `(A → B)`, `A.produces` and
`B.consumes` are compatible. The DAG has no cycles.

Failure modes: incompatible kinds; cycle detected; ambiguous wiring
(multiple producers of the same kind feed one consumer without an
explicit `EdgeSpec`).

#### Init

Inputs: `TypedPipeline`. Outputs: an `InitializedPipeline` with each
stage's `init` hook (if any) called and any returned errors collected.

Invariant at exit: every stage that supplied an `init` has been
called exactly once and either succeeded or has been recorded as
failed-to-init.

Failure modes: a stage's `init` throws; configuration validation
fails (`configSchema` mismatch).

#### Execute

Inputs: `InitializedPipeline`. Outputs: a `RunResult`.

Invariant at exit: every reachable stage instance has either run to
completion, run partially with collected errors, or is recorded as
not-run (skipped because an upstream failed).

Failure modes: stage throws; cancellation; deadline exceeded.

#### Dispose

Inputs: `InitializedPipeline`. Outputs: cleanup completed.

Invariant at exit: every `dispose` hook has been called regardless of
outcome. The cache has been flushed. Watchers have been torn down.

Failure modes: a `dispose` throws — logged as a warning, never
escalated; the run already had a final outcome.

### 3.3 DAG construction

Given a `TypedPipeline` (resolve + typecheck phases complete), the
DAG is built as follows:

1. **Compute kind providers and consumers.** For every instance,
   record which kind it produces and which it consumes.
2. **Match by kind.** For each consumer, find the most recent
   declared producer of a compatible kind. "Most recent" is the
   instance appearing latest in the `stages` array before the
   consumer, with fallback to any prior instance if none follows.
3. **Honor explicit `wires`.** If `wires` are given, they override
   inference. Conflicts with inferred wires are an error unless the
   explicit wire is a strict subset.
4. **Verify acyclicity.** Topological sort; if it fails, error
   pointing at the offending cycle.
5. **Identify outputs.** Stages that produce `DeployArtifact`,
   `RequestHandler`, `Feed`, or `SearchIndex` and have no consumer
   are output sinks. They must each appear in `OutputSpec` if more
   than one exists.

Multiple consumers of one producer's output are allowed and treated
as fan-out. The producer's stream is replayed once per consumer in
the simple implementation; clever implementations may multiplex with
backpressure.

### 3.4 Type compatibility checking

Implements FM01 §2.6 verbatim:

1. Name match or subtype.
2. Major-version match, minor-version compatibility.
3. Discriminant equality if both declare one.
4. Constraint satisfaction with best-effort warnings on unrecognised
   constraint keys.

A failure produces a `TypecheckError` listing every incompatible edge,
not just the first. This matters for usability — users want to see
all wiring problems in one pass, not chase them one at a time.

---

## 4. Execution Model

### 4.1 Topological execution

The DAG is walked in topological order. Stages without unmet
dependencies run; their outputs feed dependent stages; the cycle
continues until every reachable stage is done.

The orchestrator maintains:

- A **ready queue** of instances whose dependencies have completed.
- An **in-flight set** of instances currently executing.
- A **completed set** of instances whose outputs are available.

When an instance completes, the orchestrator:

1. Records its outputs (in memory or in the cache).
2. Looks up its consumers.
3. For each consumer with all dependencies now satisfied, adds it to
   the ready queue.
4. Schedules from the ready queue up to `maxConcurrency`.

### 4.2 Streaming and fan-out

For stages that produce a `Stream<K>`, the orchestrator does not wait
for the stream to complete before scheduling consumers. Instead, each
emitted value is dispatched to the next stage as soon as it arrives.

A consumer that takes a single `K` (not a stream) is invoked **once
per produced value**, in parallel up to `maxConcurrency`. A consumer
that takes a `Stream<K>` is invoked **once**, with an `AsyncIterable`
that lazily yields the upstream's values.

Fan-out: if N stages consume the producer's output, the orchestrator
provides each with its own iterator. Implementations should multiplex
internally to avoid recomputing the producer.

### 4.3 Parallelism control

The `settings.maxConcurrency` cap applies across the whole pipeline.
A future stage-level annotation (open question §16) may override per
stage. Within a stage, concurrent invocations are independent — stages
must remain pure (FM01 §3.3).

### 4.4 Stream backpressure

The naive `AsyncIterable` semantics provide pull-based backpressure
— the consumer asks for the next value; the producer doesn't push.
This works as long as no intermediate stage buffers without bound.

Discipline:

- Stages MUST NOT accumulate buffered outputs beyond their declared
  buffer window (default: 64 items).
- A stage that needs to look at the entire stream first
  (e.g. a collector) declares so via `consumes: { ...kind, kind: "Stream" }`
  and is invoked once with the full iterable; it controls its own
  buffering.
- The orchestrator reads from a `Stream` lazily, never eagerly
  drains.

### 4.5 Resource limits

Each run has a deadline (`settings.deadlineMs`) past which the
orchestrator cancels everything in flight. Per-stage deadlines are
not in v0; if a single slow stage is the bottleneck, it shows up in
the metrics and gets fixed there.

Memory is not directly capped by the orchestrator — that's a host
concern (Tauri shell, CLI, CI). The orchestrator does emit a
`high_memory` event when an in-flight stage's reported memory exceeds
a configurable threshold, surfacing it for the host to act on.

---

## 5. Cache

### 5.1 What is cached

Per stage instance, the result of `run(input, config, ctx)` for a
given `(input, config)` pair. The cache stores the outputs, keyed by
a deterministic hash.

### 5.2 Cache key derivation

```
cache_key = blake3(
  "forme-cache-v1\0"  ||
  stage.name          ||
  "\0"                ||
  stage.version       ||
  "\0"                ||
  canonical_json(config)              ||
  "\0"                ||
  input_revision      ||  // RevisionId from FM01 §7
  "\0"                ||
  capability_set_hash    // sorted, joined declared capabilities
)
```

For a stage that consumes a `Stream`, `input_revision` is the BLAKE3
hash of the concatenated revisions of every streamed input.

For sources (which take `Void`), `input_revision` is the hash of
external state the source observed (e.g. for `source-fs`, the hash
of the directory listing including mtimes and per-file hashes).

### 5.3 Cache backends

```typescript
export interface CacheBackend {
  get(key: string): Promise<CacheEntry | null>;
  put(key: string, entry: CacheEntry): Promise<void>;
  invalidate(key: string): Promise<void>;
  /** Optional bulk invalidation by prefix (e.g. all entries for one stage). */
  invalidatePrefix?(prefix: string): Promise<void>;
  /** Garbage collect entries older than `olderThanMs`. */
  gc(olderThanMs: number): Promise<number>;
  dispose(): Promise<void>;
}

export interface CacheEntry {
  /** When this entry was written. */
  readonly writtenMs: number;
  /** Total size of the encoded payload. */
  readonly sizeBytes: number;
  /** The serialised payload — kind-specific encoder controlled. */
  readonly payload: Uint8Array;
  /** Hash of payload, for integrity verification. */
  readonly contentHash: string;
}

export const MemoryCache:     () => CacheBackend;
export const FilesystemCache: (root: string) => CacheBackend;
```

`MemoryCache` is the test default — fast, no persistence. `FilesystemCache`
stores under `cacheDir`, organised as `<key prefix>/<full key>` to keep
directories small. A future S3/R2-backed `RemoteCache` can implement the
interface for distributed builds; that lives in its own package, not
the kernel of FM03.

### 5.4 Serialisation

Each kind has a canonical encoder. For `ContentNode`, `Collection`,
`Asset`, the encoder is JSON for the metadata plus a byte payload for
binary data. For `RenderedPage`, the encoder includes the HTML, the
used-style and used-island lists, and a manifest of asset references.

All encoders are deterministic — same logical value, same bytes. This
is what makes the `contentHash` integrity check meaningful: a cache
read that doesn't decode to a value with the recorded hash is treated
as corruption and re-computed.

### 5.5 Cache invalidation

Three triggers:

1. **Key change.** Any change in stage version, config, input
   revision, or capability set produces a new key. The old entry stays
   until GC. This is the common "automatic" invalidation.
2. **Explicit invalidation.** `forme cache clear --stage <name>`
   invalidates by stage prefix. `forme cache clear --all` wipes
   everything.
3. **Time-based GC.** Entries older than `gcAge` (default 30 days) are
   purged on next run.

The orchestrator never writes a cache entry whose key already exists
with a different `contentHash` — that would indicate a non-deterministic
stage, which is a bug, and we surface it as a warning rather than
silently overwriting.

### 5.6 Cache scope

The default cache scope is **per pipeline directory** (one cache per
project). Optional scopes:

- **Per-user** — `~/.forme/cache` shared across projects. Useful for
  expensive shared work (image optimisation, syntax highlight).
- **Per-machine** — `/var/cache/forme` (Linux), `/Library/Caches/forme`
  (macOS). For shared developer machines.

Per-user and per-machine require explicit opt-in in `settings` because
they cross trust boundaries — a project's stages should not trust
results computed from a different project's inputs unless the keys
genuinely guarantee equivalence.

---

## 6. Incremental Rebuild

### 6.1 The mental model

A pipeline run produces a per-instance map:

```
instance_id → { input_revisions, output_revisions }
```

Stored alongside the cache, this lets the next run determine which
instances need re-execution: any instance whose input revisions
changed, plus everything downstream of those.

### 6.2 Change detection

On each run:

1. **Sources** rerun unconditionally (they observe external state).
2. For every other instance, compare its current input revisions
   against the stored set.
3. If any input revision differs → re-execute.
4. If every input revision matches → reuse cached output, mark as
   `cacheHits++`.

### 6.3 Partial DAG re-execution

The orchestrator computes the **affected set**: instances whose
inputs changed plus the transitive downstream closure. Only these run.
Untouched downstream instances pull their cached outputs, which
preserves byte-stability of the build artifact between runs that
modify only a subset.

This is the same algorithm used by the existing
`code/programs/go/build-tool/` package's affected-package detection,
adapted to per-instance granularity.

### 6.4 First-run behaviour

First run with an empty cache: every instance executes; the cache is
populated. Subsequent runs use the cache opportunistically.

### 6.5 Invalidation through revisions

Because revisions are content-addressed (FM01 §7.3), incremental
rebuild is naturally rename-safe and reproducible. A file moved from
`a.md` to `b.md` with no content change has the same `RevisionId` →
its downstream stages don't re-run.

A file edited but renamed back to the original location has a new
`RevisionId` → downstream re-runs.

This is one of the subtle wins of the identity scheme: rename
churn (which is common in active editing) does not blow up build times.

---

## 7. Watch Mode

### 7.1 What watching does

`forme watch` runs a pipeline once, then keeps a watcher attached to
every source's storage root. When source files change, it triggers a
rebuild.

A `WatchSession` exposes:

```typescript
session.results()      // AsyncIterable<RunResult>, one per build
session.rebuild()      // force a build now
session.stop()         // tear down the session
```

### 7.2 Source watching

For `source-fs`-shaped stages, the orchestrator subscribes to the
`StorageApi.watch` stream (FM01 §4.8.1). The watch stream yields
`StorageChange` events; the orchestrator translates them into a
"these source identities are dirty" set.

For sources that don't support watching (database sources, hosted
CMS), watch mode falls back to polling at `settings.pollIntervalMs`.

### 7.3 Debouncing

Edits often arrive in bursts (file save → editor re-saves → linter
modifies). The orchestrator debounces: changes within
`settings.debounceMs` (default 200 ms) are coalesced into a single
rebuild trigger.

### 7.4 Live preview integration

The dev server (`forme-dev-server`, defined in FM07) subscribes to a
`WatchSession`'s results and serves them. When a build completes, the
dev server pushes a hot-reload signal to connected browsers via
WebSocket; static asset URLs include content hashes so cache
invalidation is automatic.

### 7.5 Watch mode and the cache

Watch mode uses the **memory cache** in front of the persistent
backend by default — this keeps successive incremental builds fast
without I/O on every cache check. The persistent cache is still
written to in the background so that a CLI run after the watch
session can reuse the work.

---

## 8. Reproducible Builds

A reproducible build means: given identical inputs, the build
produces byte-identical outputs.

Enable with `settings.reproducibleBuild = true`. Effects:

1. **Time freezes.** `ctx.time.nowMs()` and `nowIso()` return a fixed
   timestamp for the entire run. The fixed value is the input
   pipeline's max input mtime, falling back to `0` if no inputs have
   timestamps.
2. **Iteration order is sorted.** Sources iterate paths in
   lexicographic order, not filesystem order.
3. **Hashes use deterministic encoders.** Already true for all kernel
   serialisers; reproducible mode just verifies it.
4. **Random seeds are deterministic.** Stages that need randomness
   (e.g. for jitter) read from `ctx.random.deterministic(name)` which
   seeds from the cache key. This is an addition to `StageContext`
   that this spec adds; the rationale is in §16.
5. **Telemetry is suppressed.** No timestamps in output artifacts,
   no per-run identifiers. Telemetry events still fire internally but
   are not embedded in artifacts.

A test suite for reproducibility runs every reference pipeline twice
in reproducible mode and diffs every artifact byte-for-byte. CI fails
on any drift.

---

## 9. Error Handling

### 9.1 Per-stage error boundaries

Each stage invocation runs inside a try/catch that:

1. Catches anything thrown synchronously or in a returned Promise.
2. Wraps non-`StageError`s in `StageError { code: "UNCAUGHT", cause }`.
3. Tags the error with `stageName`, `inputId`, `inputPath` if not
   already present.
4. Records the error against the instance's `StageRunSummary`.
5. Either escalates (fail-fast) or continues (best-effort) per
   `settings.bestEffort`.

### 9.2 Fail-fast (default)

A non-recoverable `StageError` immediately:

1. Triggers the run's `cancellation` token.
2. Allows in-flight stages to finish their current invocation (no
   forced kill).
3. Skips any not-yet-started stages.
4. Calls `dispose` on every initialised stage.
5. Reports `RunResult { outcome: "failed", errors: [...] }`.

### 9.3 Best-effort

`settings.bestEffort = true` changes the policy:

1. Recoverable errors are recorded but do not halt the pipeline.
2. Non-recoverable errors halt only the affected branch — downstream
   instances of that input are skipped, but other inputs continue.
3. The run completes with `outcome: "partial"` if any errors occurred.
4. Outputs are produced for the inputs that succeeded.

This is the right mode for editor preview ("let me see most of the
site even if one post is broken") and the wrong mode for CI ("any
failure must block").

### 9.4 Capability errors

`CapabilityError` always escalates — it indicates a misconfigured
manifest, not a runtime problem. Best-effort does not soften it.

### 9.5 Error reporting shape

Errors are surfaced in `RunResult.errors` and through structured
logs. Each error carries:

- `code` (machine-readable)
- `message` (human-readable)
- `stageName`, `instanceId`
- `inputId`, `inputPath` if applicable
- `recoverable` flag
- Optional `fields` for stage-specific context
- `cause` (the original throw, if wrapped)

Drivers (CLI, dev server, editor) format these for their own UIs.
The orchestrator does not pretty-print; that is the driver's job.

---

## 10. Cancellation

### 10.1 Token plumbing

A run holds a single `CancellationToken` (FM01 §4.4) created by:

- The CLI's SIGINT handler (Ctrl-C from the user)
- The watch mode's "abandon previous build" logic
- The deadline timer
- An explicit error escalation

Every `StageContext` constructed during a run carries the same token.
Stages are expected to honour it at safe points.

### 10.2 Cleanup

When cancellation fires:

1. The orchestrator stops scheduling new invocations.
2. In-flight stages have their token's `cancelled` flag set to true.
3. Stages that called `throwIfCancelled()` throw `CancellationError`
   and unwind.
4. Stages that didn't check are allowed to complete their current
   invocation (we don't kill a hung stage; that's a deadline matter).
5. `dispose` runs for every initialised stage.
6. The run reports `outcome: "cancelled"` with a non-empty
   `errors` array containing the cancellation reason.

### 10.3 Partial outputs

A cancelled run may have produced partial cached results before
cancellation. These are kept in the cache (their keys are
deterministic; revisiting will reuse them). What is *not* produced
is a `DeployArtifact` — emitters check cancellation early and
short-circuit.

---

## 11. Observability

### 11.1 Logger

The orchestrator routes `ctx.logger` calls through the
`OrchestratorOptions.logger`. Default: a structured-JSON logger to
stderr.

Conventions:

- Stages log at `info` for milestones, `debug` for internals,
  `warn` for non-fatal issues, `error` only for fatal-to-this-input
  problems.
- The orchestrator wraps every stage invocation with a `child`
  logger carrying `{ stage, instance, inputId }` so stage logs are
  correlatable.

### 11.2 Per-stage metrics

`StageRunSummary` is the canonical surface (§3.1). Drivers may render
it as a CLI table, a watch-mode dashboard, or an editor-side timing
overlay. Underlying counters maintained by the orchestrator:

```
items_consumed
items_produced
elapsed_ms
cache_hits
cache_misses
errors_total
errors_recoverable
errors_fatal
```

### 11.3 Trace export

Optionally, the orchestrator emits OpenTelemetry traces. Each stage
invocation is a span; the run is the root span. Spans carry the same
fields as the structured logs. The exporter target is configured via
`OrchestratorOptions.telemetry`.

This is opt-in. By default the orchestrator emits nothing.

---

## 12. Plugin Host Integration (Preview)

Full plugin-host design is FM02. The orchestrator's relationship to
the host is narrow:

```typescript
export interface PluginHost {
  /** Resolve a stage reference to a loaded Stage. */
  loadStage(ref: StageRef): Promise<Stage<KindDescriptor, KindDescriptor>>;

  /** Verify a capability grant is consistent with the plugin's manifest. */
  validateCapability(
    stage: Stage<KindDescriptor, KindDescriptor>,
    capability: Capability
  ): Promise<void>;

  /** Construct a sandbox-wrapped StageContext given declared capabilities. */
  buildContext(
    stage: Stage<KindDescriptor, KindDescriptor>,
    capabilities: readonly Capability[],
    runtimeCtx: RuntimeContext
  ): StageContext;
}
```

The orchestrator never reaches inside a stage's package boundary or
sandbox. It calls `loadStage`, `validateCapability`, and `buildContext`
— that's the entire surface.

When `pluginHost` is omitted (TypeScript-only flows where stages are
imported directly), the orchestrator uses an internal
`DefaultDirectImportHost` that:

- Treats every import as already-loaded and trusted.
- Validates capabilities against `stage.capabilities` with no
  manifest involved.
- Builds an unsandboxed `StageContext` (no `vm`, no `Worker`).

This default is appropriate for the v0 dogfood — first-party stages
the developer wrote — and is **never** appropriate for third-party
plugins. The CLI refuses to use it when any `StageRef` (vs direct
import) appears in the config.

---

## 13. Package Layout

Three new npm packages under `code/packages/typescript/`. Each
follows the repo standards (`package.json`, `BUILD`, `BUILD_windows`,
`README.md`, `CHANGELOG.md`).

### 13.1 `@coding-adventures/forme-pipeline-config`

Pure types and parsers for pipeline configs. Depends only on
`forme-types` and `forme-errors`.

- `src/config-types.ts` — `PipelineConfig`, `PipelineSettings`,
  `StageInstanceSpec`, `StageRef`, `EdgeSpec`, `OutputSpec`
- `src/parse-toml.ts` — `parseTomlConfig(text: string): PipelineConfig`
- `src/parse-ts.ts` — `loadTsConfig(path: string): Promise<PipelineConfig>`
- `src/validate.ts` — `validateConfig(config: PipelineConfig, host?): Promise<void>`
- `src/errors.ts` — `ConfigError`

### 13.2 `@coding-adventures/forme-cache`

The cache interface and built-in backends.

- `src/cache-backend.ts` — `CacheBackend`, `CacheEntry`
- `src/memory-cache.ts` — `MemoryCache`
- `src/filesystem-cache.ts` — `FilesystemCache`
- `src/keys.ts` — `cacheKey(...)`, `capabilitySetHash(...)`
- `src/codecs/` — per-kind encoders
- `src/integrity.ts` — `verifyEntry`

Depends on `forme-types`, `forme-identity`, `forme-errors`.

### 13.3 `@coding-adventures/forme-orchestrator`

The runtime that ties the kernel, the config, and the cache together.

- `src/orchestrator.ts` — `createOrchestrator`, `Orchestrator`
- `src/build-pipeline.ts` — resolve + typecheck + DAG construction
- `src/typecheck.ts` — kind compatibility checking
- `src/dag.ts` — `PipelineDag` data structure
- `src/scheduler.ts` — topological execution + parallelism
- `src/streaming.ts` — fan-out, multiplexing, backpressure
- `src/incremental.ts` — affected-set computation
- `src/watch.ts` — `WatchSession`
- `src/repro-build.ts` — reproducible-build mode
- `src/error-handling.ts` — boundaries + reporting
- `src/default-host.ts` — `DefaultDirectImportHost`
- `src/types.ts` — public API types

Depends on `forme-types`, `forme-stage`, `forme-capability`,
`forme-identity`, `forme-errors`, `forme-pipeline-config`,
`forme-cache`. Optionally on FM02's plugin host package when it
exists.

### 13.4 Dependency graph addition to FM01

```
                               forme-types  ◄── (unchanged)
                                    ▲
                                    │
              ┌─────────────────────┼─────────────────────┐
              │                     │                     │
       forme-pipeline-       forme-cache          forme-orchestrator
            config                                      ▲
                                                        │
                                          (depends on all above
                                           plus all FM01 packages)
```

### 13.5 BUILD ordering

Per `lessons.md` 2026-04-21, leaf-to-root in `BUILD` and
`BUILD_windows`:

```
forme-types → forme-errors → forme-identity → forme-pipeline-config
                                              forme-cache
                                              forme-stage
                                              forme-capability
                                              forme-manifest
                                                    ▼
                                              forme-orchestrator
```

---

## 14. Testing Contract

### 14.1 `forme-pipeline-config`

- Round-trip: `parseTomlConfig(serialiseTomlConfig(c)) === c` for
  representative configs.
- TS form: `loadTsConfig(...)` correctly imports and validates.
- Validation: every documented rejection reason has a test.
- Type-level tests: the `PipelineConfig` shape is preserved across
  versions.

### 14.2 `forme-cache`

- Backend conformance: every backend (`MemoryCache`, `FilesystemCache`)
  passes a shared conformance suite — `get`, `put`, `invalidate`,
  `gc`.
- Key derivation: same `(stage, config, input_revision, capabilities)`
  always yields the same key; any change yields a different key.
- Codec round-trip: every kind serialises and deserialises losslessly.
- Integrity: a mutated payload fails `verifyEntry`.

### 14.3 `forme-orchestrator`

Unit tests:

- DAG construction: every documented inference rule.
- Typecheck: every compatibility rule, every documented rejection.
- Scheduler: parallelism respects `maxConcurrency`; readiness
  cascades correctly; cancellation propagates.
- Streaming: fan-out delivers to multiple consumers; backpressure
  prevents unbounded memory.
- Cache: hits skip execution; misses populate; integrity failures
  re-execute.
- Incremental: affected-set is exactly the changed-and-downstream
  set.
- Watch: changes coalesce per `debounceMs`; unsubscribe stops watcher.
- Errors: fail-fast halts and disposes; best-effort continues and
  reports.
- Repro: two consecutive runs in repro mode produce byte-identical
  artifacts.

Integration tests:

- A reference Astro-shape pipeline (`source-fs → parse-markdown
  → collect-chronological → render-static → emit-fs`) with a
  fixture content directory builds successfully and produces
  the expected `dist/` contents.
- Modify one input file, re-run with cache, verify only affected
  stages execute.
- Cancel mid-run, verify clean teardown.
- Fail one input in best-effort mode, verify other inputs complete.

### 14.4 Coverage target

≥ 95% line and branch across all three packages. The scheduler is the
trickiest area; property tests using `fast-check` over random DAGs
are encouraged.

---

## 15. Examples

### 15.1 Smallest possible pipeline

Two stages, no streaming, no fan-out, no cache.

```typescript
import { createOrchestrator } from "@coding-adventures/forme-orchestrator";
import { Kinds, defineStage } from "@coding-adventures/forme-types";

const greetSource = defineStage({
  name:        "greet-source",
  version:     "0.1.0",
  apiVersion:  1,
  description: "produces a single greeting source",
  consumes:    Kinds.Void,
  produces:    Kinds.ContentSource,
  capabilities: [],
  configSchema: null,
  async run() {
    return {
      kind: "ContentSource",
      path: "greeting.txt",
      bytes: new TextEncoder().encode("hello world"),
      mimeType: "text/plain",
      identity: "greeting" as any,
      revision: "blake3:00" as any,
      providerMeta: {},
    };
  },
});

const printSink = defineStage({
  name:        "print-sink",
  version:     "0.1.0",
  apiVersion:  1,
  description: "logs the source content",
  consumes:    Kinds.ContentSource,
  produces:    Kinds.Void,
  capabilities: [],
  configSchema: null,
  async run(source, _config, ctx) {
    ctx.logger.info("got source", { path: source.path });
  },
});

const orchestrator = createOrchestrator({});
const pipeline = await orchestrator.buildPipeline({
  name: "smallest",
  settings: {
    storageRoot: ".",
    cacheDir: null,
    reproducibleBuild: false,
    maxConcurrency: 1,
    logLevel: "info",
    bestEffort: false,
    deadlineMs: null,
  },
  stages: [
    { stage: greetSource },
    { stage: printSink },
  ],
});

const result = await orchestrator.runOnce(pipeline);
console.log(result.outcome); // "success"
```

### 15.2 A blog pipeline (real)

```typescript
// forme.config.ts

import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";

import sourceFs        from "@forme/source-fs";
import parseMarkdown   from "@forme/parse-markdown";
import transformHl     from "@forme/transform-syntax-highlight";
import transformAlh    from "@forme/transform-autolink-headings";
import collectChrono   from "@forme/collect-chronological";
import renderStatic    from "@forme/render-static";
import feedRss         from "@forme/feed-rss";
import feedSitemap     from "@forme/feed-sitemap";
import emitFs          from "@forme/emit-fs";

export default {
  name: "my-blog",
  settings: {
    storageRoot:       "./content",
    cacheDir:          "./.forme/cache",
    reproducibleBuild: false,
    maxConcurrency:    null,
    logLevel:          "info",
    bestEffort:        false,
    deadlineMs:        300_000,
  },
  stages: [
    { stage: sourceFs,      config: { glob: "posts/**/*.md" } },
    { stage: parseMarkdown, config: { gfm: true } },
    { stage: transformHl,   config: { theme: "github-dark" } },
    { stage: transformAlh },
    { stage: collectChrono, config: { route: "/posts/:slug" } },
    { stage: renderStatic,  config: { theme: "default" } },
    { stage: feedRss,       config: { out: "/feed.xml", title: "My Blog" } },
    { stage: feedSitemap,   config: { out: "/sitemap.xml" } },
    { stage: emitFs,        config: { out: "./dist" } },
  ],
} satisfies PipelineConfig;
```

Run via `forme run` (or `forme watch` for dev).

### 15.3 Multi-output: web + email + RSS from one source

```typescript
import sourceFs        from "@forme/source-fs";
import parseMarkdown   from "@forme/parse-markdown";
import collectChrono   from "@forme/collect-chronological";
import renderStatic    from "@forme/render-static";
import renderEmail     from "@forme/render-email";
import feedRss         from "@forme/feed-rss";
import emitFs          from "@forme/emit-fs";
import emitMailgun     from "@forme/emit-email-campaign";

export default {
  name: "newsletter-and-blog",
  settings: { /* … */ },
  stages: [
    { stage: sourceFs,      config: { glob: "posts/**/*.md" } },
    { stage: parseMarkdown, config: { gfm: true } },
    { stage: collectChrono, id: "collected" },

    // Web fork
    { stage: renderStatic, id: "render-web",  config: { theme: "default" } },
    { stage: emitFs,                         config: { out: "./dist" } },

    // Email fork — same upstream content, different render+emit
    { stage: renderEmail,  id: "render-mail", config: { theme: "newsletter" } },
    { stage: emitMailgun,                    config: { domain: "mg.example.com" } },

    // RSS feed (any single subscriber to `collected`)
    { stage: feedRss,                        config: { out: "/feed.xml" } },
  ],
  outputs: [
    { fromInstance: "render-web",  name: "web" },
    { fromInstance: "render-mail", name: "email" },
  ],
} satisfies PipelineConfig;
```

Three consumers of `collected` (`render-web`, `render-mail`,
`feedRss`); the orchestrator delivers the collection to each. No
duplication of content, no duplication of pipeline plumbing.

### 15.4 Watch mode

```typescript
import { createOrchestrator } from "@coding-adventures/forme-orchestrator";
import config from "./forme.config";

const orchestrator = createOrchestrator({});
const pipeline = await orchestrator.buildPipeline(config);

const session = orchestrator.watch(pipeline);

for await (const result of session.results()) {
  console.log(`build ${result.outcome} in ${result.elapsedMs}ms`);
  if (result.errors.length) {
    for (const err of result.errors) {
      console.error(err.message);
    }
  }
}
```

---

## 16. Open Questions

1. **Per-stage concurrency annotation.** Today `maxConcurrency` is a
   pipeline-wide setting. Some stages benefit from "I am IO-bound,
   run me at 16x" while others want "I am CPU-bound, cap me at 4."
   Add a `Stage.concurrency: number | "unlimited"` field? Defer to
   v1 unless real bottleneck.
2. **`ctx.random`.** Reproducible builds need deterministic randomness
   when stages need any. The §8 sketch reads from `cacheKey`. Worth
   a tighter spec — exact API, seeding contract.
3. **Cross-pipeline cache sharing.** The per-user cache scope is
   noted but not specified in detail. How do stages opt in?
4. **Distributed builds.** A `RemoteCacheBackend` plus a
   `RemoteScheduler` would let a CI matrix run stages on different
   machines. Out of scope for v0; the cache backend interface is
   designed to allow it.
5. **Configuration evolution.** What does it look like to ship a
   change to `PipelineConfig` shape itself? The TS form gets
   compiler help; the TOML form needs an explicit migration story.
   Not v0.
6. **Streaming over the network.** A `Stream<RenderedPage>` could in
   principle fan out to remote workers. Same answer as #4.
7. **Error grouping for editor UIs.** Editor surfaces want errors
   grouped by file, by type, by severity. Today the `RunResult`
   exposes flat `errors`. A grouping helper in a separate package
   (`forme-error-grouping`?) probably belongs outside the orchestrator.
8. **Backpressure with concrete numerical tuning.** §4.4 says "default
   buffer of 64 items" — this is a guess. Empirical benchmarks needed.
9. **Watch mode re-runs in best-effort by default?** Probably yes;
   a single broken file shouldn't blank the preview. Make it explicit.
10. **Determinism of fan-out ordering.** When N consumers receive
    items from a single producer, do they all see the items in the
    same order? They should, but the simple multiplexer may not
    guarantee it. Codify.

---

## 17. Success Criteria

FM03 is complete when:

1. **All three packages exist** under `code/packages/typescript/forme-*`,
   each with `package.json`, `BUILD`, `BUILD_windows`, `README.md`,
   `CHANGELOG.md`.
2. **Test coverage exceeds 95%** across the three packages.
3. **The reference blog pipeline (§15.2) builds successfully** with
   a fixture content directory and produces the expected `dist/`
   layout.
4. **Two consecutive reproducible-build runs produce
   byte-identical outputs** across every artifact.
5. **Incremental rebuild correctness** — the integration test that
   modifies one fixture file confirms only the affected stages
   re-run; cached results are reused.
6. **Watch mode debounces correctly** — a burst of fixture writes
   yields one rebuild, not many.
7. **`MemoryCache` and `FilesystemCache` both pass the shared
   conformance suite.**
8. **Cancellation tested end-to-end** — a long-running fixture
   cancels cleanly with all `dispose` hooks called.
9. **Best-effort mode works** — the integration test that breaks one
   input verifies the others succeed and the `RunResult` reports
   `outcome: "partial"`.
10. **Documentation** is complete enough that FM06 (AOT compiler) can
    reference orchestrator services without supplementary text.

---

## Appendix A — `PipelineDag` data structure

```typescript
export interface PipelineDag {
  /** All instances, keyed by ID. */
  readonly instances: ReadonlyMap<string, ResolvedInstance>;
  /** Producer → consumers. */
  readonly forward: ReadonlyMap<string, readonly string[]>;
  /** Consumer → producers. */
  readonly backward: ReadonlyMap<string, readonly string[]>;
  /** Topological order (one valid linearisation). */
  readonly topoOrder: readonly string[];
  /** Stages that produce a final output (no consumers). */
  readonly sinks: readonly string[];
  /** Stages that have no producer (sources). */
  readonly sources: readonly string[];
}

export interface ResolvedInstance {
  readonly id: string;
  readonly stage: Stage<KindDescriptor, KindDescriptor>;
  readonly config: unknown;
  readonly capabilities: readonly Capability[];
}
```

---

## Appendix B — Glossary

Terms introduced in this spec; see FM00 Appendix B and FM01
Appendix B for the broader Forme vocabulary.

- **Affected set** — instances whose inputs changed plus their
  transitive downstream closure; the set the orchestrator re-executes
  during incremental rebuild.
- **Cache backend** — a pluggable implementation of `CacheBackend`.
  Built-ins: `MemoryCache`, `FilesystemCache`.
- **Driver** — a thin adapter wrapping the orchestrator for a specific
  host (CLI, dev server, editor, CI).
- **DAG** — directed acyclic graph; the structural form of a pipeline.
- **Edge** — a typed connection from one instance's output to another's
  input.
- **Pipeline** — a constructed DAG of stage instances ready to run.
- **PipelineConfig** — the user-authored description that becomes a
  pipeline after resolution.
- **Run** — a single end-to-end execution of a pipeline.
- **Sink** — an instance with no consumers; its output is a final
  artifact.
- **Source** — an instance with no producers; reads external state.
- **StageInstance** — a stage definition plus configuration for one
  particular use of it.
- **Watcher** — the per-source change subscription that drives
  incremental rebuild and live preview.

---

## Appendix C — Pointers to sibling specs

- **FM00** — Forme vision
- **FM01** — Kernel: types, kinds, stages, capabilities, identity, manifest
- **FM02** (next) — Plugin host: loading, sandboxing, extension registry
- **FM04** — Style IR
- **FM05** — Interactivity IR
- **FM06** — AOT compiler
- **FM07** — Dev server, CLI, and shell integration

## Appendix D — This is a living document

Like FM00 and FM01, FM03 evolves as implementation lands. Where
running code disagrees with this spec, the code wins and the spec is
updated; the history of the tension is part of the project's record.
