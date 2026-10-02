/** Atomic authoring transactions with persistent bounded undo and redo. */

import { canonicalJson, encodeCanonicalJson } from "./canonical.js";
import { AuthoringError, invalidState } from "./error.js";
import { resolveLimits, validateAuthoringProject } from "./project.js";
import type {
  AuthoringCommand,
  AuthoringLimits,
  AuthoringProject,
  AuthoringSession,
  AuthoringStorage,
  OpenAuthoringSessionOptions,
  StoredAuthoringState,
} from "./types.js";

interface PersistedSession {
  readonly schemaVersion: 1;
  readonly historyLimit: number;
  readonly cursor: number;
  readonly history: readonly AuthoringProject[];
}

function abortIfNeeded(signal?: AbortSignal): void {
  if (signal?.aborted) throw signal.reason ?? new DOMException("The operation was aborted", "AbortError");
}

function validateRevision(value: unknown, source: "stored" | "adapter"): string {
  if (typeof value !== "string" || value.length < 1 || value.length > 1_024 || /[\u0000-\u001f\u007f]/.test(value)) {
    invalidState(`${source} revision is invalid`);
  }
  return value;
}

function exactObject(value: unknown, keys: readonly string[]): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Object.getPrototypeOf(value) !== Object.prototype) invalidState("expected a plain object");
  const descriptors = Object.getOwnPropertyDescriptors(value);
  const actual = Object.keys(descriptors).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) invalidState("stored object has missing or unknown fields");
  if (Object.values(descriptors).some((item) => !("value" in item) || !item.enumerable)) invalidState("stored object contains an accessor or hidden field");
  return value as Record<string, unknown>;
}

function decodeStored(stored: StoredAuthoringState, limits: AuthoringLimits): PersistedSession {
  if (!(stored.bytes instanceof Uint8Array)) invalidState("stored bytes are not a Uint8Array");
  if (stored.bytes.byteLength > limits.maxJsonBytes) invalidState("stored bytes exceed the hard byte limit");
  let text: string;
  try { text = new TextDecoder("utf-8", { fatal: true }).decode(stored.bytes); } catch { invalidState("stored bytes are not valid UTF-8"); }
  let parsed: unknown;
  try { parsed = JSON.parse(text!); } catch { invalidState("stored bytes are not valid JSON"); }
  const node = exactObject(parsed, ["schemaVersion", "historyLimit", "cursor", "history"]);
  if (node.schemaVersion !== 1) invalidState("unsupported schema version");
  if (!Number.isSafeInteger(node.historyLimit) || (node.historyLimit as number) < 1 || (node.historyLimit as number) > limits.maxHistoryEntries) invalidState("history limit is invalid");
  if (!Array.isArray(node.history) || node.history.length < 1 || node.history.length > (node.historyLimit as number)) invalidState("history length is invalid");
  if (!Number.isSafeInteger(node.cursor) || (node.cursor as number) < 0 || (node.cursor as number) >= node.history.length) invalidState("history cursor is invalid");
  const history = node.history.map((project) => validateAuthoringProject(project, limits));
  const projectId = history[0]!.projectId;
  if (history.some((project) => project.projectId !== projectId)) invalidState("history crosses project identities");
  const result: PersistedSession = {
    schemaVersion: 1,
    historyLimit: node.historyLimit as number,
    cursor: node.cursor as number,
    history,
  };
  if (canonicalJson(result) !== text!) invalidState("stored JSON is not canonical");
  validateRevision(stored.revision, "stored");
  return result;
}

function findDocument(project: AuthoringProject, id: string): number {
  const index = project.documents.findIndex((document) => document.id === id);
  if (index < 0) throw new AuthoringError("DOCUMENT_NOT_FOUND", "The selected authoring document does not exist.");
  return index;
}

function applyCommand(project: AuthoringProject, command: AuthoringCommand, limits: AuthoringLimits): AuthoringProject {
  if (command === null || typeof command !== "object") throw new AuthoringError("INVALID_COMMAND", "Authoring commands must be objects.");
  switch (command.type) {
    case "create-document":
      return validateAuthoringProject({
        ...project,
        documents: [...project.documents, command.document],
        activeDocumentId: command.activate ? command.document.id : project.activeDocumentId,
      }, limits);
    case "remove-document": {
      const index = findDocument(project, command.documentId);
      return validateAuthoringProject({
        ...project,
        documents: project.documents.filter((_, itemIndex) => itemIndex !== index),
        activeDocumentId: project.activeDocumentId === command.documentId ? null : project.activeDocumentId,
      }, limits);
    }
    case "update-document-metadata": {
      const index = findDocument(project, command.documentId);
      return validateAuthoringProject({
        ...project,
        documents: project.documents.map((document, itemIndex) => itemIndex === index
          ? { ...document, title: command.title, slug: command.slug, status: command.status }
          : document),
      }, limits);
    }
    case "replace-document-body": {
      const index = findDocument(project, command.documentId);
      return validateAuthoringProject({
        ...project,
        documents: project.documents.map((document, itemIndex) => itemIndex === index
          ? { ...document, body: command.body }
          : document),
      }, limits);
    }
    case "configure-site":
      return validateAuthoringProject({
        ...project,
        title: command.title,
        site: { baseUrl: command.baseUrl, themeId: command.themeId },
      }, limits);
    case "set-active-document":
      return validateAuthoringProject({ ...project, activeDocumentId: command.documentId }, limits);
    default:
      throw new AuthoringError("INVALID_COMMAND", "The authoring command type is not supported.");
  }
}

