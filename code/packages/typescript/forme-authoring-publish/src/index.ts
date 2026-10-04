/**
 * Exact-revision publication for the Forme authoring shell.
 *
 * Editing, building, deploying, and durable acknowledgement are deliberately
 * separate commit points. This coordinator gives them one fail-closed order:
 * snapshot -> build -> FM08 validation -> reviewed target -> retirement ->
 * exact-revision authoring transaction.
 */

import { createHash } from "node:crypto";
import {
  validateAuthoringProject,
  type AuthoringProject,
  type AuthoringPublicationCommand,
  type AuthoringSession,
} from "@coding-adventures/forme-authoring-core";
import {
  canonicalDeployManifest,
  createVerifiedContentReader,
  parseDeployManifest,
  type ContentStore,
  type DeployManifest,
} from "@coding-adventures/forme-deploy-runner-core";

export interface AuthoringPublishInput {
  readonly revision: string;
  readonly project: AuthoringProject;
}

export interface PreparedAuthoringPublication {
  readonly manifest: unknown;
  readonly contentStore: ContentStore;
  release(): Promise<void>;
}

export interface AuthoringPublicationBuilder {
  build(input: AuthoringPublishInput, signal: AbortSignal): Promise<PreparedAuthoringPublication>;
}

export interface AuthoringPublishTargetReview {
  readonly targetId: string;
  readonly label: string;
  readonly destination: string;
}

export interface AuthoringPublishTargetInput {
  readonly manifest: DeployManifest;
  readonly contentStore: ContentStore;
  readonly manifestSha256: string;
}

export interface AuthoringPublishTargetResult {
  readonly outcome: "success" | "failed" | "indeterminate";
  readonly manifestSha256: string;
}

export interface AuthoringPublishTarget {
  readonly review: AuthoringPublishTargetReview;
  /** Rejection is contractually pre-commit; uncertain post-commit work resolves indeterminate. */
  publish(
    input: AuthoringPublishTargetInput,
    signal: AbortSignal,
  ): Promise<AuthoringPublishTargetResult>;
}

export interface AuthoringPublishDiagnostic {
  readonly severity: "error";
  readonly code: string;
  readonly message: string;
}

export type AuthoringPublishOutcome = "published" | "failed" | "cancelled" | "indeterminate";

export interface AuthoringPublishAttempt {
  readonly outcome: AuthoringPublishOutcome;
  readonly revision: string;
  readonly manifestSha256: string | null;
  readonly targetId: string;
  readonly diagnostics: readonly AuthoringPublishDiagnostic[];
}

export interface AuthoringPublishState {
  readonly phase: "idle" | "building" | "deploying" | "recording" | "published" | "failed" | "indeterminate" | "disposed";
  readonly revision: string | null;
  readonly manifestSha256: string | null;
  readonly diagnostics: readonly AuthoringPublishDiagnostic[];
}

export interface CreateAuthoringPublisherOptions {
  readonly builder: AuthoringPublicationBuilder;
  readonly target: AuthoringPublishTarget;
}

export interface AuthoringPublisher {
  readonly target: AuthoringPublishTargetReview;
  readonly state: AuthoringPublishState;
  publish(session: AuthoringSession, signal?: AbortSignal): Promise<AuthoringPublishAttempt>;
  dispose(): Promise<void>;
}

