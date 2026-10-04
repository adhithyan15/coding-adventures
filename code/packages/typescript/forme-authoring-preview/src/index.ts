/**
 * Exact-revision authoring preview composition.
 *
 * The authoring core knows no filesystem and the editor owns no build
 * capability.  This coordinator is the narrow bridge between those worlds:
 * it snapshots one persisted revision, asks a host to materialize an isolated
 * pipeline, and consumes that pipeline through FM03's existing watch surface.
 */

import {
  validateAuthoringProject,
  type AuthoringProject,
  type AuthoringSession,
} from "@coding-adventures/forme-authoring-core";
import { snapshotFromOutputs, type PreviewSnapshot } from "@coding-adventures/forme-dev-server";
import type {
  Orchestrator,
  Pipeline,
  RunError,
  RunResult,
  WatchSession,
} from "@coding-adventures/forme-orchestrator";

export interface AuthoringPreviewDiagnostic {
  readonly severity: "error";
  readonly code: string;
  readonly stageName: string;
  readonly instanceId: string;
  readonly message: string;
}

export type AuthoringPreviewOutcome = "ready" | "failed" | "cancelled" | "superseded";

export interface AuthoringPreviewAttempt {
  readonly outcome: AuthoringPreviewOutcome;
  readonly revision: string;
  readonly buildId: string | null;
  readonly diagnostics: readonly AuthoringPreviewDiagnostic[];
}

export interface AuthoringPreviewState {
  readonly phase: "idle" | "building" | "ready" | "failed" | "disposed";
  readonly activeRevision: string | null;
  readonly lastGoodRevision: string | null;
  readonly lastGoodBuildId: string | null;
  readonly diagnostics: readonly AuthoringPreviewDiagnostic[];
}

export interface AuthoringPreviewInput {
  readonly revision: string;
  readonly project: AuthoringProject;
}

export interface PreparedAuthoringPreview {
  readonly pipeline: Pipeline;
  release(): Promise<void>;
}

export interface AuthoringPreviewMaterializer {
  prepare(input: AuthoringPreviewInput, signal: AbortSignal): Promise<PreparedAuthoringPreview>;
}

export interface PublishedAuthoringPreview extends PreviewSnapshot {
  readonly revision: string;
}

export interface FailedAuthoringPreview {
  readonly revision: string;
  readonly diagnostics: readonly AuthoringPreviewDiagnostic[];
}

export interface AuthoringPreviewPublisher {
  publish(snapshot: PublishedAuthoringPreview): void | Promise<void>;
  publishFailure(failure: FailedAuthoringPreview): void | Promise<void>;
}

export interface CreateAuthoringPreviewOptions {
  readonly orchestrator: Pick<Orchestrator, "watch">;
  readonly materializer: AuthoringPreviewMaterializer;
  readonly publisher: AuthoringPreviewPublisher;
  readonly debounceMs?: number;
}

export interface AuthoringPreviewCoordinator {
  readonly state: AuthoringPreviewState;
  request(session: AuthoringSession): Promise<AuthoringPreviewAttempt>;
  dispose(): Promise<void>;
}

