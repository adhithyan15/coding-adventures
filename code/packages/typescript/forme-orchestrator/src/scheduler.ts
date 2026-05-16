/**
 * Pipeline scheduler — executes a DAG sequentially in topological order.
 *
 * v0 simplifications (deferred to follow-up):
 *
 *   - **Sequential execution.**  Each stage runs to completion before
 *     the next starts.  `settings.maxConcurrency` is honoured at "1".
 *     Real parallelism for fan-out / fan-in lands in v1.
 *
 *   - **No streaming pipelining.**  When a stage produces a Stream<X>,
 *     we drain it fully into memory before passing per-value to the
 *     downstream consumer.  This is correct (the consumer is invoked
 *     once per value) but not lazy — large streams allocate fully.
 *     Lazy streaming lands in v1 alongside parallelism.
 *
 *   - **No incremental rebuild.**  Every run executes every stage;
 *     the cache backend exists but isn't hit yet.  Incremental
 *     rebuild (FM03 §6) lands when the orchestrator gains revision
 *     tracking per instance.
 *
 *   - **Reproducible-build mode is wired through.**  When
 *     `settings.reproducibleBuild = true`, every StageContext receives
 *     a frozenClock pinned at `REPRO_BUILD_FROZEN_TIMESTAMP_MS` (0
 *     in v0; FM03 §8 max-input-mtime derivation pending source-stage
 *     revision tracking).  Iteration-order sorting and the
 *     deterministic-random `ctx.random` API remain deferred to v1.
 *
 * What v0 *does* implement:
 *
 *   - Topological execution
 *   - Per-stage StageContext construction with denied-by-default
 *     capability APIs
 *   - init/dispose lifecycle hooks
 *   - Fail-fast and best-effort error handling
 *   - Cancellation propagation
 *   - Per-stage timing + error counts in StageRunSummary
 */

import {
  CancellationError,
  StageError,
} from "@coding-adventures/forme-errors";
import {
  consoleLogger,
  createCancellationTokenSource,
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
  frozenClock,
  inMemoryCache,
  inMemoryEventBus,
  noOpTelemetryEmitter,
  systemClock,
} from "@coding-adventures/forme-stage";
import type {
  CancellationToken,
  Clock,
  Logger,
  StageContext,
  StageInitContext,
} from "@coding-adventures/forme-stage";
import type { JsonValue } from "@coding-adventures/forme-types";
import type { ResolvedInstance, PipelineDag } from "./dag.js";
import type { RunError, RunOutcome, StageRunSummary } from "./types.js";

/** Per-instance run state held during execution. */
interface RunState {
  /** Output value from the stage's run().  For streams, the fully-drained array. */
  output: unknown;
  /** Whether output is from a Stream<X> producer (consumers iterate). */
  isStreamOutput: boolean;
  /** init() was called — must dispose. */
  initialized: boolean;
  /** Per-stage summary accumulator. */
  summary: {
    instanceId: string;
    stageName: string;
    itemsConsumed: number;
    itemsProduced: number;
    elapsedMs: number;
    cacheHits: number;
    cacheMisses: number;
    outcome: "success" | "skipped" | "failed";
    errorCount: number;
  };
}

export interface SchedulerOptions {
  readonly logger: Logger;
  readonly cancellation: CancellationToken;
  readonly bestEffort: boolean;
  /**
   * When true, every StageContext receives a `frozenClock` whose wall
   * time is fixed for the duration of the run.  Combined with the
   * existing determinism in stage I/O (no module-level state, no
   * ambient I/O, identity-by-content), this lets two runs of the same
   * pipeline against the same inputs produce byte-identical outputs
   * (FM03 §8).
   *
   * v0 of reproducible mode pins time only.  Iteration-order sorting
   * (FM03 §8 item 2) lives in source stages; randomness gating (item
   * 4) requires a deterministic `ctx.random` which is itself FM01
   * future work.  Telemetry suppression (item 5) is handled by the
   * orchestrator-wide `telemetry` option, not here.
   *
   * Default: false.
   */
  readonly reproducibleBuild?: boolean;
}

