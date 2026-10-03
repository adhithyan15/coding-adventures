/** Atomic authoring transactions with persistent bounded undo and redo. */

import { canonicalJson, canonicalJsonByteLength, encodeCanonicalJson } from "./canonical.js";
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
  if (value === null || typeof value !== "object") invalidState("expected a plain object");
  let prototype: object | null;
  let descriptors: PropertyDescriptorMap;
  try {
    prototype = Object.getPrototypeOf(value);
    descriptors = Object.getOwnPropertyDescriptors(value);
  } catch {
    invalidState("object cannot be inspected safely");
  }
  if (prototype! !== Object.prototype) invalidState("expected a plain object");
  if (Reflect.ownKeys(descriptors!).some((key) => typeof key !== "string")) invalidState("stored object contains a symbol field");
  const actual = Object.keys(descriptors!).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) invalidState("stored object has missing or unknown fields");
  if (Object.values(descriptors!).some((item) => !("value" in item) || !item.enumerable)) invalidState("stored object contains an accessor or hidden field");
  return Object.fromEntries(Object.entries(descriptors!).map(([key, descriptor]) => [key, descriptor.value]));
}

function decodeStored(stored: StoredAuthoringState, limits: AuthoringLimits): { readonly state: PersistedSession; readonly revision: string } {
  const storedFields = exactObject(stored, ["bytes", "revision"]);
  if (!(storedFields.bytes instanceof Uint8Array)) invalidState("stored bytes are not a Uint8Array");
  if (storedFields.bytes.byteLength > limits.maxJsonBytes) invalidState("stored bytes exceed the hard byte limit");
  let text: string;
  try { text = new TextDecoder("utf-8", { fatal: true }).decode(storedFields.bytes); } catch { invalidState("stored bytes are not valid UTF-8"); }
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
  return { state: result, revision: validateRevision(storedFields.revision, "stored") };
}

function snapshotStorage(value: unknown): AuthoringStorage {
  if (value === null || typeof value !== "object") invalidState("storage adapter must be an object");
  const method = (name: "load" | "compareAndSwap"): Function => {
    let cursor: object | null = value;
    for (let depth = 0; cursor !== null && depth < 16; depth += 1) {
      let descriptor: PropertyDescriptor | undefined;
      try { descriptor = Object.getOwnPropertyDescriptor(cursor, name); } catch { invalidState("storage adapter cannot be inspected safely"); }
      if (descriptor !== undefined) {
        if (!("value" in descriptor) || typeof descriptor.value !== "function") invalidState(`storage adapter ${name} must be a data method`);
        return descriptor.value;
      }
      try { cursor = Object.getPrototypeOf(cursor); } catch { invalidState("storage adapter cannot be inspected safely"); }
    }
    invalidState(`storage adapter is missing ${name}`);
  };
  const load = method("load") as AuthoringStorage["load"];
  const compareAndSwap = method("compareAndSwap") as AuthoringStorage["compareAndSwap"];
  return {
    load: load.bind(value),
    compareAndSwap: compareAndSwap.bind(value),
  };
}

function findDocument(project: AuthoringProject, id: string): number {
  const index = project.documents.findIndex((document) => document.id === id);
  if (index < 0) throw new AuthoringError("DOCUMENT_NOT_FOUND", "The selected authoring document does not exist.");
  return index;
}

function validateCommand(value: unknown): AuthoringCommand {
  if (value === null || typeof value !== "object") {
    throw new AuthoringError("INVALID_COMMAND", "Authoring commands must be plain objects.");
  }
  let prototype: object | null;
  try { prototype = Object.getPrototypeOf(value); } catch {
    throw new AuthoringError("INVALID_COMMAND", "The authoring command cannot be inspected safely.");
  }
  if (prototype !== Object.prototype) throw new AuthoringError("INVALID_COMMAND", "Authoring commands must be plain objects.");
  let descriptors: PropertyDescriptorMap;
  try { descriptors = Object.getOwnPropertyDescriptors(value); } catch {
    throw new AuthoringError("INVALID_COMMAND", "The authoring command cannot be inspected safely.");
  }
  if (Reflect.ownKeys(descriptors).some((key) => typeof key !== "string")) {
    throw new AuthoringError("INVALID_COMMAND", "The authoring command contains a symbol field.");
  }
  const typeDescriptor = descriptors.type;
  if (typeDescriptor === undefined || !("value" in typeDescriptor) || typeof typeDescriptor.value !== "string") {
    throw new AuthoringError("INVALID_COMMAND", "The authoring command type must be a data field.");
  }
  let expected: readonly string[];
  switch (typeDescriptor.value) {
    case "create-document": expected = ["type", "document", "activate"]; break;
    case "remove-document": expected = ["type", "documentId"]; break;
    case "update-document-metadata": expected = ["type", "documentId", "title", "slug", "status"]; break;
    case "replace-document-body": expected = ["type", "documentId", "body"]; break;
    case "configure-site": expected = ["type", "title", "baseUrl", "themeId"]; break;
    case "set-active-document": expected = ["type", "documentId"]; break;
    default: throw new AuthoringError("INVALID_COMMAND", "The authoring command type is not supported.");
  }
  const actual = Object.keys(descriptors).sort();
  const sortedExpected = [...expected].sort();
  if (actual.length !== sortedExpected.length || actual.some((key, index) => key !== sortedExpected[index])) {
    throw new AuthoringError("INVALID_COMMAND", "The authoring command has missing or unknown fields.");
  }
  if (Object.values(descriptors).some((item) => !("value" in item) || !item.enumerable)) {
    throw new AuthoringError("INVALID_COMMAND", "The authoring command contains an accessor or hidden field.");
  }
  const snapshot = Object.fromEntries(Object.entries(descriptors).map(([key, descriptor]) => [key, descriptor.value])) as unknown as AuthoringCommand;
  if (snapshot.type === "create-document" && typeof snapshot.activate !== "boolean") {
    throw new AuthoringError("INVALID_COMMAND", "The create-document activate field must be a boolean.");
  }
  return snapshot;
}

