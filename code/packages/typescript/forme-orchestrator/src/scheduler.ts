/**
 * Pipeline scheduler — executes a stable DAG-ready queue with one shared
 * stage/per-item concurrency budget.
 *
 * Remaining v0 simplifications:
 *
 *   - **Exact affected scheduling with explicit side effects.** Observed
 *     sources and untouched capability-free instances restore validated
 *     topology-scoped materialized checkpoints. Changed instances and their
 *     downstream closure execute, retaining per-item cache reuse. A
 *     capability-bearing stage is restorable only when its explicit replay
 *     hook successfully reapplies the effects represented by its checkpoint.
 *
 *   - **Reproducible-build mode is wired through.**  When
 *     `settings.reproducibleBuild = true`, every StageContext receives
 *     a frozenClock pinned at `REPRO_BUILD_FROZEN_TIMESTAMP_MS` (0
 *     in v0; FM03 §8 max-input-mtime derivation remains a separate clock
 *     policy even though source revisions are now tracked). Iteration-order sorting and the
 *     deterministic-random `ctx.random` API remain deferred to v1.
 *
 * What v0 *does* implement:
 *
 *   - Stable concurrent DAG readiness and ordered per-item parallelism
 *   - Live, bounded stream fan-out with permit-aware upstream pulls
 *   - Lazy validated stream-checkpoint restoration
 *   - Per-stage StageContext construction with denied-by-default
 *     capability APIs
 *   - init/dispose lifecycle hooks
 *   - Fail-fast and best-effort error handling
 *   - Cancellation propagation
 *   - Deterministic named-input fan-in with replayable stream inputs
 *   - Per-stage timing + error counts in StageRunSummary
 */

import {
  CancellationError,
  StageError,
} from "@coding-adventures/forme-errors";
import { randomUUID } from "node:crypto";
import { mkdtemp, open, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
  ExternalStateManifest,
  InputPortMap,
  Logger,
  StageContext,
  StageInitContext,
} from "@coding-adventures/forme-stage";
import type { JsonValue, RevisionId } from "@coding-adventures/forme-types";
import type { CacheBackend, CacheEntry } from "@coding-adventures/forme-cache";
import { cacheKey, makeEntry } from "@coding-adventures/forme-cache";
import {
  computeBinaryRevisionId,
  computeRevisionId,
  isLogicalIdShape,
  isRevisionIdShape,
} from "@coding-adventures/forme-identity";
import type { ResolvedInstance, PipelineDag } from "./dag.js";
import type { RunError, RunOutcome, StageRunSummary } from "./types.js";
import {
  decodeCachedStageOutput,
  decodeCacheValue,
  encodeCachedStageOutput,
  encodeCacheValue,
  type CachedStageOutput,
} from "./cache-codec.js";
import type { PipelineRevisionLedger } from "./revision-ledger.js";
import {
  canCheckpointInstance,
  instanceCheckpointKey,
  loadInstanceCheckpoint,
  persistInstanceCheckpoint,
} from "./instance-checkpoint.js";
import { createConcurrencyPool } from "./concurrency.js";
import type { ConcurrencyPermit, ConcurrencyPool } from "./concurrency.js";
import { createBoundedFanOut, DEFAULT_STREAM_WINDOW } from "./streaming.js";
import {
  commitStreamCheckpoint,
  createStreamCheckpointWriter,
  loadStreamCheckpoint,
  STREAM_CHECKPOINT_NODE_SCHEMA,
  streamCheckpointNodeKey,
  streamCheckpointRevision,
} from "./stream-checkpoint.js";
import type { StreamCheckpointManifest } from "./stream-checkpoint.js";

/** Per-instance run state held during execution. */
interface RunState {
  /** Stage output; only a caller-visible terminal stream retains a drained array. */
  output: unknown;
  /** Whether output is from a Stream<X> producer (consumers iterate). */
  isStreamOutput: boolean;
  /** init() was called — must dispose. */
  initialized: boolean;
  /** This instance changed directly or belongs to an affected downstream closure. */
  affected: boolean;
  /** Output was restored as one validated materialized checkpoint. */
  restored: boolean;
  /** Published live branches, keyed by the consuming instance and port. */
  liveBranches: Map<string, AsyncIterable<unknown>>;
  /** Settles only after a published stream is fully consumed and finalized. */
  streamCompletion: Promise<void> | null;
  /** Finalized stream tree awaiting publication at the final input key. */
  pendingStreamCheckpoint: StreamCheckpointManifest | null;
  /** Failure from background per-item production after readiness publication. */
  backgroundFailure: { readonly present: true; readonly error: unknown } | null;
  /** One transport failure observed while pulling this instance's stream. */
  transportFailure: {
    readonly error: unknown;
    /** True when the failure originated in an upstream stream. */
    readonly inherited: boolean;
  } | null;
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
    inputRevision: RevisionId | null;
    outputRevision: RevisionId | null;
    externalStateRevision: RevisionId | null;
    inputChanged: boolean | null;
  };
}

const DEFAULT_EDGE_PORT = "default";
const DETACH_ASYNC_INPUT = Symbol("forme.detachAsyncInput");

/** Stable hand-off that orders each ready instance's first run admission. */
interface AdmissionTurn {
  readonly wait: Promise<void>;
  release(): void;
}