/**
 * Fixed wall-clock timestamp emitted by the reproducible-build clock.
 *
 * Per FM03 §8: "the fixed value is the input pipeline's max input
 * mtime, falling back to 0 if no inputs have timestamps."  v0 always
 * uses the fallback (0) because the orchestrator doesn't yet thread
 * input mtimes from sources to here.  When source-fs's revision
 * tracking lands, this constant becomes an input-derived value; the
 * `frozenClock` factory is unchanged.
 *
 * 0 = midnight UTC on 1970-01-01.  Exposed so tests can assert on
 * the exact value the orchestrator's frozen clock returns.
 */
export const REPRO_BUILD_FROZEN_TIMESTAMP_MS = 0;

export interface SchedulerResult {
  readonly outcome: RunOutcome;
  readonly outputs: Map<string, unknown>;
  readonly summaries: readonly StageRunSummary[];
  readonly errors: readonly RunError[];
}

/**
 * Execute the DAG.  Returns the per-instance outputs (keyed by
 * instance id), per-stage summaries, and any collected errors.
 *
 * On fail-fast error: cancels remaining work, runs `dispose` on
 * everything that was initialised, returns outcome = "failed".
 *
 * On cancellation: returns outcome = "cancelled".
 *
 * On best-effort with errors: returns outcome = "partial" and
 * continues past recoverable failures.
 */
export async function executeDag(
  dag: PipelineDag,
  options: SchedulerOptions,
): Promise<SchedulerResult> {
  const states = new Map<string, RunState>();
  const errors: RunError[] = [];
  const outputs = new Map<string, unknown>();

  // Choose a clock factory once for the whole run.  Reproducible-build
  // mode hands every stage a frozenClock; otherwise systemClock.
  const newClock = clockFactory(options);

  // Init pass: call init() on every stage that has one.  If any init
  // throws, we abort before any run() is called and surface the failure.
  for (const id of dag.topoOrder) {
    const inst = dag.instances.get(id)!;
    states.set(id, makeState(inst));
    if (typeof inst.stage.init !== "function") continue;
    const initCtx: StageInitContext = makeInitContext(inst, options, newClock);
    try {
      await inst.stage.init(inst.config, initCtx);
      states.get(id)!.initialized = true;
    } catch (err) {
      // Init failure → fail the whole run (no per-input concept yet).
      const re = toRunError(err, inst);
      errors.push(re);
      await disposeAll(dag, states, options);
      const summaries = Array.from(states.values()).map(s => ({
        ...s.summary,
        outcome: "failed" as const,
      }));
      return { outcome: "failed", outputs, summaries, errors };
    }
  }

  // Execute pass.
  let cancelled = false;
  let anyRecoverableErrors = false;
  let anyFatal = false;

  for (const id of dag.topoOrder) {
    if (options.cancellation.cancelled) {
      cancelled = true;
      states.get(id)!.summary.outcome = "skipped";
      continue;
    }
    if (anyFatal) {
      // Skip downstream stages after a fatal error.
      states.get(id)!.summary.outcome = "skipped";
      continue;
    }
    const state = states.get(id)!;
    const inst = dag.instances.get(id)!;
    const startMonotonic = options.cancellation.cancelled ? 0 : Date.now();

    try {
      // Sources have no input; non-sources read from their producer's
      // output (which we previously stored in `outputs`).
      const inputs = collectInputs(inst, states);
      const ctx: StageContext = makeRunContext(inst, options, newClock);

      if (inst.stage.consumes.name === "Stream" || inst.stage.consumes.name === "Void"
          || isSingleProducer(inst, dag, states)) {
        // One invocation: source / collector-style / single-input.
        const result = await inst.stage.run(inputs.value as never, inst.config, ctx);
        const stored = await materialize(result, inst.stage.produces.name === "Stream");
        state.output = stored.value;
        state.isStreamOutput = stored.isStream;
        state.summary.itemsConsumed = inputs.itemCount;
        state.summary.itemsProduced = stored.isStream
          ? (stored.value as unknown[]).length
          : 1;
      } else {
        // Multi-invocation: producer was a stream, consumer takes one
        // value at a time.  Iterate over the producer's drained array.
        // Mark the result stream-shaped so downstream consumers
        // iterate again — semantically this stage produced N values.
        const list = inputs.value as unknown[];
        const collected: unknown[] = [];
        for (const item of list) {
          options.cancellation.throwIfCancelled();
          const r = await inst.stage.run(item as never, inst.config, ctx);
          const sub = await materialize(r, false);
          collected.push(sub.value);
        }
        state.output = collected;
        state.isStreamOutput = true;
        state.summary.itemsConsumed = list.length;
        state.summary.itemsProduced = collected.length;
      }
      state.summary.outcome = "success";
    } catch (err) {
      if (err instanceof CancellationError) {
        cancelled = true;
        state.summary.outcome = "skipped";
        continue;
      }
      const runError = toRunError(err, inst);
      errors.push(runError);
      state.summary.outcome = "failed";
      state.summary.errorCount = 1;
      if (runError.recoverable && options.bestEffort) {
        anyRecoverableErrors = true;
        // Continue to next stage; downstream stages that need this
        // output will see undefined and will likely fail too — but
        // best-effort is best-effort.
      } else {
        // Fail-fast OR non-recoverable best-effort: stop scheduling more.
        anyFatal = true;
      }
    } finally {
      state.summary.elapsedMs = Date.now() - startMonotonic;
    }
  }

  // Sinks → outputs map (keyed by instance id; OutputSpec naming
  // happens in the run.ts wrapper that knows about the config).
  for (const sinkId of dag.sinks) {
    const state = states.get(sinkId)!;
    if (state.summary.outcome === "success") {
      outputs.set(sinkId, state.output);
    }
  }

  // Always dispose, regardless of outcome.
  await disposeAll(dag, states, options);

  const outcome: RunOutcome = cancelled
    ? "cancelled"
    : anyFatal
      ? "failed"
      : anyRecoverableErrors
        ? "partial"
        : "success";

  return {
    outcome,
    outputs,
    summaries: dag.topoOrder.map(id => ({ ...states.get(id)!.summary })),
    errors,
  };
}