const MAX_REVISION_SCALARS = 1_024;
const MAX_TARGET_ID_SCALARS = 256;
const MAX_LABEL_SCALARS = 512;
const MAX_DESTINATION_SCALARS = 2_048;
const SAFE_TARGET_ID = /^[a-z0-9]+(?:[._-][a-z0-9]+)*$/;
const UNSAFE_TEXT = /[\u0000-\u001f\u007f-\u009f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/u;
const APPLY = Reflect.apply;
const ABORT = AbortController.prototype.abort;
const ABORTED = Object.getOwnPropertyDescriptor(AbortSignal.prototype, "aborted")?.get;
const ADD_EVENT_LISTENER = EventTarget.prototype.addEventListener;
const REMOVE_EVENT_LISTENER = EventTarget.prototype.removeEventListener;

interface SafePrepared {
  readonly manifest: DeployManifest;
  readonly contentStore: ContentStore;
  release(): Promise<void>;
}

const PREPARATION_CLEANUP_FAILED = Symbol("preparation cleanup failed");

interface ActiveAction {
  readonly abort: AbortController;
  readonly promise: Promise<AuthoringPublishAttempt>;
}

export function createAuthoringPublisher(options: CreateAuthoringPublisherOptions): AuthoringPublisher {
  return new Publisher(options);
}

class Publisher implements AuthoringPublisher {
  readonly target: AuthoringPublishTargetReview;
  private readonly build: AuthoringPublicationBuilder["build"];
  private readonly deploy: AuthoringPublishTarget["publish"];
  private currentState: AuthoringPublishState = state("idle", null, null, []);
  private active: ActiveAction | null = null;
  private disposed = false;
  private poisoned = false;

  constructor(options: CreateAuthoringPublisherOptions) {
    try {
      const fields = exactDataObject(options, ["builder", "target"], "publish options");
      const builder = fields.builder;
      const target = fields.target;
      this.build = captureMethod<AuthoringPublicationBuilder["build"]>(builder, "build", "builder");
      this.deploy = captureMethod<AuthoringPublishTarget["publish"]>(target, "publish", "target");
      const targetFields = exactDataObject(target, ["publish", "review"], "publish target");
      this.target = targetReview(targetFields.review);
    } catch {
      throw new TypeError("authoring publisher options could not be inspected safely");
    }
  }

  get state(): AuthoringPublishState { return this.currentState; }

  publish(session: AuthoringSession, signal?: AbortSignal): Promise<AuthoringPublishAttempt> {
    if (this.disposed) return Promise.reject(new Error("authoring publisher is disposed"));
    let snapshot: SessionSnapshot;
    try { snapshot = snapshotSession(session); } catch {
      return Promise.reject(new TypeError("authoring session could not be inspected safely"));
    }
    if (this.poisoned) {
      return Promise.resolve(attempt(
        "indeterminate",
        snapshot.revision,
        null,
        this.target.targetId,
        [diagnostic("E_PUBLISH_RECONCILE", "Publication state must be reconciled before retrying.")],
      ));
    }
    if (this.active !== null) {
      return Promise.resolve(attempt(
        "failed",
        snapshot.revision,
        null,
        this.target.targetId,
        [diagnostic("E_PUBLISH_BUSY", "Another publication action is already active.")],
      ));
    }
    const abort = new AbortController();
    const unlink = forwardAbort(signal, abort);
    this.currentState = state("building", snapshot.revision, null, []);
    const promise = this.run(snapshot, abort.signal).finally(() => {
      unlink();
      if (this.active?.promise === promise) this.active = null;
    });
    this.active = { abort, promise };
    return promise;
  }

  async dispose(): Promise<void> {
    if (this.disposed) return;
    this.disposed = true;
    const active = this.active;
    if (active !== null) {
      APPLY(ABORT, active.abort, []);
      try { await active.promise; } catch { /* public actions convert operational failures */ }
    }
    this.currentState = state("disposed", null, null, []);
  }

  private async run(
    snapshot: SessionSnapshot,
    signal: AbortSignal,
  ): Promise<AuthoringPublishAttempt> {
    let prepared: SafePrepared | null = null;
    let manifestSha256: string | null = null;
    try {
      throwIfAborted(signal);
      const raw = await APPLY(this.build, undefined, [
        Object.freeze({ revision: snapshot.revision, project: snapshot.project }),
        signal,
      ]) as PreparedAuthoringPublication;
      try {
        prepared = await safePrepared(raw, signal);
      } catch (error) {
        if (error === PREPARATION_CLEANUP_FAILED) {
          return this.poisonedAttempt(snapshot.revision, null, "E_PUBLISH_CLEANUP", "Publication build cleanup failed.");
        }
        throw error;
      }
      manifestSha256 = createHash("sha256")
        .update(canonicalDeployManifest(prepared.manifest), "utf8")
        .digest("base64");
      if (currentRevision(snapshot.session, signal) !== snapshot.revision) {
        const cleanup = await retire(prepared);
        prepared = null;
        if (!cleanup) return this.poisonedAttempt(snapshot.revision, manifestSha256, "E_PUBLISH_CLEANUP", "Publication build cleanup failed.");
        return this.failed(snapshot.revision, manifestSha256, "E_PUBLISH_STALE", "The authoring project changed before deployment.");
      }
      throwIfAborted(signal);
      this.currentState = state("deploying", snapshot.revision, manifestSha256, []);
      let rawResult: unknown;
      try {
        rawResult = await APPLY(this.deploy, undefined, [Object.freeze({
          manifest: prepared.manifest,
          contentStore: prepared.contentStore,
          manifestSha256,
        }), signal]);
      } catch {
        const cleanup = await retire(prepared);
        prepared = null;
        if (!cleanup) return this.poisonedAttempt(snapshot.revision, manifestSha256, "E_PUBLISH_CLEANUP", "Publication build cleanup failed.");
        return this.failedOrCancelled(snapshot.revision, manifestSha256, signal, "E_PUBLISH_TARGET", "The reviewed target rejected publication before commit.");
      }
      const result = targetResult(rawResult, manifestSha256);
      if (result === null) {
        const cleanup = await retire(prepared);
        prepared = null;
        if (!cleanup) return this.poisonedAttempt(snapshot.revision, manifestSha256, "E_PUBLISH_CLEANUP", "Publication build cleanup failed.");
        return this.poisonedAttempt(snapshot.revision, manifestSha256, "E_PUBLISH_TARGET", "The reviewed target returned an indeterminate acknowledgement.");
      }
      const cleanup = await retire(prepared);
      prepared = null;
      if (!cleanup) return this.poisonedAttempt(snapshot.revision, manifestSha256, "E_PUBLISH_CLEANUP", "Publication build cleanup failed.");
      if (result === "failed") {
        return this.failedOrCancelled(snapshot.revision, manifestSha256, signal, "E_PUBLISH_TARGET", "The reviewed target did not publish the site.");
      }
      if (result === "indeterminate") {
        return this.poisonedAttempt(snapshot.revision, manifestSha256, "E_PUBLISH_INDETERMINATE", "The reviewed target could not confirm publication state.");
      }
      this.currentState = state("recording", snapshot.revision, manifestSha256, []);
      const command: AuthoringPublicationCommand = Object.freeze({
        type: "record-publication",
        publication: Object.freeze({
          authoringRevision: snapshot.revision,
          manifestSha256,
          targetId: this.target.targetId,
        }),
      });
      try {
        await APPLY(snapshot.record, undefined, [snapshot.revision, command, signal]);
      } catch {
        return this.poisonedAttempt(snapshot.revision, manifestSha256, "E_PUBLISH_RECORD", "Published output could not be recorded durably.");
      }
      const value = attempt("published", snapshot.revision, manifestSha256, this.target.targetId, []);
      this.currentState = state("published", snapshot.revision, manifestSha256, []);
      return value;
    } catch {
      if (prepared !== null) {
        const cleanup = await retire(prepared);
        if (!cleanup) return this.poisonedAttempt(snapshot.revision, manifestSha256, "E_PUBLISH_CLEANUP", "Publication build cleanup failed.");
      }
      return this.failedOrCancelled(snapshot.revision, manifestSha256, signal, "E_PUBLISH_BUILD", "The exact authoring revision could not be prepared for publication.");
    }
  }

  private failed(revision: string, identity: string | null, code: string, message: string): AuthoringPublishAttempt {
    const diagnostics = [diagnostic(code, message)];
    this.currentState = state("failed", revision, identity, diagnostics);
    return attempt("failed", revision, identity, this.target.targetId, diagnostics);
  }

  private failedOrCancelled(
    revision: string,
    identity: string | null,
    signal: AbortSignal,
    code: string,
    message: string,
  ): AuthoringPublishAttempt {
    if (isAborted(signal)) {
      const diagnostics = [diagnostic("E_PUBLISH_CANCELLED", "Publication was cancelled before commit.")];
      this.currentState = state("failed", revision, identity, diagnostics);
      return attempt("cancelled", revision, identity, this.target.targetId, diagnostics);
    }
    return this.failed(revision, identity, code, message);
  }

  private poisonedAttempt(revision: string, identity: string | null, code: string, message: string): AuthoringPublishAttempt {
    this.poisoned = true;
    const diagnostics = [diagnostic(code, message)];
    this.currentState = state("indeterminate", revision, identity, diagnostics);
    return attempt("indeterminate", revision, identity, this.target.targetId, diagnostics);
  }
}

async function safePrepared(value: unknown, signal: AbortSignal): Promise<SafePrepared> {
  const release = captureOwnMethod<PreparedAuthoringPublication["release"]>(value, "release", "prepared publication");
  let released = false;
  const retireOnce = async (): Promise<void> => {
    if (released) return;
    released = true;
    await APPLY(release, value, []);
  };
  try {
    const fields = exactDataObject(value, ["manifest", "contentStore", "release"], "prepared publication");
    const manifest = parseDeployManifest(fields.manifest);
    const contentStore = safeContentStore(fields.contentStore);
    await createVerifiedContentReader(manifest, contentStore, { signal }).preflight();
    return Object.freeze({
      manifest,
      contentStore,
      release: retireOnce,
    });
  } catch (error) {
    try { await retireOnce(); } catch { throw PREPARATION_CLEANUP_FAILED; }
    throw error;
  }
}

function safeContentStore(value: unknown): ContentStore {
  const get = captureMethod<ContentStore["get"]>(value, "get", "content store");
  const has = captureMethod<ContentStore["has"]>(value, "has", "content store");
  const hashes = captureMethod<ContentStore["hashes"]>(value, "hashes", "content store");
  return Object.freeze({ get, has, hashes });
}

async function retire(prepared: SafePrepared): Promise<boolean> {
  try { await prepared.release(); return true; } catch { return false; }
}

interface SessionSnapshot {
  readonly revision: string;
  readonly project: AuthoringProject;
  readonly record: AuthoringSession["dispatchAtRevision"];
  readonly session: AuthoringSession;
}

function snapshotSession(session: AuthoringSession): SessionSnapshot {
  if (session === null || typeof session !== "object") throw new TypeError("session must be an object");
  const revision = exactIdentity(session.storageRevision, MAX_REVISION_SCALARS, "revision");
  const project = validateAuthoringProject(session.project);
  const record = captureMethod<AuthoringSession["dispatchAtRevision"]>(session, "dispatchAtRevision", "session");
  return Object.freeze({ revision, project, record, session });
}

function currentRevision(session: AuthoringSession, signal: AbortSignal): string {
  throwIfAborted(signal);
  return exactIdentity(session.storageRevision, MAX_REVISION_SCALARS, "revision");
}

function captureMethod<T extends Function>(value: unknown, name: string, label: string): T {
  if (value === null || (typeof value !== "object" && typeof value !== "function")) throw new TypeError(`${label} must be an object`);
  let cursor: object | null = value as object;
  for (let depth = 0; cursor !== null && depth < 16; depth += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(cursor, name);
    if (descriptor !== undefined) {
      if (!("value" in descriptor) || typeof descriptor.value !== "function") throw new TypeError(`${label} ${name} must be a data method`);
      const method = descriptor.value as T;
      const bound = ((...args: unknown[]) => APPLY(method, value, args)) as unknown as T;
      return bound;
    }
    cursor = Object.getPrototypeOf(cursor);
  }
  throw new TypeError(`${label} is missing ${name}`);
}

function captureOwnMethod<T extends Function>(value: unknown, name: string, label: string): T {
  if (value === null || typeof value !== "object" || Object.getPrototypeOf(value) !== Object.prototype) {
    throw new TypeError(`${label} must be a plain object`);
  }
  const descriptor = Object.getOwnPropertyDescriptor(value, name);
  if (descriptor === undefined || !("value" in descriptor) || typeof descriptor.value !== "function") {
    throw new TypeError(`${label} ${name} must be a data method`);
  }
  return descriptor.value as T;
}

function exactDataObject(
  value: unknown,
  keys: readonly string[],
  label: string,
): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Object.getPrototypeOf(value) !== Object.prototype) {
    throw new TypeError(`${label} must be a plain object`);
  }
  const ownKeys = Reflect.ownKeys(value as object);
  if (ownKeys.some((key) => typeof key !== "string")) throw new TypeError(`${label} contains symbol fields`);
  const actual = [...ownKeys as string[]].sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    throw new TypeError(`${label} has missing or unknown fields`);
  }
  const result: Record<string, unknown> = {};
  for (const key of actual) {
    const descriptor = Object.getOwnPropertyDescriptor(value as object, key);
    if (descriptor === undefined || !("value" in descriptor) || !descriptor.enumerable) throw new TypeError(`${label} contains accessors`);
    result[key] = descriptor.value;
  }
  return result;
}