export interface SchedulerOptions {
  readonly logger: Logger;
  readonly cancellation: CancellationToken;
  readonly bestEffort: boolean;
  readonly cache: CacheBackend;
  readonly useCache: boolean;
  /** One pipeline-wide bound shared by stage and per-item invocations. */
  readonly maxConcurrency: number;
  /** Prior successful revisions for affected-set scheduling. */
  readonly previousLedger?: PipelineRevisionLedger | null;
  /** Topology-specific namespace used for materialized output checkpoints. */
  readonly checkpointNamespace?: string;
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
  const replaySpool = createReplaySpool();
  const lifecycle = createCancellationTokenSource();
  let activeRunOptions: SchedulerOptions | null = null;
  let disposed = false;
  let cancelled = options.cancellation.cancelled;
  const cancelLifecycle = (): void => {
    cancelled = true;
    lifecycle.cancel(options.cancellation.reason ?? undefined);
  };
  if (options.cancellation.cancelled) cancelLifecycle();
  else options.cancellation.signal.addEventListener("abort", cancelLifecycle, { once: true });
  try {
    const runOptions: SchedulerOptions = {
      ...options,
      cancellation: lifecycle.token,
    };
    activeRunOptions = runOptions;
    const disposeOnce = async (): Promise<void> => {
      if (disposed) return;
      disposed = true;
      await disposeAll(dag, states, runOptions);
    };
    const pool = createConcurrencyPool(options.maxConcurrency, lifecycle.token);

  // Choose a clock factory once for the whole run.  Reproducible-build
  // mode hands every stage a frozenClock; otherwise systemClock.
  const newClock = clockFactory(options);

  // Init pass: call init() on every stage that has one.  If any init
  // throws, we abort before any run() is called and surface the failure.
  for (const id of dag.topoOrder) {
    const inst = dag.instances.get(id)!;
    states.set(id, makeState(inst));
    if (typeof inst.stage.init !== "function") continue;
    const initCtx: StageInitContext = makeInitContext(inst, runOptions, newClock);
    try {
      await inst.stage.init(inst.config, initCtx);
      states.get(id)!.initialized = true;
    } catch (err) {
      // Init failure → fail the whole run (no per-input concept yet).
      const re = toRunError(err, inst);
      errors.push(re);
      await disposeOnce();
      const summaries = Array.from(states.values()).map(s => ({
        ...s.summary,
        outcome: "failed" as const,
      }));
      return { outcome: "failed", outputs, summaries, errors };
    }
  }

  // Execute pass. Readiness orchestration itself does not consume a permit;
  // every Stage.run/replay/externalState invocation does. This lets a
  // materialized per-item stage submit all of its ordered invocations without
  // holding a parent permit and deadlocking the shared budget.
  let anyRecoverableErrors = false;
  let anyFatal = false;
  const topoIndex = new Map(dag.topoOrder.map((id, index) => [id, index]));
  const remainingDependencies = new Map<string, number>();
  const consumers = new Map<string, string[]>();
  const streamEdges = new Map<string, string[]>();
  for (const id of dag.topoOrder) {
    const inst = dag.instances.get(id)!;
    const dependencies = new Set<string>();
    if (inst.producer !== null) {
      dependencies.add(inst.producer);
      const edges = streamEdges.get(inst.producer) ?? [];
      edges.push(streamEdgeKey(id, DEFAULT_EDGE_PORT));
      streamEdges.set(inst.producer, edges);
    }
    for (const [port, producerId] of inst.inputProducers) {
      dependencies.add(producerId);
      const edges = streamEdges.get(producerId) ?? [];
      edges.push(streamEdgeKey(id, port));
      streamEdges.set(producerId, edges);
    }
    remainingDependencies.set(id, dependencies.size);
    for (const producerId of dependencies) {
      const downstream = consumers.get(producerId) ?? [];
      downstream.push(id);
      downstream.sort((left, right) => topoIndex.get(left)! - topoIndex.get(right)!);
      consumers.set(producerId, downstream);
    }
  }

  const runInvocation = async <T>(
    inst: ResolvedInstance,
    invoke: (permit: ConcurrencyPermit) => Promise<T> | T,
  ): Promise<T> => pool.run(async permit => {
    try {
      return await invoke(permit);
    } catch (error) {
      if (!(error instanceof CancellationError)) {
        const invocationError = toRunError(error, inst);
        if (!(invocationError.recoverable && runOptions.bestEffort)
            && states.get(inst.id)?.transportFailure?.error !== error) {
          // Close the pool before this permit is released, so FIFO dispatch
          // cannot start the next queued invocation after a fatal failure.
          lifecycle.cancel(`fatal stage failure in ${inst.id}`);
        }
      }
      throw error;
    }
  });

  const publishLiveStream = (
    inst: ResolvedInstance,
    state: RunState,
    source: AsyncIterable<unknown>,
    optionsForStream: {
      readonly restored: boolean;
      readonly expectedOutputRevision?: RevisionId;
      readonly expectedItemCount?: number;
      readonly inputPermitContext?: PermitContext;
      readonly sourceUsesPool?: boolean;
    },
  ): void => {
    const edgeKeys = streamEdges.get(inst.id) ?? [];
    const needsCheckpointBranch = runOptions.useCache
      && !optionsForStream.restored
      && canCheckpointInstance(inst);
    const needsOutputBranch = dag.sinks.includes(inst.id);
    // A cache writer is fail-open and may detach after any append failure.
    // Keep one cache-independent drain whenever no sink already guarantees a
    // complete traversal, so cache health cannot truncate stream semantics or
    // the logical output revision.
    const needsRevisionBranch = !needsOutputBranch;
    const branchCount = edgeKeys.length
      + (needsCheckpointBranch ? 1 : 0)
      + (needsOutputBranch ? 1 : 0)
      + (needsRevisionBranch ? 1 : 0);
    let sourceTerminalReported = false;
    const reportSourceTerminal = (error: unknown): void => {
      if (sourceTerminalReported) return;
      sourceTerminalReported = true;
      if (error instanceof CancellationError && lifecycle.token.cancelled) return;
      const inherited = optionsForStream.inputPermitContext?.transportFailure?.error
        === error;
      state.transportFailure ??= { error, inherited };
      if (inherited) return;
      const terminal = toRunError(error, inst);
      if (!(terminal.recoverable && runOptions.bestEffort)) {
        // Prevent fresh work from starting, but allow already-yielded
        // consumers to reacquire and observe the original terminal error
        // after draining their buffered prefix.
        anyFatal = true;
        pool.stopNewTasks(`fatal stream failure in ${inst.id}`);
      }
    };
    const pooledSource = optionsForStream.sourceUsesPool === false
      ? source
      : poolBackedIterable(
          source,
          pool,
          reportSourceTerminal,
          optionsForStream.inputPermitContext,
        );
    let sourceOpened = false;
    let settleSource!: () => void;
    let failSource!: (error: unknown) => void;
    const sourceFinished = new Promise<void>((resolve, reject) => {
      settleSource = resolve;
      failSource = reject;
    });
    const revisionObserver = createStreamRevisionObserver();
    const trackedSource = trackIterableCompletion(
      pooledSource,
      settleSource,
      failSource,
      () => { sourceOpened = true; },
      value => { revisionObserver.append(value); },
      reportSourceTerminal,
    );
    const fanOut = createBoundedFanOut(
      trackedSource,
      branchCount,
      lifecycle.token,
    );
    let activeBranches = branchCount;
    let settleBranches!: () => void;
    const branchesFinished = new Promise<void>(resolve => { settleBranches = resolve; });
    const takeBranch = (index: number): AsyncIterable<unknown> =>
      trackBranchCompletion(fanOut.branches[index]!, () => {
        activeBranches -= 1;
        if (activeBranches === 0) settleBranches();
      });
    let branchIndex = 0;
    for (const key of edgeKeys) {
      state.liveBranches.set(key, takeBranch(branchIndex++));
    }

    const internalTasks: Promise<void>[] = [];
    const checkpoint = { manifest: null as StreamCheckpointManifest | null };
    if (needsCheckpointBranch) {
      const checkpointInput = takeBranch(branchIndex++);
      const writer = createStreamCheckpointWriter(runOptions.cache, lifecycle.token);
      internalTasks.push((async () => {
        try {
          for await (const item of checkpointInput) await writer.append(item);
          checkpoint.manifest = await writer.finalize();
        } catch (error) {
          if (error instanceof CancellationError && lifecycle.token.cancelled) throw error;
          runOptions.logger.warn(
            `stream checkpoint write skipped for ${inst.stage.name} (${inst.id})`,
            { error: String(error) },
          );
        }
      })());
    }

    const sinkValues: unknown[] | null = needsOutputBranch ? [] : null;
    if (needsOutputBranch) {
      const outputInput = takeBranch(branchIndex++);
      internalTasks.push((async () => {
        for await (const item of outputInput) sinkValues!.push(item);
      })());
    }
    if (needsRevisionBranch) {
      const revisionInput = takeBranch(branchIndex++);
      internalTasks.push((async () => {
        for await (const _item of revisionInput) {
          // Pulling is the work: the revision observer runs before fan-out.
        }
      })());
    }

    state.isStreamOutput = true;
    state.restored = optionsForStream.restored;
    const transportFinished = Promise.race([
      sourceFinished,
      branchesFinished.then(async () => {
        if (sourceOpened) await sourceFinished;
      }),
    ]);
    state.streamCompletion = (async () => {
      const taskResults = await Promise.allSettled(internalTasks);
      let transportFailed = false;
      let transportError: unknown;
      try {
        await transportFinished;
      } catch (error) {
        transportFailed = true;
        transportError = error;
      }
      const failedTask = taskResults.find(
        (result): result is PromiseRejectedResult => result.status === "rejected",
      );
      if (transportFailed) throw transportError;
      if (failedTask !== undefined) throw failedTask.reason;
      const observedRevision = revisionObserver.finalize();
      state.output = sinkValues ?? undefined;
      state.summary.itemsProduced = optionsForStream.expectedItemCount
        ?? checkpoint.manifest?.itemCount
        ?? sinkValues?.length
        ?? observedRevision?.itemCount
        ?? revisionObserver.itemCount;
      state.summary.outputRevision = optionsForStream.expectedOutputRevision
        ?? checkpoint.manifest?.outputRevision
        ?? observedRevision?.outputRevision
        ?? (sinkValues === null ? null : revisionForValue(sinkValues));
      state.pendingStreamCheckpoint = checkpoint.manifest;
    })();
    // The DAG may continue scheduling between publication and the final join.
    // Observe rejection immediately while preserving it for final accounting.
    void state.streamCompletion.catch(() => {});
  };

  const executeInstance = async (id: string, admission: AdmissionTurn): Promise<void> => {
    const state = states.get(id)!;
    const inst = dag.instances.get(id)!;
    await admission.wait;
    const startMonotonic = Date.now();

    try {
      if (lifecycle.token.cancelled) {
        state.summary.outcome = "skipped";
        return;
      }
      // Sources have no input; non-sources read from their producer's
      // output (which we previously stored in `outputs`).
      const inputs = hasNamedInputs(inst)
        ? collectPortInputs(inst, states, source =>
            makeSpoolReplayableAsyncIterable(
              source,
              replaySpool,
            ))
        : collectInputs(inst, states);
      const ctx: StageContext = makeRunContext(inst, runOptions, newClock);
      if (typeof inst.stage.externalState === "function") {
        if (inst.producer !== null || inst.inputProducers.size !== 0) {
          throw new Error(
            `scheduler: externalState is only valid on source instances; ${JSON.stringify(inst.id)} has producers`,
          );
        }
        const manifest = await runInvocation(inst, async () => {
          const observed = await inst.stage.externalState!(inst.config, ctx);
          // Validation is part of the guarded invocation: an invalid manifest
          // must close queued work before this permit can dispatch it.
          validateExternalStateManifest(observed, inst.id);
          return observed;
        });
        state.summary.externalStateRevision = manifest.revision;
      }
      state.summary.inputRevision = state.summary.externalStateRevision
        ?? revisionForValue(cacheInputValue(inst, states));

      const prior = options.previousLedger?.instances[id];
      state.summary.inputChanged = prior === undefined
        ? null
        : prior.inputRevision !== state.summary.inputRevision;
      const upstreamAffected = hasAffectedProducer(inst, states);
      const knownUnchanged = prior !== undefined
        && state.summary.inputRevision !== null
        && state.summary.inputChanged === false
        && !upstreamAffected;

      if (
        runOptions.useCache
        && knownUnchanged
        && prior.outputRevision !== null
        && runOptions.checkpointNamespace !== undefined
        && canCheckpointInstance(inst)
      ) {
        if (inst.stage.produces.name === "Stream" || isPromotedStreamInstance(inst, states)) {
          const restoredStream = await loadStreamCheckpoint(
            runOptions.cache,
            instanceCheckpointKey(
              runOptions.checkpointNamespace,
              inst,
              state.summary.inputRevision!,
            ),
            prior.outputRevision,
            lifecycle.token,
            runOptions.logger,
          );
          if (restoredStream !== null) {
            await detachAsyncInputs(inputs.value);
            publishLiveStream(inst, state, restoredStream.values, {
              restored: true,
              expectedOutputRevision: restoredStream.outputRevision,
              expectedItemCount: restoredStream.itemCount,
            });
            state.summary.itemsConsumed = inputs.itemCount;
            state.summary.cacheHits = 1;
            state.summary.outputRevision = restoredStream.outputRevision;
            state.summary.outcome = "skipped";
            admission.release();
            return;
          }
        }
        const restored = await loadInstanceCheckpoint(
          runOptions.cache,
          runOptions.checkpointNamespace,
          inst,
          state.summary.inputRevision!,
          prior.outputRevision,
          runOptions.logger,
        );
        if (restored !== null) {
          let replaySucceeded = true;
          if (typeof inst.stage.replay === "function") {
            try {
              lifecycle.token.throwIfCancelled();
              // Replay failure is deliberately fail-open: release the permit
              // and fall through to a normal guarded Stage.run invocation.
              await pool.run(() =>
                inst.stage.replay!(restored.value as never, inst.config, ctx));
            } catch (error) {
              if (error instanceof CancellationError) throw error;
              replaySucceeded = false;
              runOptions.logger.warn(
                `instance checkpoint replay failed open for ${inst.stage.name} (${inst.id})`,
                { error: String(error) },
              );
            }
          }
          if (replaySucceeded) {
            await detachAsyncInputs(inputs.value);
            state.output = restored.value;
            state.isStreamOutput = restored.isStream;
            state.restored = true;
            state.summary.itemsConsumed = inputs.itemCount;
            state.summary.itemsProduced = restored.isStream
              ? (restored.value as unknown[]).length
              : 1;
            state.summary.cacheHits = 1;
            state.summary.outputRevision = prior.outputRevision;
            state.summary.outcome = "skipped";
            return;
          }
        }
      }

      const scheduledAffected = prior === undefined
        || state.summary.inputRevision === null
        || state.summary.inputChanged !== false
        || upstreamAffected;

      if (hasNamedInputs(inst) || inst.stage.consumes.name === "Stream"
          || inst.stage.consumes.name === "Void" || isSingleProducer(inst, dag, states)) {
        // One invocation. Live stream inputs release their permit around each
        // iterator operation, so a one-permit pipeline can still make
        // upstream progress. Stream outputs are published immediately rather
        // than materialized behind this invocation.
        const usesLiveInput = hasLiveProducer(inst, states);
        const inputPermitContext: PermitContext = {
          current: null,
          inputTail: Promise.resolve(),
          inputFailed: null,
          transportFailure: null,
        };
        const guardedInput = permitAwareInput(inputs.value, inputPermitContext);
        const stored = await runInvocation(inst, async permit => {
          const execute = async (): Promise<CachedStageOutput> => {
            admission.release();
            let liveOutput = false;
            return withPermitContext(inputPermitContext, permit, async () => {
              try {
                const result = await inst.stage.run(
                  guardedInput as never,
                  inst.config,
                  ctx,
                );
                if (isAsyncIterable(result)) {
                  liveOutput = true;
                  return { value: result, isStream: true };
                }
                return materialize(result, inst.stage.produces.name === "Stream");
              } catch (error) {
                if (inputPermitContext.transportFailure?.error === error) {
                  state.transportFailure ??= { error, inherited: true };
                }
                throw error;
              } finally {
                // An output generator may consume its inputs only when the
                // scheduler later pulls it. Its stream-completion path owns
                // detachment; every ordinary return/failure closes now.
                if (!liveOutput) await detachAsyncInputs(guardedInput);
              }
            });
          };
          if (inst.stage.produces.name === "Stream" || usesLiveInput) {
            return { ...(await execute()), cacheHit: false, cacheMiss: false };
          }
          return runCached(inst, cacheInputValue(inst, states), runOptions, execute);
        });
        state.summary.cacheHits += stored.cacheHit ? 1 : 0;
        state.summary.cacheMisses += stored.cacheMiss ? 1 : 0;
        state.summary.itemsConsumed = inputs.itemCount;
        if (stored.isStream && isAsyncIterable(stored.value)) {
          if (!scheduledAffected && prior?.outputRevision != null) {
            // The validated external/input revision is authoritative for a
            // deterministic stage even when its output checkpoint failed
            // open. This provisional value lets downstream exact scheduling
            // proceed without waiting behind unstarted bounded branches.
            state.summary.outputRevision = prior.outputRevision;
          }
          publishLiveStream(inst, state, stored.value as AsyncIterable<unknown>, {
            restored: false,
            inputPermitContext,
          });
          state.streamCompletion = settleWithCleanup(
            state.streamCompletion!,
            () => detachAsyncInputs(guardedInput),
            runOptions.logger,
            `stream input cleanup failed for ${inst.stage.name} (${inst.id})`,
          );
          void state.streamCompletion.catch(() => {});
          state.summary.outcome = "success";
          state.affected = scheduledAffected;
          // On an incremental run, an un-restored producer must establish its
          // final content revision before affected-set scheduling can decide
          // whether downstream work is actually necessary.
          return;
        }
        state.output = stored.value;
        state.isStreamOutput = stored.isStream;
        state.summary.itemsProduced = stored.isStream
          ? (stored.value as unknown[]).length
          : 1;
      } else {
        // Multi-invocation: a stream producer feeds a per-item consumer.
        // Live inputs are pulled cooperatively under yielded permits; legacy
        // materialized checkpoints retain the array path below.
        if (isAsyncIterable(inputs.value)) {
          const iterator = (inputs.value as AsyncIterable<unknown>)[Symbol.asyncIterator]();
          const channel = createBoundedAsyncChannel<unknown>(DEFAULT_STREAM_WINDOW);
          if (!scheduledAffected && prior?.outputRevision != null) {
            state.summary.outputRevision = prior.outputRevision;
          }
          publishLiveStream(inst, state, channel, {
            restored: false,
            sourceUsesPool: false,
          });
          state.summary.outcome = "success";
          state.affected = scheduledAffected;
          admission.release();
          let nextIndex = 0;
          let nextToEmit = 0;
          let nextTail = Promise.resolve<IteratorResult<unknown>>({
            done: false,
            value: undefined,
          });
          let emitTail = Promise.resolve();
          const pending = new Map<number, {
            readonly result: CacheRunResult;
            readonly reservedSlot: boolean;
          }>();
          let reorderSlots = DEFAULT_STREAM_WINDOW;
          const reorderWaiters: Array<() => void> = [];
          const acquireReorderSlot = async (): Promise<void> => {
            if (reorderSlots > 0) {
              reorderSlots -= 1;
              return;
            }
            await new Promise<void>(resolve => { reorderWaiters.push(resolve); });
          };
          const releaseReorderSlot = (): void => {
            const waiter = reorderWaiters.shift();
            if (waiter !== undefined) waiter();
            else reorderSlots += 1;
          };
          let successfulCacheHits = 0;
          let successfulCacheMisses = 0;
          let fatalFailure: { readonly index: number; readonly error: unknown } | null = null;
          let recoverableFailure: { readonly index: number; readonly error: unknown } | null = null;
          let cancellationFailure: { readonly index: number; readonly error: unknown } | null = null;
          let reorderFailure: { readonly error: unknown } | null = null;
          const currentReorderFailure = (): { readonly error: unknown } | null =>
            reorderFailure;
          const currentProcessingFailure = (): { readonly error: unknown } | null =>
            fatalFailure ?? recoverableFailure ?? cancellationFailure;
          const rememberFailure = (index: number, error: unknown): void => {
            if (error instanceof CancellationError) {
              if (cancellationFailure === null || index < cancellationFailure.index) {
                cancellationFailure = { index, error };
              }
              return;
            }
            const itemError = toRunError(error, inst);
            if (itemError.recoverable && runOptions.bestEffort) {
              if (recoverableFailure === null || index < recoverableFailure.index) {
                recoverableFailure = { index, error };
              }
            } else if (fatalFailure === null || index < fatalFailure.index) {
              fatalFailure = { index, error };
            }
          };
          const abortReorder = (error: unknown): void => {
            reorderFailure ??= { error };
            pending.clear();
            while (reorderWaiters.length > 0) reorderWaiters.shift()!();
            channel.fail(error);
          };
          const takeNext = (): Promise<IteratorResult<unknown>> => {
            const next = nextTail.then(() => iterator.next());
            nextTail = next;
            return next;
          };
          const emitOrdered = async (index: number, result: CacheRunResult): Promise<void> => {
            // The next required index must always be able to enter and drain
            // the reorder window; reserving a slot for it would deadlock when
            // all other workers finish ahead of a slow index zero.
            const reservedSlot = index !== nextToEmit;
            if (reservedSlot) await acquireReorderSlot();
            const failureBeforeQueue = currentReorderFailure();
            if (failureBeforeQueue !== null) throw failureBeforeQueue.error;
            const wait = emitTail;
            let release!: () => void;
            emitTail = new Promise<void>(resolve => { release = resolve; });
            await wait;
            try {
              const failureBeforeEmit = currentReorderFailure();
              if (failureBeforeEmit !== null) throw failureBeforeEmit.error;
              pending.set(index, { result, reservedSlot });
              while (pending.has(nextToEmit)) {
                const ready = pending.get(nextToEmit)!;
                pending.delete(nextToEmit);
                await channel.push(ready.result.value);
                if (ready.reservedSlot) releaseReorderSlot();
                nextToEmit += 1;
              }
            } finally {
              release();
            }
          };
          const worker = async (): Promise<void> => {
            while (true) {
              let itemIndex = -1;
              let itemResult: CacheRunResult | null;
              try {
                itemResult = await runInvocation(inst, async permit => {
                  let next: IteratorResult<unknown>;
                  try {
                    next = await permit.yieldWhile(takeNext);
                  } catch (error) {
                    state.transportFailure ??= { error, inherited: true };
                    throw error;
                  }
                  if (next.done) return null;
                  itemIndex = nextIndex++;
                  lifecycle.token.throwIfCancelled();
                  return runCached(inst, next.value, runOptions, async () =>
                    materialize(
                      await inst.stage.run(next.value as never, inst.config, ctx),
                      false,
                    ));
                });
              } catch (error) {
                rememberFailure(itemIndex < 0 ? Number.MAX_SAFE_INTEGER : itemIndex, error);
                abortReorder(error);
                throw error;
              }
              if (itemResult === null) return;
              successfulCacheHits += itemResult.cacheHit ? 1 : 0;
              successfulCacheMisses += itemResult.cacheMiss ? 1 : 0;
              try {
                await emitOrdered(itemIndex, itemResult);
              } catch (error) {
                rememberFailure(itemIndex, error);
                abortReorder(error);
                throw error;
              }
            }
          };
          const processing = (async (): Promise<void> => {
            let failed = false;
            let primaryError: unknown;
            try {
              await Promise.allSettled(
                Array.from(
                  { length: Math.min(runOptions.maxConcurrency, DEFAULT_STREAM_WINDOW) },
                  () => worker(),
                ),
              );
              state.summary.cacheHits += successfulCacheHits;
              state.summary.cacheMisses += successfulCacheMisses;
              const selectedFailure = currentProcessingFailure();
              if (selectedFailure !== null) throw selectedFailure.error;
            } catch (error) {
              failed = true;
              primaryError = error;
            } finally {
              try {
                if (typeof iterator.return === "function") await iterator.return();
              } catch (cleanupError) {
                runOptions.logger.warn(
                  `stream input cleanup failed for ${inst.stage.name} (${inst.id})`,
                  { error: String(cleanupError) },
                );
              }
            }
            if (failed) throw primaryError;
            channel.close();
          })();
          void processing.catch(error => {
            state.backgroundFailure = { present: true, error };
            channel.fail(error);
          });
          const transportCompletion = state.streamCompletion!;
          state.streamCompletion = (async () => {
            const [work, transport] = await Promise.allSettled([
              processing,
              transportCompletion,
            ]);
            if (work.status === "rejected") throw work.reason;
            if (transport.status === "rejected") throw transport.reason;
          })();
          void state.streamCompletion.catch(() => {});
          return;
        } else {
        // Mark the result stream-shaped so downstream consumers iterate again.
        const list = inputs.value as unknown[];
        const collected = new Array<unknown>(list.length);
        let itemAdmissionTail = Promise.resolve();
        const itemTasks = list.map((item, index) => {
          const waitForPriorItem = itemAdmissionTail;
          let releaseNextItem!: () => void;
          itemAdmissionTail = new Promise<void>(resolve => { releaseNextItem = resolve; });
          let itemReleased = false;
          const releaseItem = (): void => {
            if (itemReleased) return;
            itemReleased = true;
            releaseNextItem();
          };
          return (async () => {
            try {
              const sub = await runInvocation(inst, async () => {
                await waitForPriorItem;
                lifecycle.token.throwIfCancelled();
                return runCached(inst, item, runOptions, async () => {
                  releaseItem();
                  if (index === 0) admission.release();
                  return materialize(
                    await inst.stage.run(item as never, inst.config, ctx),
                    false,
                  );
                });
              });
              collected[index] = sub.value;
              return sub;
            } catch (error) {
              if (!(error instanceof CancellationError)) {
                const itemError = toRunError(error, inst);
                if (!(itemError.recoverable && runOptions.bestEffort)
                    && states.get(inst.id)?.transportFailure?.error !== error) {
                  lifecycle.cancel(`fatal stage failure in ${inst.id}`);
                }
              }
              throw error;
            } finally {
              // Cache hits never enter the execute callback; cancellation can
              // reject before the invocation callback starts. Either way the
              // ordered admission chain must continue.
              releaseItem();
              if (index === 0) admission.release();
            }
          })();
        });
        if (itemTasks.length === 0) admission.release();
        const itemResults = await Promise.allSettled(itemTasks);
        let fatalItemFailed = false;
        let firstFatalItemError: unknown;
        let recoverableItemFailed = false;
        let firstRecoverableItemError: unknown;
        let cancelledItem = false;
        let firstItemCancellation: unknown;
        for (const result of itemResults) {
          if (result.status === "fulfilled") {
            state.summary.cacheHits += result.value.cacheHit ? 1 : 0;
            state.summary.cacheMisses += result.value.cacheMiss ? 1 : 0;
          } else if (result.reason instanceof CancellationError) {
            if (!cancelledItem) {
              cancelledItem = true;
              firstItemCancellation = result.reason;
            }
          } else {
            const itemError = toRunError(result.reason, inst);
            if (itemError.recoverable && runOptions.bestEffort) {
              if (!recoverableItemFailed) {
                recoverableItemFailed = true;
                firstRecoverableItemError = result.reason;
              }
            } else if (!fatalItemFailed) {
              fatalItemFailed = true;
              firstFatalItemError = result.reason;
            }
          }
        }
        // A later fatal sibling must not be hidden by an earlier recoverable
        // error. Within each severity, Promise.allSettled preserves input
        // order, which makes the selected diagnostic deterministic.
        if (fatalItemFailed) throw firstFatalItemError;
        if (recoverableItemFailed) throw firstRecoverableItemError;
        if (cancelledItem) throw firstItemCancellation;
        state.output = collected;
        state.isStreamOutput = true;
        state.summary.itemsConsumed = list.length;
        state.summary.itemsProduced = collected.length;
        }
      }
      state.summary.outputRevision = revisionForValue(state.output);
      state.summary.outcome = "success";
      state.affected = scheduledAffected
        || prior?.outputRevision !== state.summary.outputRevision;
      if (
        runOptions.useCache
        && runOptions.checkpointNamespace !== undefined
        && state.summary.inputRevision !== null
        && state.summary.outputRevision !== null
        && canCheckpointInstance(inst)
        && !hasLiveProducer(inst, states)
      ) {
        await persistInstanceCheckpoint(
          runOptions.cache,
          runOptions.checkpointNamespace,
          inst,
          state.summary.inputRevision,
          { value: state.output, isStream: state.isStreamOutput },
          runOptions.logger,
        );
      }
    } catch (err) {
      if (state.transportFailure?.inherited === true
          && state.transportFailure.error === err) {
        state.summary.outcome = "skipped";
        return;
      }
      if (err instanceof CancellationError) {
        state.summary.outcome = "skipped";
        if (!anyFatal && !lifecycle.token.cancelled) {
          cancelled = true;
          lifecycle.cancel(err.message);
        }
        return;
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
        anyFatal = true;
        if (state.transportFailure?.error !== err) {
          lifecycle.cancel(`fatal stage failure in ${inst.id}`);
        }
      }
    } finally {
      admission.release();
      state.summary.elapsedMs = Date.now() - startMonotonic;
    }
  };

  const ready = dag.topoOrder.filter(id => remainingDependencies.get(id) === 0);
  const inFlight = new Map<string, Promise<string>>();
  const launched = new Set<string>();
  let admissionTail = Promise.resolve();
  while (ready.length > 0 || inFlight.size > 0) {
    while (ready.length > 0 && !lifecycle.token.cancelled && !anyFatal) {
      const id = ready.shift()!;
      launched.add(id);
      const wait = admissionTail;
      let releaseNext!: () => void;
      admissionTail = new Promise<void>(resolve => { releaseNext = resolve; });
      let released = false;
      const admission: AdmissionTurn = {
        wait,
        release() {
          if (released) return;
          released = true;
          releaseNext();
        },
      };
      inFlight.set(id, executeInstance(id, admission).then(() => id));
    }
    if (inFlight.size === 0) break;
    const completedId = await Promise.race(inFlight.values());
    inFlight.delete(completedId);
    if (lifecycle.token.cancelled || anyFatal) continue;
    for (const consumerId of consumers.get(completedId) ?? []) {
      const remaining = remainingDependencies.get(consumerId)! - 1;
      remainingDependencies.set(consumerId, remaining);
      if (remaining === 0) {
        ready.push(consumerId);
        ready.sort((left, right) => topoIndex.get(left)! - topoIndex.get(right)!);
      }
    }
  }
  for (const id of dag.topoOrder) {
    if (!launched.has(id)) states.get(id)!.summary.outcome = "skipped";
  }

  // Static fan-out edges whose consumers never reached input collection must
  // still detach. Otherwise a fatal sibling or early cancellation leaves the
  // producer waiting forever for a branch that nobody owns.
  for (const id of dag.topoOrder) {
    const state = states.get(id)!;
    const unclaimed = [...state.liveBranches.values()];
    state.liveBranches.clear();
    const cleanupResults = await Promise.allSettled(unclaimed.map(detachAsyncInputs));
    for (const result of cleanupResults) {
      if (result.status === "rejected") {
        runOptions.logger.warn(
          `unclaimed stream branch cleanup failed for ${id}`,
          { error: String(result.reason) },
        );
      }
    }
  }

  // Readiness is released when a stream transport is published, not when its
  // source finishes. Before finalizing the run, join every transport so
  // checkpoint manifests, item counts, and late iterator failures are fully
  // accounted for.
  for (const id of dag.topoOrder) {
    const state = states.get(id)!;
    if (state.streamCompletion === null) continue;
    try {
      await state.streamCompletion;
    } catch (error) {
      if (state.transportFailure?.inherited === true
          && state.transportFailure.error === error) {
        state.summary.outcome = "skipped";
        continue;
      }
      if (error instanceof CancellationError) {
        if (!anyFatal && state.backgroundFailure === null) cancelled = true;
        if (state.backgroundFailure === null) state.summary.outcome = "skipped";
      } else {
        const inst = dag.instances.get(id)!;
        const runError = toRunError(error, inst);
        if (!errors.some(candidate => candidate.instanceId === id)) errors.push(runError);
        state.summary.outcome = "failed";
        state.summary.errorCount = 1;
        if (runError.recoverable && options.bestEffort) anyRecoverableErrors = true;
        else anyFatal = true;
      }
    }
    if (state.backgroundFailure !== null) {
      if (state.backgroundFailure.error instanceof CancellationError) {
        state.summary.outcome = "skipped";
        continue;
      }
      const inst = dag.instances.get(id)!;
      const runError = toRunError(state.backgroundFailure.error, inst);
      if (!errors.some(candidate => candidate.instanceId === id)) errors.push(runError);
      state.summary.outcome = "failed";
      state.summary.errorCount = 1;
      if (runError.recoverable && options.bestEffort) anyRecoverableErrors = true;
      else anyFatal = true;
    }
  }

  // A live input's content revision is not known until its producer closes.
  // Recompute those ledger fields now and publish non-stream checkpoints at
  // their final keys rather than the provisional readiness-time key.
  for (const id of dag.topoOrder) {
    const state = states.get(id)!;
    const inst = dag.instances.get(id)!;
    if (!hasLiveProducer(inst, states)) continue;
    state.summary.inputRevision = revisionForValue(cacheInputValue(inst, states));
    state.summary.itemsConsumed = liveProducerItemCount(inst, states);
    const prior = options.previousLedger?.instances[id];
    state.summary.inputChanged = prior === undefined
      ? null
      : prior.inputRevision !== state.summary.inputRevision;
    state.affected = prior === undefined
      || state.summary.inputChanged !== false
      || hasAffectedProducer(inst, states)
      || prior.outputRevision !== state.summary.outputRevision;
    if (
      state.summary.outcome === "success"
      && state.streamCompletion === null
      && runOptions.useCache
      && runOptions.checkpointNamespace !== undefined
      && state.summary.inputRevision !== null
      && state.summary.outputRevision !== null
      && canCheckpointInstance(inst)
    ) {
      await persistInstanceCheckpoint(
        runOptions.cache,
        runOptions.checkpointNamespace,
        inst,
        state.summary.inputRevision,
        { value: state.output, isStream: state.isStreamOutput },
        runOptions.logger,
      );
    }
  }

  // Publish stream manifests only after every live upstream has finalized and
  // the consuming instance's final input revision is known.
  for (const id of dag.topoOrder) {
    const state = states.get(id)!;
    const inst = dag.instances.get(id)!;
    if (
      state.pendingStreamCheckpoint === null
      || state.summary.outcome !== "success"
      || runOptions.checkpointNamespace === undefined
      || state.summary.inputRevision === null
      || !canCheckpointInstance(inst)
    ) continue;
    try {
      await commitStreamCheckpoint(
        runOptions.cache,
        instanceCheckpointKey(
          runOptions.checkpointNamespace,
          inst,
          state.summary.inputRevision,
        ),
        state.pendingStreamCheckpoint,
      );
    } catch (error) {
      if (error instanceof CancellationError && lifecycle.token.cancelled) {
        cancelled = true;
      }
      runOptions.logger.warn(
        `stream checkpoint commit skipped for ${inst.stage.name} (${inst.id})`,
        { error: String(error) },
      );
    }
  }

  // Sinks → outputs map (keyed by instance id; OutputSpec naming
  // happens in the run.ts wrapper that knows about the config).
  for (const sinkId of dag.sinks) {
    const state = states.get(sinkId)!;
    if (state.summary.outcome === "success" || state.restored) {
      outputs.set(sinkId, state.output);
    }
  }

  // Always dispose, regardless of outcome.
  await disposeOnce();

  const outcome: RunOutcome = anyFatal
    ? "failed"
    : cancelled
      ? "cancelled"
      : anyRecoverableErrors
        ? "partial"
        : "success";

  return {
    outcome,
    outputs,
    summaries: dag.topoOrder.map(id => ({ ...states.get(id)!.summary })),
    errors: errors.sort((left, right) =>
      topoIndex.get(left.instanceId)! - topoIndex.get(right.instanceId)!),
  };
  } finally {
    if (!disposed && activeRunOptions !== null && states.size > 0) {
      disposed = true;
      await disposeAll(dag, states, activeRunOptions);
    }
    // The caller may reuse a long-lived token across many runs. Do not retain
    // completed scheduler state through its AbortSignal listener.
    options.cancellation.signal.removeEventListener("abort", cancelLifecycle);
    try {
      await replaySpool.dispose();
    } catch (error) {
      options.logger.warn("named-input replay spool cleanup failed", { error: String(error) });
    }
  }
}