// ─── Helpers ──────────────────────────────────────────────────────────────

function makeState(inst: ResolvedInstance): RunState {
  return {
    output: undefined,
    isStreamOutput: false,
    initialized: false,
    summary: {
      instanceId: inst.id,
      stageName: inst.stage.name,
      itemsConsumed: 0,
      itemsProduced: 0,
      elapsedMs: 0,
      cacheHits: 0,
      cacheMisses: 0,
      outcome: "success", // mutated below
      errorCount: 0,
    },
  };
}

function isSingleProducer(
  inst: ResolvedInstance,
  dag: PipelineDag,
  states: Map<string, RunState>,
): boolean {
  if (inst.producer === null) return true;
  const prod = states.get(inst.producer);
  if (!prod) return true;
  // If producer's output isn't a stream, this consumer takes one value.
  return !prod.isStreamOutput;
}

interface CollectedInput {
  value: unknown;
  itemCount: number;
}

function collectInputs(
  inst: ResolvedInstance,
  states: Map<string, RunState>,
): CollectedInput {
  if (inst.producer === null) {
    return { value: undefined, itemCount: 0 };
  }
  const prod = states.get(inst.producer);
  if (!prod) return { value: undefined, itemCount: 0 };
  if (prod.isStreamOutput) {
    // Stream-typed input: pass the array.  Consumer is a collector.
    if (inst.stage.consumes.name === "Stream") {
      const list = prod.output as unknown[];
      return { value: makeAsyncIterableFromArray(list), itemCount: list.length };
    }
    // Stream-producer feeding single-input consumer: caller iterates.
    const list = prod.output as unknown[];
    return { value: list, itemCount: list.length };
  }
  return { value: prod.output, itemCount: 1 };
}

async function materialize(
  result: unknown,
  expectStream: boolean,
): Promise<{ value: unknown; isStream: boolean }> {
  if (isAsyncIterable(result)) {
    const collected: unknown[] = [];
    for await (const item of result as AsyncIterable<unknown>) {
      collected.push(item);
    }
    return { value: collected, isStream: true };
  }
  if (result instanceof Promise) {
    const v = await result;
    return materialize(v, expectStream);
  }
  return { value: result, isStream: false };
}