const MAX_DEBOUNCE_MS = 60_000;
const MAX_REVISION_SCALARS = 1_024;
const MAX_DIAGNOSTICS = 64;
const MAX_CODE_SCALARS = 128;
const MAX_IDENTITY_SCALARS = 256;
const MAX_MESSAGE_SCALARS = 2_048;
const UNSAFE_TEXT = /[\u0000-\u001f\u007f-\u009f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/u;

interface RequestTask {
  readonly input: AuthoringPreviewInput;
  readonly resolve: (attempt: AuthoringPreviewAttempt) => void;
  settled: boolean;
}

interface ActiveTask {
  readonly task: RequestTask;
  readonly abort: AbortController;
  watch: WatchSession | null;
  stopPromise: Promise<void> | null;
}

interface SafePrepared {
  readonly pipeline: Pipeline;
  release(): Promise<void>;
}

interface IdleChanges extends AsyncIterable<unknown> {
  close(): void;
}

/**
 * Construct a coordinator without granting it ambient authority.  Every host
 * method is captured once so later mutation of an adapter cannot change the
 * reviewed boundary midway through a preview.
 */
export function createAuthoringPreview(options: CreateAuthoringPreviewOptions): AuthoringPreviewCoordinator {
  return new PreviewCoordinator(options);
}

class PreviewCoordinator implements AuthoringPreviewCoordinator {
  private readonly watch: Pick<Orchestrator, "watch">["watch"];
  private readonly prepare: AuthoringPreviewMaterializer["prepare"];
  private readonly publish: AuthoringPreviewPublisher["publish"];
  private readonly publishFailure: AuthoringPreviewPublisher["publishFailure"];
  private readonly debounceMs: number;
  private currentState: AuthoringPreviewState = freezeState({
    phase: "idle",
    activeRevision: null,
    lastGoodRevision: null,
    lastGoodBuildId: null,
    diagnostics: [],
  });
  private pending: RequestTask | null = null;
  private active: ActiveTask | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private ready = false;
  private pumping: Promise<void> | null = null;
  private disposed = false;

  constructor(options: CreateAuthoringPreviewOptions) {
    if (options === null || typeof options !== "object") throw new TypeError("preview options must be an object");
    this.debounceMs = options.debounceMs ?? 100;
    if (!Number.isSafeInteger(this.debounceMs) || this.debounceMs < 0 || this.debounceMs > MAX_DEBOUNCE_MS) {
      throw new RangeError(`preview debounceMs must be an integer from 0 through ${MAX_DEBOUNCE_MS}`);
    }
    this.watch = captureMethod(options.orchestrator, "watch", "orchestrator");
    this.prepare = captureMethod(options.materializer, "prepare", "materializer");
    this.publish = captureMethod(options.publisher, "publish", "publisher");
    this.publishFailure = captureMethod(options.publisher, "publishFailure", "publisher");
  }

  get state(): AuthoringPreviewState {
    return this.currentState;
  }

  request(session: AuthoringSession): Promise<AuthoringPreviewAttempt> {
    if (this.disposed) return Promise.reject(new Error("authoring preview is disposed"));
    let input: AuthoringPreviewInput;
    try {
      input = snapshotSession(session);
    } catch (error) {
      return Promise.reject(error);
    }
    return new Promise(resolve => {
      const task: RequestTask = { input, resolve, settled: false };
      if (this.pending !== null) this.finish(this.pending, "superseded", null, []);
      this.pending = task;
      if (this.active !== null) {
        this.finish(this.active.task, "superseded", null, []);
        this.cancelActive(this.active);
      }
      this.armDebounce();
    });
  }

  async dispose(): Promise<void> {
    if (this.disposed) return;
    this.disposed = true;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    this.ready = false;
    if (this.pending !== null) {
      this.finish(this.pending, "cancelled", null, []);
      this.pending = null;
    }
    if (this.active !== null) {
      this.finish(this.active.task, "cancelled", null, []);
      this.cancelActive(this.active);
    }
    if (this.pumping !== null) await this.pumping;
    this.currentState = freezeState({
      ...this.currentState,
      phase: "disposed",
      activeRevision: null,
      diagnostics: [],
    });
  }

  private armDebounce(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.ready = false;
    this.timer = setTimeout(() => {
      this.timer = null;
      this.ready = true;
      this.startPump();
    }, this.debounceMs);
  }

  private startPump(): void {
    if (this.pumping !== null || this.disposed || !this.ready || this.pending === null) return;
    this.pumping = this.pump().finally(() => {
      this.pumping = null;
      if (!this.disposed && this.ready && this.pending !== null) this.startPump();
    });
  }

  private async pump(): Promise<void> {
    while (!this.disposed && this.ready && this.pending !== null) {
      const task = this.pending;
      this.pending = null;
      this.ready = false;
      await this.execute(task);
    }
  }

  private async execute(task: RequestTask): Promise<void> {
    const active: ActiveTask = {
      task,
      abort: new AbortController(),
      watch: null,
      stopPromise: null,
    };
    this.active = active;
    this.currentState = freezeState({
      ...this.currentState,
      phase: "building",
      activeRevision: task.input.revision,
      diagnostics: [],
    });

    let prepared: SafePrepared | null = null;
    let idle: IdleChanges | null = null;
    let snapshot: PreviewSnapshot | null = null;
    let attempt: AuthoringPreviewAttempt | null = null;
    try {
      let rawPrepared: PreparedAuthoringPreview;
      try {
        rawPrepared = await this.prepare(task.input, active.abort.signal);
      } catch {
        if (task.settled || this.disposed || active.abort.signal.aborted) return;
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_PREPARE", "Preview input could not be prepared.");
        return;
      }
      if (task.settled || this.disposed || active.abort.signal.aborted) {
        try {
          prepared = safePrepared(rawPrepared);
        } catch {
          // A superseded request is already terminal. Malformed late host data
          // must not reject the internal pump or block the newest revision.
        }
        return;
      }
      try {
        prepared = safePrepared(rawPrepared);
      } catch {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_PREPARE", "Preview input could not be prepared.");
        return;
      }

      idle = idleChanges();
      try {
        active.watch = this.watch(prepared.pipeline, { changes: idle, debounceMs: 0 });
      } catch {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_RUN", "Preview pipeline could not be started.");
        return;
      }

      let next: IteratorResult<RunResult>;
      try {
        const stream = active.watch.results();
        next = await stream[Symbol.asyncIterator]().next();
      } catch {
        if (task.settled || this.disposed || active.abort.signal.aborted) return;
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_RUN", "Preview pipeline did not complete safely.");
        return;
      }
      if (task.settled || this.disposed || active.abort.signal.aborted) return;
      if (next.done) {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_RUN", "Preview pipeline ended without a result.");
        return;
      }

      let outcome: unknown;
      try { outcome = dataField(next.value, "outcome"); } catch {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_RUN", "Preview pipeline returned an invalid result.");
        return;
      }
      if (outcome === "cancelled") {
        attempt = makeAttempt("cancelled", task.input.revision, null, []);
        return;
      }
      if (outcome !== "success") {
        const diagnostics = normalizeDiagnostics(next.value);
        attempt = makeAttempt("failed", task.input.revision, safeBuildId(next.value), diagnostics);
        return;
      }

      const buildId = safeBuildId(next.value);
      if (buildId === null) {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_OUTPUT", "Preview output was invalid.");
        return;
      }
      try {
        const outputs = dataField(next.value, "outputs") as Readonly<Record<string, unknown>>;
        snapshot = snapshotFromOutputs(buildId, outputs);
      } catch {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_OUTPUT", "Preview output was invalid.");
        return;
      }
      attempt = makeAttempt("ready", task.input.revision, buildId, []);
    } finally {
      idle?.close();
      if (active.watch !== null) await this.stopWatch(active);
      let released = true;
      if (prepared !== null) {
        try { await prepared.release(); } catch { released = false; }
      }
      if (this.active === active) this.active = null;

      if (!task.settled && !this.disposed) {
        if (!released) {
          attempt = failedAttempt(task.input.revision, "E_PREVIEW_RELEASE", "Preview input could not be released safely.");
          snapshot = null;
        }
        if (attempt === null) {
          attempt = makeAttempt(active.abort.signal.aborted ? "cancelled" : "failed", task.input.revision, null, []);
        }
        await this.publishAttempt(task, attempt, snapshot);
      }
    }
  }

  private async publishAttempt(
    task: RequestTask,
    attempt: AuthoringPreviewAttempt,
    snapshot: PreviewSnapshot | null,
  ): Promise<void> {
    let finalAttempt = attempt;
    if (attempt.outcome === "ready" && snapshot !== null) {
      try {
        await this.publish(Object.freeze({ ...snapshot, revision: task.input.revision }));
      } catch {
        finalAttempt = failedAttempt(task.input.revision, "E_PREVIEW_PUBLISH", "Preview output could not be published.");
      }
    } else if (attempt.outcome === "failed") {
      try {
        await this.publishFailure(Object.freeze({ revision: task.input.revision, diagnostics: attempt.diagnostics }));
      } catch {
        finalAttempt = failedAttempt(task.input.revision, "E_PREVIEW_PUBLISH", "Preview failure could not be published.");
      }
    }

    if (finalAttempt.outcome === "ready") {
      this.currentState = freezeState({
        phase: "ready",
        activeRevision: task.input.revision,
        lastGoodRevision: task.input.revision,
        lastGoodBuildId: finalAttempt.buildId,
        diagnostics: [],
      });
    } else if (finalAttempt.outcome === "failed") {
      this.currentState = freezeState({
        ...this.currentState,
        phase: "failed",
        activeRevision: task.input.revision,
        diagnostics: finalAttempt.diagnostics,
      });
    } else if (finalAttempt.outcome === "cancelled") {
      this.currentState = freezeState({
        ...this.currentState,
        phase: "idle",
        activeRevision: task.input.revision,
        diagnostics: [],
      });
    }
    this.finishWithAttempt(task, finalAttempt);
  }

  private cancelActive(active: ActiveTask): void {
    active.abort.abort();
    if (active.watch !== null) void this.stopWatch(active);
  }

  private stopWatch(active: ActiveTask): Promise<void> {
    if (active.stopPromise === null) {
      active.stopPromise = Promise.resolve(active.watch?.stop()).catch(() => {});
    }
    return active.stopPromise;
  }

  private finish(
    task: RequestTask,
    outcome: AuthoringPreviewOutcome,
    buildId: string | null,
    diagnostics: readonly AuthoringPreviewDiagnostic[],
  ): void {
    this.finishWithAttempt(task, makeAttempt(outcome, task.input.revision, buildId, diagnostics));
  }

  private finishWithAttempt(task: RequestTask, attempt: AuthoringPreviewAttempt): void {
    if (task.settled) return;
    task.settled = true;
    task.resolve(attempt);
  }
}

function snapshotSession(session: AuthoringSession): AuthoringPreviewInput {
  if (session === null || typeof session !== "object") throw new TypeError("authoring session must be an object");
  const project = validateAuthoringProject(session.project);
  const revision = boundedText(session.storageRevision, MAX_REVISION_SCALARS, "revision");
  if (revision.length === 0) throw new TypeError("authoring revision must not be empty");
  return Object.freeze({ revision, project });
}

function safePrepared(value: unknown): SafePrepared {
  if (value === null || typeof value !== "object" || Object.getPrototypeOf(value) !== Object.prototype) {
    throw new TypeError("prepared preview must be a plain object");
  }
  const ownKeys = Reflect.ownKeys(value);
  if (ownKeys.length !== 2 || !ownKeys.includes("pipeline") || !ownKeys.includes("release")) {
    throw new TypeError("prepared preview has missing or unknown fields");
  }
  const pipeline = dataField(value, "pipeline") as Pipeline;
  if (pipeline === null || typeof pipeline !== "object") throw new TypeError("prepared preview pipeline is invalid");
  const releaseDescriptor = Object.getOwnPropertyDescriptor(value, "release");
  if (releaseDescriptor === undefined || !("value" in releaseDescriptor) || typeof releaseDescriptor.value !== "function") {
    throw new TypeError("prepared preview release is invalid");
  }
  let released = false;
  return {
    pipeline,
    async release() {
      if (released) return;
      released = true;
      await releaseDescriptor.value.call(value);
    },
  };
}

function captureMethod<T extends object, K extends keyof T>(value: T, key: K, label: string): T[K] {
  if (value === null || typeof value !== "object") throw new TypeError(`${label} must be an object`);
  let cursor: object | null = value;
  for (let depth = 0; cursor !== null && depth < 16; depth += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(cursor, key);
    if (descriptor !== undefined) {
      if (!("value" in descriptor) || typeof descriptor.value !== "function") {
        throw new TypeError(`${label} ${String(key)} must be a data method`);
      }
      return descriptor.value.bind(value) as T[K];
    }
    cursor = Object.getPrototypeOf(cursor);
  }
  throw new TypeError(`${label} is missing ${String(key)}`);
}

function dataField(value: unknown, key: string): unknown {
  if (value === null || typeof value !== "object") throw new TypeError("expected an object");
  const descriptor = Object.getOwnPropertyDescriptor(value, key);
  if (descriptor === undefined || !("value" in descriptor) || !descriptor.enumerable) {
    throw new TypeError(`expected data field ${key}`);
  }
  return descriptor.value;
}

function safeBuildId(result: unknown): string | null {
  try {
    return boundedText(dataField(result, "buildId"), MAX_REVISION_SCALARS, "build ID");
  } catch {
    return null;
  }
}

function normalizeDiagnostics(result: unknown): readonly AuthoringPreviewDiagnostic[] {
  try {
    const errors = dataField(result, "errors");
    if (!Array.isArray(errors)) throw new TypeError("errors must be an array");
    const diagnostics: AuthoringPreviewDiagnostic[] = [];
    const count = Math.min(errors.length, MAX_DIAGNOSTICS);
    for (let index = 0; index < count; index += 1) {
      const descriptor = Object.getOwnPropertyDescriptor(errors, String(index));
      if (descriptor === undefined || !("value" in descriptor)) throw new TypeError("diagnostic is not data");
      diagnostics.push(normalizeDiagnostic(descriptor.value));
    }
    if (diagnostics.length === 0) return genericDiagnostic("E_PREVIEW_BUILD", "Preview build failed.");
    return Object.freeze(diagnostics);
  } catch {
    return genericDiagnostic("E_PREVIEW_DIAGNOSTIC", "Preview diagnostics could not be inspected safely.");
  }
}

function normalizeDiagnostic(error: RunError): AuthoringPreviewDiagnostic {
  return Object.freeze({
    severity: "error" as const,
    code: boundedDiagnosticField(dataField(error, "code"), MAX_CODE_SCALARS, "E_PREVIEW_DIAGNOSTIC"),
    stageName: boundedDiagnosticField(dataField(error, "stageName"), MAX_IDENTITY_SCALARS, "unknown-stage"),
    instanceId: boundedDiagnosticField(dataField(error, "instanceId"), MAX_IDENTITY_SCALARS, "unknown-instance"),
    message: boundedDiagnosticField(dataField(error, "message"), MAX_MESSAGE_SCALARS, "Preview build failed."),
  });
}

function boundedDiagnosticField(value: unknown, limit: number, fallback: string): string {
  try { return boundedText(value, limit, "diagnostic"); } catch { return fallback; }
}

function boundedText(value: unknown, limit: number, label: string): string {
  if (typeof value !== "string" || UNSAFE_TEXT.test(value)) throw new TypeError(`${label} is invalid`);
  const scalars = Array.from(value);
  return scalars.length <= limit ? value : scalars.slice(0, limit).join("");
}

function genericDiagnostic(code: string, message: string): readonly AuthoringPreviewDiagnostic[] {
  return Object.freeze([Object.freeze({
    severity: "error" as const,
    code,
    stageName: "preview",
    instanceId: "preview",
    message,
  })]);
}

function failedAttempt(revision: string, code: string, message: string): AuthoringPreviewAttempt {
  return makeAttempt("failed", revision, null, genericDiagnostic(code, message));
}

function makeAttempt(
  outcome: AuthoringPreviewOutcome,
  revision: string,
  buildId: string | null,
  diagnostics: readonly AuthoringPreviewDiagnostic[],
): AuthoringPreviewAttempt {
  return Object.freeze({ outcome, revision, buildId, diagnostics: Object.freeze([...diagnostics]) });
}

function freezeState(state: AuthoringPreviewState): AuthoringPreviewState {
  return Object.freeze({ ...state, diagnostics: Object.freeze([...state.diagnostics]) });
}

/** A closeable never-yielding stream keeps the real watch loop alive. */
function idleChanges(): IdleChanges {
  let closed = false;
  let resolve: ((result: IteratorResult<unknown>) => void) | null = null;
  const iterator: AsyncIterator<unknown> = {
    next() {
      if (closed) return Promise.resolve({ done: true, value: undefined });
      return new Promise<IteratorResult<unknown>>(nextResolve => { resolve = nextResolve; });
    },
    return() {
      closed = true;
      resolve?.({ done: true, value: undefined });
      resolve = null;
      return Promise.resolve({ done: true, value: undefined });
    },
  };
  return {
    [Symbol.asyncIterator]: () => iterator,
    close() { void iterator.return?.(); },
  };
}