// ─── Helpers ──────────────────────────────────────────────────────────────

function makeState(inst: ResolvedInstance): RunState {
  return {
    output: undefined,
    isStreamOutput: false,
    initialized: false,
    affected: false,
    restored: false,
    liveBranches: new Map(),
    streamCompletion: null,
    pendingStreamCheckpoint: null,
    backgroundFailure: null,
    transportFailure: null,
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
      inputRevision: null,
      outputRevision: null,
      externalStateRevision: null,
      inputChanged: null,
    },
  };
}

function hasAffectedProducer(
  inst: ResolvedInstance,
  states: Map<string, RunState>,
): boolean {
  if (inst.producer !== null && states.get(inst.producer)?.affected) return true;
  for (const producerId of inst.inputProducers.values()) {
    if (states.get(producerId)?.affected) return true;
  }
  return false;
}

function hasLiveProducer(
  inst: ResolvedInstance,
  states: Map<string, RunState>,
): boolean {
  if (inst.producer !== null && states.get(inst.producer)?.streamCompletion != null) return true;
  for (const producerId of inst.inputProducers.values()) {
    if (states.get(producerId)?.streamCompletion != null) return true;
  }
  return false;
}

function liveProducerItemCount(
  inst: ResolvedInstance,
  states: Map<string, RunState>,
): number {
  let count = inst.producer === null
    ? 0
    : (states.get(inst.producer)?.summary.itemsProduced ?? 0);
  for (const producerId of inst.inputProducers.values()) {
    count += states.get(producerId)?.summary.itemsProduced ?? 0;
  }
  return count;
}

