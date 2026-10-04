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
import { types as nodeTypes } from "node:util";
import type {
  Orchestrator,
  Pipeline,
  RunError,
  RunResult,
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
  publish(
    snapshot: PublishedAuthoringPreview,
    commitIfCurrent: AuthoringPreviewCommit,
    signal: AbortSignal,
  ): void | Promise<void>;
  publishFailure(
    failure: FailedAuthoringPreview,
    commitIfCurrent: AuthoringPreviewCommit,
    signal: AbortSignal,
  ): void | Promise<void>;
}

/** All externally visible publisher mutation must occur inside this callback. */
export type AuthoringPreviewCommit = (commit: () => void) => boolean;

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
const MAX_OUTPUTS = 256;
const MAX_OUTPUT_NAME_SCALARS = 256;
const MAX_FILES = 10_000;
const MAX_PATH_SCALARS = 2_048;
const MAX_FILE_BYTES = 16 * 1024 * 1024;
const MAX_TOTAL_OUTPUT_BYTES = 128 * 1024 * 1024;
const UNSAFE_TEXT = /[\u0000-\u001f\u007f-\u009f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/u;
const PATH_SEGMENT = /^[A-Za-z0-9._~!$&'()*+,;=@-]+$/;
const WINDOWS_RESERVED = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i;
const PROTOTYPE_SEGMENTS = new Set(["__proto__", "constructor", "prototype"]);
const TYPED_ARRAY_PROTOTYPE = Object.getPrototypeOf(Uint8Array.prototype) as object;
const TYPED_ARRAY_BYTE_LENGTH = Object.getOwnPropertyDescriptor(TYPED_ARRAY_PROTOTYPE, "byteLength")?.get;
const UINT8_SET = Uint8Array.prototype.set;

interface RequestTask {
  readonly input: AuthoringPreviewInput;
  readonly generation: number;
  readonly resolve: (attempt: AuthoringPreviewAttempt) => void;
  settled: boolean;
}

interface ActiveTask {
  readonly task: RequestTask;
  readonly abort: AbortController;
  watch: SafeWatch | null;
  stopPromise: Promise<boolean> | null;
}

interface SafePrepared {
  readonly pipeline: Pipeline;
  release(): Promise<void>;
}

interface SafeWatch {
  results(): unknown;
  stop(): unknown;
}

interface SafeRunResult {
  readonly outcome: RunResult["outcome"];
  readonly buildId: unknown;
  readonly outputs: unknown;
  readonly errors: unknown;
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
  private disposePromise: Promise<void> | null = null;
  private generation = 0;

  constructor(options: CreateAuthoringPreviewOptions) {
    if (options === null || typeof options !== "object") throw new TypeError("preview options must be an object");
    let debounce: unknown;
    let orchestrator: unknown;
    let materializer: unknown;
    let publisher: unknown;
    try {
      debounce = optionalDataField(options, "debounceMs");
      orchestrator = dataField(options, "orchestrator");
      materializer = dataField(options, "materializer");
      publisher = dataField(options, "publisher");
    } catch {
      throw new TypeError("preview options could not be inspected safely");
    }
    const debounceMs = debounce ?? 100;
    if (typeof debounceMs !== "number" || !Number.isSafeInteger(debounceMs) || debounceMs < 0 || debounceMs > MAX_DEBOUNCE_MS) {
      throw new RangeError(`preview debounceMs must be an integer from 0 through ${MAX_DEBOUNCE_MS}`);
    }
    this.debounceMs = debounceMs;
    try {
      this.watch = captureMethod(orchestrator as Pick<Orchestrator, "watch">, "watch", "orchestrator");
      this.prepare = captureMethod(materializer as AuthoringPreviewMaterializer, "prepare", "materializer");
      this.publish = captureMethod(publisher as AuthoringPreviewPublisher, "publish", "publisher");
      this.publishFailure = captureMethod(publisher as AuthoringPreviewPublisher, "publishFailure", "publisher");
    } catch {
      throw new TypeError("preview adapter methods could not be inspected safely");
    }
  }

  get state(): AuthoringPreviewState {
    return this.currentState;
  }

  request(session: AuthoringSession): Promise<AuthoringPreviewAttempt> {
    if (this.disposed) return Promise.reject(new Error("authoring preview is disposed"));
    let input: AuthoringPreviewInput;
    try {
      input = snapshotSession(session);
    } catch {
      return Promise.reject(new TypeError("authoring session could not be inspected safely"));
    }
    return new Promise(resolve => {
      const task: RequestTask = { input, generation: ++this.generation, resolve, settled: false };
      if (this.pending !== null) this.finish(this.pending, "superseded", null, []);
      this.pending = task;
      if (this.active !== null) {
        this.finish(this.active.task, "superseded", null, []);
        this.cancelActive(this.active);
      }
      this.currentState = freezeState({
        ...this.currentState,
        phase: "building",
        activeRevision: input.revision,
        diagnostics: [],
      });
      this.armDebounce();
    });
  }

  dispose(): Promise<void> {
    if (this.disposePromise !== null) return this.disposePromise;
    this.disposed = true;
    this.disposePromise = this.performDispose();
    return this.disposePromise;
  }

  private async performDispose(): Promise<void> {
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
    if (task.generation === this.generation) {
      this.currentState = freezeState({
        ...this.currentState,
        phase: "building",
        activeRevision: task.input.revision,
        diagnostics: [],
      });
    }

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
        active.watch = safeWatch(this.watch(prepared.pipeline, { changes: idle, debounceMs: 0 }));
      } catch {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_RUN", "Preview pipeline could not be started.");
        return;
      }

      let next: { readonly done: boolean; readonly value?: unknown };
      try {
        next = await firstResult(active.watch.results());
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
      let runResult: SafeRunResult;
      try {
        runResult = safeRunResult(next.value);
      } catch {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_RUN", "Preview pipeline returned an invalid result.");
        return;
      }
      const outcome = runResult.outcome;
      if (outcome === "cancelled") {
        attempt = makeAttempt("cancelled", task.input.revision, null, []);
        return;
      }
      if (outcome !== "success") {
        const diagnostics = normalizeDiagnostics(runResult.errors);
        attempt = makeAttempt("failed", task.input.revision, safeBuildId(runResult.buildId), diagnostics);
        return;
      }

      const buildId = safeBuildId(runResult.buildId);
      if (buildId === null) {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_OUTPUT", "Preview output was invalid.");
        return;
      }
      try {
        snapshot = safeSnapshotFromOutputs(buildId, runResult.outputs);
      } catch {
        attempt = failedAttempt(task.input.revision, "E_PREVIEW_OUTPUT", "Preview output was invalid.");
        return;
      }
      attempt = makeAttempt("ready", task.input.revision, buildId, []);
    } finally {
      idle?.close();
      let stopped = true;
      let released = true;
      try {
        if (active.watch !== null) stopped = await this.stopWatch(active);
      } finally {
        if (prepared !== null) {
          try { await prepared.release(); } catch { released = false; }
        }
      }

      if (!task.settled && !this.disposed) {
        if (!stopped) {
          attempt = failedAttempt(task.input.revision, "E_PREVIEW_CLEANUP", "Preview pipeline could not be retired safely.");
          snapshot = null;
        } else if (!released) {
          attempt = failedAttempt(task.input.revision, "E_PREVIEW_RELEASE", "Preview input could not be released safely.");
          snapshot = null;
        }
        if (attempt === null) {
          attempt = makeAttempt(active.abort.signal.aborted ? "cancelled" : "failed", task.input.revision, null, []);
        }
        await this.publishAttempt(active, attempt, snapshot);
      }
      if (this.active === active) this.active = null;
    }
  }

  private async publishAttempt(
    active: ActiveTask,
    attempt: AuthoringPreviewAttempt,
    snapshot: PreviewSnapshot | null,
  ): Promise<void> {
    const task = active.task;
    if (!this.isCurrent(active)) return;
    let finalAttempt = attempt;
    if (attempt.outcome === "ready" && snapshot !== null) {
      finalAttempt = await this.runPublisher(
        active,
        commit => this.publish(Object.freeze({ ...snapshot, revision: task.input.revision }), commit, active.abort.signal),
        attempt,
        "Preview output could not be published.",
      );
    } else if (attempt.outcome === "failed") {
      finalAttempt = await this.runPublisher(
        active,
        commit => this.publishFailure(
          Object.freeze({ revision: task.input.revision, diagnostics: attempt.diagnostics }),
          commit,
          active.abort.signal,
        ),
        attempt,
        "Preview failure could not be published.",
      );
    }

    if (!this.isCurrent(active)) return;

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

  private async runPublisher(
    active: ActiveTask,
    publish: (commit: AuthoringPreviewCommit) => void | Promise<void>,
    success: AuthoringPreviewAttempt,
    failureMessage: string,
  ): Promise<AuthoringPreviewAttempt> {
    let open = true;
    let used = false;
    let committed = false;
    let commitStarted = false;
    const commit: AuthoringPreviewCommit = mutation => {
      if (!open || used || !this.isCurrent(active)) return false;
      used = true;
      commitStarted = true;
      const returned = mutation();
      if (returned !== undefined) throw new TypeError("preview publisher commit must be synchronous");
      committed = true;
      return true;
    };
    const publisher = Promise.resolve().then(() => publish(commit)).then(
      () => ({ kind: "settled" as const }),
      () => ({ kind: "rejected" as const }),
    );
    let abortListener: (() => void) | null = null;
    const aborted = new Promise<{ readonly kind: "aborted" }>(resolve => {
      abortListener = () => {
        open = false;
        resolve({ kind: "aborted" });
      };
      if (active.abort.signal.aborted) abortListener();
      else active.abort.signal.addEventListener("abort", abortListener, { once: true });
    });
    const result = await Promise.race([publisher, aborted]);
    if (abortListener !== null) active.abort.signal.removeEventListener("abort", abortListener);
    open = false;
    if (result.kind === "aborted") return success;
    if (result.kind === "rejected") {
      return failedAttempt(
        active.task.input.revision,
        commitStarted ? "E_PREVIEW_PUBLISH_INDETERMINATE" : "E_PREVIEW_PUBLISH",
        commitStarted ? "Preview publication completed indeterminately." : failureMessage,
      );
    }
    if (!this.isCurrent(active)) return success;
    if (!committed) {
      return failedAttempt(active.task.input.revision, "E_PREVIEW_PUBLISH", failureMessage);
    }
    return success;
  }

  private isCurrent(active: ActiveTask): boolean {
    return this.active === active
      && active.task.generation === this.generation
      && !active.task.settled
      && !this.disposed
      && !active.abort.signal.aborted;
  }

  private cancelActive(active: ActiveTask): void {
    active.abort.abort();
    if (active.watch !== null) void this.stopWatch(active);
  }

  private stopWatch(active: ActiveTask): Promise<boolean> {
    if (active.stopPromise === null) {
      active.stopPromise = Promise.resolve().then(() => active.watch?.stop()).then(() => true, () => false);
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
  const project = validateAuthoringProject(snapshotProperty(session, "project", "authoring session"));
  const revision = exactBoundedText(
    snapshotProperty(session, "storageRevision", "authoring session"),
    MAX_REVISION_SCALARS,
    "revision",
  );
  if (revision.length === 0) throw new TypeError("authoring revision must not be empty");
  return Object.freeze({ revision, project });
}

function snapshotProperty(value: object, key: PropertyKey, label: string): unknown {
  let cursor: object | null = value;
  for (let depth = 0; cursor !== null && depth < 16; depth += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(cursor, key);
    if (descriptor !== undefined) {
      if ("value" in descriptor) return descriptor.value;
      if (typeof descriptor.get !== "function") throw new TypeError(`${label} ${String(key)} is unreadable`);
      return Reflect.apply(descriptor.get, value, []);
    }
    cursor = Object.getPrototypeOf(cursor);
  }
  throw new TypeError(`${label} is missing ${String(key)}`);
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

function captureCallable(value: unknown, key: PropertyKey, label: string): (...args: readonly unknown[]) => unknown {
  if (value === null || typeof value !== "object") throw new TypeError(`${label} must be an object`);
  let cursor: object | null = value;
  for (let depth = 0; cursor !== null && depth < 16; depth += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(cursor, key);
    if (descriptor !== undefined) {
      if (!("value" in descriptor) || typeof descriptor.value !== "function") {
        throw new TypeError(`${label} ${String(key)} must be a data method`);
      }
      return descriptor.value.bind(value) as (...args: readonly unknown[]) => unknown;
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

function optionalDataField(value: unknown, key: string): unknown {
  if (value === null || typeof value !== "object") throw new TypeError("expected an object");
  const descriptor = Object.getOwnPropertyDescriptor(value, key);
  if (descriptor === undefined) return undefined;
  if (!("value" in descriptor) || !descriptor.enumerable) throw new TypeError(`expected data field ${key}`);
  return descriptor.value;
}

function safeWatch(value: unknown): SafeWatch {
  return Object.freeze({
    results: captureCallable(value, "results", "watch session"),
    stop: captureCallable(value, "stop", "watch session"),
  });
}

async function firstResult(stream: unknown): Promise<{ readonly done: boolean; readonly value?: unknown }> {
  const iteratorFactory = captureCallable(stream, Symbol.asyncIterator, "watch result stream");
  const iterator = iteratorFactory();
  const next = captureCallable(iterator, "next", "watch result iterator");
  const raw = await next();
  const done = dataField(raw, "done");
  if (typeof done !== "boolean") throw new TypeError("iterator result done must be a boolean");
  if (done) return Object.freeze({ done: true });
  return Object.freeze({ done: false, value: dataField(raw, "value") });
}

function safeRunResult(value: unknown): SafeRunResult {
  const outcome = dataField(value, "outcome");
  if (outcome !== "success" && outcome !== "partial" && outcome !== "failed" && outcome !== "cancelled") {
    throw new TypeError("preview outcome is invalid");
  }
  return Object.freeze({
    outcome,
    buildId: dataField(value, "buildId"),
    outputs: dataField(value, "outputs"),
    errors: dataField(value, "errors"),
  });
}

function safeBuildId(value: unknown): string | null {
  try {
    const buildId = exactBoundedText(value, MAX_REVISION_SCALARS, "build ID");
    return buildId.length === 0 ? null : buildId;
  } catch {
    return null;
  }
}

function normalizeDiagnostics(errors: unknown): readonly AuthoringPreviewDiagnostic[] {
  try {
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
  try { return truncateDiagnosticText(value, limit); } catch { return fallback; }
}

function exactBoundedText(value: unknown, limit: number, label: string): string {
  if (typeof value !== "string") throw new TypeError(`${label} is invalid`);
  let count = 0;
  for (const scalar of value) {
    const codeUnit = scalar.charCodeAt(0);
    const loneSurrogate = scalar.length === 1 && codeUnit >= 0xd800 && codeUnit <= 0xdfff;
    if (loneSurrogate || UNSAFE_TEXT.test(scalar) || ++count > limit) throw new TypeError(`${label} is invalid`);
  }
  return value;
}

function truncateDiagnosticText(value: unknown, limit: number): string {
  if (typeof value !== "string") throw new TypeError("diagnostic is invalid");
  let result = "";
  let count = 0;
  for (const scalar of value) {
    if (count === limit) break;
    const codeUnit = scalar.charCodeAt(0);
    if ((scalar.length === 1 && codeUnit >= 0xd800 && codeUnit <= 0xdfff) || UNSAFE_TEXT.test(scalar)) {
      throw new TypeError("diagnostic is invalid");
    }
    result += scalar;
    count += 1;
  }
  return result;
}

function safeSnapshotFromOutputs(buildId: string, value: unknown): PreviewSnapshot {
  const outputNames = boundedRecordKeys(value, MAX_OUTPUTS, "preview outputs");
  const copiedOutputs: Record<string, unknown> = Object.create(null) as Record<string, unknown>;
  const portablePaths: string[] = [];
  let fileCount = 0;
  let totalBytes = 0;
  for (const outputName of outputNames) {
    exactBoundedText(outputName, MAX_OUTPUT_NAME_SCALARS, "output name");
    const artifact = dataField(value, outputName);
    const variant = dataField(artifact, "variant");
    if (dataField(variant, "kind") !== "dist-tree") throw new TypeError("preview output is not a dist tree");
    const files = dataField(artifact, "files");
    const paths = boundedRecordKeys(files, MAX_FILES - fileCount, "preview files");
    const copiedFiles: Record<string, Uint8Array> = Object.create(null) as Record<string, Uint8Array>;
    for (const path of paths) {
      fileCount += 1;
      exactBoundedText(path, MAX_PATH_SCALARS, "artifact path");
      validatePortablePath(path);
      const bytes = dataField(files, path);
      const copied = copyBoundedBytes(bytes);
      totalBytes += copied.byteLength;
      if (!Number.isSafeInteger(totalBytes) || totalBytes > MAX_TOTAL_OUTPUT_BYTES) {
        throw new TypeError("preview output exceeds the aggregate byte limit");
      }
      portablePaths.push(path);
      copiedFiles[path] = copied;
    }
    copiedOutputs[outputName] = { variant: { kind: "dist-tree" }, files: copiedFiles };
  }
  rejectPortableCollisions(portablePaths);
  const snapshot = snapshotFromOutputs(buildId, copiedOutputs);
  return Object.freeze({ buildId: snapshot.buildId, files: new ImmutableSnapshotFiles(snapshot.files) });
}

function boundedRecordKeys(value: unknown, limit: number, label: string): readonly string[] {
  if (value === null || typeof value !== "object") throw new TypeError(`${label} must be an object`);
  if (nodeTypes.isProxy(value)) throw new TypeError(`${label} must not be a proxy`);
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) throw new TypeError(`${label} must be a plain record`);
  const strings: string[] = [];
  for (const key in value) {
    if (!Object.hasOwn(value, key)) continue;
    if (strings.length === limit) throw new TypeError(`${label} exceeds its entry limit`);
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    if (descriptor === undefined || !("value" in descriptor) || !descriptor.enumerable) {
      throw new TypeError(`${label} contains a non-data field`);
    }
    strings.push(key);
  }
  return strings;
}

function copyBoundedBytes(value: unknown): Uint8Array {
  if (!(value instanceof Uint8Array) || TYPED_ARRAY_BYTE_LENGTH === undefined) {
    throw new TypeError("preview file bytes are invalid");
  }
  let byteLength: number;
  try {
    byteLength = Reflect.apply(TYPED_ARRAY_BYTE_LENGTH, value, []) as number;
  } catch {
    throw new TypeError("preview file bytes are invalid");
  }
  if (byteLength > MAX_FILE_BYTES) throw new TypeError("preview file bytes are invalid");
  const copy = new Uint8Array(byteLength);
  try {
    Reflect.apply(UINT8_SET, copy, [value]);
  } catch {
    throw new TypeError("preview file bytes are invalid");
  }
  return copy;
}

function validatePortablePath(path: string): void {
  if (path.length === 0 || path.length > 2_048 || path.startsWith("/") || path.startsWith("\\") || /^[A-Za-z]:/.test(path)) {
    throw new TypeError("artifact path must be a portable relative path");
  }
  if (path.includes("\\")) throw new TypeError("artifact path must use '/' separators");
  for (const segment of path.split("/")) {
    if (segment.length === 0 || segment === "." || segment === "..") throw new TypeError("artifact path has an unsafe segment");
    if (!PATH_SEGMENT.test(segment) || segment.length > 255) throw new TypeError("artifact path has a non-portable segment");
    if (segment.endsWith(".") || segment.endsWith(" ")) throw new TypeError("artifact path has an unsafe suffix");
    if (WINDOWS_RESERVED.test(segment)) throw new TypeError("artifact path uses a reserved device name");
    if (PROTOTYPE_SEGMENTS.has(segment)) throw new TypeError("artifact path uses a reserved object name");
  }
}

function rejectPortableCollisions(paths: readonly string[]): void {
  const folded = paths.map(path => path.toLowerCase()).sort();
  for (let index = 1; index < folded.length; index += 1) {
    const previous = folded[index - 1]!;
    const current = folded[index]!;
    if (current === previous || current.startsWith(`${previous}/`)) {
      throw new TypeError("preview outputs contain a non-portable path collision");
    }
  }
}

class ImmutableSnapshotFiles implements ReadonlyMap<string, Uint8Array> {
  readonly #files: ReadonlyMap<string, Uint8Array>;

  constructor(files: ReadonlyMap<string, Uint8Array>) {
    this.#files = files;
    Object.freeze(this);
  }

  get size(): number { return this.#files.size; }

  has(key: string): boolean { return this.#files.has(key); }

  get(key: string): Uint8Array | undefined {
    const bytes = this.#files.get(key);
    return bytes === undefined ? undefined : new Uint8Array(bytes);
  }

  keys(): MapIterator<string> { return this.#files.keys(); }

  *values(): MapIterator<Uint8Array> {
    for (const bytes of this.#files.values()) yield new Uint8Array(bytes);
  }

  *entries(): MapIterator<[string, Uint8Array]> {
    for (const [path, bytes] of this.#files) yield [path, new Uint8Array(bytes)];
  }

  [Symbol.iterator](): MapIterator<[string, Uint8Array]> { return this.entries(); }

  forEach(
    callbackfn: (value: Uint8Array, key: string, map: ReadonlyMap<string, Uint8Array>) => void,
    thisArg?: unknown,
  ): void {
    for (const [path, bytes] of this.#files) callbackfn.call(thisArg, new Uint8Array(bytes), path, this);
  }
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