function targetReview(value: unknown): AuthoringPublishTargetReview {
  const fields = exactDataObject(value, ["targetId", "label", "destination"], "target review");
  const targetId = exactIdentity(fields.targetId, MAX_TARGET_ID_SCALARS, "target identity");
  if (!SAFE_TARGET_ID.test(targetId)) throw new TypeError("target identity must be portable lowercase text");
  return Object.freeze({
    targetId,
    label: exactText(fields.label, MAX_LABEL_SCALARS, "target label"),
    destination: exactText(fields.destination, MAX_DESTINATION_SCALARS, "target destination"),
  });
}

function targetResult(value: unknown, expectedIdentity: string): AuthoringPublishTargetResult["outcome"] | null {
  try {
    const fields = exactDataObject(value, ["outcome", "manifestSha256"], "target result");
    if (fields.manifestSha256 !== expectedIdentity) return null;
    if (fields.outcome !== "success" && fields.outcome !== "failed" && fields.outcome !== "indeterminate") return null;
    return fields.outcome;
  } catch { return null; }
}

function exactIdentity(value: unknown, maximum: number, label: string): string {
  return exactText(value, maximum, label);
}

function exactText(value: unknown, maximum: number, label: string): string {
  if (typeof value !== "string" || value.length === 0 || value.trim() !== value) throw new TypeError(`${label} must be non-empty trimmed text`);
  let scalars = 0;
  for (const _scalar of value) scalars += 1;
  if (scalars > maximum || UNSAFE_TEXT.test(value)) throw new TypeError(`${label} is unsafe or too long`);
  return value;
}