function revisionForValue(value: unknown): RevisionId | null {
  if (value === undefined) return null;
  try {
    return computeBinaryRevisionId(encodeCacheValue(value));
  } catch {
    return null;
  }
}

function validateExternalStateManifest(
  manifest: ExternalStateManifest,
  instanceId: string,
): void {
  const prefix = `scheduler: invalid external state from ${JSON.stringify(instanceId)}`;
  if (manifest?.version !== 1 || !Array.isArray(manifest.entries)) {
    throw new Error(`${prefix}: expected a version 1 manifest with entries`);
  }
  if (!isRevisionIdShape(manifest.revision)) {
    throw new Error(`${prefix}: revision is malformed`);
  }
  let previousLocator: string | null = null;
  for (const [index, entry] of manifest.entries.entries()) {
    if (typeof entry?.locator !== "string" || entry.locator.length === 0) {
      throw new Error(`${prefix}: entries[${index}].locator must be non-empty`);
    }
    if (previousLocator !== null && entry.locator <= previousLocator) {
      throw new Error(`${prefix}: entries must have unique locators in ascending order`);
    }
    if (!isRevisionIdShape(entry.revision)) {
      throw new Error(`${prefix}: entries[${index}].revision is malformed`);
    }
    if (entry.identity !== undefined && !isLogicalIdShape(entry.identity)) {
      throw new Error(`${prefix}: entries[${index}].identity is malformed`);
    }
    previousLocator = entry.locator;
  }
  const expected = computeRevisionId({
    version: manifest.version,
    entries: manifest.entries.map(entry => ({
      locator: entry.locator,
      ...(entry.identity === undefined ? {} : { identity: entry.identity }),
      revision: entry.revision,
    })),
  });
  if (manifest.revision !== expected) {
    throw new Error(`${prefix}: revision does not match canonical entries`);
  }
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

function isPromotedStreamInstance(
  inst: ResolvedInstance,
  states: Map<string, RunState>,
): boolean {
  return inst.stage.inputPorts === undefined
    && inst.stage.consumes.name !== "Stream"
    && inst.stage.produces.name !== "Stream"
    && inst.producer !== null
    && states.get(inst.producer)?.isStreamOutput === true;
}

interface CollectedInput {
  value: unknown;
  itemCount: number;
}

function hasNamedInputs(inst: ResolvedInstance): boolean {
  return inst.stage.inputPorts !== undefined;
}

function collectPortInputs(
  inst: ResolvedInstance,
  states: Map<string, RunState>,
  replay: (source: AsyncIterable<unknown>, port: string) => AsyncIterable<unknown>,
): CollectedInput {
  const input: Record<string, unknown> = Object.create(null);
  const defaultInput = collectProducerInput(
    inst.producer,
    inst.stage.consumes.name === "Stream",
    states,
    inst.id,
    DEFAULT_EDGE_PORT,
  );
  input.default = isAsyncIterable(defaultInput.value)
    ? replay(defaultInput.value as AsyncIterable<unknown>, DEFAULT_EDGE_PORT)
    : defaultInput.value;
  let itemCount = defaultInput.itemCount;

  const declared: InputPortMap = inst.stage.inputPorts ?? {};
  for (const port of Object.keys(declared).sort()) {
    const producerId = inst.inputProducers.get(port);
    if (producerId === undefined) {
      throw new Error(
        `scheduler: instance ${JSON.stringify(inst.id)} has no producer for named input ${JSON.stringify(port)}`,
      );
    }
    const sideInput = collectProducerInput(
      producerId,
      declared[port]!.name === "Stream",
      states,
      inst.id,
      port,
    );
    input[port] = isAsyncIterable(sideInput.value)
      ? replay(sideInput.value as AsyncIterable<unknown>, port)
      : sideInput.value;
    itemCount += sideInput.itemCount;
  }
  return { value: Object.freeze(input), itemCount };
}

function collectProducerInput(
  producerId: string | null,
  expectsStream: boolean,
  states: Map<string, RunState>,
  consumerId: string,
  port: string,
): CollectedInput {
  if (producerId === null) return { value: undefined, itemCount: 0 };
  const producer = states.get(producerId);
  if (!producer) return { value: undefined, itemCount: 0 };
  const live = producer.liveBranches.get(streamEdgeKey(consumerId, port));
  if (live !== undefined) {
    producer.liveBranches.delete(streamEdgeKey(consumerId, port));
    if (!expectsStream) {
      throw new Error(
        `scheduler: live stream from ${JSON.stringify(producerId)} cannot feed a single-value named input`,
      );
    }
    return { value: live, itemCount: producer.summary.itemsProduced };
  }
  if (producer.isStreamOutput) {
    const list = producer.output as unknown[];
    if (!expectsStream) {
      throw new Error(
        `scheduler: materialized stream from ${JSON.stringify(producerId)} cannot feed a single-value named input`,
      );
    }
    return { value: makeAsyncIterableFromArray(list), itemCount: list.length };
  }
  if (expectsStream) {
    throw new Error(
      `scheduler: single value from ${JSON.stringify(producerId)} cannot feed a stream named input`,
    );
  }
  return { value: producer.output, itemCount: 1 };
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
  const live = prod.liveBranches.get(streamEdgeKey(inst.id, DEFAULT_EDGE_PORT));
  if (live !== undefined) {
    prod.liveBranches.delete(streamEdgeKey(inst.id, DEFAULT_EDGE_PORT));
    return { value: live, itemCount: prod.summary.itemsProduced };
  }
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

function cacheInputValue(inst: ResolvedInstance, states: Map<string, RunState>): unknown {
  if (hasNamedInputs(inst)) {
    const value: Record<string, unknown> = { default: rawProducerOutput(inst.producer, states) };
    for (const port of [...inst.inputProducers.keys()].sort()) {
      value[port] = rawProducerOutput(inst.inputProducers.get(port)!, states);
    }
    return value;
  }
  return rawProducerOutput(inst.producer, states);
}

function rawProducerOutput(producerId: string | null, states: Map<string, RunState>): unknown {
  if (producerId === null) return undefined;
  const producer = states.get(producerId);
  if (producer?.streamCompletion !== null && producer?.streamCompletion !== undefined) {
    return producer.summary.outputRevision === null
      ? { streamFrom: producerId }
      : { streamRevision: producer.summary.outputRevision };
  }
  return producer?.output;
}

function streamEdgeKey(consumerId: string, port: string): string {
  return JSON.stringify([consumerId, port]);
}

function poolBackedIterable<T>(
  source: AsyncIterable<T>,
  pool: ConcurrencyPool,
  onTerminal: (error: unknown) => void,
  inputPermitContext?: PermitContext,
): AsyncIterable<T> {
  let terminalReported = false;
  const reportTerminal = (error: unknown): void => {
    if (terminalReported) return;
    terminalReported = true;
    onTerminal(error);
  };
  return {
    [Symbol.asyncIterator](): AsyncIterator<T> {
      const iterator = source[Symbol.asyncIterator]();
      return {
        next: async () => {
          let started = false;
          try {
            return await pool.run(permit => {
              started = true;
              return inputPermitContext === undefined
                ? iterator.next()
                : withPermitContext(inputPermitContext, permit, () => iterator.next());
            });
          } catch (error) {
            if (started) reportTerminal(error);
            throw error;
          }
        },
        return: async value => {
          const operation = () => typeof iterator.return === "function"
            ? iterator.return(value)
            : Promise.resolve({ done: true, value });
          let started = false;
          try {
            return await pool.run(permit => {
              started = true;
              return inputPermitContext === undefined
                ? operation()
                : withPermitContext(inputPermitContext, permit, operation);
            });
          } catch (error) {
            if (!(error instanceof CancellationError) || started) throw error;
            // Admission is closed during cancellation, but iterator cleanup
            // must still run and be awaited before disposal.
            return operation();
          }
        },
        throw: error => pool.run(permit => {
          const operation = () => typeof iterator.throw === "function"
            ? iterator.throw(error)
            : Promise.reject(error);
          return inputPermitContext === undefined
            ? operation()
            : withPermitContext(inputPermitContext, permit, operation);
        }),
      };
    },
  };
}

function trackIterableCompletion<T>(
  source: AsyncIterable<T>,
  complete: () => void,
  fail: (error: unknown) => void,
  open: () => void,
  observe: (value: T) => void,
  terminal: (error: unknown) => void,
): AsyncIterable<T> {
  let settled = false;
  const finish = (): void => {
    if (settled) return;
    settled = true;
    complete();
  };
  const reject = (error: unknown): void => {
    if (settled) return;
    settled = true;
    fail(error);
  };
  return {
    [Symbol.asyncIterator](): AsyncIterator<T> {
      open();
      let iterator: AsyncIterator<T>;
      try {
        iterator = source[Symbol.asyncIterator]();
      } catch (error) {
        terminal(error);
        reject(error);
        throw error;
      }
      return {
        async next(): Promise<IteratorResult<T>> {
          try {
            const result = await iterator.next();
            if (result.done) finish();
            else observe(result.value);
            return result;
          } catch (error) {
            terminal(error);
            reject(error);
            throw error;
          }
        },
        async return(value?: unknown): Promise<IteratorResult<T>> {
          try {
            const result = typeof iterator.return === "function"
              ? await iterator.return(value)
              : { done: true as const, value: value as T };
            finish();
            return result;
          } catch (error) {
            // IteratorClose is cleanup, not a new logical source read. The
            // fan-out close path owns whether to surface this error; always
            // settle source completion so cancellation cleanup cannot replace
            // the run's primary outcome or strand disposal.
            finish();
            throw error;
          }
        },
        async throw(error?: unknown): Promise<IteratorResult<T>> {
          try {
            if (typeof iterator.throw !== "function") throw error;
            const result = await iterator.throw(error);
            if (result.done) finish();
            return result;
          } catch (caught) {
            terminal(caught);
            reject(caught);
            throw caught;
          }
        },
      };
    },
  };
}

function trackBranchCompletion<T>(
  source: AsyncIterable<T>,
  complete: () => void,
): AsyncIterable<T> {
  let settled = false;
  const finish = (): void => {
    if (settled) return;
    settled = true;
    complete();
  };
  return {
    [Symbol.asyncIterator](): AsyncIterator<T> {
      const iterator = source[Symbol.asyncIterator]();
      return {
        async next(): Promise<IteratorResult<T>> {
          try {
            const result = await iterator.next();
            if (result.done) finish();
            return result;
          } catch (error) {
            finish();
            throw error;
          }
        },
        async return(value?: unknown): Promise<IteratorResult<T>> {
          try {
            return typeof iterator.return === "function"
              ? await iterator.return(value)
              : { done: true, value: value as T };
          } finally {
            finish();
          }
        },
        async throw(error?: unknown): Promise<IteratorResult<T>> {
          try {
            if (typeof iterator.throw !== "function") throw error;
            return await iterator.throw(error);
          } finally {
            finish();
          }
        },
      };
    },
  };
}

async function settleWithCleanup(
  completion: Promise<void>,
  cleanup: () => Promise<void>,
  logger: Logger,
  message: string,
): Promise<void> {
  let failed = false;
  let primaryError: unknown;
  try {
    await completion;
  } catch (error) {
    failed = true;
    primaryError = error;
  }
  try {
    await cleanup();
  } catch (error) {
    logger.warn(message, { error: String(error) });
  }
  if (failed) throw primaryError;
}

interface PermitContext {
  current: ConcurrencyPermit | null;
  inputTail: Promise<void>;
  inputFailed: { readonly error: unknown } | null;
  transportFailure: { readonly error: unknown } | null;
}

async function withPermitContext<T>(
  context: PermitContext,
  permit: ConcurrencyPermit,
  operation: () => Promise<T> | T,
): Promise<T> {
  if (context.current !== null) {
    throw new Error("stream input permit context is already active");
  }
  context.current = permit;
  try {
    return await operation();
  } finally {
    context.current = null;
  }
}

function permitAwareIterable<T>(
  source: AsyncIterable<T>,
  context: PermitContext,
): AsyncIterable<T> {
  const active = new Set<AsyncIterator<T>>();
  let opened = false;
  const waitWithPermit = async <R>(operation: () => Promise<R>): Promise<R> => {
    const permit = context.current;
    if (permit === null) {
      throw new Error("stream input read requires an active scheduler permit");
    }
    const wait = context.inputTail;
    let release!: () => void;
    context.inputTail = new Promise<void>(resolve => { release = resolve; });
    await wait;
    try {
      if (context.inputFailed !== null) throw context.inputFailed.error;
      return await permit.yieldWhile(operation);
    } catch (error) {
      context.inputFailed ??= { error };
      context.transportFailure ??= { error };
      throw error;
    } finally {
      release();
    }
  };
  return {
    [DETACH_ASYNC_INPUT]: async (): Promise<void> => {
      const inherited = source as AsyncIterable<T> & {
        [DETACH_ASYNC_INPUT]?: () => Promise<void>;
      };
      if (inherited[DETACH_ASYNC_INPUT] !== undefined) {
        active.clear();
        opened = true;
        await inherited[DETACH_ASYNC_INPUT]();
        return;
      }
      if (!opened) {
        const iterator = source[Symbol.asyncIterator]();
        opened = true;
        active.add(iterator);
      }
      const iterators = [...active];
      active.clear();
      await Promise.all(iterators.map(iterator => {
        if (typeof iterator.return !== "function") return Promise.resolve();
        const operation = () => iterator.return!().then(() => undefined);
        return context.current === null ? operation() : waitWithPermit(operation);
      }));
    },
    [Symbol.asyncIterator](): AsyncIterator<T> {
      const iterator = source[Symbol.asyncIterator]();
      opened = true;
      active.add(iterator);
      return {
        next: async () => {
          try {
            const result = await waitWithPermit(() => iterator.next());
            if (result.done) active.delete(iterator);
            return result;
          } catch (error) {
            active.delete(iterator);
            throw error;
          }
        },
        return: async value => {
          try {
            return await waitWithPermit(() =>
              typeof iterator.return === "function"
                ? iterator.return(value)
                : Promise.resolve({ done: true, value }));
          } finally {
            active.delete(iterator);
          }
        },
        throw: async error => {
          try {
            return await waitWithPermit(() =>
              typeof iterator.throw === "function"
                ? iterator.throw(error)
                : Promise.reject(error));
          } finally {
            active.delete(iterator);
          }
        },
      };
    },
  } as AsyncIterable<T>;
}

function permitAwareInput(value: unknown, context: PermitContext): unknown {
  if (isAsyncIterable(value)) {
    return permitAwareIterable(value as AsyncIterable<unknown>, context);
  }
  if (typeof value !== "object" || value === null
      || Object.getPrototypeOf(value) !== null) return value;
  const mapped: Record<string, unknown> = Object.create(null);
  let changed = false;
  for (const [key, entry] of Object.entries(value)) {
    if (isAsyncIterable(entry)) {
      mapped[key] = permitAwareIterable(entry as AsyncIterable<unknown>, context);
      changed = true;
    } else {
      mapped[key] = entry;
    }
  }
  return changed ? Object.freeze(mapped) : value;
}

interface CacheRunResult extends CachedStageOutput {
  readonly cacheHit: boolean;
  readonly cacheMiss: boolean;
}

async function runCached(
  inst: ResolvedInstance,
  input: unknown,
  options: SchedulerOptions,
  execute: () => Promise<CachedStageOutput>,
): Promise<CacheRunResult> {
  // Whole-instance checkpoints handle observed sources, exact unaffected
  // branches, and effectful stages with explicit replay. This per-invocation
  // cache remains limited to pure non-sources so effects are never replayed
  // once per stream item.
  const hasInput = inst.producer !== null || inst.inputProducers.size !== 0;
  if (!options.useCache || !hasInput || inst.capabilities.length !== 0) {
    return { ...(await execute()), cacheHit: false, cacheMiss: false };
  }

  let inputBytes: Uint8Array;
  try {
    inputBytes = encodeCacheValue(input);
  } catch {
    return { ...(await execute()), cacheHit: false, cacheMiss: false };
  }
  const key = cacheKey({
    stageName: inst.stage.name,
    stageVersion: inst.stage.version,
    stageConfig: (inst.config ?? null) as JsonValue,
    inputRevision: computeBinaryRevisionId(inputBytes),
    capabilities: inst.capabilities.map(String),
  });
  let entry: CacheEntry | null = null;
  try {
    entry = await options.cache.get(key);
  } catch (error) {
    options.logger.warn(`cache read skipped for ${inst.stage.name} (${inst.id})`, {
      error: String(error),
    });
  }
  if (entry !== null) {
    try {
      return { ...decodeCachedStageOutput(entry.payload), cacheHit: true, cacheMiss: false };
    } catch (decodeError) {
      try {
        await options.cache.invalidate(key);
      } catch (error) {
        options.logger.warn(`malformed cache entry could not be invalidated for ${inst.stage.name} (${inst.id})`, {
          decodeError: String(decodeError),
          error: String(error),
        });
      }
    }
  }

  const result = await execute();
  try {
    await options.cache.put(key, makeEntry(encodeCachedStageOutput(result)));
  } catch (error) {
    options.logger.warn(`cache write skipped for ${inst.stage.name} (${inst.id})`, {
      error: String(error),
    });
  }
  return { ...result, cacheHit: false, cacheMiss: true };
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

interface RevisionNodeRef {
  readonly key: string;
  readonly count: number;
}

interface StreamRevisionObserver {
  append(value: unknown): void;
  finalize(): { itemCount: number; outputRevision: RevisionId } | null;
  readonly itemCount: number;
}

function createStreamRevisionObserver(): StreamRevisionObserver {
  const frontier: Array<RevisionNodeRef | undefined> = [];
  let count = 0;
  let failed = false;
  const branch = (left: RevisionNodeRef, right: RevisionNodeRef): RevisionNodeRef => {
    const node = {
      schema: STREAM_CHECKPOINT_NODE_SCHEMA,
      type: "branch" as const,
      count: left.count + right.count,
      left,
      right,
    };
    return { key: streamCheckpointNodeKey(encodeCacheValue(node)), count: node.count };
  };
  return {
    get itemCount() { return count; },
    append(value: unknown): void {
      count += 1;
      if (failed) return;
      try {
        const leaf = {
          schema: STREAM_CHECKPOINT_NODE_SCHEMA,
          type: "leaf" as const,
          count: 1 as const,
          value,
        };
        let node: RevisionNodeRef = {
          key: streamCheckpointNodeKey(encodeCacheValue(leaf)),
          count: 1,
        };
        let level = 0;
        while (frontier[level] !== undefined) {
          node = branch(frontier[level]!, node);
          frontier[level] = undefined;
          level += 1;
        }
        frontier[level] = node;
      } catch {
        failed = true;
        frontier.length = 0;
      }
    },
    finalize(): { itemCount: number; outputRevision: RevisionId } | null {
      if (failed) return null;
      let root: RevisionNodeRef | null = null;
      for (let level = 0; level < frontier.length; level++) {
        const earlier = frontier[level];
        if (earlier === undefined) continue;
        root = root === null ? earlier : branch(earlier, root);
      }
      return {
        itemCount: count,
        outputRevision: streamCheckpointRevision(root?.key ?? null, count),
      };
    },
  };
}

interface BoundedAsyncChannel<T> extends AsyncIterable<T> {
  push(value: T): Promise<void>;
  close(): void;
  fail(error: unknown): void;
}

function createBoundedAsyncChannel<T>(capacity: number): BoundedAsyncChannel<T> {
  const queue: T[] = [];
  const readers: Array<{
    resolve: (result: IteratorResult<T>) => void;
    reject: (error: unknown) => void;
  }> = [];
  const spaceWaiters: Array<() => void> = [];
  let terminal: { kind: "done" } | { kind: "error"; error: unknown } | null = null;
  let iteratorTaken = false;

  const releaseSpace = (): void => { spaceWaiters.shift()?.(); };
  const settleReaders = (): void => {
    while (readers.length > 0 && queue.length > 0) {
      readers.shift()!.resolve({ done: false, value: queue.shift()! });
      releaseSpace();
    }
    if (queue.length !== 0 || terminal === null) return;
    for (const reader of readers.splice(0)) {
      if (terminal.kind === "error") reader.reject(terminal.error);
      else reader.resolve({ done: true, value: undefined });
    }
  };

  return {
    async push(value: T): Promise<void> {
      while (true) {
        if (terminal !== null) {
          throw terminal.kind === "error"
            ? terminal.error
            : new CancellationError("stream channel is closed");
        }
        const reader = readers.shift();
        if (reader !== undefined) {
          reader.resolve({ done: false, value });
          return;
        }
        if (queue.length < capacity) {
          queue.push(value);
          return;
        }
        await new Promise<void>(resolve => { spaceWaiters.push(resolve); });
      }
    },
    close(): void {
      if (terminal !== null) return;
      terminal = { kind: "done" };
      for (const wake of spaceWaiters.splice(0)) wake();
      settleReaders();
    },
    fail(error: unknown): void {
      if (terminal !== null) return;
      terminal = { kind: "error", error };
      for (const wake of spaceWaiters.splice(0)) wake();
      settleReaders();
    },
    [Symbol.asyncIterator](): AsyncIterator<T> {
      if (iteratorTaken) throw new Error("bounded async channel is single-use");
      iteratorTaken = true;
      return {
        next(): Promise<IteratorResult<T>> {
          if (queue.length > 0) {
            const value = queue.shift()!;
            releaseSpace();
            return Promise.resolve({ done: false, value });
          }
          if (terminal !== null) {
            return terminal.kind === "error"
              ? Promise.reject(terminal.error)
              : Promise.resolve({ done: true, value: undefined });
          }
          return new Promise<IteratorResult<T>>((resolve, reject) => {
            readers.push({ resolve, reject });
          });
        },
        async return(): Promise<IteratorResult<T>> {
          if (terminal === null) {
            terminal = { kind: "done" };
            queue.length = 0;
            for (const wake of spaceWaiters.splice(0)) wake();
            settleReaders();
          }
          return { done: true, value: undefined };
        },
      };
    },
  };
}

function makeAsyncIterableFromArray<T>(arr: readonly T[]): AsyncIterable<T> {
  return {
    async *[Symbol.asyncIterator]() {
      for (const item of arr) yield item;
    },
  };
}

interface ReplaySpool {
  allocatePath(): Promise<string>;
  dispose(): Promise<void>;
}

function createReplaySpool(): ReplaySpool {
  let root: Promise<string> | null = null;
  const getRoot = (): Promise<string> => {
    root ??= mkdtemp(join(tmpdir(), "forme-named-replay-"));
    return root;
  };
  return {
    async allocatePath(): Promise<string> {
      return join(await getRoot(), `${randomUUID()}.spool`);
    },
    async dispose(): Promise<void> {
      if (root === null) return;
      await rm(await root, { recursive: true, force: true });
    },
  };
}

const MAX_REPLAY_FRAME_BYTES = 256 * 1024 * 1024;

/** Replay named stream inputs from a run-scoped framed spool, never an array. */
function makeSpoolReplayableAsyncIterable<T>(
  source: AsyncIterable<T>,
  spool: ReplaySpool,
): AsyncIterable<T> {
  const upstream = source[Symbol.asyncIterator]();
  let path: string | null = null;
  let writer: Awaited<ReturnType<typeof open>> | null = null;
  let producedBytes = 0;
  let finished = false;
  let detached = false;
  let spoolFailed = false;
  let spoolFailure: unknown;
  let sourceFailure: { readonly error: unknown } | null = null;
  interface Reader {
    position: number;
    closed: boolean;
    direct: boolean;
    handle: Awaited<ReturnType<typeof open>> | null;
  }
  let owner: Reader | null = null;
  const changeWaiters: Array<() => void> = [];
  const readers = new Set<Reader>();
  const notifyChange = (): void => {
    for (const wake of changeWaiters.splice(0)) wake();
  };
  const waitForChange = (): Promise<void> =>
    new Promise<void>(resolve => { changeWaiters.push(resolve); });
  const ensureWriter = async () => {
    if (writer !== null) return writer;
    path ??= await spool.allocatePath();
    writer = await open(path, "wx");
    return writer;
  };
  const append = async (value: T): Promise<boolean> => {
    if (spoolFailed) return false;
    try {
      const payload = encodeCacheValue(value);
      if (payload.byteLength > MAX_REPLAY_FRAME_BYTES) {
        throw new RangeError("named replay item exceeds the spool frame limit");
      }
      const header = Buffer.allocUnsafe(4);
      header.writeUInt32BE(payload.byteLength, 0);
      const handle = await ensureWriter();
      await writeSpoolBytes(handle, header);
      await writeSpoolBytes(handle, payload);
      producedBytes += 4 + payload.byteLength;
      notifyChange();
      return true;
    } catch (error) {
      spoolFailed = true;
      spoolFailure = error;
      if (writer !== null) {
        await writer.close().catch(() => {});
        writer = null;
      }
      notifyChange();
      return false;
    }
  };
  const finish = async (): Promise<void> => {
    if (finished) return;
    finished = true;
    if (writer !== null) {
      try {
        await writer.close();
      } catch (error) {
        spoolFailed = true;
        spoolFailure = error;
      }
      writer = null;
    }
    notifyChange();
  };
  const fail = (error: unknown): void => {
    finished = true;
    sourceFailure = { error };
    notifyChange();
  };

  const closeReader = async (reader: Reader): Promise<void> => {
    if (reader.closed) return;
    reader.closed = true;
    readers.delete(reader);
    if (owner === reader) owner = null;
    if (reader.handle !== null) {
      const handle = reader.handle;
      reader.handle = null;
      await handle.close();
    }
    notifyChange();
  };
  const readFrame = async (reader: Reader): Promise<T> => {
    if (path === null || reader.position >= producedBytes) {
      throw new Error("named replay spool frame is unavailable");
    }
    reader.handle ??= await open(path, "r");
    const header = await readSpoolBytes(reader.handle, 4, reader.position, false);
    const length = Buffer.from(header!).readUInt32BE(0);
    const payloadPosition = reader.position + 4;
    if (length > MAX_REPLAY_FRAME_BYTES || length > producedBytes - payloadPosition) {
      throw new Error("named replay spool contains an invalid frame length");
    }
    const payload = await readSpoolBytes(reader.handle, length, payloadPosition, false);
    reader.position = payloadPosition + length;
    return decodeCacheValue(payload! as Uint8Array) as T;
  };
  const nextFor = async (reader: Reader): Promise<IteratorResult<T>> => {
    while (true) {
      if (reader.closed || detached) return { done: true, value: undefined };
      if (!reader.direct && reader.position < producedBytes) {
        return { done: false, value: await readFrame(reader) };
      }
      if (sourceFailure !== null) throw sourceFailure.error;
      if (finished) {
        if (spoolFailed && !reader.direct) throw spoolFailure;
        await closeReader(reader);
        return { done: true, value: undefined };
      }
      if (spoolFailed && !reader.direct) throw spoolFailure;
      if (owner === null) owner = reader;
      if (owner !== reader) {
        await waitForChange();
        continue;
      }
      try {
        const next = await upstream.next();
        if (next.done) {
          await finish();
          continue;
        }
        const stored = await append(next.value);
        if (stored) reader.position = producedBytes;
        else reader.direct = true;
        return { done: false, value: next.value };
      } catch (error) {
        fail(error);
        throw error;
      }
    }
  };

  const value: AsyncIterable<T> & { [DETACH_ASYNC_INPUT]: () => Promise<void> } = {
    [DETACH_ASYNC_INPUT]: async (): Promise<void> => {
      if (detached) return;
      detached = true;
      fail(new CancellationError("named stream input detached"));
      const closeResults = await Promise.allSettled([...readers].map(closeReader));
      const inherited = source as AsyncIterable<T> & {
        [DETACH_ASYNC_INPUT]?: () => Promise<void>;
      };
      if (inherited[DETACH_ASYNC_INPUT] !== undefined) {
        await inherited[DETACH_ASYNC_INPUT]();
      } else if (typeof upstream.return === "function") {
        await upstream.return();
      }
      if (writer !== null) {
        await writer.close().catch(() => {});
        writer = null;
      }
      const failedClose = closeResults.find(
        (result): result is PromiseRejectedResult => result.status === "rejected",
      );
      if (failedClose !== undefined) throw failedClose.reason;
    },
    [Symbol.asyncIterator](): AsyncIterator<T> {
      const reader: Reader = {
        position: 0,
        closed: false,
        direct: false,
        handle: null,
      };
      readers.add(reader);
      return {
        next: () => nextFor(reader),
        return: async () => {
          await closeReader(reader);
          return { done: true, value: undefined };
        },
      };
    },
  };
  return value;
}

async function writeSpoolBytes(
  handle: Awaited<ReturnType<typeof open>>,
  value: Uint8Array,
): Promise<void> {
  let offset = 0;
  while (offset < value.byteLength) {
    const result = await handle.write(value, offset, value.byteLength - offset);
    if (result.bytesWritten === 0) {
      throw new Error("named replay spool write made no progress");
    }
    offset += result.bytesWritten;
  }
}

async function readSpoolBytes(
  handle: Awaited<ReturnType<typeof open>>,
  length: number,
  position: number,
  allowEof: boolean,
): Promise<Uint8Array | null> {
  const bytes = Buffer.allocUnsafe(length);
  let offset = 0;
  while (offset < length) {
    const result = await handle.read(bytes, offset, length - offset, position + offset);
    if (result.bytesRead === 0) {
      if (allowEof && offset === 0) return null;
      throw new Error("named replay spool ended inside a frame");
    }
    offset += result.bytesRead;
  }
  return bytes;
}

async function detachAsyncInputs(value: unknown): Promise<void> {
  if (isAsyncIterable(value)) {
    const detachable = value as AsyncIterable<unknown> & {
      [DETACH_ASYNC_INPUT]?: () => Promise<void>;
    };
    if (detachable[DETACH_ASYNC_INPUT] !== undefined) {
      await detachable[DETACH_ASYNC_INPUT]();
      return;
    }
    const iterator = detachable[Symbol.asyncIterator]();
    if (typeof iterator.return === "function") await iterator.return();
    return;
  }
  if (typeof value !== "object" || value === null
      || Object.getPrototypeOf(value) !== null) return;
  await Promise.all(Object.values(value).map(detachAsyncInputs));
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