function isAsyncIterable(v: unknown): boolean {
  return typeof v === "object" && v !== null
    && typeof (v as { [Symbol.asyncIterator]?: unknown })[Symbol.asyncIterator] === "function";
}

function makeAsyncIterableFromArray<T>(arr: readonly T[]): AsyncIterable<T> {
  return {
    async *[Symbol.asyncIterator]() {
      for (const item of arr) yield item;
    },
  };
}

/**
 * Pick the clock factory based on reproducible-build mode.  Lazy-
 * builds a single frozen clock per scheduler invocation so every
 * stage in the run sees the same monotonic baseline.  The monotonic
 * source is per-context (so two parallel calls inside one stage
 * still measure relative elapsed time correctly), but the wall
 * clock is shared.
 */
function clockFactory(options: SchedulerOptions): () => Clock {
  if (!options.reproducibleBuild) {
    return systemClock;
  }
  return () =>
    frozenClock({
      timestamp: REPRO_BUILD_FROZEN_TIMESTAMP_MS,
      // Monotonic still advances per-call so any stage measuring
      // its own elapsed time gets non-zero values.  The reproducible
      // contract is on the wall clock, not the monotonic one.
      monotonicTickMs: 1,
    });
}

function makeRunContext(
  inst: ResolvedInstance,
  options: SchedulerOptions,
  newClock: () => Clock,
): StageContext {
  return {
    logger: options.logger.child({ stage: inst.stage.name, instance: inst.id }),
    cancellation: options.cancellation,
    time: newClock(),
    cache: inMemoryCache(),
    telemetry: noOpTelemetryEmitter(),
    storage: deniedStorageApi(),
    network: deniedNetworkApi(),
    env: deniedEnvApi(),
    filesystem: deniedFilesystemApi(),
    shell: deniedShellApi(),
    events: inMemoryEventBus(),
  };
}

function makeInitContext(
  inst: ResolvedInstance,
  options: SchedulerOptions,
  newClock: () => Clock,
): StageInitContext {
  // StageInitContext omits `cancellation` and `cache`, adds `config`.
  return {
    config: inst.config as JsonValue,
    logger: options.logger.child({ stage: inst.stage.name, instance: inst.id, phase: "init" }),
    time: newClock(),
    telemetry: noOpTelemetryEmitter(),
    storage: deniedStorageApi(),
    network: deniedNetworkApi(),
    env: deniedEnvApi(),
    filesystem: deniedFilesystemApi(),
    shell: deniedShellApi(),
    events: inMemoryEventBus(),
  };
}

async function disposeAll(
  dag: PipelineDag,
  states: Map<string, RunState>,
  options: SchedulerOptions,
): Promise<void> {
  // dispose() should see the same frozen clock as init() / run() did
  // when the run is reproducible — otherwise a dispose hook that
  // logs a timestamp would re-introduce non-determinism.
  const newClock = clockFactory(options);
  for (const id of dag.topoOrder) {
    const state = states.get(id)!;
    if (!state.initialized) continue;
    const inst = dag.instances.get(id)!;
    if (typeof inst.stage.dispose !== "function") continue;
    try {
      const disposeCtx = makeInitContext(inst, options, newClock);
      await inst.stage.dispose(disposeCtx);
    } catch (err) {
      // Per FM03 §3.2 Dispose: failures are logged warnings, never escalated.
      options.logger.warn(
        `dispose failed for ${inst.stage.name} (${inst.id})`,
        { error: String(err) },
      );
    }
  }
}

function toRunError(err: unknown, inst: ResolvedInstance): RunError {
  if (err instanceof StageError) {
    return {
      stageName: err.stageName ?? inst.stage.name,
      instanceId: inst.id,
      code: err.code,
      message: err.message,
      recoverable: err.recoverable,
      fields: err.fields,
    };
  }
  return {
    stageName: inst.stage.name,
    instanceId: inst.id,
    code: "UNCAUGHT",
    message: err instanceof Error ? err.message : String(err),
    recoverable: false,
    fields: {},
  };
}

// (Iterator helper polyfill removed — Array.from used directly above.)