function forwardAbort(source: AbortSignal | undefined, target: AbortController): () => void {
  if (source === undefined) return () => {};
  const abort = (): void => { APPLY(ABORT, target, []); };
  if (isAborted(source)) abort();
  else APPLY(ADD_EVENT_LISTENER, source, ["abort", abort, { once: true }]);
  return () => { try { APPLY(REMOVE_EVENT_LISTENER, source, ["abort", abort]); } catch { /* best effort */ } };
}

function throwIfAborted(signal: AbortSignal): void {
  if (isAborted(signal)) throw new DOMException("The operation was aborted", "AbortError");
}

function isAborted(signal: AbortSignal): boolean {
  if (ABORTED === undefined) throw new TypeError("AbortSignal intrinsic is unavailable");
  return APPLY(ABORTED, signal, []) as boolean;
}

function diagnostic(code: string, message: string): AuthoringPublishDiagnostic {
  return Object.freeze({ severity: "error", code, message });
}

function attempt(
  outcome: AuthoringPublishOutcome,
  revision: string,
  manifestSha256: string | null,
  targetId: string,
  diagnostics: readonly AuthoringPublishDiagnostic[],
): AuthoringPublishAttempt {
  return Object.freeze({
    outcome,
    revision,
    manifestSha256,
    targetId,
    diagnostics: Object.freeze([...diagnostics]),
  });
}

function state(
  phase: AuthoringPublishState["phase"],
  revision: string | null,
  manifestSha256: string | null,
  diagnostics: readonly AuthoringPublishDiagnostic[],
): AuthoringPublishState {
  return Object.freeze({
    phase,
    revision,
    manifestSha256,
    diagnostics: Object.freeze([...diagnostics]),
  });
}