function adapterRevision(value: unknown): string {
  let fields: Record<string, unknown>;
  try { fields = exactObject(value, ["revision"]); } catch {
    throw new AuthoringError(
      "STORAGE_INDETERMINATE",
      "The storage adapter returned an invalid result after the commit point; reload before retrying.",
    );
  }
  try { return validateRevision(fields.revision, "adapter"); } catch {
    throw new AuthoringError(
      "STORAGE_INDETERMINATE",
      "The storage adapter returned an invalid revision after the commit point; reload before retrying.",
    );
  }
}

function applyCommand(project: AuthoringProject, command: AuthoringCommand, limits: AuthoringLimits): AuthoringProject {
  const checked = validateCommand(command);
  switch (checked.type) {
    case "create-document":
      return validateAuthoringProject({
        ...project,
        documents: [...project.documents, checked.document],
        activeDocumentId: checked.activate ? checked.document.id : project.activeDocumentId,
      }, limits);
    case "remove-document": {
      const index = findDocument(project, checked.documentId);
      return validateAuthoringProject({
        ...project,
        documents: project.documents.filter((_, itemIndex) => itemIndex !== index),
        activeDocumentId: project.activeDocumentId === checked.documentId ? null : project.activeDocumentId,
      }, limits);
    }
    case "update-document-metadata": {
      const index = findDocument(project, checked.documentId);
      return validateAuthoringProject({
        ...project,
        documents: project.documents.map((document, itemIndex) => itemIndex === index
          ? { ...document, title: checked.title, slug: checked.slug, status: checked.status }
          : document),
      }, limits);
    }
    case "replace-document-body": {
      const index = findDocument(project, checked.documentId);
      return validateAuthoringProject({
        ...project,
        documents: project.documents.map((document, itemIndex) => itemIndex === index
          ? { ...document, body: checked.body }
          : document),
      }, limits);
    }
    case "configure-site":
      return validateAuthoringProject({
        ...project,
        title: checked.title,
        site: { baseUrl: checked.baseUrl, themeId: checked.themeId },
      }, limits);
    case "set-active-document":
      return validateAuthoringProject({ ...project, activeDocumentId: checked.documentId }, limits);
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
    canonicalJsonByteLength(next, this.#limits.maxJsonBytes);
    const bytes = encodeCanonicalJson(next);
    const saved = await this.#storage.compareAndSwap(this.#revision, bytes, signal);
    const revision = adapterRevision(saved);
    this.#state = next;
    this.#revision = revision;
  }
}

export async function openAuthoringSession(options: OpenAuthoringSessionOptions): Promise<AuthoringSession> {
  if (options === null || typeof options !== "object") throw new AuthoringError("INVALID_STATE", "Session options must be an object.");
  let optionDescriptors: PropertyDescriptorMap;
  try { optionDescriptors = Object.getOwnPropertyDescriptors(options); } catch { invalidState("session options cannot be inspected safely"); }
  if (Object.values(optionDescriptors).some((item) => !("value" in item) || !item.enumerable)) invalidState("session options contain an accessor or hidden field");
  if (Reflect.ownKeys(optionDescriptors).some((key) => typeof key !== "string")) invalidState("session options contain a symbol field");
  const optionKeys = Object.keys(optionDescriptors);
  if (!optionKeys.includes("storage") || optionKeys.some((key) => !["storage", "initialProject", "historyLimit", "limits", "signal"].includes(key))) invalidState("session options have missing or unknown fields");
  const checked = Object.fromEntries(Object.entries(optionDescriptors).map(([key, descriptor]) => [key, descriptor.value])) as unknown as OpenAuthoringSessionOptions;
  abortIfNeeded(checked.signal);
  const limits = resolveLimits(checked.limits);
  const storage = snapshotStorage(checked.storage);
  let loaded: StoredAuthoringState | null | undefined;
  try { loaded = await storage.load(checked.signal); } catch {
    invalidState("storage adapter load failed");
  }
  if (loaded === undefined || (loaded !== null && typeof loaded !== "object")) {
    invalidState("storage adapter returned an invalid loaded state");
  }
  if (loaded !== null) {
    const decoded = decodeStored(loaded, limits);
    return new Session(storage, decoded.state, decoded.revision, limits);
  }
  if (checked.initialProject === undefined) {
    throw new AuthoringError("MISSING_INITIAL_PROJECT", "A missing authoring store requires an explicit initial project.");
  }
  const historyLimit = checked.historyLimit ?? 100;
  if (!Number.isSafeInteger(historyLimit) || historyLimit < 1 || historyLimit > limits.maxHistoryEntries) {
    throw new AuthoringError("INVALID_LIMIT", "The history limit must be a positive safe integer within the authoring limit.");
  }
  const project = validateAuthoringProject(checked.initialProject, limits);
  const state: PersistedSession = { schemaVersion: 1, historyLimit, cursor: 0, history: [project] };
  canonicalJsonByteLength(state, limits.maxJsonBytes);
  const bytes = encodeCanonicalJson(state);
  const saved = await storage.compareAndSwap(null, bytes, checked.signal);
  return new Session(storage, state, adapterRevision(saved), limits);
}