class Session implements AuthoringSession {
  #state: PersistedSession;
  #revision: string;
  readonly #storage: AuthoringStorage;
  readonly #limits: AuthoringLimits;
  #tail: Promise<void> = Promise.resolve();

  constructor(storage: AuthoringStorage, state: PersistedSession, revision: string, limits: AuthoringLimits) {
    this.#storage = storage;
    this.#state = state;
    this.#revision = revision;
    this.#limits = limits;
  }

  get project(): AuthoringProject { return this.#state.history[this.#state.cursor]!; }
  get canUndo(): boolean { return this.#state.cursor > 0; }
  get canRedo(): boolean { return this.#state.cursor + 1 < this.#state.history.length; }
  get storageRevision(): string { return this.#revision; }

  dispatch(command: AuthoringCommand, signal?: AbortSignal): Promise<void> {
    return this.#enqueue(async () => {
      abortIfNeeded(signal);
      const project = applyCommand(this.project, command, this.#limits);
      const prefix = [...this.#state.history.slice(0, this.#state.cursor + 1), project];
      const history = prefix.length > this.#state.historyLimit
        ? prefix.slice(prefix.length - this.#state.historyLimit)
        : prefix;
      await this.#persist({ ...this.#state, history, cursor: history.length - 1 }, signal);
    });
  }

  undo(signal?: AbortSignal): Promise<void> {
    return this.#enqueue(async () => {
      abortIfNeeded(signal);
      if (!this.canUndo) throw new AuthoringError("NO_UNDO", "There is no authoring change to undo.");
      await this.#persist({ ...this.#state, cursor: this.#state.cursor - 1 }, signal);
    });
  }

  redo(signal?: AbortSignal): Promise<void> {
    return this.#enqueue(async () => {
      abortIfNeeded(signal);
      if (!this.canRedo) throw new AuthoringError("NO_REDO", "There is no authoring change to redo.");
      await this.#persist({ ...this.#state, cursor: this.#state.cursor + 1 }, signal);
    });
  }

  #enqueue(operation: () => Promise<void>): Promise<void> {
    const result = this.#tail.then(operation);
    this.#tail = result.catch(() => undefined);
    return result;
  }

  async #persist(next: PersistedSession, signal?: AbortSignal): Promise<void> {
    const bytes = encodeCanonicalJson(next);
    if (bytes.byteLength > this.#limits.maxJsonBytes) invalidState("session history exceeds the canonical byte limit");
    const saved = await this.#storage.compareAndSwap(this.#revision, bytes, signal);
    const revision = validateRevision(saved?.revision, "adapter");
    this.#state = next;
    this.#revision = revision;
  }
}

export async function openAuthoringSession(options: OpenAuthoringSessionOptions): Promise<AuthoringSession> {
  if (options === null || typeof options !== "object") throw new AuthoringError("INVALID_STATE", "Session options must be an object.");
  abortIfNeeded(options.signal);
  const limits = resolveLimits(options.limits);
  const loaded = await options.storage.load(options.signal);
  if (loaded !== null) {
    const state = decodeStored(loaded, limits);
    return new Session(options.storage, state, validateRevision(loaded.revision, "stored"), limits);
  }
  if (options.initialProject === undefined) {
    throw new AuthoringError("MISSING_INITIAL_PROJECT", "A missing authoring store requires an explicit initial project.");
  }
  const historyLimit = options.historyLimit ?? 100;
  if (!Number.isSafeInteger(historyLimit) || historyLimit < 1 || historyLimit > limits.maxHistoryEntries) {
    throw new AuthoringError("INVALID_LIMIT", "The history limit must be a positive safe integer within the authoring limit.");
  }
  const project = validateAuthoringProject(options.initialProject, limits);
  const state: PersistedSession = { schemaVersion: 1, historyLimit, cursor: 0, history: [project] };
  const bytes = encodeCanonicalJson(state);
  if (bytes.byteLength > limits.maxJsonBytes) invalidState("initial session exceeds the canonical byte limit");
  const saved = await options.storage.compareAndSwap(null, bytes, options.signal);
  return new Session(options.storage, state, validateRevision(saved?.revision, "adapter"), limits);
}
